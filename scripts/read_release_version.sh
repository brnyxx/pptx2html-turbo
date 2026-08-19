#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

version_from_file() {
  local file="$1"
  sed -n 's/^version = "\(.*\)"/\1/p' "$file" | head -1
}

declare -a version_sources=(
  "$REPO_ROOT/crates/pptx2html-core/Cargo.toml"
  "$REPO_ROOT/crates/pptx2html-cli/Cargo.toml"
  "$REPO_ROOT/crates/pptx2html-py/Cargo.toml"
  "$REPO_ROOT/crates/pptx2html-wasm/Cargo.toml"
  "$REPO_ROOT/crates/pptx2html-py/pyproject.toml"
)

declare -a seen_versions=()
for source in "${version_sources[@]}"; do
  version="$(version_from_file "$source")"
  if [ -z "$version" ]; then
    echo "failed to read version from $source" >&2
    exit 1
  fi
  seen_versions+=("$version")
done

unique_versions="$(printf '%s\n' "${seen_versions[@]}" | sort -u)"
if [ "$(printf '%s\n' "$unique_versions" | wc -l | tr -d ' ')" -ne 1 ]; then
  echo "release version mismatch across package manifests:" >&2
  for i in "${!version_sources[@]}"; do
    printf '  %s -> %s\n' "${version_sources[$i]}" "${seen_versions[$i]}" >&2
  done
  exit 1
fi

release_version="$unique_versions"

demo_file="$REPO_ROOT/crates/pptx2html-wasm/demo/index.html"
demo_versions="$(grep -Eo 'v[0-9]+\.[0-9]+\.[0-9]+' "$demo_file" | sort -u || true)"
if [ -z "$demo_versions" ]; then
  echo "failed to read demo version from $demo_file" >&2
  exit 1
fi
if [ "$(printf '%s\n' "$demo_versions" | wc -l | tr -d ' ')" -ne 1 ] ||
  [ "$demo_versions" != "v$release_version" ]; then
  echo "demo version mismatch: $demo_versions does not match manifest version v$release_version" >&2
  exit 1
fi

if [ "${1:-}" != "" ]; then
  tag="${1#refs/tags/}"
  if [ "${tag#v}" = "$tag" ]; then
    echo "release tag must start with v: $1" >&2
    exit 1
  fi
  if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "release tag must match vMAJOR.MINOR.PATCH: $1" >&2
    exit 1
  fi
  if [ "${tag#v}" != "$release_version" ]; then
    echo "release tag $tag does not match manifest version $release_version" >&2
    exit 1
  fi
fi

printf '%s\n' "$release_version"
