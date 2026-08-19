# v2.0.1 Release Notes Draft

See [`README.md`](./README.md) for the release-note workflow in this directory.
See [`pre-release-checklist.md`](./pre-release-checklist.md) before creating the tag.

## Suggested Title

`v2.0.1`

## Summary

v2.0.1 gives the browser package a descriptive primary name and a simpler entrypoint while preserving the v2.0.0 conversion engine and every published low-level export.

## Highlights

- Publish `@briank-dev/pptx-to-html` as the primary browser package.
- Keep `@briank-dev/pptx2html-turbo` on the same version and API during migration.
- Add `pptxToHtml(input, moduleOrPath?)` with lazy WASM initialization and direct `Blob`, `ArrayBuffer`, or `Uint8Array` input.
- Share the first initialization attempt and its success or failure across concurrent calls, then retry initialization on a later call after failure.
- Preserve the default initializer and all existing low-level named exports.
- Validate facade entrypoints and both npm tarballs before publication.

## Compatibility

This patch release is additive. Existing imports from `@briank-dev/pptx2html-turbo` and direct `init`/`convert` usage remain supported.

## Validation

- Rust workspace formatting, Clippy, tests, and release builds must pass from the tagged tree.
- Python evaluation tests and the exactness contract must pass.
- Both npm package names must pass package-root import, real two-slide conversion, package contract, and `npm publish --dry-run`.
- The GitHub Pages demo must expose the v2.0.1 release and primary npm package.

The v2.0.0 evidence boundary remains unchanged. This patch release does not add a PowerPoint-native strict pixel-equality claim.

## Publication Scope

- GitHub Release with validated CLI artifacts.
- Primary npm package `@briank-dev/pptx-to-html@2.0.1`.
- Legacy npm package `@briank-dev/pptx2html-turbo@2.0.1`.
- GitHub Pages WASM demo from `main`.
- The repository does not currently define crates.io or PyPI publication jobs.

## Publish Status

- Target tag: `v2.0.1`.
- Tag and publish remain pending the final human approval required by [`pre-release-checklist.md`](./pre-release-checklist.md).
