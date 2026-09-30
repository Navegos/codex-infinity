use std::time::Duration;

use chrono::TimeZone;
use chrono::Utc;
use codex_protocol::auth::KnownPlan;
use codex_protocol::auth::PlanType;
use codex_protocol::error::UsageLimitReachedError;
use codex_protocol::protocol::RateLimitReachedType;
use pretty_assertions::assert_eq;

use super::ResponsesStreamRequest;
use super::ResponsesStreamRetryState;
use super::delay_until_usage_limit_reset;
use super::handle_response_stream_error;
use super::is_auto_waitable_usage_limit;
use super::log_retry;
use crate::realtime_history::RealtimeHistoryState;
use crate::session::step_context::StepContext;
use crate::session::tests::make_session_and_context;
use codex_http_client::RetryAfter;
use codex_protocol::error::CodexErr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing_test::internal::MockWriter;

#[test]
fn auto_wait_skips_workspace_credit_and_spend_cap_errors() {
    let resets_at = Utc.with_ymd_and_hms(2026, 7, 10, 13, 0, 0).unwrap();
    for rate_limit_reached_type in [
        RateLimitReachedType::WorkspaceOwnerCreditsDepleted,
        RateLimitReachedType::WorkspaceMemberCreditsDepleted,
        RateLimitReachedType::WorkspaceOwnerUsageLimitReached,
        RateLimitReachedType::WorkspaceMemberUsageLimitReached,
    ] {
        let err = UsageLimitReachedError {
            plan_type: Some(PlanType::Known(KnownPlan::Pro)),
            resets_at: Some(resets_at),
            rate_limits: None,
            promo_message: None,
            rate_limit_reached_type: Some(rate_limit_reached_type),
        };
        assert!(!is_auto_waitable_usage_limit(&err));
    }
}

#[test]
fn auto_wait_allows_rate_limit_with_reset_time() {
    let resets_at = Utc.with_ymd_and_hms(2026, 7, 10, 13, 0, 0).unwrap();
    for rate_limit_reached_type in [Some(RateLimitReachedType::RateLimitReached), None] {
        let err = UsageLimitReachedError {
            plan_type: Some(PlanType::Known(KnownPlan::Pro)),
            resets_at: Some(resets_at),
            rate_limits: None,
            promo_message: None,
            rate_limit_reached_type,
        };
        assert!(is_auto_waitable_usage_limit(&err));
    }
}

#[test]
fn auto_wait_requires_reset_time() {
    let err = UsageLimitReachedError {
        plan_type: Some(PlanType::Known(KnownPlan::Pro)),
        resets_at: None,
        rate_limits: None,
        promo_message: None,
        rate_limit_reached_type: None,
    };
    assert!(!is_auto_waitable_usage_limit(&err));
}

#[test]
fn delay_until_reset_is_zero_when_reset_time_has_passed() {
    let resets_at = Utc::now() - chrono::Duration::minutes(5);
    assert_eq!(
        delay_until_usage_limit_reset(resets_at),
        Some(Duration::from_secs(0))
    );
}

#[tokio::test]
async fn usage_limit_wait_can_be_cancelled() {
    let (session, turn_context) = make_session_and_context().await;
    let cancellation = tokio_util::sync::CancellationToken::new();
    cancellation.cancel();
    let error = CodexErr::UsageLimitReached(UsageLimitReachedError {
        plan_type: None,
        resets_at: Some(Utc::now() + chrono::Duration::hours(1)),
        rate_limits: None,
        promo_message: None,
        rate_limit_reached_type: None,
    });
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        super::wait_for_usage_limit_reset_if_applicable(
            &session,
            &turn_context,
            error,
            &cancellation,
        ),
    )
    .await
    .expect("cancelled wait should finish immediately");
    assert!(matches!(
        result.unwrap_err().details(),
        codex_protocol::error::CodexErrorDetails::TurnAborted
    ));
}

#[test]
fn server_overload_retry_starts_with_five_second_delay() {
    let state = ResponsesStreamRetryState::default();
    assert_eq!(state.server_overload_attempts, 0);
    assert_eq!(state.server_overload_retry_delay, Duration::from_secs(5));
}

#[tokio::test]
async fn server_overload_wait_rejects_non_overload_errors() {
    let (session, turn_context) = make_session_and_context().await;
    let mut state = ResponsesStreamRetryState::default();
    let cancellation = tokio_util::sync::CancellationToken::new();
    let result = super::wait_for_server_overload_retry(
        &mut state,
        &session,
        &turn_context,
        CodexErr::InternalServerError,
        &cancellation,
    )
    .await;
    assert!(matches!(
        result.unwrap_err().details(),
        codex_protocol::error::CodexErrorDetails::InternalServerError
    ));
    assert_eq!(state.server_overload_attempts, 0);
    assert_eq!(state.server_overload_retry_delay, Duration::from_secs(5));
}

#[tokio::test]
async fn server_overload_wait_can_be_cancelled_and_backs_off_exponentially() {
    let (session, turn_context) = make_session_and_context().await;
    let cancellation = tokio_util::sync::CancellationToken::new();
    cancellation.cancel();
    let mut state = ResponsesStreamRetryState::default();
    for (attempt, delay_secs) in [(1, 5), (2, 10), (3, 20)] {
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            super::wait_for_server_overload_retry(
                &mut state,
                &session,
                &turn_context,
                CodexErr::ServerOverloaded,
                &cancellation,
            ),
        )
        .await
        .expect("cancelled wait should finish immediately");
        assert!(matches!(
            result.unwrap_err().details(),
            codex_protocol::error::CodexErrorDetails::TurnAborted
        ));
        assert_eq!(state.server_overload_attempts, attempt);
        assert_eq!(
            state.server_overload_retry_delay,
            Duration::from_secs(delay_secs * 2)
        );
    }
}

#[tokio::test]
async fn sampling_retry_logs_stream_error_context() {
    let (_session, turn_context) = make_session_and_context().await;
    let buffer: &'static std::sync::Mutex<Vec<u8>> =
        Box::leak(Box::new(std::sync::Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(MockWriter::new(buffer))
        .finish();
    let _subscriber_guard = tracing::subscriber::set_default(subscriber);

    log_retry(
        ResponsesStreamRequest::Sampling,
        &turn_context,
        &CodexErr::Stream("websocket closed by server before response.completed".to_string()),
        /*retries*/ 2,
        /*max_retries*/ 5,
        Duration::from_secs(1),
    );

    let logs = String::from_utf8(
        buffer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone(),
    )
    .expect("retry log should be valid utf-8");
    assert!(logs.contains("stream disconnected - retrying sampling request"));
    assert!(logs.contains(&format!("turn_id={}", turn_context.sub_id)));
    assert!(logs.contains("retries=2"));
    assert!(logs.contains("max_retries=5"));
    assert!(logs.contains(
        "sampling_error=stream disconnected before completion: websocket closed by server before response.completed"
    ));
}

/// Time spent reporting a retry must count toward the server's original deadline.
#[tokio::test]
#[expect(
    clippy::await_holding_invalid_type,
    reason = "test holds the event-delivery lock to delay notification while virtual time advances"
)]
async fn stream_retry_preserves_deadline_across_delayed_notification() {
    let (mut session, turn_context) = make_session_and_context().await;
    let step_context = StepContext::for_test(Arc::new(turn_context));
    session.realtime_history = Some(Mutex::new(RealtimeHistoryState::default()));
    let mut client_session = session.services.model_client.new_session();
    // The second retry emits a notification even when release builds hide the first.
    let mut retry_state = ResponsesStreamRetryState {
        retries: 1,
        ..Default::default()
    };

    tokio::time::pause();
    let advice = RetryAfter::from_delay(Duration::from_secs(10)).expect("retry deadline");
    tokio::time::advance(Duration::from_secs(4)).await;

    // Hold the event-delivery lock so reporting the error consumes part of the deadline.
    let history_guard = session.realtime_history.as_ref().unwrap().lock().await;
    let retry = handle_response_stream_error(
        &mut retry_state,
        /*max_retries*/ 2,
        CodexErr::InternalServerError.with_retry_after(advice),
        &mut client_session,
        &session,
        &step_context,
        ResponsesStreamRequest::Sampling,
    );
    tokio::pin!(retry);
    assert!(futures::poll!(&mut retry).is_pending());
    tokio::time::advance(Duration::from_secs(4)).await;
    drop(history_guard);

    assert!(futures::poll!(&mut retry).is_pending());
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(futures::poll!(&mut retry).is_pending());
    retry.await.expect("retry should be allowed");
    // Tokio rounds timer deadlines up to the next millisecond.
    let resumed_at = Instant::now();
    assert!(
        (advice.deadline()..=advice.deadline() + Duration::from_millis(1)).contains(&resumed_at),
        "retry resumed at {resumed_at:?}, expected {advice:?}"
    );
}
