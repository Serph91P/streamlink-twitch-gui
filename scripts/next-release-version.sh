#!/usr/bin/env bash

set -euo pipefail

if [[ ${1-} == -- ]]; then
	shift
	tags=("$@")
elif (( $# > 0 )); then
	tags=("$@")
else
	mapfile -t tags < <(git tag --list)
fi

versions=()
for tag in "${tags[@]}"; do
	if [[ $tag =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
		versions+=("${tag#v}")
	fi
done

if (( ${#versions[@]} == 0 )); then
	printf '%s\n' "0.0.1"
	exit 0
fi

highest=$(printf '%s\n' "${versions[@]}" | sort --version-sort | tail -n 1)
IFS=. read -r major minor patch <<< "$highest"
printf '%s.%s.%s\n' "$major" "$minor" "$((patch + 1))"
