#!/usr/bin/env bash
# Prints the CHANGELOG.md section of the given version; exits 1 if absent.
set -euo pipefail
cd "$(dirname "$0")/.."
version="${1:?usage: release-notes.sh <version>}"

notes=$(awk -v v="$version" '
  /^## \[/ { printing = index($0, "## [" v "]") == 1 }
  printing && !/^## \[/ { print }
' CHANGELOG.md | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')

if [[ -z "${notes//[[:space:]]/}" ]]; then
  echo "no CHANGELOG section for $version" >&2
  exit 1
fi
printf '%s\n' "$notes"
