# Art providers

## OpenPaths

- Key: `OPENPATHS_API_KEY`
- API: `POST https://openpaths.io/v1/images/generations`
- MCP: `https://openpaths.io/mcp`, tool `generate_image`
- Auth: `Authorization: Bearer $OPENPATHS_API_KEY`
- Recommended routed model: `openpaths/auto-image`
- Common explicit models in the current project UI: `gpt-image-2`, `ra1`, `flux-pro`
- Response: OpenAI-compatible `data[]` entries with `url` or `b64_json`.

MCP arguments:

```json
{"model":"openpaths/auto-image","prompt":"...","size":"1024x1024","n":1}
```

## Netwrck

- Key: `NETWRCK_API_KEY`
- API: `POST https://netwrck.com/api/ra1-art-generator`
- Auth: JSON field `api_key`; there is no Netwrck MCP implementation in the current repository.
- Request: `api_key`, `prompt`, and `size`.
- Response: require a successful `image_url` before persisting or rendering.
- Do not retry 400, 401, or 402. Retry network errors or 502/503/504 at most once with backoff.

Use public URLs only as generation outputs or explicit reference inputs. Never upload private inputs to public hosting merely to satisfy an API URL field.
