# OpenPaths project map

- `cmd/openpaths/`: server entrypoint.
- `internal/server/server.go`: public route registration and middleware chains.
- `internal/handler/`: OpenAI-compatible HTTP, account, media, artifact, guardrail, and MCP handlers.
- `internal/router/`: model selection, auto routes, fallback, and provider dispatch.
- `internal/provider/`: provider adapters; preserve normalized request/response behavior.
- `internal/billing/`: price calculation and credit deduction.
- `internal/db/`: migrations and queries.
- `src/`: React/Vite UI and API documentation examples.
- `src/pages/Mcp.tsx` and `internal/handler/mcp.go`: MCP documentation and implementation.
- `e2e/`: Playwright coverage, including art and live video paths.

Key endpoints:

- API base: `https://openpaths.io/v1`
- MCP: `https://openpaths.io/mcp` using `Authorization: Bearer $OPENPATHS_API_KEY`
- Images: `POST /v1/images/generations` and `/v1/images/edits`
- Videos: `POST /v1/videos/generations`, `/edits`, or `/extensions`; poll `GET /v1/videos/<operation>/{job_id}` when asynchronous.

Typical checks:

```bash
cd ../openpaths
go test ./internal/handler -run MCP
go test ./...
bun run lint
bun run build
```
