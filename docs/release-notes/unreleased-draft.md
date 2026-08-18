# v2.0.0 Release Notes Draft

See [`README.md`](./README.md) for the release-note workflow in this directory.
See [`pre-release-checklist.md`](./pre-release-checklist.md) before creating the tag.

## Suggested Title

`v2.0.0`

## Summary

v2.0.0 is the first semver-major release after the converter expanded its typed PPTX preservation model, deterministic fallback diagnostics, and visual-fidelity evaluation surface. It ships the public API changes that were intentionally held for a major version.

## Highlights

- Cover all 56 bounded semantic capability-matrix entries with no semantic `unparsed` state; no entry claims `exact`.
- Preserve notes and comments, actions, timing and transitions, bounded media, advanced effects, table styles, embedded package metadata, custom XML, presentation extensions, synchronization metadata, thumbnails, theme overrides, and user-defined tags through typed models or deterministic fallbacks.
- Improve preset geometry, degenerate arc handling, overflowing shapes, brackets, mixed-script RTL paragraphs, chart fallbacks, and diagnostic ordering on real-world decks.
- Add exhaustive adjustment coverage for all 300 official preset/adjustment pairs across 187 presets and 900 deterministic low/default/high cases.
- Refresh the GitHub Pages demo with isolated timing/action runtime execution, canonical diagnostic counts, current release links, a fit-width default, and accessible status, zoom, and output-frame contracts.

## Breaking Changes

- Add `Bullet::Picture`; exhaustive `Bullet` matches must add an arm.
- Add public action fields and enums, including `Shape::actions`, `TextRun::actions`, and `FallbackKind::ActionMetadata`.
- Add table-style and merge metadata to public table structs and store `TableStyleReference::definition` as `Option<Box<TableStyle>>`.
- Add presentation-owned embedded inventory and other typed metadata fields that require external struct literals to migrate or use `..Default::default()`.
- Add ordered `ConversionResult::diagnostics`; external Rust consumers must construct results with `ConversionResult::new(html, slide_count)` instead of struct literals.

## Validation

- Rust workspace formatting, Clippy, tests, and release builds must pass from the tagged tree.
- Python evaluation helper tests, exactness-contract generation, wheel build/install smoke tests, and WASM package contract/runtime smoke tests must pass.
- Real Chrome, Firefox, and WebKit load the Pages artifact and convert the same real 10-slide deck with ordered diagnostics, isolated renderer scripts, and no console or page errors.
- The external seven-deck real-world corpus converts all 186 slides. LibreOffice proxy comparison records a 96.843607% mean and an 88.967165% minimum with no blank, corrupt, or missing slide pairs observed.
- The exhaustive adjustment proxy records a 98.926092% mean and a 98.265724% minimum across 900 cases.
- [`v2.0.0-validation.md`](./v2.0.0-validation.md) records the environment, evidence boundary, report hashes, and strict PowerPoint-native blocker for these figures.

LibreOffice scores are proxy evidence only. This release does not claim PowerPoint-native strict pixel equality without genuine Windows PowerPoint reference exports and validated provenance.

## Publication Scope

- GitHub Release with validated CLI artifacts.
- Public npm package `@briank-dev/pptx2html-turbo@2.0.0`.
- GitHub Pages WASM demo from `main`.
- The repository does not currently define crates.io or PyPI publication jobs.

## Publish Status

- Target tag: `v2.0.0`.
- Tag and publish remain pending the final human approval required by [`pre-release-checklist.md`](./pre-release-checklist.md).
