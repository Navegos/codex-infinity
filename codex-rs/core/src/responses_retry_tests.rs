use std::time::Duration;

use chrono::TimeZone;
use chrono::Utc;
use codex_protocol::auth::KnownPlan;
use codex_protocol::auth::PlanType;
use codex_protocol::error::UsageLimitReachedError;
use codex_protocol::protocol::RateLimitReachedType;
use pretty_assertions::assert_eq;

use super::ResponsesStreamRequest;
use super::delay_until_usage_limit_reset;
use super::is_auto_waitable_usage_limit;
use super::log_retry;
use crate::session::tests::make_session_and_context;
use codex_protocol::error::CodexErr;
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
        &CodexErr::Stream(
            "websocket closed by server before response.completed".to_string(),
            None,
        ),
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
