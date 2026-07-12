use std::fmt;

use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;

use crate::domain::stream::TwitchLoginChallenge;
use crate::twitch::token_store::{Credentials, TokenStore, TokenStoreError};

const DEVICE_URL: &str = "https://id.twitch.tv/oauth2/device";
const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";
const VALIDATION_INTERVAL_SECONDS: u64 = 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwitchConfig {
    client_id: String,
}

impl TwitchConfig {
    pub fn new(client_id: impl Into<String>) -> Result<Self, ConfigError> {
        let client_id = client_id.into();
        if client_id.trim().is_empty() {
            return Err(ConfigError::MissingClientId);
        }
        Ok(Self { client_id })
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Twitch client ID is not configured")]
    MissingClientId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub query: Vec<(String, String)>,
    pub form: Vec<(String, String)>,
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &"[REDACTED]")
            .field("query", &self.query)
            .field("form", &"[REDACTED]")
            .finish()
    }
}

impl HttpRequest {
    pub fn form_value(&self, key: &str) -> Option<&str> {
        self.form
            .iter()
            .find_map(|(name, value)| (name == key).then_some(value.as_str()))
    }

    pub fn query_value(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find_map(|(name, value)| (name == key).then_some(value.as_str()))
    }

    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find_map(|(name, value)| name.eq_ignore_ascii_case(key).then_some(value.as_str()))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl fmt::Debug for HttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &"[REDACTED]")
            .finish()
    }
}

impl HttpResponse {
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find_map(|(name, value)| name.eq_ignore_ascii_case(key).then_some(value.as_str()))
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Twitch authorization is still pending")]
    Pending,
    #[error("Twitch authorization expired")]
    Expired,
    #[error("Twitch authorization was denied")]
    AccessDenied,
    #[error("Twitch authentication returned HTTP {0}")]
    HttpStatus(u16),
    #[error("Twitch authentication response was invalid")]
    InvalidResponse,
    #[error("Twitch authentication transport failed")]
    Transport,
    #[error("Twitch credentials are invalid")]
    InvalidCredentials,
    #[error(transparent)]
    TokenStore(#[from] TokenStoreError),
}

#[async_trait]
pub trait HttpTransport: Clone + Send + Sync + 'static {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, AuthError>;
}

#[derive(Clone, Default)]
pub struct ReqwestTransport(reqwest::Client);

#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, AuthError> {
        let mut request_headers = reqwest::header::HeaderMap::new();
        for (key, value) in &request.headers {
            let key = reqwest::header::HeaderName::try_from(key.as_str())
                .map_err(|_| AuthError::Transport)?;
            let value = reqwest::header::HeaderValue::try_from(value.as_str())
                .map_err(|_| AuthError::Transport)?;
            request_headers.insert(key, value);
        }
        let mut builder = match request.method {
            HttpMethod::Get => self.0.get(&request.url),
            HttpMethod::Post => self.0.post(&request.url),
        }
        .headers(request_headers)
        .query(&request.query);
        if !request.form.is_empty() {
            builder = builder.form(&request.form);
        }
        let response = builder.send().await.map_err(|_| AuthError::Transport)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(key, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (key.to_string(), value.to_owned()))
            })
            .collect();
        let body = response.text().await.map_err(|_| AuthError::Transport)?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

#[derive(Clone)]
pub struct AuthClient<T, S> {
    client_id: String,
    scopes: Vec<String>,
    transport: T,
    store: S,
}

pub struct DeviceLoginAttempt {
    pub challenge: TwitchLoginChallenge,
    device_code: String,
}

impl DeviceLoginAttempt {
    pub fn device_code(&self) -> &str {
        &self.device_code
    }
}

impl fmt::Debug for DeviceLoginAttempt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceLoginAttempt")
            .field("challenge", &self.challenge)
            .field("device_code", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollResult {
    Pending,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationReason {
    Startup,
    Hourly,
    Unauthorized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedIdentity {
    pub client_id: String,
    pub login: String,
    pub user_id: String,
    pub expires_in_seconds: u64,
    pub scopes: Vec<String>,
}

#[derive(Deserialize)]
struct DeviceResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u32,
    interval: u32,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct OAuthErrorResponse {
    message: String,
}

#[derive(Deserialize)]
struct ValidationResponse {
    client_id: String,
    login: String,
    user_id: String,
    expires_in: u64,
    scopes: Vec<String>,
}

impl<T: HttpTransport, S: TokenStore> AuthClient<T, S> {
    pub fn new<I, V>(client_id: impl Into<String>, scopes: I, transport: T, store: S) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        Self {
            client_id: client_id.into(),
            scopes: scopes.into_iter().map(Into::into).collect(),
            transport,
            store,
        }
    }

    pub async fn begin_device_login(&self) -> Result<DeviceLoginAttempt, AuthError> {
        let response = self
            .transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: DEVICE_URL.into(),
                headers: Vec::new(),
                query: Vec::new(),
                form: vec![
                    ("client_id".into(), self.client_id.clone()),
                    ("scopes".into(), self.scopes.join(" ")),
                ],
            })
            .await?;
        if response.status != 200 {
            return Err(AuthError::HttpStatus(response.status));
        }
        let payload: DeviceResponse =
            serde_json::from_str(&response.body).map_err(|_| AuthError::InvalidResponse)?;
        Ok(DeviceLoginAttempt {
            challenge: TwitchLoginChallenge {
                verification_uri: payload.verification_uri,
                user_code: payload.user_code,
                expires_in_seconds: payload.expires_in,
                polling_interval_seconds: payload.interval,
            },
            device_code: payload.device_code,
        })
    }

    pub async fn poll_device_login(&self, device_code: &str) -> Result<PollResult, AuthError> {
        let response = self
            .token_request(vec![
                ("client_id".into(), self.client_id.clone()),
                ("scopes".into(), self.scopes.join(" ")),
                ("device_code".into(), device_code.into()),
                (
                    "grant_type".into(),
                    "urn:ietf:params:oauth:grant-type:device_code".into(),
                ),
            ])
            .await?;
        if response.status == 200 {
            self.persist_token_response(response).await?;
            return Ok(PollResult::Complete);
        }
        let message = serde_json::from_str::<OAuthErrorResponse>(&response.body)
            .map(|error| error.message)
            .unwrap_or_default();
        match message.as_str() {
            "authorization_pending" => Ok(PollResult::Pending),
            "expired_token" => Err(AuthError::Expired),
            "access_denied" => Err(AuthError::AccessDenied),
            _ => Err(AuthError::HttpStatus(response.status)),
        }
    }

    pub async fn validate_session(
        &self,
        _reason: ValidationReason,
    ) -> Result<Option<ValidatedIdentity>, AuthError> {
        let Some(mut credentials) = self.store.load().await? else {
            return Ok(None);
        };
        let mut response = self.validate(credentials.access_token()).await?;
        if response.status == 401 {
            credentials = match self.refresh(credentials.refresh_token()).await {
                Ok(credentials) => credentials,
                Err(AuthError::HttpStatus(400 | 401)) => {
                    return self.invalidate_credentials().await;
                }
                Err(error) => return Err(error),
            };
            response = self.validate(credentials.access_token()).await?;
        }
        if response.status == 400 || response.status == 401 {
            return self.invalidate_credentials().await;
        }
        if response.status != 200 {
            return Err(AuthError::HttpStatus(response.status));
        }
        let payload: ValidationResponse = match serde_json::from_str(&response.body) {
            Ok(payload) => payload,
            Err(_) => return self.invalidate_credentials().await,
        };
        if payload.client_id != self.client_id {
            return self.invalidate_credentials().await;
        }
        Ok(Some(ValidatedIdentity {
            client_id: payload.client_id,
            login: payload.login,
            user_id: payload.user_id,
            expires_in_seconds: payload.expires_in,
            scopes: payload.scopes,
        }))
    }

    pub fn validation_due(now_seconds: u64, last_validation_seconds: u64) -> bool {
        now_seconds.saturating_sub(last_validation_seconds) >= VALIDATION_INTERVAL_SECONDS
    }

    pub async fn sign_out(&self) -> Result<(), AuthError> {
        self.store.delete().await?;
        Ok(())
    }

    async fn invalidate_credentials<R>(&self) -> Result<R, AuthError> {
        self.store.delete().await?;
        Err(AuthError::InvalidCredentials)
    }

    pub(crate) async fn stored_access_token(&self) -> Result<Option<String>, AuthError> {
        Ok(self
            .store
            .load()
            .await?
            .map(|credentials| credentials.access_token().to_owned()))
    }

    async fn validate(&self, access_token: &str) -> Result<HttpResponse, AuthError> {
        self.transport
            .execute(HttpRequest {
                method: HttpMethod::Get,
                url: VALIDATE_URL.into(),
                headers: vec![("Authorization".into(), format!("OAuth {access_token}"))],
                query: Vec::new(),
                form: Vec::new(),
            })
            .await
    }

    async fn refresh(&self, refresh_token: &str) -> Result<Credentials, AuthError> {
        let response = self
            .token_request(vec![
                ("client_id".into(), self.client_id.clone()),
                ("grant_type".into(), "refresh_token".into()),
                ("refresh_token".into(), refresh_token.into()),
            ])
            .await?;
        self.persist_token_response(response).await
    }

    async fn token_request(&self, form: Vec<(String, String)>) -> Result<HttpResponse, AuthError> {
        self.transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: TOKEN_URL.into(),
                headers: Vec::new(),
                query: Vec::new(),
                form,
            })
            .await
    }

    async fn persist_token_response(
        &self,
        response: HttpResponse,
    ) -> Result<Credentials, AuthError> {
        if response.status != 200 {
            return Err(AuthError::HttpStatus(response.status));
        }
        let payload: TokenResponse =
            serde_json::from_str(&response.body).map_err(|_| AuthError::InvalidResponse)?;
        let credentials = Credentials::new(payload.access_token, payload.refresh_token);
        self.store.store(credentials.clone()).await?;
        Ok(credentials)
    }
}
