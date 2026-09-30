use crate::error::ApiError;
use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use std::collections::BTreeMap;

pub(crate) type ChatToolMapping = BTreeMap<String, ChatToolIdentity>;

#[derive(Clone, Deserialize)]
pub(crate) struct ChatToolIdentity {
    pub name: String,
    pub namespace: Option<String>,
    pub custom: bool,
}

pub(crate) fn restore_tool_call(
    mapping: &ChatToolMapping,
    name: String,
    arguments: String,
    call_id: String,
) -> Result<ResponseItem, ApiError> {
    let identity = mapping.get(&name);
    let namespace = identity.and_then(|tool| tool.namespace.clone());
    let name = identity.map_or(name, |tool| tool.name.clone());
    if identity.is_some_and(|tool| tool.custom) {
        #[derive(Deserialize)]
        struct CustomInput {
            input: String,
        }
        let input: CustomInput = serde_json::from_str(&arguments).map_err(|_| {
            ApiError::Stream("invalid custom tool input from Chat Completions".into())
        })?;
        Ok(ResponseItem::CustomToolCall {
            id: None,
            status: None,
            name,
            namespace,
            input: input.input,
            call_id,
            internal_chat_message_metadata_passthrough: None,
        })
    } else {
        Ok(ResponseItem::FunctionCall {
            id: None,
            name,
            namespace,
            arguments,
            call_id,
            encrypted_function_args: None,
            internal_chat_message_metadata_passthrough: None,
        })
    }
}

pub(crate) fn wire_tool_name(name: &str, namespace: Option<&str>) -> String {
    namespace.map_or_else(|| name.to_string(), |ns| format!("{ns}__{name}"))
}
