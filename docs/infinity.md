# Codex Infinity tools

Sign in at <https://codex-infinity.com/account> and copy your account API key.
Set `CODEX_INFINITY_API_KEY` in the environment where you run Codex. Avoid putting
credentials in shell history, prompts, or repository files.

```sh
codex infinity connect
codex infinity status
```

`connect` shows sign-in instructions when the key is missing. When it is present,
both commands verify account authentication and report the effective MCP setting.
They do not launch paid tasks or change your browser login. Status does not perform
an MCP handshake; the next agent session initializes the server and lists tools.

Codex automatically adds the `codex-infinity` MCP server at
`https://codex-infinity.com/api/mcp` when this environment variable is nonempty.
The configuration contains an environment reference, never the secret itself.
Explicit settings for this server or the legacy `codex_infinity` alias take precedence, including tool restrictions;
organization MCP requirements still apply. The server is optional, so an outage
does not prevent `codex exec` from starting.

To disable automatic tools, add this to `config.toml`:

```toml
[mcp_servers.codex-infinity]
url = "https://codex-infinity.com/api/mcp"
bearer_token_env_var = "CODEX_INFINITY_API_KEY"
enabled = false
```

The server provides the site's tools catalog, cloud tasks and models, image tools,
and library skills via `read_skill`. Availability depends on the deployed server
and account. Skills can be read by the agent through MCP; they are not installed
or executed automatically on the local machine. Paid operations use your account's
billing and the agent's configured approval policy.

## Sync selected provider keys

Key upload is explicit and only reads the selected environment variables:

```sh
codex infinity sync-keys --provider openai --provider openrouter
```

| Provider | Local environment variable |
| --- | --- |
| `openai` | `OPENAI_API_KEY` |
| `anthropic` | `ANTHROPIC_API_KEY` |
| `google` | `GOOGLE_AI_KEY` (`GOOGLE_API_KEY` fallback) |
| `deepseek` | `DEEPSEEK_API_KEY` |
| `zai` | `ZAI_API_KEY` |
| `openrouter` | `OPENROUTER_API_KEY` |
| `netwrck` | `NETWRCK_API_KEY` |
| `fal` | `FAL_API_KEY` (`FAL_KEY` fallback) |
| `openpaths` | `OPENPATHS_API_KEY` |
| `cursor` | `CURSOR_API_KEY` |

All selected variables must be nonempty before any request is sent. The upload
updates those provider keys in your Infinity account and preserves other keys.
Repeated provider selections are deduplicated. It does not scan credential files,
upload Codex/Claude OAuth sessions, or download cloud secrets. The website returns
masked keys, and the CLI never prints keys or response bodies. Requests do not
follow redirects when authenticating or uploading keys.

## Chat Completions providers

The agent's Chat Completions adapter supports namespaced function tools and wraps
freeform tools such as code execution and patching in an `input` string parameter.
It restores their original identity and input after streaming, and preserves calls
and outputs in conversation history. Local routing metadata is removed before the
HTTP request. Responses-only web search and tool search are omitted; use available
MCP tools for provider-independent functionality. Colliding flattened names fail
explicitly instead of routing to the wrong tool.

Rust callers constructing raw `ChatRequest` values should use `ChatRequest::new(body,
headers)`; requests now retain private tool-routing metadata. `ChatRequestBuilder`
configures that metadata automatically for adapted Codex tools.
