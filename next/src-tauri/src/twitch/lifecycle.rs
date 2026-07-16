use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::{Mutex, watch};

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
    startup_ready: watch::Sender<bool>,
}

impl<B: SessionLifecycle> ValidationRunner<B> {
    pub fn new(backend: B, session: Arc<Mutex<TwitchSession>>) -> Self {
        let (startup_ready, _) = watch::channel(false);
        Self {
            backend,
            session,
            startup_ready,
        }
    }

    pub async fn run_once(&self, reason: ValidationReason) -> Result<(), LifecycleError> {
        let result = match self.backend.validate(reason).await {
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
        };
        if reason == ValidationReason::Startup {
            self.startup_ready.send_replace(true);
        }
        result
    }

    pub async fn session_after_startup(&self) -> TwitchSession {
        let mut startup_ready = self.startup_ready.subscribe();
        startup_ready
            .wait_for(|ready| *ready)
            .await
            .expect("startup readiness sender is retained by the validation runner");
        self.session.lock().await.clone()
    }
}
