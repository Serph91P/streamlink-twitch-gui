use std::sync::Arc;

use async_trait::async_trait;
use tauri::State;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::domain::stream::{TwitchLoginChallenge, TwitchSession, TwitchUser};
use crate::twitch::auth::{
    AuthClient, AuthError, ConfigError, PollResult, ReqwestTransport, TwitchConfig,
    ValidatedIdentity, ValidationReason,
};
use crate::twitch::client::{HelixClient, StoredTokenProvider, TokioSleeper};
use crate::twitch::lifecycle::{LifecycleError, SessionLifecycle, ValidationRunner};
use crate::twitch::login::{LoginRegistry, clear_cancelled_login};
use crate::twitch::models::{FollowedChannel, Game, SearchChannel, Stream, User};
use crate::twitch::pagination::Page;
use crate::twitch::token_store::OsTokenStore;

type DesktopAuth = AuthClient<ReqwestTransport, OsTokenStore>;
type DesktopHelix = HelixClient<
    ReqwestTransport,
    StoredTokenProvider<ReqwestTransport, OsTokenStore>,
    TokioSleeper,
>;

#[derive(Clone)]
pub(crate) struct DesktopLifecycle {
    auth: DesktopAuth,
    helix: DesktopHelix,
}

pub struct TwitchState {
    auth: DesktopAuth,
    helix: DesktopHelix,
    login: Mutex<LoginRegistry>,
    session: Arc<Mutex<TwitchSession>>,
    validation: ValidationRunner<DesktopLifecycle>,
    credential_gate: Arc<Mutex<()>>,
}

impl TwitchState {
    pub fn new(client_id: &str) -> Result<Self, ConfigError> {
        let config = TwitchConfig::new(client_id)?;
        let transport = ReqwestTransport::default();
        let store = OsTokenStore::new("io.github.streamlink.twitch-gui", "twitch-oauth");
        let auth = AuthClient::new(
            config.client_id(),
            ["user:read:follows"],
            transport.clone(),
            store,
        );
        let helix = HelixClient::new(
            config.client_id(),
            transport,
            StoredTokenProvider::new(auth.clone()),
            TokioSleeper,
        );
        let session = Arc::new(Mutex::new(TwitchSession::Anonymous));
        let validation = ValidationRunner::new(
            DesktopLifecycle {
                auth: auth.clone(),
                helix: helix.clone(),
            },
            session.clone(),
        );
        Ok(Self {
            auth,
            helix,
            login: Mutex::new(LoginRegistry::default()),
            session,
            validation,
            credential_gate: Arc::new(Mutex::new(())),
        })
    }

    pub(crate) fn validation_schedule(
        &self,
    ) -> (ValidationRunner<DesktopLifecycle>, Arc<Mutex<()>>) {
        (self.validation.clone(), self.credential_gate.clone())
    }
}

fn safe_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn get_twitch_session(state: State<'_, TwitchState>) -> Result<TwitchSession, String> {
    Ok(state.validation.session_after_startup().await)
}

#[tauri::command]
pub async fn begin_twitch_login(
    state: State<'_, TwitchState>,
    attempt_id: String,
) -> Result<TwitchLoginChallenge, String> {
    let attempt = state.auth.begin_device_login().await.map_err(safe_error)?;
    let challenge = attempt.challenge.clone();
    state
        .login
        .lock()
        .await
        .install(attempt_id, attempt.device_code())
        .map_err(safe_error)?;
    Ok(challenge)
}

#[tauri::command]
pub async fn poll_twitch_login(
    state: State<'_, TwitchState>,
    attempt_id: String,
) -> Result<TwitchSession, String> {
    let (device_code, cancellation) = state
        .login
        .lock()
        .await
        .poll(&attempt_id)
        .map_err(safe_error)?;
    let _credential_guard = state.credential_gate.lock().await;
    let result = state
        .auth
        .poll_device_login_cancellable(&device_code, &cancellation)
        .await;
    match result {
        Ok(PollResult::Pending) => return Ok(state.session.lock().await.clone()),
        Ok(PollResult::Complete) => {}
        Err(error) => {
            state.login.lock().await.complete(&attempt_id);
            if matches!(error, AuthError::Cancelled) {
                clear_cancelled_login(&state.auth, state.session.as_ref())
                    .await
                    .map_err(safe_error)?;
            }
            return Err(safe_error(error));
        }
    }
    if cancellation.is_cancelled() {
        clear_cancelled_login(&state.auth, state.session.as_ref())
            .await
            .map_err(safe_error)?;
        return Err(safe_error(AuthError::Cancelled));
    }
    state
        .validation
        .run_once(ValidationReason::Startup)
        .await
        .map_err(safe_error)?;
    if cancellation.is_cancelled() || !state.login.lock().await.complete(&attempt_id) {
        clear_cancelled_login(&state.auth, state.session.as_ref())
            .await
            .map_err(safe_error)?;
        return Err(safe_error(AuthError::Cancelled));
    }
    Ok(state.session.lock().await.clone())
}

#[tauri::command]
pub async fn cancel_twitch_login(
    state: State<'_, TwitchState>,
    attempt_id: String,
) -> Result<(), String> {
    if !state.login.lock().await.cancel(&attempt_id) {
        return Ok(());
    }
    let _credential_guard = state.credential_gate.lock().await;
    clear_cancelled_login(&state.auth, state.session.as_ref())
        .await
        .map_err(safe_error)
}

#[tauri::command]
pub async fn sign_out_twitch(state: State<'_, TwitchState>) -> Result<(), String> {
    state.login.lock().await.cancel_active();
    let _credential_guard = state.credential_gate.lock().await;
    clear_cancelled_login(&state.auth, state.session.as_ref())
        .await
        .map_err(safe_error)
}

async fn load_session(
    helix: &DesktopHelix,
    identity: ValidatedIdentity,
) -> Result<TwitchSession, LifecycleError> {
    let page = helix
        .users(&[], &CancellationToken::new())
        .await
        .map_err(|_| LifecycleError::ProfileUnavailable)?;
    let user = page
        .items
        .into_iter()
        .find(|user| user.id == identity.user_id)
        .ok_or(LifecycleError::ProfileUnavailable)?;
    let expires_at = (OffsetDateTime::now_utc()
        + Duration::seconds(i64::try_from(identity.expires_in_seconds).unwrap_or(i64::MAX)))
    .format(&Rfc3339)
    .map_err(|_| LifecycleError::ValidationFailed)?;
    Ok(TwitchSession::Authenticated {
        user: TwitchUser {
            id: user.id,
            login: user.login,
            display_name: user.display_name,
            profile_image_url: user.profile_image_url,
        },
        expires_at,
    })
}

#[async_trait]
impl SessionLifecycle for DesktopLifecycle {
    async fn validate(
        &self,
        reason: ValidationReason,
    ) -> Result<Option<TwitchSession>, LifecycleError> {
        let identity = self.auth.validate_session(reason).await.map_err(|error| {
            if matches!(error, AuthError::InvalidCredentials) {
                LifecycleError::InvalidCredentials
            } else {
                LifecycleError::ValidationFailed
            }
        })?;
        match identity {
            Some(identity) => load_session(&self.helix, identity).await.map(Some),
            None => Ok(None),
        }
    }

    async fn clear_credentials(&self) -> Result<(), LifecycleError> {
        self.auth
            .sign_out()
            .await
            .map_err(|_| LifecycleError::CleanupFailed)
    }
}

pub async fn run_validation_schedule<B: SessionLifecycle>(
    runner: ValidationRunner<B>,
    credential_gate: Arc<Mutex<()>>,
) {
    {
        let _credential_guard = credential_gate.lock().await;
        if let Err(error) = runner.run_once(ValidationReason::Startup).await {
            eprintln!("{error}");
        }
    }
    let mut interval = tokio::time::interval_at(
        tokio::time::Instant::now() + std::time::Duration::from_secs(60 * 60),
        std::time::Duration::from_secs(60 * 60),
    );
    loop {
        interval.tick().await;
        let _credential_guard = credential_gate.lock().await;
        if let Err(error) = runner.run_once(ValidationReason::Hourly).await {
            eprintln!("{error}");
        }
    }
}

fn cancellation() -> CancellationToken {
    CancellationToken::new()
}

#[tauri::command]
pub async fn twitch_users(
    state: State<'_, TwitchState>,
    logins: Vec<String>,
) -> Result<Page<User>, String> {
    let logins: Vec<&str> = logins.iter().map(String::as_str).collect();
    state
        .helix
        .users(&logins, &cancellation())
        .await
        .map_err(safe_error)
}

#[tauri::command]
pub async fn twitch_streams(
    state: State<'_, TwitchState>,
    user_id: Option<String>,
    cursor: Option<String>,
) -> Result<Page<Stream>, String> {
    state
        .helix
        .streams(user_id.as_deref(), cursor.as_deref(), &cancellation())
        .await
        .map_err(safe_error)
}

#[tauri::command]
pub async fn twitch_followed_streams(
    state: State<'_, TwitchState>,
    user_id: String,
    cursor: Option<String>,
) -> Result<Page<Stream>, String> {
    state
        .helix
        .followed_streams(&user_id, cursor.as_deref(), &cancellation())
        .await
        .map_err(safe_error)
}

#[tauri::command]
pub async fn twitch_followed_channels(
    state: State<'_, TwitchState>,
    user_id: String,
    cursor: Option<String>,
) -> Result<Page<FollowedChannel>, String> {
    state
        .helix
        .followed_channels(&user_id, cursor.as_deref(), &cancellation())
        .await
        .map_err(safe_error)
}

#[tauri::command]
pub async fn twitch_top_games(
    state: State<'_, TwitchState>,
    cursor: Option<String>,
) -> Result<Page<Game>, String> {
    state
        .helix
        .top_games(cursor.as_deref(), &cancellation())
        .await
        .map_err(safe_error)
}

#[tauri::command]
pub async fn twitch_search_channels(
    state: State<'_, TwitchState>,
    query: String,
    cursor: Option<String>,
) -> Result<Page<SearchChannel>, String> {
    state
        .helix
        .search_channels(&query, cursor.as_deref(), &cancellation())
        .await
        .map_err(safe_error)
}

#[tauri::command]
pub async fn twitch_search_categories(
    state: State<'_, TwitchState>,
    query: String,
    cursor: Option<String>,
) -> Result<Page<Game>, String> {
    state
        .helix
        .search_categories(&query, cursor.as_deref(), &cancellation())
        .await
        .map_err(safe_error)
}
