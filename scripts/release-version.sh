#!/usr/bin/env bash
# Print the version used in package metadata and release asset names.
# A numeric release tag (v1.2.3, 1.2.3-rc.1) gives that version with `-` and
# `+` turned into dots, which deb, pacman and rpm all accept. Any other build
# gets a unique development version: <Cargo version>.dev.<run>.<short sha>.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
base_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_root/crates/captureport/Cargo.toml" | head -n1)
version="${base_version}.dev.${GITHUB_RUN_NUMBER:-0}.$(printf %.7s "${GITHUB_SHA:-local}")"
if [[ "${GITHUB_REF_TYPE:-}" == tag && "${GITHUB_REF_NAME:-}" =~ ^v?([0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?)$ ]]; then
  version=${BASH_REMATCH[1]//-/.}
  version=${version//+/.}
fi
printf '%s\n' "$version"
