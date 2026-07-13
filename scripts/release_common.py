#!/usr/bin/env python3
import re


VERSION_RE = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
TARGET_SHA_RE = re.compile(r"[0-9a-f]{40}")
PRODUCT = "streamlink-twitch-gui"


def validate_version(version: str) -> str:
    if not VERSION_RE.fullmatch(version):
        raise ValueError(f"invalid release version: {version!r}")
    return version


def validate_target_sha(target_sha: str) -> str:
    if not TARGET_SHA_RE.fullmatch(target_sha):
        raise ValueError("target SHA must be a lowercase 40-character commit SHA")
    return target_sha


def platform_asset_names(version: str) -> dict[str, set[str]]:
    validate_version(version)
    prefix = f"{PRODUCT}_{version}"
    return {
        "linux-x64": {
            f"{prefix}_linux_x64.AppImage",
            f"{prefix}_linux_x64.deb",
        },
        "windows-x64": {
            f"{prefix}_windows_x64-setup.exe",
            f"{prefix}_windows_x64.msi",
        },
        "macos-x64": {f"{prefix}_macos_x64.dmg"},
        "macos-arm64": {f"{prefix}_macos_arm64.dmg"},
    }


def checksum_name(version: str) -> str:
    validate_version(version)
    return f"{PRODUCT}_{version}_SHA256SUMS.txt"


def sbom_name(version: str) -> str:
    validate_version(version)
    return f"{PRODUCT}_{version}.cdx.json"


def expected_asset_names(version: str) -> set[str]:
    names = set().union(*platform_asset_names(version).values())
    names.update({checksum_name(version), sbom_name(version)})
    return names
