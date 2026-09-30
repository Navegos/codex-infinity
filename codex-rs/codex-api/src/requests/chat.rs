use crate::error::ApiError;
use crate::provider::Provider;
use crate::requests::chat_tools::ChatToolMapping;
use crate::requests::chat_tools::wire_tool_name;
use crate::requests::headers::build_session_headers;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ImageReference;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use http::HeaderMap;
use serde_json::Value;
use serde_json::json;

pub struct ChatRequest {
    pub body: Value,
    pub headers: HeaderMap,
    pub(crate) tool_mapping: ChatToolMapping,
}

impl ChatRequest {
    /// Creates a raw request without namespace or custom-tool routing. Use
    /// `ChatRequestBuilder` when adapting Codex tools for Chat Completions.
    pub fn new(body: Value, headers: HeaderMap) -> Self {
        Self {
            body,
            headers,
            tool_mapping: ChatToolMapping::new(),
        }
    }
}

pub struct ChatRequestBuilder<'a> {
    model: &'a str,
    instructions: &'a str,
    input: &'a [ResponseItem],
    tools: &'a [Value],
    session_id: Option<String>,
    reasoning_effort: Option<&'a str>,
    enable_thinking: bool,
}

impl<'a> ChatRequestBuilder<'a> {
    pub fn new(
        model: &'a str,
        instructions: &'a str,
        input: &'a [ResponseItem],
        tools: &'a [Value],
    ) -> Self {
        Self {
            model,
            instructions,
            input,
            tools,
            session_id: None,
            reasoning_effort: None,
            enable_thinking: false,
        }
    }

    pub fn session_id(mut self, session_id: Option<String>) -> Self {
        self.session_id = session_id;
        self
    }

    pub fn reasoning(mut self, effort: Option<&'a str>, enabled: bool) -> Self {
        self.reasoning_effort = effort;
        self.enable_thinking = enabled;
        self
    }

    pub fn build(self, _provider: &Provider) -> Result<ChatRequest, ApiError> {
        let mut tools = self.tools.to_vec();
        let mut tool_mapping = ChatToolMapping::new();
        for tool in &mut tools {
            if let Some(metadata) = tool
                .as_object_mut()
                .and_then(|object| object.remove("x_codex"))
            {
                let name = tool["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                tool_mapping.insert(
                    name,
                    serde_json::from_value(metadata).map_err(|_| {
                        ApiError::Stream("invalid local Chat Completions tool metadata".into())
                    })?,
                );
            }
        }
        let mut messages = vec![json!({"role": "system", "content": self.instructions})];
        let mut pending_reasoning = String::new();
        for item in self.input {
            match item {
                ResponseItem::Message { role, content, .. } => {
                    let wire_role = if role == "developer" { "system" } else { role };
                    let mut text = String::new();
                    let mut parts = Vec::new();
                    let mut multimodal = false;
                    for part in content {
                        match part {
                            ContentItem::InputText { text: value }
                            | ContentItem::OutputText { text: value } => {
                                text.push_str(value);
                                parts.push(json!({"type": "text", "text": value}));
                            }
                            ContentItem::InputImage { image, .. } => {
                                if let ImageReference::Inline { image_url } = image {
                                    multimodal = true;
                                    parts.push(json!({
                                        "type": "image_url",
                                        "image_url": {"url": image_url}
                                    }));
                                }
                            }
                            ContentItem::InputAudio { .. } => {}
                        }
                    }
                    let content = if multimodal && wire_role != "assistant" {
                        Value::Array(parts)
                    } else {
                        Value::String(text)
                    };
                    let mut message = json!({"role": wire_role, "content": content});
                    if wire_role == "assistant"
                        && !pending_reasoning.is_empty()
                        && let Some(object) = message.as_object_mut()
                    {
                        object.insert(
                            "reasoning_content".into(),
                            Value::String(std::mem::take(&mut pending_reasoning)),
                        );
                    }
                    messages.push(message);
                }
                ResponseItem::FunctionCall {
                    name,
                    namespace,
                    arguments,
                    call_id,
                    ..
                } => push_tool_call(
                    &mut messages,
                    call_id,
                    &wire_tool_name(name, namespace.as_deref()),
                    arguments,
                    (!pending_reasoning.is_empty()).then(|| std::mem::take(&mut pending_reasoning)),
                ),
                ResponseItem::CustomToolCall {
                    name,
                    namespace,
                    input,
                    call_id,
                    ..
                } => push_tool_call(
                    &mut messages,
                    call_id,
                    &wire_tool_name(name, namespace.as_deref()),
                    &json!({"input": input}).to_string(),
                    (!pending_reasoning.is_empty()).then(|| std::mem::take(&mut pending_reasoning)),
                ),
                ResponseItem::FunctionCallOutput {
                    call_id, output, ..
                } => {
                    push_tool_output(&mut messages, call_id.as_deref(), output);
                }
                ResponseItem::CustomToolCallOutput {
                    call_id, output, ..
                } => {
                    push_tool_output(&mut messages, Some(call_id), output);
                }
                ResponseItem::Reasoning {
                    content: Some(content),
                    ..
                } => {
                    for item in content {
                        match item {
                            ReasoningItemContent::ReasoningText { text }
                            | ReasoningItemContent::Text { text } => {
                                pending_reasoning.push_str(text)
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "tools": tools,
        });
        if let Some(object) = body.as_object_mut() {
            if self.tools.is_empty() {
                object.remove("tools");
            } else {
                object.insert("tool_choice".into(), json!("auto"));
            }
            if self.enable_thinking {
                object.insert("thinking".into(), json!({"type": "enabled"}));
            }
            if let Some(effort) = self.reasoning_effort {
                object.insert("reasoning_effort".into(), json!(effort));
            }
        }
        Ok(ChatRequest {
            body,
            tool_mapping,
            headers: build_session_headers(self.session_id, None),
        })
    }
}

fn push_tool_output(
    messages: &mut Vec<Value>,
    call_id: Option<&str>,
    output: &FunctionCallOutputPayload,
) {
    let content = match &output.body {
        FunctionCallOutputBody::Text(text) => text.clone(),
        FunctionCallOutputBody::ContentItems(items) => items
            .iter()
            .filter_map(|item| match item {
                codex_protocol::models::FunctionCallOutputContentItem::InputText { text } => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    };
    messages.push(json!({
        "role": "tool",
        "tool_call_id": call_id.unwrap_or_default(),
        "content": content
    }));
}

fn push_tool_call(
    messages: &mut Vec<Value>,
    call_id: &str,
    name: &str,
    arguments: &str,
    reasoning_content: Option<String>,
) {
    let tool_call = json!({
        "id": call_id,
        "type": "function",
        "function": {"name": name, "arguments": arguments}
    });
    if let Some(last) = messages.last_mut()
        && last.get("role").and_then(Value::as_str) == Some("assistant")
        && last.get("content").is_some_and(Value::is_null)
        && let Some(calls) = last.get_mut("tool_calls").and_then(Value::as_array_mut)
    {
        calls.push(tool_call);
    } else {
        let mut message = json!({"role": "assistant", "content": null, "tool_calls": [tool_call]});
        if let Some(reasoning_content) = reasoning_content
            && let Some(object) = message.as_object_mut()
        {
            object.insert("reasoning_content".into(), Value::String(reasoning_content));
        }
        messages.push(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::RetryConfig;
    use codex_protocol::models::ContentItem;
    use std::time::Duration;

    pub(super) fn provider() -> Provider {
        Provider {
            name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com".into(),
            query_params: None,
            headers: HeaderMap::new(),
            retry: RetryConfig {
                max_attempts: 1,
                base_delay: Duration::from_millis(1),
                retry_429: true,
                retry_5xx: true,
                retry_transport: true,
            },
            stream_idle_timeout: Duration::from_secs(1),
        }
    }

    #[test]
    fn builds_deepseek_thinking_request() {
        let input = vec![
            ResponseItem::Message {
                id: None,
                role: "developer".into(),
                content: vec![ContentItem::InputText {
                    text: "developer note".into(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::Message {
                id: None,
                role: "user".into(),
                content: vec![ContentItem::InputText {
                    text: "hello".into(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
        ];
        let request = ChatRequestBuilder::new("deepseek-v4-pro", "be useful", &input, &[])
            .reasoning(Some("max"), true)
            .build(&provider())
            .unwrap();
        assert_eq!(request.body["model"], "deepseek-v4-pro");
        assert_eq!(request.body["thinking"]["type"], "enabled");
        assert_eq!(request.body["reasoning_effort"], "max");
        assert_eq!(request.body["messages"][1]["role"], "system");
        assert!(request.body.get("tools").is_none());
    }
}

#[cfg(test)]
#[path = "chat_round_trip_tests.rs"]
mod round_trip_tests;
