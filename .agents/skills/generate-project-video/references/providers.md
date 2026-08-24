# Video providers

## OpenPaths

- Key: `OPENPATHS_API_KEY`
- Create: `POST https://openpaths.io/v1/videos/generations`
- Edit/extend: `POST /v1/videos/edits` or `/v1/videos/extensions`
- Poll: `GET /v1/videos/generations/{job_id}` for asynchronous creates.
- Auth: `Authorization: Bearer $OPENPATHS_API_KEY`
- Routed model: `auto-video`
- Useful fields: `model`, `prompt`, `image_url`, `duration`, `resolution`, `aspect_ratio`, and `output_format`.
- Result URL may be `video_url` or nested under `result.video_url`.

## Netwrck

- Key: `NETWRCK_API_KEY`
- Text-to-video: `POST https://netwrck.com/api/ltx-text-to-video`
- Image-to-video: `POST https://netwrck.com/api/ltx-2.3-image-to-video`
- WAN image-to-video: `POST https://netwrck.com/api/wan`
- Auth: JSON field `api_key`; no Netwrck MCP server is present in the current repo.
- LTX fields: `prompt`, optional `image_url`, `duration`, `resolution`, `aspect_ratio`, `fps`, `generate_audio`, and `response_mode`.
- Poll a returned `job_id` at `GET /api/fal/status/{job_id}?api_key=...`.

## app.nz MCP

The app.nz MCP server exposes Cogs rather than one generic video tool. Call `list_cog_templates` with category `Video`, inspect required inputs/licensing/cost, then call `run_cog_template`. Use `get_cog_prediction` for asynchronous status.

Never assume a live model's accepted duration or resolution. Use model metadata or a dry run and surface provider validation errors unchanged.
