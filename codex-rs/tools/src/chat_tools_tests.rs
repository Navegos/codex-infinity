use super::*;
use crate::FreeformTool;
use crate::FreeformToolFormat;
use crate::ResponsesApiNamespace;
use pretty_assertions::assert_eq;

#[test]
fn wraps_namespaced_custom_tools_and_preserves_routing() {
    let custom = FreeformTool {
        name: "exec".into(),
        description: "Run code".into(),
        defer_loading: Some(true),
        format: FreeformToolFormat {
            r#type: "grammar".into(),
            syntax: "lark".into(),
            definition: "start: /.+/".into(),
        },
    };
    let tools = [ToolSpec::Namespace(ResponsesApiNamespace {
        name: "functions".into(),
        description: String::new(),
        tools: vec![ResponsesApiNamespaceTool::Custom(custom)],
    })];
    assert_eq!(
        create_tools_json_for_chat_completions_api(&tools).unwrap(),
        vec![json!({
            "type": "function", "function": {
                "name": "functions__exec", "description": "Run code", "strict": true,
                "parameters": {"type": "object", "properties": {
                    "input": {"type": "string", "description": "The complete raw tool input."}
                }, "required": ["input"], "additionalProperties": false}
            }, "x_codex": {"name": "exec", "namespace": "functions", "custom": true}
        })]
    );
    assert!(
        create_tools_json_for_chat_completions_api(&[tools[0].clone(), tools[0].clone()]).is_err()
    );
}

#[test]
fn function_schema_removes_deferred_loading() {
    let tools = [ToolSpec::Function(crate::ResponsesApiTool {
        name: "run".into(),
        description: "Run".into(),
        strict: false,
        defer_loading: Some(true),
        parameters: crate::JsonSchema::object(
            Default::default(),
            /*required*/ None,
            /*additional_properties*/ None,
        ),
        output_schema: None,
    })];
    assert_eq!(
        create_tools_json_for_chat_completions_api(&tools).unwrap(),
        vec![json!({
            "type": "function", "function": {"name": "run", "description": "Run", "strict": false,
                "parameters": {"type": "object", "properties": {}}},
            "x_codex": {"name": "run", "namespace": null, "custom": false}
        })]
    );
}
