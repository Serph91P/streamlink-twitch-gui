use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;
use streamlink_twitch_gui_lib::twitch::auth::{
    AuthClient, AuthError, HttpRequest, HttpResponse, HttpTransport, PollResult, TwitchConfig,
    ValidationReason,
};
use streamlink_twitch_gui_lib::twitch::token_store::{MemoryTokenStore, TokenStore};

#[derive(Clone, Default)]
struct FakeHttp {
    requests: Arc<Mutex<Vec<HttpRequest>>>,
    responses: Arc<Mutex<VecDeque<HttpResponse>>>,
}

impl FakeHttp {
    fn with_responses(responses: Vec<HttpResponse>) -> Self {
        Self {
            requests: Arc::default(),
            responses: Arc::new(Mutex::new(responses.into())),
        }
    }
}

#[async_trait]
impl HttpTransport for FakeHttp {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, AuthError> {
        self.requests.lock().unwrap().push(request);
        Ok(self.responses.lock().unwrap().pop_front().unwrap())
    }
}

fn response(status: u16, body: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: Vec::new(),
        body: body.to_string(),
    }
}

fn client(http: FakeHttp, store: MemoryTokenStore) -> AuthClient<FakeHttp, MemoryTokenStore> {
    AuthClient::new("public-client", ["user:read:follows"], http, store)
}

#[tokio::test]
async fn requests_an_official_public_client_device_challenge() {
    let http = FakeHttp::with_responses(vec![response(
        200,
        json!({
            "device_code": "secret-device-code",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://www.twitch.tv/activate",
            "expires_in": 600,
            "interval": 5
        }),
    )]);
    let requests = http.requests.clone();

    let attempt = client(http, MemoryTokenStore::default())
        .begin_device_login()
        .await
        .unwrap();

    assert_eq!(attempt.challenge.user_code, "ABCD-EFGH");
    assert!(!format!("{attempt:?}").contains("secret-device-code"));
    let request = &requests.lock().unwrap()[0];
    assert_eq!(request.url, "https://id.twitch.tv/oauth2/device");
    assert_eq!(request.form_value("client_id"), Some("public-client"));
    assert_eq!(request.form_value("scopes"), Some("user:read:follows"));
    assert!(request.form_value("client_secret").is_none());
    assert!(!format!("{request:?}").contains("public-client"));
}

#[tokio::test]
async fn polling_reports_pending_success_expiry_and_denial_without_leaking_secrets() {
    let scenarios = [
        (400, json!({"message": "authorization_pending"}), "pending"),
        (400, json!({"message": "expired_token"}), "expired"),
        (400, json!({"message": "access_denied"}), "denied"),
    ];

    for (status, body, expected) in scenarios {
        let auth = client(
            FakeHttp::with_responses(vec![response(status, body)]),
            MemoryTokenStore::default(),
        );
        let result = auth.poll_device_login("device-code").await;
        match expected {
            "pending" => assert_eq!(result.unwrap(), PollResult::Pending),
            "expired" => assert!(matches!(result, Err(AuthError::Expired))),
            _ => assert!(matches!(result, Err(AuthError::AccessDenied))),
        }
    }

    let store = MemoryTokenStore::default();
    let auth = client(
        FakeHttp::with_responses(vec![response(
            200,
            json!({
                "access_token": "access-secret",
                "refresh_token": "refresh-secret",
                "expires_in": 14400,
                "scope": ["user:read:follows"],
                "token_type": "bearer"
            }),
        )]),
        store.clone(),
    );
    assert_eq!(
        auth.poll_device_login("device-code").await.unwrap(),
        PollResult::Complete
    );
    let credentials = store.load().await.unwrap().unwrap();
    assert_eq!(credentials.access_token(), "access-secret");
    assert!(!format!("{credentials:?}").contains("access-secret"));
}

#[tokio::test]
async fn startup_validation_refreshes_and_atomically_persists_rotated_credentials() {
    let store = MemoryTokenStore::with_tokens("old-access", "old-refresh");
    let http = FakeHttp::with_responses(vec![
        response(
            401,
            json!({"status": 401, "message": "invalid access token"}),
        ),
        response(
            200,
            json!({
                "access_token": "new-access",
                "refresh_token": "new-refresh",
                "expires_in": 14400,
                "scope": ["user:read:follows"],
                "token_type": "bearer"
            }),
        ),
        response(
            200,
            json!({
                "client_id": "public-client",
                "login": "tester",
                "user_id": "42",
                "expires_in": 14400,
                "scopes": ["user:read:follows"]
            }),
        ),
    ]);
    let requests = http.requests.clone();

    let validation = client(http, store.clone())
        .validate_session(ValidationReason::Startup)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(validation.user_id, "42");
    let stored = store.load().await.unwrap().unwrap();
    assert_eq!(stored.access_token(), "new-access");
    assert_eq!(stored.refresh_token(), "new-refresh");
    let requests = requests.lock().unwrap();
    assert_eq!(requests[1].form_value("grant_type"), Some("refresh_token"));
    assert_eq!(requests[1].form_value("refresh_token"), Some("old-refresh"));
    assert!(requests[1].form_value("client_secret").is_none());
}

#[tokio::test]
async fn hourly_validation_is_deterministic_and_sign_out_removes_credentials() {
    assert!(!AuthClient::<FakeHttp, MemoryTokenStore>::validation_due(
        3_599, 0
    ));
    assert!(AuthClient::<FakeHttp, MemoryTokenStore>::validation_due(
        3_600, 0
    ));

    let store = MemoryTokenStore::with_tokens("access", "refresh");
    let auth = client(FakeHttp::default(), store.clone());
    auth.sign_out().await.unwrap();
    assert!(store.load().await.unwrap().is_none());
}

#[tokio::test]
async fn revoked_credentials_are_deleted_during_startup_and_hourly_validation() {
    for reason in [ValidationReason::Startup, ValidationReason::Hourly] {
        let store = MemoryTokenStore::with_tokens("revoked-access", "revoked-refresh");
        let auth = client(
            FakeHttp::with_responses(vec![
                response(401, json!({"message": "invalid access token"})),
                response(400, json!({"message": "invalid refresh token"})),
            ]),
            store.clone(),
        );

        let error = auth.validate_session(reason).await.unwrap_err();

        assert!(matches!(error, AuthError::InvalidCredentials));
        assert!(store.load().await.unwrap().is_none());
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("revoked-access"));
        assert!(!rendered.contains("revoked-refresh"));
    }
}

#[tokio::test]
async fn no_token_validation_is_anonymous_and_token_safe() {
    let result = client(FakeHttp::default(), MemoryTokenStore::default())
        .validate_session(ValidationReason::Startup)
        .await;

    assert_eq!(result.unwrap(), None);
    assert!(!format!("{:?}", AuthError::InvalidCredentials).contains("token"));
}

#[test]
fn empty_client_id_is_rejected_with_a_safe_configuration_error() {
    let error = TwitchConfig::new("   ").unwrap_err();

    assert_eq!(error.to_string(), "Twitch client ID is not configured");
    assert!(!format!("{error:?}").contains("client_secret"));
}
