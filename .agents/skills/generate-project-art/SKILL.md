---
name: generate-project-art
description: Generate a new bitmap image or project art asset through the Netwrck RA1 API or OpenPaths image API/MCP using NETWRCK_API_KEY or OPENPATHS_API_KEY. Use when the user asks to create artwork, illustrations, hero images, thumbnails, textures, concept art, or other image assets with these services or for the sibling Netwrck, OpenPaths, app.nz, or mojojojo projects. Do not use for editing an existing local image when the built-in image-editing tool is the requested path.
---

# Generate Project Art

Generate through authenticated project services without exposing credentials.

## Choose the path

- Prefer OpenPaths for auto-routing, a named catalog model, or MCP-native generation.
- Prefer Netwrck for RA1 or when the user explicitly names Netwrck/ebank.
- If an OpenPaths MCP tool named `generate_image` is connected, call it with `model`, `prompt`, `size`, and `n`.
- Otherwise run `scripts/generate_art.py`. Read [references/providers.md](references/providers.md) when choosing models or integrating the response.

## Workflow

1. Determine prompt, provider, aspect/size, count, destination, and whether transparent output is needed.
2. Keep the API key in its existing environment variable. Never echo it, pass it on argv, write it to a prompt, or commit it.
3. Use `--dry-run` when validating request shape without spending credits.
4. Run the real generation only when the user requested generation; provider calls may be billable.
5. Download the selected output into the target repo when requested, inspect it visually, and report the model/provider and final path.

```bash
python3 .agents/skills/generate-project-art/scripts/generate_art.py \
  --provider openpaths --prompt "..." --size 1024x1024 --output asset.png

python3 .agents/skills/generate-project-art/scripts/generate_art.py \
  --provider netwrck --prompt "..." --output asset.webp
```

Do not run broad repository tests for a standalone generated asset. Run the owning project's visual or asset check when integrating it into UI.
