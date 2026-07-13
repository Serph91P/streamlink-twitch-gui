#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

from release_common import validate_version


def main() -> None:
    parser = argparse.ArgumentParser(description="Create the unsigned Tauri release config")
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    validate_version(args.version)

    config = {
        "version": args.version,
        "bundle": {"createUpdaterArtifacts": False},
    }
    args.output.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
