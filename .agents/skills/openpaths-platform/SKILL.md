---
name: openpaths-platform
description: Work on the sibling ../openpaths Go model gateway and React site, including OpenAI-compatible APIs, provider routing, billing, model catalog, image/video generation, artifacts, guardrails, and the hosted OpenPaths MCP server. Use for implementation, diagnosis, review, testing, or documentation tasks whose code lives in OpenPaths. Do not use for a one-off generated asset; use generate-project-art or generate-project-video instead.
---

# OpenPaths Platform

Work from `../openpaths`.

## Workflow

1. Read `CLAUDE.md` and any nearer instructions.
2. Inspect `git status --short` and preserve unrelated work.
3. Read [references/project-map.md](references/project-map.md) for the affected surface.
4. Trace public routes through `internal/server/server.go` into handlers, router/provider logic, billing, and recording before editing.

## Parity and credentials

- Keep OpenAI-compatible HTTP behavior, UI examples, model metadata, and MCP tools aligned when a shared capability changes.
- The MCP endpoint is `https://openpaths.io/mcp`; its implementation currently exposes chat, model listing, image generation, embeddings, and web search. Video generation remains an HTTP API.
- Read `OPENPATHS_API_KEY` only from the environment. Never expose it in logs, diffs, examples, shell output, or model context.
- Use `$generate-project-art` or `$generate-project-video` for actual media generation.

## Verify

- Go changes: run the narrow test, then `go test ./...` from the repo root.
- Frontend changes: use `bun run lint` and the narrowest useful Playwright spec; run `bun run build` when routing or prerendered metadata changes.
- MCP changes: run the MCP handler tests and verify `src/pages/Mcp.tsx` remains accurate.
- Live paid-provider tests require explicit user intent; do not turn ordinary verification into billable generation.
