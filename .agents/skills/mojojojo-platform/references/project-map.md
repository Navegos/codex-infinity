# mojojojo project map

- Root Go files: web/API control plane, auth, billing, placement, data, vector search, deployments, and scaling.
- `runner/`: Python worker, jail, warm pool, package environments, Mojo acceleration, and cache promotion.
- `scale/`: dry-run-by-default capacity policy and provider adapters.
- `sdk/python/`: Python SDK, decorators, and CLI.
- `web/`: browser runtime, highlighter, and placement router.
- `wasmworker/`: Cloudflare edge execution tier.
- `templates/` and `static/`: server-rendered site and assets.
- `scripts/visualbench.mjs`: two-viewport visual capture.
- `docs/notebooks.md` and `docs/persistent-services.md`: hosted product contracts.

Commands:

```bash
cd ../mojojojo
go build -o mojojojo . && go test ./...
.venv/bin/python -m pytest runner/tests sdk/python/tests -q
node --test web/mojojojo.test.mjs web/hl.test.mjs web/route.test.mjs
node --test wasmworker/test/edge.test.mjs
bash tests/e2e.sh
node scripts/visualbench.mjs
```

Run only the tiers affected by the change while iterating. `bash deploy.sh` is a production operation, not a routine test.
