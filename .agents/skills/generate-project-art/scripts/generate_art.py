#!/usr/bin/env python3
"""Generate one image through OpenPaths or Netwrck without argv secrets."""

import argparse
import base64
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path


def redact(text):
    for name in ("OPENPATHS_API_KEY", "NETWRCK_API_KEY"):
        if secret := os.environ.get(name):
            text = text.replace(secret, f"${name}")
    return text


def request_json(url, payload, headers, timeout):
    body = json.dumps(payload).encode()
    request = urllib.request.Request(url, body, {"Content-Type": "application/json", **headers})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return json.loads(response.read().decode())
    except urllib.error.HTTPError as error:
        detail = redact(error.read(8192).decode(errors="replace"))
        raise SystemExit(f"generation failed with HTTP {error.code}: {detail}") from None
    except urllib.error.URLError as error:
        raise SystemExit(f"generation request failed: {error.reason}") from None


def image_items(response):
    if isinstance(response.get("data"), list):
        return [item for item in response["data"] if isinstance(item, dict)]
    if response.get("image_url"):
        return [{"url": response["image_url"]}]
    if isinstance(response.get("images"), list):
        return [item if isinstance(item, dict) else {"url": item} for item in response["images"]]
    return []


def save_image(item, output, timeout):
    if item.get("b64_json"):
        data = base64.b64decode(item["b64_json"], validate=True)
    elif item.get("url"):
        try:
            with urllib.request.urlopen(item["url"], timeout=timeout) as response:
                data = response.read()
        except urllib.error.URLError as error:
            raise SystemExit(f"image download failed: {error.reason}") from None
    else:
        raise SystemExit("generation succeeded but returned neither url nor b64_json")
    path = Path(output)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return str(path.resolve())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider", choices=("openpaths", "netwrck"), default="openpaths")
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--model", default="openpaths/auto-image", help="OpenPaths model id")
    parser.add_argument("--size", default="1024x1024")
    parser.add_argument("--count", type=int, default=1)
    parser.add_argument("--output", help="Download the first generated image to this path")
    parser.add_argument("--base-url", help="Override the provider API base URL")
    parser.add_argument("--timeout", type=float, default=180)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    if args.count < 1 or args.count > 8:
        parser.error("--count must be between 1 and 8")
    if args.provider == "netwrck" and args.count != 1:
        parser.error("Netwrck RA1 currently generates one image per request")

    if args.provider == "openpaths":
        env_name = "OPENPATHS_API_KEY"
        base = (args.base_url or "https://openpaths.io/v1").rstrip("/")
        url = f"{base}/images/generations"
        payload = {"model": args.model, "prompt": args.prompt, "size": args.size, "n": args.count}
        safe_headers = {"Authorization": f"Bearer ${env_name}"}
        key = os.environ.get(env_name, "")
        headers = {"Authorization": f"Bearer {key}"}
    else:
        env_name = "NETWRCK_API_KEY"
        base = (args.base_url or "https://netwrck.com").rstrip("/")
        url = f"{base}/api/ra1-art-generator"
        payload = {"api_key": os.environ.get(env_name, ""), "prompt": args.prompt, "size": args.size}
        safe_headers = {}
        headers = {}

    if args.dry_run:
        safe_payload = dict(payload)
        if "api_key" in safe_payload:
            safe_payload["api_key"] = f"${env_name}"
        print(json.dumps({"method": "POST", "url": url, "headers": safe_headers, "json": safe_payload}, indent=2))
        return
    if not os.environ.get(env_name):
        raise SystemExit(f"{env_name} is required")

    response = request_json(url, payload, headers, args.timeout)
    items = image_items(response)
    if not items:
        print(json.dumps(response, indent=2), file=sys.stderr)
        raise SystemExit("generation returned no image")

    result = {"provider": args.provider, "model": args.model if args.provider == "openpaths" else "ra1", "images": items}
    if args.output:
        result["output"] = save_image(items[0], args.output, args.timeout)
    print(redact(json.dumps(result, indent=2)))


if __name__ == "__main__":
    main()
