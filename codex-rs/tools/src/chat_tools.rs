//! Chat Completions schemas with local routing metadata removed by ChatRequestBuilder.

use crate::ResponsesApiNamespaceTool;
use crate::ToolSpec;
use serde_json::Value;
use serde_json::json;
use std::collections::HashSet;

/// Converts callable tools to function schemas. `x_codex` preserves local routing
/// for namespace and freeform calls and must not be sent to the provider.
pub fn create_tools_json_for_chat_completions_api(
    tools: &[ToolSpec],
) -> Result<Vec<Value>, serde_json::Error> {
    let mut output = Vec::new();
    let mut names = HashSet::new();
    let mut add = |mut tool: Value, namespace: Option<&str>| -> Result<(), serde_json::Error> {
        let name = tool["name"].as_str().unwrap_or_default().to_string();
        let wire_name = namespace.map_or_else(|| name.clone(), |ns| format!("{ns}__{name}"));
        if !names.insert(wire_name.clone()) {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("duplicate Chat Completions tool name: {wire_name}"),
            )));
        }
        let custom = tool.get("format").is_some();
        let function = if custom {
            json!({
                "name": wire_name, "description": tool["description"],
                "parameters": {"type": "object", "properties": {
                    "input": {"type": "string", "description": "The complete raw tool input."}
                }, "required": ["input"], "additionalProperties": false},
                "strict": true
            })
        } else {
            tool["name"] = json!(wire_name);
            if let Some(object) = tool.as_object_mut() {
                object.remove("defer_loading");
            }
            tool
        };
        output.push(json!({"type": "function", "function": function,
            "x_codex": {"name": name, "namespace": namespace, "custom": custom}}));
        Ok(())
    };
    for tool in tools {
        match tool {
            ToolSpec::Function(tool) => add(serde_json::to_value(tool)?, /*namespace*/ None)?,
            ToolSpec::Freeform(tool) => add(serde_json::to_value(tool)?, /*namespace*/ None)?,
            ToolSpec::Namespace(namespace) => {
                for tool in &namespace.tools {
                    let value = match tool {
                        ResponsesApiNamespaceTool::Function(tool) => serde_json::to_value(tool)?,
                        ResponsesApiNamespaceTool::Custom(tool) => serde_json::to_value(tool)?,
                    };
                    add(value, Some(&namespace.name))?;
                }
            }
            // These require Responses server-side execution or tool-search events.
            ToolSpec::WebSearch { .. } | ToolSpec::ToolSearch { .. } => {}
        }
    }
    Ok(output)
}

#[cfg(test)]
#[path = "chat_tools_tests.rs"]
mod tests;
