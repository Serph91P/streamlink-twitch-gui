use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;
use streamlink_twitch_gui_lib::twitch::auth::{
    AuthClient, AuthError, HttpRequest, HttpResponse, HttpTransport,
};
use streamlink_twitch_gui_lib::twitch::client::{
    AccessToken, HelixClient, HelixError, RetrySleeper, StaticTokenProvider, StoredTokenProvider,
    TokenProvider,
};
use streamlink_twitch_gui_lib::twitch::token_store::{MemoryTokenStore, TokenStore};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct FakeHttp {
    requests: Arc<Mutex<Vec<HttpRequest>>>,
    responses: Arc<Mutex<VecDeque<HttpResponse>>>,
}

#[async_trait]
impl HttpTransport for FakeHttp {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, AuthError> {
        self.requests.lock().unwrap().push(request);
        Ok(self.responses.lock().unwrap().pop_front().unwrap())
    }
}

#[derive(Clone, Default)]
struct FakeSleeper(Arc<Mutex<Vec<u64>>>);

type TestClient = HelixClient<FakeHttp, StaticTokenProvider, FakeSleeper>;
type RequestLog = Arc<Mutex<Vec<HttpRequest>>>;

#[async_trait]
impl RetrySleeper for FakeSleeper {
    async fn sleep_ms(&self, milliseconds: u64) {
        self.0.lock().unwrap().push(milliseconds);
    }
}

fn response(status: u16, body: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: vec![
            ("Ratelimit-Limit".into(), "800".into()),
            ("Ratelimit-Remaining".into(), "799".into()),
            ("Ratelimit-Reset".into(), "1700000000".into()),
        ],
        body: body.to_string(),
    }
}

fn client(responses: Vec<HttpResponse>) -> (TestClient, RequestLog, FakeSleeper) {
    let requests: RequestLog = Arc::default();
    let http = FakeHttp {
        requests: requests.clone(),
        responses: Arc::new(Mutex::new(responses.into())),
    };
    let sleeper = FakeSleeper::default();
    (
        HelixClient::new(
            "client-id",
            http,
            StaticTokenProvider::new("token"),
            sleeper.clone(),
        ),
        requests,
        sleeper,
    )
}

#[tokio::test]
async fn normalizes_all_required_helix_resources_and_headers() {
    let fixtures = vec![
        response(
            200,
            json!({"data":[{"id":"1","login":"alice","display_name":"Alice","profile_image_url":"https://example/alice.png"}]}),
        ),
        response(
            200,
            json!({"data":[{"id":"s1","user_id":"1","user_login":"alice","user_name":"Alice","game_id":"g1","game_name":"Chess","title":"Live","viewer_count":12,"started_at":"2026-01-01T00:00:00Z","thumbnail_url":"https://example/{width}x{height}.jpg","is_mature":false}],"pagination":{"cursor":"next"}}),
        ),
        response(200, json!({"data":[],"pagination":{}})),
        response(
            200,
            json!({"data":[{"broadcaster_id":"1","broadcaster_login":"alice","broadcaster_name":"Alice","followed_at":"2026-01-01T00:00:00Z"}],"pagination":{}}),
        ),
        response(
            200,
            json!({"data":[{"id":"g1","name":"Chess","box_art_url":"https://example/{width}x{height}.jpg","igdb_id":"9"}],"pagination":{}}),
        ),
        response(
            200,
            json!({"data":[{"broadcaster_language":"en","broadcaster_login":"alice","display_name":"Alice","game_id":"g1","game_name":"Chess","id":"1","is_live":true,"tags":["calm"],"thumbnail_url":"https://example/thumb.jpg","title":"Live","started_at":"2026-01-01T00:00:00Z"}],"pagination":{}}),
        ),
        response(
            200,
            json!({"data":[{"id":"g1","name":"Chess","box_art_url":"https://example/{width}x{height}.jpg"}],"pagination":{}}),
        ),
    ];
    let (client, requests, _) = client(fixtures);
    let cancel = CancellationToken::new();

    assert_eq!(
        client.users(&["alice"], &cancel).await.unwrap().items[0].display_name,
        "Alice"
    );
    let streams = client.streams(None, None, &cancel).await.unwrap();
    assert_eq!(streams.items[0].viewer_count, 12);
    assert_eq!(streams.next_cursor.as_deref(), Some("next"));
    assert_eq!(streams.rate_limit.unwrap().remaining, Some(799));
    client.followed_streams("1", None, &cancel).await.unwrap();
    assert_eq!(
        client
            .followed_channels("1", None, &cancel)
            .await
            .unwrap()
            .items[0]
            .broadcaster_login,
        "alice"
    );
    assert_eq!(
        client.top_games(None, &cancel).await.unwrap().items[0].name,
        "Chess"
    );
    assert!(
        client
            .search_channels("ali", None, &cancel)
            .await
            .unwrap()
            .items[0]
            .is_live
    );
    assert_eq!(
        client
            .search_categories("che", None, &cancel)
            .await
            .unwrap()
            .items[0]
            .id,
        "g1"
    );

    for request in requests.lock().unwrap().iter() {
        assert_eq!(request.header("Client-Id"), Some("client-id"));
        assert_eq!(request.header("Authorization"), Some("Bearer token"));
    }
}

#[tokio::test]
async fn paginates_retries_only_transient_failures_and_supports_cancellation() {
    let (client, requests, sleeper) = client(vec![
        response(429, json!({"error":"Too Many Requests"})),
        response(503, json!({"error":"Unavailable"})),
        response(200, json!({"data":[],"pagination":{}})),
        response(400, json!({"error":"Bad Request"})),
    ]);
    let cancel = CancellationToken::new();
    client
        .top_games(Some("cursor-value"), &cancel)
        .await
        .unwrap();
    assert_eq!(sleeper.0.lock().unwrap().as_slice(), &[250, 500]);
    assert_eq!(
        requests.lock().unwrap()[0].query_value("after"),
        Some("cursor-value")
    );

    let error = client.top_games(None, &cancel).await.unwrap_err();
    assert!(matches!(error, HelixError::HttpStatus(400)));
    assert_eq!(requests.lock().unwrap().len(), 4);

    cancel.cancel();
    let error = client.top_games(None, &cancel).await.unwrap_err();
    assert!(matches!(error, HelixError::Cancelled));
    assert_eq!(requests.lock().unwrap().len(), 4);
}

#[derive(Clone)]
struct RotatingTokenProvider {
    token: Arc<Mutex<String>>,
    recoveries: Arc<Mutex<usize>>,
}

#[async_trait]
impl TokenProvider for RotatingTokenProvider {
    async fn access_token(&self) -> Result<AccessToken, HelixError> {
        Ok(AccessToken::new(self.token.lock().unwrap().clone()))
    }

    async fn recover_unauthorized(&self) -> Result<(), HelixError> {
        *self.recoveries.lock().unwrap() += 1;
        *self.token.lock().unwrap() = "rotated-token".into();
        Ok(())
    }
}

#[tokio::test]
async fn unauthorized_recovers_once_and_retries_with_the_rotated_token() {
    for final_status in [200, 401] {
        let requests: RequestLog = Arc::default();
        let http = FakeHttp {
            requests: requests.clone(),
            responses: Arc::new(Mutex::new(
                vec![
                    response(401, json!({"error":"Unauthorized"})),
                    response(final_status, json!({"data":[],"pagination":{}})),
                ]
                .into(),
            )),
        };
        let recoveries = Arc::new(Mutex::new(0));
        let provider = RotatingTokenProvider {
            token: Arc::new(Mutex::new("old-token".into())),
            recoveries: recoveries.clone(),
        };
        let client = HelixClient::new("client-id", http, provider, FakeSleeper::default());

        let result = client.top_games(None, &CancellationToken::new()).await;

        assert_eq!(*recoveries.lock().unwrap(), 1);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0].header("Authorization"),
            Some("Bearer old-token")
        );
        assert_eq!(
            requests[1].header("Authorization"),
            Some("Bearer rotated-token")
        );
        if final_status == 401 {
            assert!(matches!(result, Err(HelixError::HttpStatus(401))));
        } else {
            result.unwrap();
        }
    }
}

#[tokio::test]
async fn stored_provider_retries_with_atomically_rotated_credentials() {
    let requests: RequestLog = Arc::default();
    let http = FakeHttp {
        requests: requests.clone(),
        responses: Arc::new(Mutex::new(
            vec![
                response(401, json!({"error":"Unauthorized"})),
                response(401, json!({"message":"invalid access token"})),
                response(
                    200,
                    json!({
                        "access_token":"new-access",
                        "refresh_token":"new-refresh"
                    }),
                ),
                response(
                    200,
                    json!({
                        "client_id":"client-id",
                        "login":"tester",
                        "user_id":"42",
                        "expires_in":3600,
                        "scopes":["user:read:follows"]
                    }),
                ),
                response(200, json!({"data":[],"pagination":{}})),
            ]
            .into(),
        )),
    };
    let store = MemoryTokenStore::with_tokens("old-access", "old-refresh");
    let auth = AuthClient::new(
        "client-id",
        ["user:read:follows"],
        http.clone(),
        store.clone(),
    );
    let client = HelixClient::new(
        "client-id",
        http,
        StoredTokenProvider::new(auth),
        FakeSleeper::default(),
    );

    client
        .top_games(None, &CancellationToken::new())
        .await
        .unwrap();

    let stored = store.load().await.unwrap().unwrap();
    assert_eq!(stored.access_token(), "new-access");
    assert_eq!(stored.refresh_token(), "new-refresh");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 5);
    assert_eq!(
        requests[0].header("Authorization"),
        Some("Bearer old-access")
    );
    assert_eq!(
        requests[4].header("Authorization"),
        Some("Bearer new-access")
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.ends_with("/oauth2/validate"))
            .count(),
        2
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.ends_with("/oauth2/token"))
            .count(),
        1
    );
}

#[derive(Clone)]
struct BlockingHttp {
    started: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl HttpTransport for BlockingHttp {
    async fn execute(&self, _request: HttpRequest) -> Result<HttpResponse, AuthError> {
        self.started.notify_one();
        self.release.notified().await;
        Ok(response(200, json!({"data":[],"pagination":{}})))
    }
}

#[tokio::test]
async fn cancellation_interrupts_an_in_flight_transport_request() {
    let started = Arc::new(Notify::new());
    let client = HelixClient::new(
        "client-id",
        BlockingHttp {
            started: started.clone(),
            release: Arc::new(Notify::new()),
        },
        StaticTokenProvider::new("token"),
        FakeSleeper::default(),
    );
    let cancel = CancellationToken::new();
    let request = client.top_games(None, &cancel);
    tokio::pin!(request);

    tokio::select! {
        _ = started.notified() => cancel.cancel(),
        result = &mut request => panic!("request completed before cancellation: {result:?}"),
    }

    assert!(matches!(request.await, Err(HelixError::Cancelled)));
}

#[derive(Clone)]
struct BlockingSleeper {
    started: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl RetrySleeper for BlockingSleeper {
    async fn sleep_ms(&self, _milliseconds: u64) {
        self.started.notify_one();
        self.release.notified().await;
    }
}

#[tokio::test]
async fn cancellation_interrupts_an_in_flight_retry_wait() {
    let started = Arc::new(Notify::new());
    let requests: RequestLog = Arc::default();
    let client = HelixClient::new(
        "client-id",
        FakeHttp {
            requests: requests.clone(),
            responses: Arc::new(Mutex::new(
                vec![response(503, json!({"error":"Unavailable"}))].into(),
            )),
        },
        StaticTokenProvider::new("token"),
        BlockingSleeper {
            started: started.clone(),
            release: Arc::new(Notify::new()),
        },
    );
    let cancel = CancellationToken::new();
    let request = client.top_games(None, &cancel);
    tokio::pin!(request);

    tokio::select! {
        _ = started.notified() => cancel.cancel(),
        result = &mut request => panic!("request completed before cancellation: {result:?}"),
    }

    assert!(matches!(request.await, Err(HelixError::Cancelled)));
    assert_eq!(requests.lock().unwrap().len(), 1);
}
