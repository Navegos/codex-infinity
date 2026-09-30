use super::*;
use pretty_assertions::assert_eq;

#[test]
fn no_key_leaves_configuration_unchanged() {
    for key in [None, Some(""), Some(" \n ")] {
        let mut servers = HashMap::new();
        add_from_api_key(&mut servers, key);
        assert_eq!(servers, HashMap::new());
    }
}

#[test]
fn automatic_configuration_uses_only_an_environment_reference() {
    let mut servers = HashMap::new();
    add_from_api_key(&mut servers, Some("test-secret-never-persist"));
    let expected: McpServerConfig = toml::from_str(
        r#"
url = "https://codex-infinity.com/api/mcp"
bearer_token_env_var = "CODEX_INFINITY_API_KEY"
startup_timeout_sec = 10
tool_timeout_sec = 180
"#,
    )
    .unwrap();
    assert_eq!(
        servers,
        HashMap::from([(SERVER_NAME.to_string(), expected)])
    );
    assert!(!toml::to_string(&servers).unwrap().contains("test-secret"));
}

#[test]
fn explicit_disabled_and_custom_servers_are_preserved() {
    for server_name in [SERVER_NAME, "codex_infinity"] {
        for enabled in [true, false] {
            let existing: McpServerConfig = toml::from_str(&format!(
                r#"
url = "https://example.com/custom"
enabled = {enabled}
enabled_tools = ["read_skill"]
"#
            ))
            .unwrap();
            let mut servers = HashMap::from([(server_name.to_string(), existing)]);
            let expected = servers.clone();
            add_from_api_key(&mut servers, Some("test-key"));
            assert_eq!(servers, expected);
        }
    }
}
