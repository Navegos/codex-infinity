---
name: manifoldgen-platform
description: Work on the ManifoldGen image, video, audio, voice, gallery, billing, and Studio platform, or use its hosted MCP server for pricing, semantic media search, generation, and durable job retrieval. Use for implementation, diagnosis, review, testing, deployment, or generated-media tasks in a manifoldgen-site repository, and when the user asks to use MANIFOLDGEN_API_KEY or ManifoldGen MCP tools.
---

# ManifoldGen Platform

Use the repository for code changes and the hosted MCP server for live catalog or generation work. Keep those modes separate: an MCP generation changes the user's remote account and can spend credits; editing or testing the repository does not.

## Choose the path

- For source changes, locate the `manifoldgen-site` repository, read its `AGENTS.md`, inspect its dirty state, and preserve unrelated work.
- For live pricing, search, generation, or job status, prefer the configured `manifoldgen` MCP server.
- If the current tool inventory does not contain ManifoldGen MCP tools, do not claim that they are connected. Read [references/mcp-and-api.md](references/mcp-and-api.md), verify configuration, and explain that a new Codex session may be required after changing MCP configuration.
- Use direct REST calls only when MCP is unavailable or the requested operation is not exposed through MCP.

## Work in the repository

1. Inspect the route and the owning module before editing. The Go `fasthttp` API lives in `server/`, the Next.js app in `frontend/`, workers in `workers/`, and deployment configuration in `deploy/` plus `deploy.sh`.
2. Reuse the existing API-key authentication, credit settlement, durable jobs, uploads, and provider-routing helpers. Do not create parallel billing or authentication paths.
3. Keep asynchronous media operations durable. Return the public job ID/status URL and use the existing polling and repair flow.
4. Update the API documentation when an endpoint, service name, MCP tool, request field, billing rule, or response shape changes.
5. Format Go changes with `gofmt`. Run `(cd server && go test ./...)`; run the frontend type/build or focused browser checks when frontend behavior changes.
6. Treat `deploy.sh` as a production mutation: inspect its inputs and checks first, then run it only when deployment is explicitly requested.

## Use the MCP server

1. Use `get_pricing` before a credit-spending call unless the user already approved a known current price.
2. Use `search_media` for public image, video, and audio discovery. A bearer key may add the account's private audio results.
3. Use `generate_media` only for the number and kind of assets requested. It requires `MANIFOLDGEN_API_KEY` and can spend account credits.
4. Save the job ID returned by asynchronous video or sound-effect calls. Poll with `get_job`; use `list_jobs` to recover recent account jobs.
5. Return durable media URLs and the final charged cost when present. Do not present a queued job as a completed asset.

## Protect credentials and spend

- Read `MANIFOLDGEN_API_KEY` from the environment. Never print, paste, commit, or place it in browser code.
- Do not use `MANIFOLD_ADMIN_API_KEY` for ordinary MCP calls.
- Do not start batches, high-step image runs, long videos, or repeated retries unless their scope and likely spend are explicit.
- Treat 401 as missing or stale credentials, 402 as insufficient credits, and 429/503 as retryable with bounded backoff. Avoid resubmitting generation calls when a durable job ID may already exist.

Read [references/mcp-and-api.md](references/mcp-and-api.md) for configuration, tool-to-REST mappings, and fallback examples.
