#!/usr/bin/env python3
import argparse
import hashlib
import json
import tomllib
import uuid
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote

from release_common import (
    checksum_name,
    sbom_name,
    updater_assets,
    validate_target_sha,
    validate_version,
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_updater_manifest(
    directory: Path, version: str, tag: str, repository: str, target_sha: str
) -> Path:
    validate_version(version)
    validate_target_sha(target_sha)
    if tag != f"v{version}":
        raise ValueError("release tag must be v followed by the release version")
    if repository.count("/") != 1:
        raise ValueError("repository must use the owner/name format")

    platforms = {}
    for target, (asset_name, signature_name) in updater_assets(version).items():
        signature = (directory / signature_name).read_text(encoding="utf-8").strip()
        if len(signature) < 32 or "\x00" in signature:
            raise ValueError(f"invalid updater signature: {signature_name}")
        platforms[target] = {
            "signature": signature,
            "url": (
                f"https://github.com/{repository}/releases/download/"
                f"{quote(tag, safe='')}/{quote(asset_name, safe='')}"
            ),
        }

    manifest = {
        "version": version,
        "notes": f"Streamlink Twitch GUI {version}",
        "pub_date": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_commit": target_sha,
        "platforms": platforms,
    }
    output = directory / "latest.json"
    output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return output


def npm_components(package_lock: Path) -> list[dict]:
    lock = json.loads(package_lock.read_text(encoding="utf-8"))
    components = []
    seen = set()
    for package_path, package in sorted(lock.get("packages", {}).items()):
        if not package_path or "node_modules/" not in package_path:
            continue
        name = package.get("name") or package_path.rsplit("node_modules/", 1)[-1]
        version = package.get("version")
        if not version:
            continue
        purl = f"pkg:npm/{quote(name, safe='/')}@{version}"
        if purl in seen:
            continue
        seen.add(purl)
        component = {
            "type": "library",
            "bom-ref": purl,
            "name": name,
            "version": version,
            "purl": purl,
        }
        if package.get("license"):
            component["licenses"] = [{"expression": package["license"]}]
        components.append(component)
    return components


def cargo_components(cargo_lock: Path) -> list[dict]:
    lock = tomllib.loads(cargo_lock.read_text(encoding="utf-8"))
    components = []
    seen = set()
    for package in sorted(
        lock.get("package", []), key=lambda item: (item["name"], item["version"])
    ):
        name = package["name"]
        version = package["version"]
        purl = f"pkg:cargo/{quote(name, safe='')}@{version}"
        if purl in seen:
            continue
        seen.add(purl)
        components.append(
            {
                "type": "library",
                "bom-ref": purl,
                "name": name,
                "version": version,
                "purl": purl,
            }
        )
    return components


def write_sbom(
    directory: Path,
    version: str,
    package_lock: Path,
    cargo_lock: Path,
    target_sha: str,
) -> Path:
    validate_version(version)
    validate_target_sha(target_sha)
    components = npm_components(package_lock) + cargo_components(cargo_lock)
    for path in sorted(directory.iterdir()):
        if not path.is_file() or path.name in {
            checksum_name(version),
            sbom_name(version),
        }:
            continue
        components.append(
            {
                "type": "file",
                "bom-ref": f"release-file:{path.name}",
                "name": path.name,
                "hashes": [{"alg": "SHA-256", "content": sha256(path)}],
            }
        )

    document = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": f"urn:uuid:{uuid.uuid5(uuid.NAMESPACE_URL, f'streamlink-twitch-gui:{version}')}",
        "version": 1,
        "metadata": {
            "timestamp": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "component": {
                "type": "application",
                "name": "streamlink-twitch-gui",
                "version": version,
            },
            "properties": [
                {
                    "name": "io.github.streamlink-twitch-gui.source-commit",
                    "value": target_sha,
                }
            ],
        },
        "components": components,
    }
    output = directory / sbom_name(version)
    output.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    return output


def write_checksums(directory: Path, version: str) -> Path:
    output = directory / checksum_name(version)
    paths = sorted(
        path for path in directory.iterdir() if path.is_file() and path != output
    )
    output.write_text(
        "".join(f"{sha256(path)}  {path.name}\n" for path in paths),
        encoding="utf-8",
    )
    return output


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate release supply-chain metadata")
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--target-sha", required=True)
    parser.add_argument("--package-lock", type=Path, required=True)
    parser.add_argument("--cargo-lock", type=Path, required=True)
    args = parser.parse_args()
    write_updater_manifest(
        args.directory, args.version, args.tag, args.repository, args.target_sha
    )
    write_sbom(
        args.directory,
        args.version,
        args.package_lock,
        args.cargo_lock,
        args.target_sha,
    )
    write_checksums(args.directory, args.version)


if __name__ == "__main__":
    main()
