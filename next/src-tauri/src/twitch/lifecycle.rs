use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::domain::stream::TwitchSession;
use crate::twitch::auth::ValidationReason;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleError {
    #[error("Twitch credentials are invalid")]
    InvalidCredentials,
    #[error("Twitch session validation failed")]
    ValidationFailed,
    #[error("Twitch user profile is unavailable")]
    ProfileUnavailable,
    #[error("Twitch credentials could not be cleared")]
    CleanupFailed,
}

#[async_trait]
pub trait SessionLifecycle: Clone + Send + Sync + 'static {
    async fn validate(
        &self,
        reason: ValidationReason,
    ) -> Result<Option<TwitchSession>, LifecycleError>;
    async fn clear_credentials(&self) -> Result<(), LifecycleError>;
}

#[derive(Clone)]
pub struct ValidationRunner<B> {
    backend: B,
    session: Arc<Mutex<TwitchSession>>,
}

impl<B: SessionLifecycle> ValidationRunner<B> {
    pub fn new(backend: B, session: Arc<Mutex<TwitchSession>>) -> Self {
        Self { backend, session }
    }

    pub async fn run_once(&self, reason: ValidationReason) -> Result<(), LifecycleError> {
        match self.backend.validate(reason).await {
            Ok(session) => {
                *self.session.lock().await = session.unwrap_or(TwitchSession::Anonymous);
                Ok(())
            }
            Err(LifecycleError::InvalidCredentials) => {
                let cleanup = self.backend.clear_credentials().await;
                *self.session.lock().await = TwitchSession::Anonymous;
                cleanup?;
                Err(LifecycleError::InvalidCredentials)
            }
            Err(error) => Err(error),
        }
    }
}
