//! Explicit short waits remain responsive while background polling defaults stay long.

use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_target_windows;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_short_poll_returns_while_background_command_is_running() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_target_windows!(Ok(()), "uses a POSIX sleep command");
    let server = responses::start_mock_server().await;
    let mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_function_call(
                    "start",
                    "exec_command",
                    &json!({"cmd": "sleep 60", "yield_time_ms": 250}).to_string(),
                ),
                responses::ev_completed("start-response"),
            ]),
            responses::sse(vec![
                responses::ev_function_call(
                    "poll",
                    "write_stdin",
                    &json!({"session_id": 1000, "chars": "", "yield_time_ms": 1000}).to_string(),
                ),
                responses::ev_completed("poll-response"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("done", "still running"),
                responses::ev_completed("done-response"),
            ]),
        ],
    )
    .await;
    let test = test_codex()
        .with_config(|config| {
            config
                .permissions
                .set_permission_profile(PermissionProfile::Disabled)
                .expect("allow the background sleep command");
        })
        .build_with_auto_env(&server)
        .await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "start and briefly poll a background command".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    let requests = mock.requests();
    let output = requests[2]
        .function_call_output_text("poll")
        .expect("poll output");
    assert!(
        output.contains("Process running with session ID"),
        "{output}"
    );
    test.codex.shutdown_and_wait().await?;
    Ok(())
}
