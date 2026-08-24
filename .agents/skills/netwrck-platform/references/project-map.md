# Netwrck project map

- `search_server_go/`: primary Go/fasthttp server, handlers, routing, Pongo templates, and Go tests.
- `search_server_go/main.go`: route registration and host dispatch. Check domain validation before assuming reachability.
- `search_server_go/templates/`: server-rendered pages. API documentation lives under `templates/shared/api-docs-*.pongo`.
- `search_server_go/static/js/`: browser code; root `build.js` and `package.json` define targeted bundles.
- `api_doc_tests/`: authenticated API documentation tests. `bun run test:api-docs:real` uses `NETWRCK_API_KEY`.
- `search_server_go/ra1_handler.go`: RA1 image generation and accounting.
- `search_server_go/api_proxy.go`, `wan_i2v.go`, and related handlers: WAN/LTX video generation and upstream dispatch.
- `search_server_go/domain_validation.go`: per-host route allowlists.
- `search_server_go/deploy.sh`: blue/green backend deployment. Root `deploy.sh` handles static/CDN assets and must precede backend deploy when required.

Useful checks:

```bash
cd ../netwrck/search_server_go
go test . -run 'TestName'
go build ./...

cd ..
bun run build:target -- tool:video-editor
bun run test:api-docs:go
```

Production mutation is never implied by a code task. Confirm database targets before writes; repository `.env` files may point at production.
