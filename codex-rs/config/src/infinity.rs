//! Optional Codex Infinity tools, authenticated using an environment reference.

use std::collections::HashMap;
use std::time::Duration;

use crate::McpServerConfig;
use crate::McpServerTransportConfig;

const SERVER_NAME: &str = "codex-infinity";
const API_KEY_ENV: &str = "CODEX_INFINITY_API_KEY";

/// Adds Infinity's MCP server when an API key is present. Explicit configuration,
/// including `enabled = false`, wins. Call before applying MCP requirements.
pub fn add_infinity_mcp_server(servers: &mut HashMap<String, McpServerConfig>) {
    add_from_api_key(servers, std::env::var(API_KEY_ENV).ok().as_deref());
}

fn add_from_api_key(servers: &mut HashMap<String, McpServerConfig>, api_key: Option<&str>) {
    if servers.contains_key("codex_infinity") || api_key.is_none_or(|key| key.trim().is_empty()) {
        return;
    }
    servers
        .entry(SERVER_NAME.to_string())
        .or_insert_with(|| McpServerConfig {
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://codex-infinity.com/api/mcp".to_string(),
                bearer_token_env_var: Some(API_KEY_ENV.to_string()),
                http_headers: None,
                env_http_headers: None,
                http_headers_helper: None,
            },
            auth: Default::default(),
            environment_id: crate::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
            omit_tools_from: None,
            disabled_reason: None,
            startup_timeout_sec: Some(Duration::from_secs(10)),
            tool_timeout_sec: Some(Duration::from_secs(180)),
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth: None,
            oauth_resource: None,
            tools: HashMap::new(),
        });
}

#[cfg(test)]
#[path = "infinity_tests.rs"]
mod tests;
