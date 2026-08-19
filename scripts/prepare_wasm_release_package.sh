#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <package-version>" >&2
  exit 1
fi

PACKAGE_VERSION="$1"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG_DIR="$REPO_ROOT/crates/pptx2html-wasm/pkg"

required_env_vars=(
  PACKAGE_NAME
  PACKAGE_AUTHOR
  PACKAGE_HOMEPAGE
  PACKAGE_BUGS_URL
  PACKAGE_DESCRIPTION
)

for var_name in "${required_env_vars[@]}"; do
  if [ -z "${!var_name:-}" ]; then
    echo "missing required environment variable: $var_name" >&2
    exit 1
  fi
done

cd "$REPO_ROOT"

wasm-pack build crates/pptx2html-wasm --target web --release --scope pptx2html
cp crates/pptx2html-wasm/README.md "$PKG_DIR/"
cp LICENSE "$PKG_DIR/"
cp crates/pptx2html-wasm/npm/index.js "$PKG_DIR/"
cp crates/pptx2html-wasm/npm/index.d.ts "$PKG_DIR/"

PACKAGE_VERSION="$PACKAGE_VERSION" node -e '
  const fs = require("fs");
  const pkgPath = "./crates/pptx2html-wasm/pkg/package.json";
  const pkg = require(pkgPath);
  pkg.name = process.env.PACKAGE_NAME;
  pkg.version = process.env.PACKAGE_VERSION;
  pkg.main = "./index.js";
  pkg.module = "./index.js";
  pkg.types = "./index.d.ts";
  pkg.files = [...new Set([...(pkg.files || []), "index.js", "index.d.ts"])];
  pkg.description = process.env.PACKAGE_DESCRIPTION;
  pkg.keywords = ["pptx", "powerpoint", "html", "converter", "wasm", "presentation", "ooxml", "ecma-376"];
  pkg.author = process.env.PACKAGE_AUTHOR;
  pkg.homepage = process.env.PACKAGE_HOMEPAGE;
  const homepage = String(process.env.PACKAGE_HOMEPAGE || "").replace(/\/$/, "");
  if (homepage) {
    const repoUrl = homepage.startsWith("git+")
      ? homepage
      : `git+${homepage.endsWith(".git") ? homepage : `${homepage}.git`}`;
    pkg.repository = { type: "git", url: repoUrl };
  }
  pkg.bugs = { url: process.env.PACKAGE_BUGS_URL };
  pkg.exports = {
    ".": {
      import: "./index.js",
      types: "./index.d.ts"
    }
  };
  fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
'

node crates/pptx2html-wasm/tests/node-smoke.mjs
node crates/pptx2html-wasm/tests/package-root-smoke.mjs "$PACKAGE_NAME"
node crates/pptx2html-wasm/tests/check-package-contract.mjs \
  "$PKG_DIR" \
  "$PACKAGE_VERSION" \
  "$PACKAGE_NAME"
