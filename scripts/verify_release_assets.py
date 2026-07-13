#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

from release_common import (
    checksum_name,
    expected_asset_names,
    sbom_name,
    validate_target_sha,
    validate_version,
)
from release_metadata import sha256


def read_checksums(path: Path) -> dict[str, str]:
    checksums = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.split("  ", 1)
        if len(parts) != 2 or len(parts[0]) != 64:
            raise ValueError(f"invalid checksum line: {line!r}")
        digest, name = parts
        if name in checksums:
            raise ValueError(f"duplicate checksum entry: {name}")
        checksums[name] = digest
    return checksums


def verify_release(
    directory: Path,
    version: str,
    tag: str,
    repository: str,
    target_sha: str,
) -> None:
    validate_version(version)
    validate_target_sha(target_sha)
    if tag != f"v{version}":
        raise ValueError("release tag does not match version")
    repository_parts = repository.split("/")
    if len(repository_parts) != 2 or not all(repository_parts):
        raise ValueError("repository must use the owner/name format")
    expected = expected_asset_names(version)
    actual = {path.name for path in directory.iterdir() if path.is_file()}
    if actual != expected:
        raise ValueError(
            f"release asset set mismatch; missing={sorted(expected - actual)}, "
            f"unexpected={sorted(actual - expected)}"
        )

    checksum_file = checksum_name(version)
    checksums = read_checksums(directory / checksum_file)
    covered = expected - {checksum_file}
    if set(checksums) != covered:
        raise ValueError("checksum coverage does not match release assets")
    for name, digest in checksums.items():
        if sha256(directory / name) != digest:
            raise ValueError(f"checksum mismatch: {name}")

    sbom = json.loads((directory / sbom_name(version)).read_text(encoding="utf-8"))
    if sbom.get("bomFormat") != "CycloneDX" or sbom.get("specVersion") != "1.6":
        raise ValueError("SBOM is not CycloneDX 1.6")
    property_items = sbom.get("metadata", {}).get("properties", [])
    source_property = "io.github.streamlink-twitch-gui.source-commit"
    if sum(item.get("name") == source_property for item in property_items) > 1:
        raise ValueError(f"duplicate SBOM property: {source_property}")
    properties = {item.get("name"): item.get("value") for item in property_items}
    if properties.get(source_property) != target_sha:
        raise ValueError("SBOM source commit mismatch")
    file_component_items = [
        component
        for component in sbom.get("components", [])
        if component.get("type") == "file"
    ]
    file_component_names = [component.get("name") for component in file_component_items]
    if len(file_component_names) != len(set(file_component_names)):
        raise ValueError("duplicate SBOM file component name")
    file_components = {
        component.get("name"): component for component in file_component_items
    }
    sbom_inputs = expected - {checksum_file, sbom_name(version)}
    if set(file_components) != sbom_inputs:
        raise ValueError("SBOM file coverage does not match release inputs")
    for name in sbom_inputs:
        hashes = file_components[name].get("hashes", [])
        expected_hash = sha256(directory / name)
        if not any(
            item.get("alg") == "SHA-256" and item.get("content") == expected_hash
            for item in hashes
        ):
            raise ValueError(f"SBOM hash mismatch: {name}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Verify a complete draft release")
    subparsers = parser.add_subparsers(dest="command", required=True)
    version_parser = subparsers.add_parser("version")
    version_parser.add_argument("value")
    assets_parser = subparsers.add_parser("assets")
    assets_parser.add_argument("directory", type=Path)
    assets_parser.add_argument("--version", required=True)
    assets_parser.add_argument("--tag", required=True)
    assets_parser.add_argument("--repository", required=True)
    assets_parser.add_argument("--target-sha", required=True)
    args = parser.parse_args()
    if args.command == "version":
        print(validate_version(args.value))
    else:
        verify_release(
            args.directory,
            args.version,
            args.tag,
            args.repository,
            args.target_sha,
        )
        print(f"verified {len(expected_asset_names(args.version))} release assets")


if __name__ == "__main__":
    main()
