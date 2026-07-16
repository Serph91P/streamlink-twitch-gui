use std::collections::VecDeque;

use tokio_util::sync::CancellationToken;

use crate::domain::stream::TwitchSession;

use super::{
    auth::{AuthClient, AuthError, HttpTransport},
    token_store::TokenStore,
};

const CANCELLED_ATTEMPT_LIMIT: usize = 32;

struct ActiveLogin {
    attempt_id: String,
    device_code: String,
    cancellation: CancellationToken,
}

#[derive(Default)]
pub struct LoginRegistry {
    active: Option<ActiveLogin>,
    cancelled: VecDeque<String>,
}

impl LoginRegistry {
    pub fn install(
        &mut self,
        attempt_id: impl Into<String>,
        device_code: impl Into<String>,
    ) -> Result<CancellationToken, AuthError> {
        let attempt_id = attempt_id.into();
        if !valid_attempt_id(&attempt_id) {
            return Err(AuthError::InvalidAttempt);
        }
        if self
            .cancelled
            .iter()
            .any(|cancelled| cancelled == &attempt_id)
        {
            return Err(AuthError::Cancelled);
        }
        if let Some(previous) = self.active.take() {
            previous.cancellation.cancel();
            self.remember_cancelled(previous.attempt_id);
        }
        let cancellation = CancellationToken::new();
        self.active = Some(ActiveLogin {
            attempt_id,
            device_code: device_code.into(),
            cancellation: cancellation.clone(),
        });
        Ok(cancellation)
    }

    pub fn poll(&self, attempt_id: &str) -> Result<(String, CancellationToken), AuthError> {
        let active = self
            .active
            .as_ref()
            .filter(|active| active.attempt_id == attempt_id)
            .ok_or(AuthError::InvalidAttempt)?;
        Ok((active.device_code.clone(), active.cancellation.clone()))
    }

    pub fn cancel(&mut self, attempt_id: &str) -> bool {
        let active = self
            .active
            .as_ref()
            .is_some_and(|active| active.attempt_id == attempt_id)
            .then(|| self.active.take())
            .flatten();
        if let Some(active) = active {
            active.cancellation.cancel();
            self.remember_cancelled(active.attempt_id);
            true
        } else {
            self.remember_cancelled(attempt_id.to_owned());
            false
        }
    }

    pub fn complete(&mut self, attempt_id: &str) -> bool {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.attempt_id == attempt_id)
        {
            self.active.take();
            true
        } else {
            false
        }
    }

    pub fn cancel_active(&mut self) -> bool {
        let Some(active) = self.active.take() else {
            return false;
        };
        active.cancellation.cancel();
        self.remember_cancelled(active.attempt_id);
        true
    }

    fn remember_cancelled(&mut self, attempt_id: String) {
        if !valid_attempt_id(&attempt_id)
            || self
                .cancelled
                .iter()
                .any(|cancelled| cancelled == &attempt_id)
        {
            return;
        }
        self.cancelled.push_back(attempt_id);
        if self.cancelled.len() > CANCELLED_ATTEMPT_LIMIT {
            self.cancelled.pop_front();
        }
    }
}

fn valid_attempt_id(attempt_id: &str) -> bool {
    !attempt_id.is_empty()
        && attempt_id.len() <= 128
        && attempt_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub async fn clear_cancelled_login<T: HttpTransport, S: TokenStore>(
    auth: &AuthClient<T, S>,
    session: &tokio::sync::Mutex<TwitchSession>,
) -> Result<(), AuthError> {
    auth.sign_out().await?;
    *session.lock().await = TwitchSession::Anonymous;
    Ok(())
}
