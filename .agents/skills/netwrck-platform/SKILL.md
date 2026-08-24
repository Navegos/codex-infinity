---
name: netwrck-platform
description: Work on the sibling ../netwrck multitenant Go/fasthttp platform, including netwrck.com, ebank.nz, v5games.com, Pongo templates, JavaScript bundles, art/video APIs, databases, tests, visual checks, and deployment investigation. Use for implementation, diagnosis, review, or API integration tasks whose code lives in the Netwrck repository. Do not use for merely generating a media asset; use generate-project-art or generate-project-video instead.
---

# Netwrck Platform

Work from `../netwrck`. Resolve the real path before acting because it may be a symlink.

## Start safely

1. Read `AGENTS.md` and `CLAUDE.md` completely.
2. Inspect `git status --short`; preserve every unrelated change and never stash it.
3. Read [references/project-map.md](references/project-map.md) for the relevant surface.
4. Treat `search_server_go` as the primary application unless the requested path proves otherwise.

## Implement

- Keep multitenant host allowlists and handler registration in sync. A registered handler is not automatically reachable on every domain.
- Prefer a narrow Go/template/JavaScript change. Rebuild only affected bundles while iterating.
- Keep keys in environment variables. Use `NETWRCK_API_KEY` for developer media calls and `OPENPATHS_API_KEY` only for existing OpenPaths-backed paths. Never print, commit, or interpolate their values into source.
- For direct media creation, invoke `$generate-project-art` or `$generate-project-video`.
- Do not start a second dev server, kill the running Go server, or deploy unless the user explicitly requests it.

## Verify

- Go: run the narrow package/test first, then from `search_server_go` run `go build ./...` and the relevant `go test` command.
- Frontend bundles: use `bun run build:target -- <target>` and the closest JavaScript test.
- UI/templates: capture and compare the applicable VisualBench suite.
- Live pages: use the repo-prescribed `curls` and `js_error_checker.py`, not plain `curl`.
- Report exactly what was tested and what remained untested.
