use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use std::collections::HashMap;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gpt_6_1_model_switch_preserves_current_provider_settings() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let server = responses::start_mock_server().await;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("response-1"),
            responses::ev_assistant_message("message-1", "done"),
            responses::ev_completed("response-1"),
        ]),
    )
    .await;
    let test = test_codex()
        .with_model("gpt-6-sol")
        .with_config(|config| {
            config.model_provider.name = "Configured OpenAI".to_string();
            config.model_provider.supports_websockets = false;
            config.model_provider.http_headers = Some(HashMap::from([(
                "x-infinity-provider".to_string(),
                "preserved".into(),
            )]));
        })
        .build_with_auto_env(&server)
        .await?;

    test.codex
        .start_or_steer_turn(
            TurnInputRequest::user_input(vec![UserInput::Text {
                text: "switch models".to_string(),
                text_elements: Vec::new(),
            }])
            .with_thread_settings(ThreadSettingsOverrides {
                model: Some("gpt-6.1-sol".to_string()),
                ..Default::default()
            }),
        )
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let request = response.single_request();
    assert_eq!(request.body_json()["model"], "gpt-6.1-sol");
    assert_eq!(
        request.header("x-infinity-provider"),
        Some("preserved".to_string())
    );
    Ok(())
}
