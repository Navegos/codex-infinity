---
name: generate-project-video
description: Generate text-to-video or image-to-video project assets through the Netwrck LTX/WAN APIs or OpenPaths video API using NETWRCK_API_KEY or OPENPATHS_API_KEY, with optional app.nz MCP Cog execution. Use when the user asks for motion clips, hero loops, demos, image animation, generated video, or video assets for Netwrck, OpenPaths, app.nz, or mojojojo. Do not use for ordinary transcoding or editing an existing local clip without generation.
---

# Generate Project Video

Use authenticated project services and keep live, billable calls intentional.

## Choose the path

- Prefer OpenPaths for `auto-video`, catalog routing, or a named provider model.
- Prefer Netwrck for LTX 2.3 text/image-to-video or WAN when explicitly requested.
- If app.nz MCP is connected, `list_cog_templates` and `run_cog_template` can discover and run scale-to-zero video Cogs.
- OpenPaths MCP currently has no video-generation tool; use its HTTP API for video.

Read [references/providers.md](references/providers.md), then run `scripts/generate_video.py` for OpenPaths or Netwrck.

## Workflow

1. Decide text-to-video versus image-to-video, model, duration, resolution, aspect ratio, and output path.
2. For image-to-video, use a user-approved public/reference URL. Do not publish private local media to create one.
3. Keep keys in environment variables only; never print them or pass them on argv.
4. Use `--dry-run` to inspect a request without spending credits.
5. Submit the real request only when the user requested generation, poll bounded asynchronous jobs, download the output, and inspect playable metadata or a frame.
6. When shipping to web, prefer an efficient WebM/AV1 asset with an MP4 fallback when the project supports both.

```bash
python3 .agents/skills/generate-project-video/scripts/generate_video.py \
  --provider openpaths --prompt "Slow camera push-in, drifting clouds" --output clip.mp4

python3 .agents/skills/generate-project-video/scripts/generate_video.py \
  --provider netwrck --image-url https://example.com/frame.png \
  --prompt "Hair and fabric move gently" --output clip.mp4
```

Run the owning project's visual/browser check after integrating a clip into UI.
