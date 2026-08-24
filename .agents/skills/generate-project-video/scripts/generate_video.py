#!/usr/bin/env python3
"""Generate text/image-to-video through OpenPaths or Netwrck and poll jobs."""

import argparse
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


def redact(text):
    for name in ("OPENPATHS_API_KEY", "NETWRCK_API_KEY"):
        if secret := os.environ.get(name):
            text = text.replace(secret, f"${name}")
    return text


def request_json(url, payload, headers, timeout):
    body = None if payload is None else json.dumps(payload).encode()
    request = urllib.request.Request(url, body, headers | ({"Content-Type": "application/json"} if body else {}))
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            raw = response.read().decode()
            return response.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as error:
        detail = redact(error.read(8192).decode(errors="replace"))
        raise SystemExit(f"provider returned HTTP {error.code}: {detail}") from None
    except urllib.error.URLError as error:
        raise SystemExit(f"provider request failed: {error.reason}") from None


def video_url(value):
    if not isinstance(value, dict):
        return ""
    for key in ("video_url", "output_url"):
        if isinstance(value.get(key), str) and value[key]:
            return value[key]
    video = value.get("video")
    if isinstance(video, dict) and isinstance(video.get("url"), str):
        return video["url"]
    for key in ("result", "output", "data"):
        found = video_url(value.get(key))
        if found:
            return found
    return ""


def job_id(value):
    for key in ("job_id", "id", "request_id"):
        if isinstance(value.get(key), str) and value[key]:
            return value[key]
    return ""


def save_video(url, output, timeout):
    try:
        with urllib.request.urlopen(url, timeout=timeout) as response:
            data = response.read()
    except urllib.error.URLError as error:
        raise SystemExit(f"video download failed: {error.reason}") from None
    path = Path(output)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return str(path.resolve())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider", choices=("openpaths", "netwrck"), default="openpaths")
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--image-url")
    parser.add_argument("--model", default="auto-video", help="OpenPaths model id")
    parser.add_argument("--duration", type=int, default=6)
    parser.add_argument("--resolution", default="720p")
    parser.add_argument("--aspect-ratio", default="16:9")
    parser.add_argument("--output-format", choices=("mp4", "webm"))
    parser.add_argument("--output", help="Download the completed video to this path")
    parser.add_argument("--netwrck-engine", choices=("ltx", "wan"), default="ltx")
    parser.add_argument("--generate-audio", action="store_true")
    parser.add_argument("--base-url", help="Override the provider base URL")
    parser.add_argument("--timeout", type=float, default=900, help="Total poll timeout in seconds")
    parser.add_argument("--poll-interval", type=float, default=2)
    parser.add_argument("--no-poll", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    if args.duration < 1:
        parser.error("--duration must be positive")
    if args.netwrck_engine == "wan" and not args.image_url:
        parser.error("Netwrck WAN requires --image-url")

    if args.provider == "openpaths":
        env_name = "OPENPATHS_API_KEY"
        base = (args.base_url or "https://openpaths.io/v1").rstrip("/")
        url = f"{base}/videos/generations"
        payload = {
            "model": args.model,
            "prompt": args.prompt,
            "duration": args.duration,
            "resolution": args.resolution,
            "aspect_ratio": args.aspect_ratio,
            "async": True,
        }
        if args.image_url:
            payload["image_url"] = args.image_url
        if args.output_format:
            payload["output_format"] = args.output_format
        key = os.environ.get(env_name, "")
        headers = {"Authorization": f"Bearer {key}"}
        safe_headers = {"Authorization": f"Bearer ${env_name}"}
    else:
        env_name = "NETWRCK_API_KEY"
        base = (args.base_url or "https://netwrck.com").rstrip("/")
        if args.netwrck_engine == "wan":
            url = f"{base}/api/wan"
            payload = {
                "api_key": os.environ.get(env_name, ""),
                "image_url": args.image_url,
                "prompt": args.prompt,
                "num_frames": args.duration * 16 + 1,
                "resolution": args.resolution,
                "aspect_ratio": args.aspect_ratio,
            }
        else:
            endpoint = "ltx-2.3-image-to-video" if args.image_url else "ltx-text-to-video"
            url = f"{base}/api/{endpoint}"
            payload = {
                "api_key": os.environ.get(env_name, ""),
                "prompt": args.prompt,
                "duration": args.duration,
                "resolution": args.resolution,
                "aspect_ratio": args.aspect_ratio,
                "fps": 25,
                "generate_audio": args.generate_audio,
                "response_mode": "polling",
            }
            if args.image_url:
                payload["image_url"] = args.image_url
        headers = {}
        safe_headers = {}

    if args.dry_run:
        safe_payload = dict(payload)
        if "api_key" in safe_payload:
            safe_payload["api_key"] = f"${env_name}"
        print(json.dumps({"method": "POST", "url": url, "headers": safe_headers, "json": safe_payload}, indent=2))
        return
    if not os.environ.get(env_name):
        raise SystemExit(f"{env_name} is required")

    _, response = request_json(url, payload, headers, min(args.timeout, 600))
    result_url = video_url(response)
    identifier = job_id(response)
    if not result_url and identifier and not args.no_poll:
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            time.sleep(args.poll_interval)
            if args.provider == "openpaths":
                poll_url = f"{base}/videos/generations/{urllib.parse.quote(identifier, safe='')}"
            else:
                encoded_id = urllib.parse.quote(identifier, safe="")
                encoded_key = urllib.parse.quote(os.environ[env_name], safe="")
                poll_url = f"{base}/api/fal/status/{encoded_id}?api_key={encoded_key}"
            _, response = request_json(poll_url, None, headers, min(args.timeout, 300))
            result_url = video_url(response)
            if result_url:
                break
            if str(response.get("status", "")).lower() in {"failed", "error", "cancelled"}:
                detail = redact(json.dumps(response))
                raise SystemExit(f"video job ended with status {response.get('status')}: {detail}")

    result = {"provider": args.provider, "model": args.model if args.provider == "openpaths" else args.netwrck_engine, "job_id": identifier, "video_url": result_url, "response": response}
    if result_url and args.output:
        result["output"] = save_video(result_url, args.output, min(args.timeout, 600))
    print(redact(json.dumps(result, indent=2)))
    if not result_url and not args.no_poll:
        raise SystemExit("video generation returned no video URL before timeout")


if __name__ == "__main__":
    main()
