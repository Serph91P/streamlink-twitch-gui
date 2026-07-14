#!/usr/bin/env python3
import argparse
import json
import os
from collections.abc import Callable
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen

from release_common import validate_target_sha, validate_version


FetchJson = Callable[[str, bool], dict | None]


def _repository_path(repository: str) -> str:
    parts = repository.split("/")
    if len(parts) != 2 or not all(parts):
        raise ValueError(f"invalid GitHub repository: {repository!r}")
    return "/".join(quote(part, safe="") for part in parts)


def _validate_release_id(release_id: str | None) -> str:
    if (
        not isinstance(release_id, str)
        or not release_id.isascii()
        or not release_id.isdecimal()
        or release_id.startswith("0")
    ):
        raise ValueError("release ID must be a positive decimal integer")
    return release_id


def _git_object(payload: dict, source: str) -> tuple[str, str]:
    try:
        object_type = payload["object"]["type"]
        sha = payload["object"]["sha"]
    except (KeyError, TypeError) as error:
        raise ValueError(f"invalid GitHub tag response from {source}") from error
    if object_type not in {"commit", "tag"}:
        raise ValueError(f"tag resolves to unsupported Git object type {object_type!r}")
    return object_type, validate_target_sha(sha)


def resolve_tag_commit(
    repository: str, tag: str, fetch_json: FetchJson
) -> str | None:
    repository = _repository_path(repository)
    tag = quote(tag, safe="")
    ref_path = f"/repos/{repository}/git/ref/tags/{tag}"
    payload = fetch_json(ref_path, True)
    if payload is None:
        return None

    object_type, sha = _git_object(payload, ref_path)
    seen = set()
    while object_type == "tag":
        if sha in seen:
            raise ValueError("annotated tag chain contains a cycle")
        seen.add(sha)
        tag_path = f"/repos/{repository}/git/tags/{sha}"
        payload = fetch_json(tag_path, False)
        if payload is None:
            raise ValueError(f"annotated tag object {sha} does not exist")
        object_type, sha = _git_object(payload, tag_path)
    return sha


def verify_tag(
    repository: str,
    tag: str,
    target_sha: str,
    fetch_json: FetchJson,
    allow_missing: bool = False,
    require_draft_release: bool = False,
    release_id: str | None = None,
) -> str | None:
    target_sha = validate_target_sha(target_sha)
    if not tag.startswith("v"):
        raise ValueError("release tag must use the v<version> format")
    validate_version(tag[1:])
    tag_sha = resolve_tag_commit(repository, tag, fetch_json)
    if tag_sha is None:
        if not allow_missing and not require_draft_release:
            raise ValueError(f"release tag {tag} does not exist")
    elif tag_sha != target_sha:
        raise ValueError(
            f"release tag {tag} resolves to {tag_sha}, which does not match target {target_sha}"
        )

    if require_draft_release:
        repository_path = _repository_path(repository)
        release_id = _validate_release_id(release_id)
        release_path = f"/repos/{repository_path}/releases/{release_id}"
        release = fetch_json(release_path, False)
        try:
            release_url = release["url"]
            response_id = release["id"]
            is_draft = release["draft"]
            release_tag = release["tag_name"]
            target_commitish = release["target_commitish"]
        except (KeyError, TypeError) as error:
            raise ValueError(
                f"invalid GitHub draft release response from {release_path}"
            ) from error
        expected_url = f"https://api.github.com{release_path}"
        if release_url != expected_url:
            raise ValueError(
                f"draft release URL {release_url!r} does not match expected repository"
            )
        if type(response_id) is not int or response_id != int(release_id):
            raise ValueError(
                f"draft release ID {response_id!r} does not match expected ID {release_id}"
            )
        if is_draft is not True:
            raise ValueError(f"release {tag} is not a draft")
        if release_tag != tag:
            raise ValueError(
                f"draft release tag {release_tag!r} does not match expected tag {tag!r}"
            )
        if target_commitish != target_sha:
            raise ValueError(
                f"draft release target {target_commitish!r} does not match target {target_sha}"
            )
    return tag_sha


def github_fetch_json(token: str) -> FetchJson:
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28",
    }

    def fetch_json(path: str, allow_not_found: bool = False) -> dict | None:
        request = Request(f"https://api.github.com{path}", headers=headers)
        try:
            with urlopen(request, timeout=30) as response:
                payload = json.load(response)
        except HTTPError as error:
            if allow_not_found and error.code == 404:
                return None
            raise RuntimeError(
                f"GitHub API request for {path} failed with HTTP {error.code}"
            ) from error
        if not isinstance(payload, dict):
            raise ValueError(f"invalid GitHub API response from {path}")
        return payload

    return fetch_json


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify that a release tag resolves to the target commit"
    )
    parser.add_argument("--repository", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target-sha", required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--allow-missing", action="store_true")
    mode.add_argument("--require-draft-release", action="store_true")
    parser.add_argument("--release-id")
    args = parser.parse_args()

    token = os.environ.get("GH_TOKEN")
    if not token:
        raise SystemExit("GH_TOKEN is required")
    result = verify_tag(
        args.repository,
        args.tag,
        args.target_sha,
        github_fetch_json(token),
        args.allow_missing,
        args.require_draft_release,
        args.release_id,
    )
    if args.require_draft_release:
        print(f"verified draft release {args.tag} for target {args.target_sha}")
    elif result is None:
        print(f"release tag {args.tag} does not exist and may be created")
    else:
        print(f"verified release tag {args.tag} at {result}")


if __name__ == "__main__":
    main()
