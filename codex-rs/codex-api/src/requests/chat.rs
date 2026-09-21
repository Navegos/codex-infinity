use crate::error::ApiError;
use crate::provider::Provider;
use crate::requests::headers::build_session_headers;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::ImageReference;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use http::HeaderMap;
use serde_json::Value;
use serde_json::json;

pub struct ChatRequest {
    pub body: Value,
    pub headers: HeaderMap,
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
                    arguments,
                    call_id,
                    ..
                } => push_tool_call(
                    &mut messages,
                    call_id,
                    name,
                    arguments,
                    (!pending_reasoning.is_empty()).then(|| std::mem::take(&mut pending_reasoning)),
                ),
                ResponseItem::FunctionCallOutput {
                    call_id, output, ..
                } => {
                    let content = match &output.body {
                        FunctionCallOutputBody::Text(text) => text.clone(),
                        FunctionCallOutputBody::ContentItems(items) => items
                            .iter()
                            .filter_map(|item| {
                                match item {
                                codex_protocol::models::FunctionCallOutputContentItem::InputText {
                                    text,
                                } => Some(text.as_str()),
                                _ => None,
                            }
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    };
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": content
                    }));
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
            "tools": self.tools,
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
            headers: build_session_headers(self.session_id, None),
        })
    }
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

    fn provider() -> Provider {
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
