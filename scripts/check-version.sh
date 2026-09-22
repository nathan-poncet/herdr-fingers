#!/usr/bin/env bash
# Fails unless Cargo.toml, herdr-plugin.toml and (optionally) the given tag
# agree on the version.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
manifest_version=$(sed -n 's/^version = "\(.*\)"/\1/p' herdr-plugin.toml | head -1)

if [[ "$cargo_version" != "$manifest_version" ]]; then
  echo "Cargo.toml says $cargo_version but herdr-plugin.toml says $manifest_version" >&2
  exit 1
fi
if [[ $# -ge 1 && "$1" != "$cargo_version" ]]; then
  echo "tag says $1 but Cargo.toml says $cargo_version" >&2
  exit 1
fi
echo "version $cargo_version"
