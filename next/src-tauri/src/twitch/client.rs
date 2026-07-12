use std::fmt;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::twitch::auth::{
    AuthClient, AuthError, HttpMethod, HttpRequest, HttpResponse, HttpTransport, ValidationReason,
};
use crate::twitch::models::{FollowedChannel, Game, SearchChannel, Stream, User};
use crate::twitch::pagination::{Page, RateLimit, WirePage};

const HELIX_URL: &str = "https://api.twitch.tv/helix";
const MAX_RETRIES: usize = 2;

pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccessToken([REDACTED])")
    }
}

#[async_trait]
pub trait TokenProvider: Clone + Send + Sync + 'static {
    async fn access_token(&self) -> Result<AccessToken, HelixError>;
    async fn recover_unauthorized(&self) -> Result<(), HelixError>;
}

#[derive(Clone)]
pub struct StaticTokenProvider(ArcToken);

#[derive(Clone)]
struct ArcToken(std::sync::Arc<String>);

impl StaticTokenProvider {
    pub fn new(token: impl Into<String>) -> Self {
        Self(ArcToken(std::sync::Arc::new(token.into())))
    }
}

#[async_trait]
impl TokenProvider for StaticTokenProvider {
    async fn access_token(&self) -> Result<AccessToken, HelixError> {
        Ok(AccessToken::new(self.0.0.as_str()))
    }

    async fn recover_unauthorized(&self) -> Result<(), HelixError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct StoredTokenProvider<T, S> {
    auth: AuthClient<T, S>,
}

impl<T, S> StoredTokenProvider<T, S> {
    pub fn new(auth: AuthClient<T, S>) -> Self {
        Self { auth }
    }
}

#[async_trait]
impl<T, S> TokenProvider for StoredTokenProvider<T, S>
where
    T: HttpTransport,
    S: crate::twitch::token_store::TokenStore,
{
    async fn access_token(&self) -> Result<AccessToken, HelixError> {
        self.auth
            .stored_access_token()
            .await
            .map_err(|_| HelixError::Authentication)?
            .map(AccessToken::new)
            .ok_or(HelixError::Authentication)
    }

    async fn recover_unauthorized(&self) -> Result<(), HelixError> {
        self.auth
            .validate_session(ValidationReason::Unauthorized)
            .await
            .map_err(|_| HelixError::Authentication)?
            .ok_or(HelixError::Authentication)?;
        Ok(())
    }
}

#[async_trait]
pub trait RetrySleeper: Clone + Send + Sync + 'static {
    async fn sleep_ms(&self, milliseconds: u64);
}

#[derive(Clone, Copy, Default)]
pub struct TokioSleeper;

#[async_trait]
impl RetrySleeper for TokioSleeper {
    async fn sleep_ms(&self, milliseconds: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(milliseconds)).await;
    }
}

#[derive(Debug, Error)]
pub enum HelixError {
    #[error("Helix request was cancelled")]
    Cancelled,
    #[error("Helix returned HTTP {0}")]
    HttpStatus(u16),
    #[error("Helix response was invalid")]
    InvalidResponse,
    #[error("Helix authentication failed")]
    Authentication,
    #[error("Helix transport failed")]
    Transport,
}

#[derive(Clone)]
pub struct HelixClient<T, P, R> {
    client_id: String,
    transport: T,
    token_provider: P,
    sleeper: R,
}

impl<T: HttpTransport, P: TokenProvider, R: RetrySleeper> HelixClient<T, P, R> {
    pub fn new(client_id: impl Into<String>, transport: T, token_provider: P, sleeper: R) -> Self {
        Self {
            client_id: client_id.into(),
            transport,
            token_provider,
            sleeper,
        }
    }

    pub async fn users(
        &self,
        logins: &[&str],
        cancellation: &CancellationToken,
    ) -> Result<Page<User>, HelixError> {
        self.get("users", repeated("login", logins), cancellation)
            .await
    }

    pub async fn streams(
        &self,
        user_id: Option<&str>,
        cursor: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Page<Stream>, HelixError> {
        self.get(
            "streams",
            optional("user_id", user_id, optional("after", cursor, Vec::new())),
            cancellation,
        )
        .await
    }

    pub async fn followed_streams(
        &self,
        user_id: &str,
        cursor: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Page<Stream>, HelixError> {
        self.get(
            "streams/followed",
            optional("after", cursor, vec![("user_id".into(), user_id.into())]),
            cancellation,
        )
        .await
    }

    pub async fn followed_channels(
        &self,
        user_id: &str,
        cursor: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Page<FollowedChannel>, HelixError> {
        self.get(
            "channels/followed",
            optional("after", cursor, vec![("user_id".into(), user_id.into())]),
            cancellation,
        )
        .await
    }

    pub async fn top_games(
        &self,
        cursor: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Page<Game>, HelixError> {
        self.get(
            "games/top",
            optional("after", cursor, Vec::new()),
            cancellation,
        )
        .await
    }

    pub async fn search_channels(
        &self,
        query: &str,
        cursor: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Page<SearchChannel>, HelixError> {
        self.get(
            "search/channels",
            optional("after", cursor, vec![("query".into(), query.into())]),
            cancellation,
        )
        .await
    }

    pub async fn search_categories(
        &self,
        query: &str,
        cursor: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Page<Game>, HelixError> {
        self.get(
            "search/categories",
            optional("after", cursor, vec![("query".into(), query.into())]),
            cancellation,
        )
        .await
    }

    async fn get<D: DeserializeOwned>(
        &self,
        path: &str,
        query: Vec<(String, String)>,
        cancellation: &CancellationToken,
    ) -> Result<Page<D>, HelixError> {
        let mut retries = 0;
        let mut recovered_unauthorized = false;
        loop {
            if cancellation.is_cancelled() {
                return Err(HelixError::Cancelled);
            }
            let token = self.token_provider.access_token().await?;
            let request = self.transport.execute(HttpRequest {
                method: HttpMethod::Get,
                url: format!("{HELIX_URL}/{path}"),
                headers: vec![
                    ("Authorization".into(), format!("Bearer {}", token.expose())),
                    ("Client-Id".into(), self.client_id.clone()),
                ],
                query: query.clone(),
                form: Vec::new(),
            });
            let response = tokio::select! {
                response = request => response.map_err(map_auth_error)?,
                () = cancellation.cancelled() => return Err(HelixError::Cancelled),
            };
            if response.status == 401 && !recovered_unauthorized {
                recovered_unauthorized = true;
                self.token_provider.recover_unauthorized().await?;
                continue;
            }
            if is_retryable(response.status) && retries < MAX_RETRIES {
                let delay = retry_delay_ms(&response, retries);
                retries += 1;
                tokio::select! {
                    () = self.sleeper.sleep_ms(delay) => {},
                    () = cancellation.cancelled() => return Err(HelixError::Cancelled),
                }
                continue;
            }
            return parse_page(response);
        }
    }
}

fn optional(
    name: &str,
    value: Option<&str>,
    mut query: Vec<(String, String)>,
) -> Vec<(String, String)> {
    if let Some(value) = value {
        query.push((name.into(), value.into()));
    }
    query
}

fn repeated(name: &str, values: &[&str]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|value| (name.into(), (*value).into()))
        .collect()
}

fn is_retryable(status: u16) -> bool {
    status == 429 || (500..=599).contains(&status)
}

fn retry_delay_ms(response: &HttpResponse, retry: usize) -> u64 {
    response
        .header("Retry-After")
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1_000).min(30_000))
        .unwrap_or(250_u64 * (1 << retry))
}

fn map_auth_error(error: AuthError) -> HelixError {
    match error {
        AuthError::Transport => HelixError::Transport,
        _ => HelixError::Authentication,
    }
}

fn parse_page<D: DeserializeOwned>(response: HttpResponse) -> Result<Page<D>, HelixError> {
    if response.status != 200 {
        return Err(HelixError::HttpStatus(response.status));
    }
    let rate_limit = RateLimit {
        limit: header_number(&response, "Ratelimit-Limit"),
        remaining: header_number(&response, "Ratelimit-Remaining"),
        reset_at_epoch_seconds: header_number(&response, "Ratelimit-Reset"),
    };
    let payload: WirePage<D> =
        serde_json::from_str(&response.body).map_err(|_| HelixError::InvalidResponse)?;
    Ok(Page {
        items: payload.data,
        next_cursor: payload.pagination.cursor,
        rate_limit: Some(rate_limit),
    })
}

fn header_number<N: std::str::FromStr>(response: &HttpResponse, name: &str) -> Option<N> {
    response.header(name)?.parse().ok()
}
