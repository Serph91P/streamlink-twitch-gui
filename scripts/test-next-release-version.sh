#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
calculator="$script_dir/next-release-version.sh"

assert_version() {
	local description=$1
	local expected=$2
	shift 2
	local actual
	actual=$("$calculator" "$@")

	if [[ $actual != "$expected" ]]; then
		printf 'FAIL: %s (expected %s, got %s)\n' "$description" "$expected" "$actual" >&2
		exit 1
	fi
	printf 'PASS: %s\n' "$description"
}

assert_version "empty tags" "0.0.1" --
assert_version "malformed and prerelease tags are ignored" "1.2.4" \
	latest v1.2 v1.2.3-rc.1 v1.2.3+build v01.2.4 v1.02.4 v1.2.04 v1.2.3
assert_version "numeric ordering crosses patch digit boundaries" "0.0.11" \
	v0.0.1 v0.0.10 v0.0.9
assert_version "releases share one tag history" "0.0.4" \
	v0.0.1 v0.0.2 v0.0.3
