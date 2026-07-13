#!/usr/bin/env python3
import argparse
import shutil
from pathlib import Path

from release_common import platform_asset_names, validate_version


SOURCE_SUFFIXES = {
    "linux-x64": (".AppImage", ".AppImage.sig", ".deb"),
    "windows-x64": ("-setup.exe", "-setup.exe.sig", ".msi", ".msi.sig"),
    "macos-x64": (".app.tar.gz", ".app.tar.gz.sig", ".dmg"),
    "macos-arm64": (".app.tar.gz", ".app.tar.gz.sig", ".dmg"),
}


def find_unique(directory: Path, suffix: str) -> Path:
    matches = sorted(
        path
        for path in directory.rglob("*")
        if path.is_file() and path.name.endswith(suffix)
    )
    if len(matches) != 1:
        raise ValueError(
            f"expected exactly one source asset ending in {suffix!r}, found {len(matches)}"
        )
    return matches[0]


def prepare_assets(
    bundle_directory: Path, output_directory: Path, platform: str, version: str
) -> list[Path]:
    validate_version(version)
    if platform not in SOURCE_SUFFIXES:
        raise ValueError(f"unsupported release platform: {platform}")

    destinations = sorted(platform_asset_names(version)[platform])
    sources = SOURCE_SUFFIXES[platform]
    output_directory.mkdir(parents=True, exist_ok=True)
    copied = []
    for suffix in sources:
        source = find_unique(bundle_directory, suffix)
        destination_name = next(name for name in destinations if name.endswith(suffix))
        destination = output_directory / destination_name
        shutil.copy2(source, destination)
        copied.append(destination)
    return copied


def main() -> None:
    parser = argparse.ArgumentParser(description="Collect canonical signed Tauri assets")
    parser.add_argument("--bundle-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--platform", choices=sorted(SOURCE_SUFFIXES), required=True)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    for path in prepare_assets(
        args.bundle_dir, args.output, args.platform, args.version
    ):
        print(path)


if __name__ == "__main__":
    main()
