use async_trait::async_trait;
use streamlink_twitch_gui_lib::{
    domain::stream::{TwitchSession, TwitchUser},
    twitch::{
        auth::{AuthClient, AuthError, HttpRequest, HttpResponse, HttpTransport},
        login::{LoginRegistry, clear_cancelled_login},
        token_store::{MemoryTokenStore, TokenStore},
    },
};
use tokio::sync::Mutex;

#[derive(Clone)]
struct NoHttp;

#[async_trait]
impl HttpTransport for NoHttp {
    async fn execute(&self, _request: HttpRequest) -> Result<HttpResponse, AuthError> {
        unreachable!("cancellation cleanup must not make an HTTP request")
    }
}

#[test]
fn cancellation_is_idempotent_and_prevents_late_attempt_installation() {
    let mut registry = LoginRegistry::default();

    assert!(!registry.cancel("attempt-before-begin"));
    assert!(!registry.cancel("attempt-before-begin"));
    assert!(matches!(
        registry.install("attempt-before-begin", "device-code"),
        Err(AuthError::Cancelled)
    ));

    let cancellation = registry
        .install("active-attempt", "active-device-code")
        .unwrap();
    assert!(registry.cancel("active-attempt"));
    assert!(cancellation.is_cancelled());
    assert!(!registry.cancel("active-attempt"));
}

#[tokio::test]
async fn cancelled_login_cannot_leave_credentials_or_restore_a_session() {
    let store = MemoryTokenStore::with_tokens("cancelled-access", "cancelled-refresh");
    let auth = AuthClient::new(
        "public-client",
        ["user:read:follows"],
        NoHttp,
        store.clone(),
    );
    let session = Mutex::new(TwitchSession::Authenticated {
        user: TwitchUser {
            id: "42".into(),
            login: "viewer".into(),
            display_name: "Viewer".into(),
            profile_image_url: String::new(),
        },
        expires_at: "2030-01-01T00:00:00Z".into(),
    });

    clear_cancelled_login(&auth, &session).await.unwrap();
    clear_cancelled_login(&auth, &session).await.unwrap();

    assert!(store.load().await.unwrap().is_none());
    assert_eq!(*session.lock().await, TwitchSession::Anonymous);
}
