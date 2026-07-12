use std::fmt;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
    access_token: String,
    refresh_token: String,
}

impl Credentials {
    pub fn new(access_token: impl Into<String>, refresh_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            refresh_token: refresh_token.into(),
        }
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credentials")
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum TokenStoreError {
    #[error("credential store is unavailable")]
    Unavailable,
    #[error("stored credentials are invalid")]
    InvalidData,
}

#[async_trait]
pub trait TokenStore: Clone + Send + Sync + 'static {
    async fn load(&self) -> Result<Option<Credentials>, TokenStoreError>;
    async fn store(&self, credentials: Credentials) -> Result<(), TokenStoreError>;
    async fn delete(&self) -> Result<(), TokenStoreError>;
}

#[derive(Clone, Default)]
pub struct MemoryTokenStore(Arc<Mutex<Option<Credentials>>>);

impl MemoryTokenStore {
    pub fn with_tokens(access_token: impl Into<String>, refresh_token: impl Into<String>) -> Self {
        Self(Arc::new(Mutex::new(Some(Credentials::new(
            access_token,
            refresh_token,
        )))))
    }
}

#[async_trait]
impl TokenStore for MemoryTokenStore {
    async fn load(&self) -> Result<Option<Credentials>, TokenStoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| TokenStoreError::Unavailable)?
            .clone())
    }

    async fn store(&self, credentials: Credentials) -> Result<(), TokenStoreError> {
        *self.0.lock().map_err(|_| TokenStoreError::Unavailable)? = Some(credentials);
        Ok(())
    }

    async fn delete(&self) -> Result<(), TokenStoreError> {
        *self.0.lock().map_err(|_| TokenStoreError::Unavailable)? = None;
        Ok(())
    }
}

#[derive(Clone)]
pub struct OsTokenStore {
    service: String,
    account: String,
}

impl OsTokenStore {
    pub fn new(service: impl Into<String>, account: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            account: account.into(),
        }
    }

    fn entry(&self) -> Result<keyring::Entry, TokenStoreError> {
        keyring::Entry::new(&self.service, &self.account).map_err(|_| TokenStoreError::Unavailable)
    }
}

#[async_trait]
impl TokenStore for OsTokenStore {
    async fn load(&self) -> Result<Option<Credentials>, TokenStoreError> {
        match self.entry()?.get_password() {
            Ok(value) => serde_json::from_str(&value)
                .map(Some)
                .map_err(|_| TokenStoreError::InvalidData),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(TokenStoreError::Unavailable),
        }
    }

    async fn store(&self, credentials: Credentials) -> Result<(), TokenStoreError> {
        let value =
            serde_json::to_string(&credentials).map_err(|_| TokenStoreError::InvalidData)?;
        self.entry()?
            .set_password(&value)
            .map_err(|_| TokenStoreError::Unavailable)
    }

    async fn delete(&self) -> Result<(), TokenStoreError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(TokenStoreError::Unavailable),
        }
    }
}
