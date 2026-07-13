#!/usr/bin/env python3
import argparse
import json
import os
from pathlib import Path

from release_common import validate_version


def main() -> None:
    parser = argparse.ArgumentParser(description="Create the secret-backed Tauri config")
    parser.add_argument("--version", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--windows-thumbprint")
    args = parser.parse_args()
    validate_version(args.version)

    public_key = os.environ.get("TAURI_UPDATER_PUBLIC_KEY", "").strip()
    if len(public_key) < 32 or "REPLACE" in public_key.upper():
        raise SystemExit("TAURI_UPDATER_PUBLIC_KEY is missing or invalid")
    if args.repository.count("/") != 1:
        raise SystemExit("repository must use the owner/name format")

    config = {
        "version": args.version,
        "bundle": {"createUpdaterArtifacts": True},
        "plugins": {
            "updater": {
                "pubkey": public_key,
                "endpoints": [
                    f"https://github.com/{args.repository}/releases/latest/download/latest.json"
                ],
            }
        },
    }
    if args.windows_thumbprint:
        config["bundle"]["windows"] = {
            "certificateThumbprint": args.windows_thumbprint,
            "digestAlgorithm": "sha256",
            "timestampUrl": "http://timestamp.digicert.com",
        }
    args.output.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
