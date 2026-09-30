use super::*;
use crate::common::ResponseEvent;
use crate::requests::chat_tools::restore_tool_call;
use crate::sse::chat::process_chat_sse;
use futures::TryStreamExt;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::io::ReaderStream;

#[tokio::test]
async fn custom_tools_round_trip_through_request_stream_and_history() {
    let tools = [
        json!({"type": "function", "function": {"name": "functions__exec"},
        "x_codex": {"name": "exec", "namespace": "functions", "custom": true}}),
    ];
    let request = ChatRequestBuilder::new("cloud-model", "instructions", &[], &tools)
        .build(&super::tests::provider())
        .unwrap();
    assert_eq!(
        request.body["tools"],
        json!([{"type": "function", "function": {"name": "functions__exec"}}])
    );
    let delta = json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "call-1",
        "function": {"name": "functions__exec", "arguments": "{\"input\":\"text(1 + 1)\"}"}}]}}]});
    let body = format!("data: {delta}\n\ndata: [DONE]\n\n");
    let stream = ReaderStream::new(std::io::Cursor::new(body))
        .map_err(|error| codex_client::TransportError::Network(error.to_string()));
    let (tx, mut rx) = mpsc::channel(16);
    process_chat_sse(
        stream,
        tx,
        Duration::from_secs(1),
        /*telemetry*/ None,
        request.tool_mapping,
    )
    .await;
    let mut call = None;
    while let Some(event) = rx.recv().await {
        if let ResponseEvent::OutputItemDone(item @ ResponseItem::CustomToolCall { .. }) =
            event.unwrap()
        {
            call = Some(item);
        }
    }
    let call = call.expect("custom tool call was restored");
    assert_eq!(
        call,
        ResponseItem::CustomToolCall {
            id: None,
            status: None,
            call_id: "call-1".into(),
            name: "exec".into(),
            namespace: Some("functions".into()),
            input: "text(1 + 1)".into(),
            internal_chat_message_metadata_passthrough: None,
        }
    );
    let output: ResponseItem = serde_json::from_value(json!({
        "type": "custom_tool_call_output", "call_id": "call-1", "output": "2"
    }))
    .unwrap();
    let input = [call, output];
    let replay = ChatRequestBuilder::new("cloud-model", "instructions", &input, &tools)
        .build(&super::tests::provider())
        .unwrap();
    assert_eq!(
        replay.body["messages"],
        json!([
            {"role": "system", "content": "instructions"},
            {"role": "assistant", "content": null, "tool_calls": [{"id": "call-1", "type": "function",
                "function": {"name": "functions__exec", "arguments": "{\"input\":\"text(1 + 1)\"}"}}]},
            {"role": "tool", "tool_call_id": "call-1", "content": "2"}
        ])
    );
}

#[test]
fn namespaced_functions_restore_and_invalid_custom_input_fails() {
    let mapping = serde_json::from_value(json!({
        "cloud__launch": {"name": "launch", "namespace": "cloud", "custom": false},
        "exec": {"name": "exec", "namespace": null, "custom": true}
    }))
    .unwrap();
    assert_eq!(
        restore_tool_call(&mapping, "cloud__launch".into(), "{}".into(), "c".into()).unwrap(),
        ResponseItem::FunctionCall {
            id: None,
            name: "launch".into(),
            namespace: Some("cloud".into()),
            arguments: "{}".into(),
            call_id: "c".into(),
            encrypted_function_args: None,
            internal_chat_message_metadata_passthrough: None
        }
    );
    assert!(restore_tool_call(&mapping, "exec".into(), "{}".into(), "c".into()).is_err());
}
