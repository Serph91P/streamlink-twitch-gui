use std::collections::VecDeque;
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use streamlink_twitch_gui_lib::domain::stream::{TwitchSession, TwitchUser};
use streamlink_twitch_gui_lib::twitch::auth::ValidationReason;
use streamlink_twitch_gui_lib::twitch::lifecycle::{
    LifecycleError, SessionLifecycle, ValidationRunner,
};
use tokio::sync::{Mutex, Notify};

type ValidationResults = Arc<StdMutex<VecDeque<Result<Option<TwitchSession>, LifecycleError>>>>;

#[derive(Clone)]
struct FakeLifecycle {
    results: ValidationResults,
    reasons: Arc<StdMutex<Vec<ValidationReason>>>,
    clears: Arc<StdMutex<usize>>,
}

#[derive(Clone)]
struct BlockingLifecycle {
    started: Arc<Notify>,
    release: Arc<Notify>,
    result: TwitchSession,
}

#[async_trait]
impl SessionLifecycle for BlockingLifecycle {
    async fn validate(
        &self,
        _reason: ValidationReason,
    ) -> Result<Option<TwitchSession>, LifecycleError> {
        self.started.notify_one();
        self.release.notified().await;
        Ok(Some(self.result.clone()))
    }

    async fn clear_credentials(&self) -> Result<(), LifecycleError> {
        Ok(())
    }
}

#[async_trait]
impl SessionLifecycle for FakeLifecycle {
    async fn validate(
        &self,
        reason: ValidationReason,
    ) -> Result<Option<TwitchSession>, LifecycleError> {
        self.reasons.lock().unwrap().push(reason);
        self.results.lock().unwrap().pop_front().unwrap()
    }

    async fn clear_credentials(&self) -> Result<(), LifecycleError> {
        *self.clears.lock().unwrap() += 1;
        Ok(())
    }
}

fn authenticated() -> TwitchSession {
    TwitchSession::Authenticated {
        user: TwitchUser {
            id: "42".into(),
            login: "tester".into(),
            display_name: "Tester".into(),
            profile_image_url: "https://example/profile.png".into(),
        },
        expires_at: "2026-07-12T12:00:00Z".into(),
    }
}

#[tokio::test]
async fn first_session_read_waits_for_startup_validation() {
    let expected = authenticated();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let session = Arc::new(Mutex::new(TwitchSession::Anonymous));
    let runner = ValidationRunner::new(
        BlockingLifecycle {
            started: started.clone(),
            release: release.clone(),
            result: expected.clone(),
        },
        session,
    );

    let validation = tokio::spawn({
        let runner = runner.clone();
        async move { runner.run_once(ValidationReason::Startup).await }
    });
    started.notified().await;
    let first_read = tokio::spawn({
        let runner = runner.clone();
        async move { runner.session_after_startup().await }
    });
    tokio::task::yield_now().await;

    assert!(!first_read.is_finished());
    release.notify_one();
    validation.await.unwrap().unwrap();
    assert_eq!(first_read.await.unwrap(), expected);
}

#[tokio::test]
async fn startup_and_hourly_runs_update_only_sanitized_session_state() {
    let expected = authenticated();
    let backend = FakeLifecycle {
        results: Arc::new(StdMutex::new(
            vec![Ok(Some(expected.clone())), Ok(None)].into(),
        )),
        reasons: Arc::default(),
        clears: Arc::default(),
    };
    let session = Arc::new(Mutex::new(TwitchSession::Anonymous));
    let runner = ValidationRunner::new(backend.clone(), session.clone());

    runner.run_once(ValidationReason::Startup).await.unwrap();
    assert_eq!(*session.lock().await, expected);
    assert!(
        !serde_json::to_string(&*session.lock().await)
            .unwrap()
            .contains("token")
    );

    runner.run_once(ValidationReason::Hourly).await.unwrap();
    assert_eq!(*session.lock().await, TwitchSession::Anonymous);
    assert_eq!(
        backend.reasons.lock().unwrap().as_slice(),
        &[ValidationReason::Startup, ValidationReason::Hourly]
    );
}

#[tokio::test]
async fn invalid_credentials_are_cleared_without_panicking_or_leaking_secrets() {
    let backend = FakeLifecycle {
        results: Arc::new(StdMutex::new(
            vec![Err(LifecycleError::InvalidCredentials)].into(),
        )),
        reasons: Arc::default(),
        clears: Arc::default(),
    };
    let session = Arc::new(Mutex::new(authenticated()));
    let runner = ValidationRunner::new(backend.clone(), session.clone());

    let error = runner.run_once(ValidationReason::Hourly).await.unwrap_err();

    assert!(matches!(error, LifecycleError::InvalidCredentials));
    assert_eq!(*backend.clears.lock().unwrap(), 1);
    assert_eq!(*session.lock().await, TwitchSession::Anonymous);
    assert_eq!(error.to_string(), "Twitch credentials are invalid");
}
