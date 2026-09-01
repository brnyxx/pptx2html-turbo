# v2.1.0 Release Notes Draft

See [`README.md`](./README.md) for the release-note workflow in this directory.
See [`pre-release-checklist.md`](./pre-release-checklist.md) before creating the tag.

## Suggested Title

`v2.1.0`

## Summary

v2.1.0 adds a format-neutral document engine and bounded native conversion for DOCX, DOC, XLSX, XLS, PPT, and PDF while preserving every existing PPTX API.

## Highlights

- Add the `document2html` CLI alongside the existing `pptx2html` binary in every GitHub release archive.
- Add `document2html-core`, a native LibreOffice/Poppler adapter, a Python module, and browser format detection with explicit backend capability reporting.
- Preserve the pure-Rust PPTX renderer and all existing Rust, Python, and npm/WASM PPTX APIs.
- Add deterministic legacy XLS calculation freezing and bounded spreadsheet display-value attribution.
- Add a fail-closed seven-format evaluation pipeline with immutable conformance, blind, and security corpora.
- Keep both npm package names on the same v2.1.0 browser build.

## Compatibility

This minor release is additive. Existing imports from `@briank-dev/pptx-to-html` and `@briank-dev/pptx2html-turbo`, direct WASM exports, the `pptx2html` CLI, and the existing Python module remain supported.

Native conversion of DOCX, DOC, XLSX, XLS, and PPT requires LibreOffice and Poppler. PDF conversion requires Poppler. Browser WASM detects all seven formats but converts only PPTX; native-only requests fail with an explicit backend-unavailable error.

## Validation

- Rust workspace formatting, Clippy, checks, tests, and release builds must pass from the tagged tree.
- Both Python wheels must install in a clean environment and pass their installed-module runtime smokes.
- Both npm package names must pass package-root import, real two-slide conversion, package contract, and `npm publish --dry-run`.
- The exactness contract and the seven-format portable evidence boundary must be reproducible from the release tree.
- The GitHub Pages demo must expose the v2.1.0 release and primary npm package.

This release does not add a Microsoft Office pixel-accuracy, PowerPoint pixel-match, byte-identical-output, or PPTX exact-tier claim.

The repository, package, browser, exactness, and publication boundaries are recorded in the
[v2.1.0 validation report](./v2.1.0-validation.md).

## Publication Scope

- GitHub Release archives containing both CLI binaries.
- Primary npm package `@briank-dev/pptx-to-html@2.1.0`.
- Legacy npm package `@briank-dev/pptx2html-turbo@2.1.0`.
- GitHub Pages WASM demo from `main`.
- Rust crates, both Python bindings, and the universal WASM crate remain source distributions; the repository does not define crates.io or PyPI publication jobs.

## Publish Status

- Target tag: `v2.1.0`.
- Tag and publish remain pending the final human approval required by [`pre-release-checklist.md`](./pre-release-checklist.md).
- The local validation report is not a publication receipt. Confirm GitHub Release, both npm
  package versions, and the deployed Pages demo after their workflows complete.
