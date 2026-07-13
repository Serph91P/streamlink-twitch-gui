#!/usr/bin/env python3
import argparse
import re
from pathlib import Path


USES_RE = re.compile(r"^\s*(?:-\s*)?uses:\s*([^\s#]+)(?:\s+#\s*(\S.*))?$")
PIN_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}$")
VERSION_COMMENT_RE = re.compile(r"^v[0-9]+(?:\.[0-9]+){0,2}(?:\s|$)")


def validate_uses_line(line: str, filename: str, line_number: int) -> str | None:
    match = USES_RE.match(line)
    if not match:
        return None
    reference, comment = match.groups()
    location = f"{filename}:{line_number}"
    if reference.startswith("./"):
        return None
    if not PIN_RE.fullmatch(reference):
        return f"{location}: action must use a full 40-character commit SHA"
    if not comment or not VERSION_COMMENT_RE.match(comment):
        return f"{location}: pinned action must have a version comment"
    return None


def verify_workflows(directory: Path) -> list[str]:
    errors = []
    workflows = sorted(directory.glob("*.yml")) + sorted(directory.glob("*.yaml"))
    for workflow in workflows:
        for line_number, line in enumerate(
            workflow.read_text(encoding="utf-8").splitlines(), 1
        ):
            error = validate_uses_line(line, str(workflow), line_number)
            if error:
                errors.append(error)
    return errors


def main() -> None:
    parser = argparse.ArgumentParser(description="Reject unpinned GitHub Actions")
    parser.add_argument(
        "directory", type=Path, nargs="?", default=Path(".github/workflows")
    )
    args = parser.parse_args()
    errors = verify_workflows(args.directory)
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"verified action pins in {args.directory}")


if __name__ == "__main__":
    main()
