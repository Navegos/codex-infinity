# ManifoldGen MCP and API reference

## Codex configuration

Set the user key in the environment and configure the Streamable HTTP endpoint:

```toml
[mcp_servers.manifoldgen]
url = "https://manifoldgen.com/api/mcp"
bearer_token_env_var = "MANIFOLDGEN_API_KEY"
tool_timeout_sec = 600
default_tools_approval_mode = "prompt"
```

Codex reads MCP configuration at session startup. After adding or changing this block, start a new session before diagnosing tool discovery.

## MCP tools

| Tool | Authentication | Existing API path | Notes |
| --- | --- | --- | --- |
| `get_pricing` | none | `GET /api/pricing` | Read before a generation when current cost is unknown. |
| `search_media` | optional | `GET /api/images/semantic`, `/api/search`, or `/api/audio/search` | Public results; auth can include private audio. |
| `generate_media` | bearer key | `POST /api/service` | Can spend credits; image/music can complete synchronously, video/SFX are usually jobs. |
| `get_job` | bearer key | `GET /api/video-jobs/{id}` | Also returns audio job status when the job is audio. |
| `list_jobs` | bearer key | `GET /api/video-jobs` or `/api/audio-jobs` | Recovers recent durable work. |

The endpoint is stateless JSON-RPC 2.0 over Streamable HTTP at `POST /api/mcp`. It supports `initialize`, `ping`, `tools/list`, `tools/call`, and MCP notifications. REST failures are returned as MCP tool results with `isError: true` and the original HTTP status in `structuredContent.status`.

## REST fallback

Use a bearer key only from a trusted server or local shell:

```bash
curl https://manifoldgen.com/api/pricing

curl --get https://manifoldgen.com/api/images/semantic \
  --data-urlencode 'q=editorial glass architecture at dusk' \
  --data-urlencode 'top_k=12'

curl https://manifoldgen.com/api/service \
  -H "Authorization: Bearer $MANIFOLDGEN_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"service":"image","prompt":"Editorial glass architecture at dusk","width":1024,"height":1024,"n":1}'
```

Public service names include `image`, `video`, `audio`, `music`, `sfx`, `video_restyle`, and `video_background_removal`. Use the public API documentation for service-specific controls. Poll asynchronous jobs every 2–5 seconds and retain the first returned job ID to avoid duplicate spend.
