---
name: mojojojo-platform
description: Work on the sibling ../mojojojo repository (the mojojojo.cc site/platform the user may call mojojojo-site), including the Go web tier, Python/Mojo execution runner, sandbox, compilation cache, billing, vector/data services, SDK, browser runtime, edge worker, templates, and deployment. Use for implementation, diagnosis, review, testing, or UI work in mojojojo.
---

# mojojojo Platform

The active site repository is `../mojojojo`; no `../mojojojo-site` directory currently exists.

## Workflow

1. Read `CLAUDE.md` completely and inspect `git status --short`.
2. Read [references/project-map.md](references/project-map.md) and trace the affected tier end to end.
3. Preserve the compile/sandbox trust boundary, billing semantics, tenant isolation, cache keys, and cross-platform behavior.
4. Keep implementation changes narrow and add regression coverage at the owning tier.

## Safety

- Never make compilers or writable shared caches visible inside the sandbox.
- Treat jail paths, environment allowlists, billing clocks, storage prefixes, and deployment lifecycle as security-sensitive.
- Use `MOJOJOJO_KEY` only from the environment for authenticated API testing; never print or commit it.
- Do not deploy, restart units, or run paid remote capacity without explicit user direction.

## Verify

- Go: `go build -o mojojojo .` and the narrowest `go test`, then `go test ./...` when appropriate.
- Runner/SDK: `.venv/bin/python -m pytest <narrow paths> -q`.
- Browser: `node --test` on the affected `web/*.test.mjs` files.
- Edge: run the workerd-backed test only for edge changes.
- UI: run `node scripts/visualbench.mjs` and inspect the generated comparison.
