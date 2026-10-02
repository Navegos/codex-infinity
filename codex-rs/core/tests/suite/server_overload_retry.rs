//! Verifies that the agent automatically retries with backoff when the model
//! reports capacity pressure instead of failing the turn.

use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::sse_failed;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retries_on_server_overloaded_without_manual_restart() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse_failed(
                "resp-overloaded",
                "server_is_overloaded",
                "Selected model is at capacity. Please try a different model.",
            ),
            sse(vec![
                ev_response_created("resp-ok"),
                ev_completed("resp-ok"),
            ]),
        ],
    )
    .await;

    let test = test_codex()
        .with_config({
            let base_url = format!("{}/v1", server.uri());
            move |config| {
                config.model_provider.base_url = Some(base_url);
            }
        })
        .build(&server)
        .await
        .expect("build codex");

    test.submit_turn("hello")
        .await
        .expect("overloaded turn should succeed after automatic retry");

    assert_eq!(
        response_mock.requests().len(),
        2,
        "expected automatic retry after server overload"
    );
}

/// Capacity pressure outlives the configured stream retry budget: a provider that disables stream
/// retries still recovers instead of failing the turn.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retries_on_server_overloaded_with_stream_retries_disabled() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse_failed(
                "resp-overloaded",
                "server_is_overloaded",
                "Selected model is at capacity. Please try a different model.",
            ),
            sse(vec![
                ev_response_created("resp-ok"),
                ev_completed("resp-ok"),
            ]),
        ],
    )
    .await;

    let test = test_codex()
        .with_config({
            let base_url = format!("{}/v1", server.uri());
            move |config| {
                config.model_provider.base_url = Some(base_url);
                config.model_provider.stream_max_retries = Some(0);
            }
        })
        .build(&server)
        .await
        .expect("build codex");

    test.submit_turn("hello")
        .await
        .expect("overloaded turn should succeed after automatic retry");

    assert_eq!(
        response_mock.requests().len(),
        2,
        "expected automatic retry after server overload"
    );
}
