# app.nz project map

- `components/`, `src/`, root Vite files: React web application.
- `server/`: Go control plane and hosted Streamable HTTP MCP server.
- `cli/`: Go `app` CLI. `app mcp serve` proxies authenticated JSON-RPC to `https://app.nz/mcp`.
- `app-app/`: Go desktop host, local bridge, editor/browser tools, ACP, and stdio MCP host.
- `worker/`, `sites-worker/`, `comfy-worker/`, other workers: execution and deployment surfaces.
- `server/mcp_server.go`, `server/mcp_*.go`: cloud MCP tool schemas and dispatch.
- `app-app/internal/mcp/`: desktop MCP tools and external MCP aggregation.
- `docs/`: feature contracts such as animation, music, and Cogs.
- `tests/e2e/`, `visualbench/`: browser and visual verification.

MCP setup:

```bash
app login
app mcp install codex
codex mcp list
```

The hosted alternative is `https://app.nz/mcp` with an app.nz bearer key. The CLI bridge is preferred because it uses the saved login without placing a token in Codex config.

Useful checks:

```bash
cd ../app-site
bun run test:quick
bun run test:server
bun run test:cli
bun run test:app-app
bun run test:creation-network
```
