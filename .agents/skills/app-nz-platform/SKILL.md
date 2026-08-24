---
name: app-nz-platform
description: Work on the sibling ../app-site app.nz platform across the React web app, Go server, Go CLI, desktop app, workers, creation network, Cogs/Comfy workflows, and cloud/desktop MCP surfaces. Use for implementation, diagnosis, review, testing, UI, CLI, or MCP tasks in app-site where web, CLI, desktop, and MCP parity matters.
---

# app.nz Platform

Work from `../app-site`.

## Workflow

1. Read `AGENTS.md` completely and inspect `git status --short`.
2. Read [references/project-map.md](references/project-map.md), then inspect the closest implementation and tests.
3. Identify every public surface affected: web, server API, CLI, desktop bridge, cloud MCP, desktop MCP, and documentation.
4. Implement the smallest coherent parity-safe change.

## Rules

- Use Bun only for JavaScript tooling: `bun install --frozen-lockfile`, `bun run`, and `bunx`.
- Preserve existing Go CLI/desktop/MCP parity and public tool schemas.
- Prefer the authenticated `app mcp serve` stdio bridge for Codex; it keeps the saved key out of argv and model context.
- Never print or commit `APP_API_KEY`, `OPENPATHS_API_KEY`, or `NETWRCK_API_KEY`.
- For generated art/video assets, invoke the dedicated media skill and then add VisualBench coverage where the asset affects UI.

## Verify

- Start with `bun run test:quick`.
- Use `bun run test:creation-network` for feed, Manim, CLI, desktop MCP, or browser creation flows.
- Scope Go tests to `server`, `cli`, `worker`, or `app-app` while iterating.
- Run the narrowest `bun run test:e2e:fast -- <spec>` for UI work and inspect `visualbench/` output.
- Reserve `bun run test` or `bun run test:release` for the final release gate.
