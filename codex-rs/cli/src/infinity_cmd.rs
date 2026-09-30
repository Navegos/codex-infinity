use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use codex_core::config::Config;
use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClient;
use codex_http_client::HttpClientBuilder;
use codex_utils_cli::CliConfigOverrides;

const ORIGIN: &str = "https://codex-infinity.com";
const API_KEY_ENV: &str = "CODEX_INFINITY_API_KEY";
const SETUP: &str = "Sign in at https://codex-infinity.com/account and copy your API key.\nSet CODEX_INFINITY_API_KEY in your shell, then run `codex infinity connect`.\nCodex automatically enables Infinity MCP tools when the key is present.";

#[derive(Debug, Parser)]
pub(crate) struct InfinityCli {
    #[command(subcommand)]
    command: InfinityCommand,
}

#[derive(Debug, clap::Subcommand)]
enum InfinityCommand {
    /// Verify your Infinity key and automatic cloud-tool connection, or show login instructions.
    Connect,
    /// Check authentication and whether Infinity tools are enabled for this workspace.
    Status,
    /// Upload only the selected provider keys from environment variables to your Infinity account.
    SyncKeys {
        /// Provider to upload; repeat to select several. Existing cloud keys are replaced.
        #[arg(long, value_enum, required = true)]
        provider: Vec<Provider>,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum Provider {
    Openai,
    Anthropic,
    Google,
    Deepseek,
    Zai,
    Openrouter,
    Netwrck,
    Fal,
    Openpaths,
    Cursor,
}

impl Provider {
    fn key_fields(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Openai => ("openai_api_key", &["OPENAI_API_KEY"]),
            Self::Anthropic => ("anthropic_api_key", &["ANTHROPIC_API_KEY"]),
            Self::Google => ("google_ai_key", &["GOOGLE_AI_KEY", "GOOGLE_API_KEY"]),
            Self::Deepseek => ("deepseek_api_key", &["DEEPSEEK_API_KEY"]),
            Self::Zai => ("zai_api_key", &["ZAI_API_KEY"]),
            Self::Openrouter => ("openrouter_api_key", &["OPENROUTER_API_KEY"]),
            Self::Netwrck => ("netwrck_api_key", &["NETWRCK_API_KEY"]),
            Self::Fal => ("fal_api_key", &["FAL_API_KEY", "FAL_KEY"]),
            Self::Openpaths => ("openpaths_api_key", &["OPENPATHS_API_KEY"]),
            Self::Cursor => ("cursor_api_key", &["CURSOR_API_KEY"]),
        }
    }
}

impl InfinityCli {
    pub(crate) async fn run(self, overrides: CliConfigOverrides) -> Result<()> {
        let key = std::env::var(API_KEY_ENV).unwrap_or_default();
        if key.trim().is_empty() {
            if matches!(self.command, InfinityCommand::Connect) {
                println!("{SETUP}");
                return Ok(());
            }
            bail!("{API_KEY_ENV} is not set. {SETUP}");
        }
        let config = Config::load_with_cli_overrides(
            overrides.parse_overrides().map_err(anyhow::Error::msg)?,
        )
        .await?;
        let client = HttpClientBuilder::new()
            .without_redirects()
            .without_request_logging()
            .build_respecting_outbound_proxy_policy(
                &config.http_client_factory(),
                ORIGIN,
                ClientRouteClass::Api,
            )?;
        match self.command {
            InfinityCommand::Connect | InfinityCommand::Status => {
                verify_account(&client, ORIGIN, key.trim()).await?;
                println!("Authenticated with codex-infinity.com.");
                let enabled = config
                    .mcp_servers
                    .get()
                    .get("codex-infinity")
                    .or_else(|| config.mcp_servers.get().get("codex_infinity"))
                    .is_some_and(|server| server.enabled);
                if enabled {
                    println!("Infinity MCP tools are enabled. Start a Codex session to use them.");
                } else {
                    println!(
                        "Infinity MCP tools are disabled by your configuration or requirements."
                    );
                }
            }
            InfinityCommand::SyncKeys { provider } => {
                let payload = selected_keys(&provider, |name| std::env::var(name).ok())?;
                upload_keys(&client, ORIGIN, key.trim(), &payload).await?;
                println!(
                    "Synced {} selected provider key(s) to codex-infinity.com.",
                    payload.len()
                );
            }
        }
        Ok(())
    }
}

fn selected_keys(
    providers: &[Provider],
    read_env: impl Fn(&str) -> Option<String>,
) -> Result<BTreeMap<String, String>> {
    let mut payload = BTreeMap::new();
    for provider in providers {
        let (field, env_names) = provider.key_fields();
        let value = env_names
            .iter()
            .find_map(|env| read_env(env).filter(|value| !value.trim().is_empty()));
        let Some(value) = value else {
            let env = env_names.join(" or ");
            bail!("{env} is not set; no provider keys were uploaded");
        };
        payload.insert(field.to_string(), value.trim().to_string());
    }
    Ok(payload)
}

async fn verify_account(client: &HttpClient, origin: &str, key: &str) -> Result<()> {
    let response = client
        .get(format!("{origin}/api/account/provider-keys"))
        .bearer_auth(key)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .context("could not reach Codex Infinity")?;
    if !response.status().is_success() {
        bail!(
            "Infinity authentication check failed (HTTP {})",
            response.status().as_u16()
        );
    }
    Ok(())
}

async fn upload_keys(
    client: &HttpClient,
    origin: &str,
    key: &str,
    payload: &BTreeMap<String, String>,
) -> Result<()> {
    let response = client
        .post(format!("{origin}/api/account/provider-keys"))
        .bearer_auth(key)
        .json(payload)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .context("could not upload provider keys to Codex Infinity")?;
    // Never print response bodies: a remote error could echo a submitted credential.
    if !response.status().is_success() {
        bail!(
            "Infinity key sync failed (HTTP {})",
            response.status().as_u16()
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "infinity_cmd_tests.rs"]
mod tests;
