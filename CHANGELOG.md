# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [2.1.0] - 2026-08-31

### Added

- Add the format-neutral `document2html-core` API and bounded native conversion for DOCX, DOC, XLSX, XLS, PPT, and PDF while retaining the pure-Rust PPTX path
- Add the `document2html` CLI binary, Python module, and browser WASM format-detection/capability surface
- Add a fail-closed seven-format evaluation pipeline with immutable conformance, blind, and security corpora plus signed portable capture contracts

### Changed

- Package both `pptx2html` and `document2html` in GitHub release archives and validate both Python wheels before release creation
- Enforce one release version across all eight Rust crates, the Python project metadata, and the browser demo
- Freeze legacy XLS calculation state before native rendering and preserve bounded spreadsheet number-format semantics across Rust and portable reference extraction

### Fixed

- Preserve owner-specific XML fallback behavior while retaining package-wide size and DOCTYPE safety checks
- Normalize invalid Poppler font-name bytes without rejecting otherwise complete UTF-8 PDF conversion output
- Exclude an upstream-declared broken Apache POI PPT fixture from positive corpus selection and admit the next valid pinned source without changing producer quotas

## [2.0.2] - 2026-08-26

### Changed

- Update repository metadata to the new GitHub namespace `brnyxx/pptx2html-turbo` (username change; old `kim62210` URLs redirect)

## [2.0.1] - 2026-08-19

### Added

- Publish the browser package primarily as `@briank-dev/pptx-to-html`
- Add `pptxToHtml(input, moduleOrPath?)` for lazy WASM initialization and direct `Blob`, `ArrayBuffer`, or `Uint8Array` conversion

### Changed

- Continue publishing `@briank-dev/pptx2html-turbo` with the same version and API during migration
- Validate facade entrypoints, npm tarball contents, and both package names in CI and release workflows
- Make concurrent calls share the first initialization attempt and its success or failure, then retry initialization on a later call after failure
- Harden manual npm publication inputs, pin the publish-time `wasm-pack` version, and make partial dual-publish runs resumable

## [2.0.0] - 2026-08-18

### Timing and transitions

- Preserve ordered slide transition/timing XML and approximately execute only bounded cut/fade and click/with-previous/after-previous appear, disappear, or fade behavior on resolved slide shapes
- Keep automatic advance, loops, unbounded effects, unsupported commands, and unresolved targets inert with typed `PRESENTATIONML_TIMING_FALLBACK` diagnostics
- Preserve finite start-condition delays up to 10000 ms and exact raw unsupported timing nodes in typed fallback diagnostics
- Keep timing inventory private to conversion so the pre-v2.0 public `Slide`, capability enums, and existing `TimingInventory` API remain source-compatible

### Breaking API changes

- Add the public `Bullet::Picture` variant for typed DrawingML picture bullets.
- Rust consumers with exhaustive `Bullet` matches must add a `Bullet::Picture` arm.
- Ship this public enum change in v2.0.0; consumers upgrading from v1.x must update exhaustive matches.
- Add `Presentation::embedded_inventory` as typed presentation-owned fallback state while retaining the original public `model::embedded::EmbeddedInventory` marker. Existing exhaustive `Presentation` literals must add `embedded_inventory: Default::default()` (or switch to `..Default::default()`); an external-crate compile contract pins both the legacy `E0063` failure and the migrated literal. This avoids ambient parser state and semantic overloading of unrelated public fields.
- Add ordered `ConversionResult::diagnostics`; consumers upgrading from v1.x must replace external struct literals with `ConversionResult::new(html, slide_count)` and populate metadata fields afterwards.
- Restrict embedded previews to a dependency-free safe PNG subset: CRC-valid IHDR/IDAT/IEND-only, bounded 8-bit non-interlaced RGBA with stored-zlib/filter-0 scanlines. Other PNG forms and JPEG/GIF/WebP previews fall back to placeholders.
- Bound unknown package-part inventory to 128 sorted entries and 32 KiB of part-name metadata, followed by one deterministic omitted-count diagnostic.

### Rendering / Public API
- Preserve slide notes, notes-master associations, legacy comments/authors, and modern comments/authors as typed off-canvas metadata in the existing deterministic diagnostics JSON
- Retain exact comment text when authors are unresolved with `COMMENT_AUTHOR_UNRESOLVED`, reject unsafe or spoofed annotation relationships before package access, and preserve unknown modern comment extensions as raw fallback XML
- Preserve typed annotation records outside public `Presentation` so existing exhaustive struct literals remain source-compatible
- Preserve click and mouse-over actions as typed `ActionSet` data across shapes, pictures, connectors, graphic frames, and shape/table text runs
- Render only strict product-allowlisted `http`, `https`, and `mailto` links, use actual presentation slide order for internal navigation, and keep unsafe or unsupported actions inert with stable diagnostics
- Preserve group and table graphic-frame action ownership, require exact PresentationML owner stacks, and use stable owner-derived run/table-cell diagnostic identities
- Keep safe legacy run hyperlinks pointer-reachable above enclosing shape, group, and table actions without enabling plain or unsafe legacy runs
- Add public `Shape::actions`, `TextRun::actions`, typed action enums, and `FallbackKind::ActionMetadata`; consumers upgrading from v1.x must migrate exhaustive matches and struct literals
- Resolve package-defined DrawingML table styles in Office region precedence order, including theme-aware fills, text, outer/inside borders, explicit-cell overrides, and logical merged-cell coordinates
- Preserve unavailable built-in and invalid table style IDs plus all six flags in `TABLE_STYLE_DEFINITION_UNAVAILABLE` diagnostics without synthesizing Office appearances
- Add `TableData::style`, `TableCell::h_merge`, `TableCell::explicit_borders`, `TableCellStyle::fill_ref`, `TableStyle::table_background_ref`, and `TableStyle::unsupported_references`; store `TableStyleReference::definition` as `Option<Box<TableStyle>>` so table metadata does not inflate every `ShapeType`; consumers upgrading from v1.x must migrate external struct literals
- Reject unsafe/external table-style relationships and invalid table-style XML with stable diagnostics, and preserve per-table diagnostic identity from `cNvPr`
- Preserve table-style `fillRef` index/color/modifiers; resolve parsed theme fills and diagnose unavailable non-solid theme fills without inventing a solid replacement or claiming exact non-solid resolution
- Preserve scoped table-style `tblBg/effectRef` and border-side `lnRef` index/color/modifiers and diagnose them as unsupported without discarding sibling styles or inventing effects/lines

### Validation

- Cover all 56 bounded semantic capability-matrix entries with no semantic `unparsed` state; no entry is `exact`
- Exercise all 300 official preset/adjustment pairs across 187 presets through 900 deterministic low/default/high cases and keep every official adjustment key classified
- Validate seven canonical external real-world decks containing 186 slides without conversion failures, blank slides, corrupt renders, or missing candidate/reference pairs
- Record a 96.843607% LibreOffice proxy corpus mean and 88.967165% minimum for that external corpus in the [v2.0.0 validation report](docs/release-notes/v2.0.0-validation.md); this is proxy evidence and does not replace PowerPoint-native strict comparison
- Keep PowerPoint-native pixel equality behind native Windows PowerPoint references and provenance validation
- Keep the Windows CLI on stable Rust by using safe cross-platform hard-link identity checks instead of nightly-only metadata APIs
- Install the complete evaluation dependency set and Chromium runtime before CI, release, and npm publication browser tests
- Keep official supplement bytes, validation, and timing event sequencing deterministic across Windows and Linux hosted runners
- Canonicalize rounded negative-zero SVG coordinates so official preset paths serialize identically across operating systems

### Fixtures / Documentation
- Extend `notes-comments.pptx` with a modern comment extension payload and escaped script-closing boundary for raw-metadata safety coverage
- Expand `actions.pptx` to three presentation-ordered slides with nonsequential part names plus external, navigation, hover, media, blocked, shape, nested group, table-frame/table-run, picture, and connector stimuli
- Expand the completion table deck to a region matrix with explicit fill/noFill overrides and a horizontal merge row
- Keep header/footer-relative row and column band origins approximate without claiming PowerPoint equivalence
- Synchronize README, architecture, binding, evaluation, release, and issue-reporting documentation with the v2.0.0 public surfaces and publication scope
- Update the GitHub Pages demo to execute isolated timing/action runtimes, display canonical diagnostic counts, expose current release links, default to fit-width zoom, and enforce accessible status, zoom, and output-frame contracts
- Replace unsupported `pixel-perfect` npm marketing copy with the evidence-backed `high-fidelity` description

## [1.1.0] - 2026-04-14

### Rendering / Public API
- Add exact-layout whole-slide scale across the Rust core, CLI, Python bindings, and WASM/browser APIs
- Keep scale image-like by enlarging the whole slide surface without recomputing coordinates or reflowing text

### Demo / Docs
- Add slider + numeric whole-slide zoom controls to the browser WASM demo
- Update the root README and package README examples to document the released scale parameter and no-reflow semantics

### Release Prep / CI
- Reserve the `1.1.0` package line across Cargo, Python, and WASM manifests so tag validation cannot drift back onto the published `1.0.5` line
- Run CI on `feature/slide-scale-output` during release prep so the branch receives the same multi-platform verification signal before merge
- Upgrade GitHub Actions workflow dependencies to Node 24-compatible major versions across CI, release, npm publish, and demo deploy lanes
- Normalize the generated npm package repository metadata and document a local `npm publish --dry-run` fallback when workflow-dispatch permissions are unavailable

## [1.0.5] - 2026-04-14

### Rendering — Text Fidelity
- Detect unbreakable tokens that span adjacent text runs before opting into emergency wrapping
- Honor paragraph-level default font sizes when classifying narrow autofit text for emergency wrapping
- Honor inherited text-style font sizes when classifying narrow autofit text for emergency wrapping
- Keep `spAutoFit` text bodies on the grow-to-fit path instead of forcing emergency wrapping for long unbreakable tokens
- Preserve inherited `lnSpcReduction` when child `normAutofit` overrides only change `fontScale`
- Treat non-breaking spaces as unbreakable during wrap classification
- Treat soft hyphen as a discretionary break opportunity during wrap classification
- Treat fullwidth and ideographic forms as East Asian break opportunities during wrap classification
- Treat mixed East Asian/Latin script boundaries as natural break opportunities during wrap classification
- Keep CJK non-starter punctuation attached to the preceding glyph during wrap classification
- Treat slash-separated text as having ordinary break opportunities during wrap classification
- Treat hyphen-separated text as having ordinary break opportunities during wrap classification
- Keep CJK opening punctuation attached to the following glyph during wrap classification
- Keep CJK closing angle-bracket punctuation attached to the preceding glyph during wrap classification
- Keep CJK white square brackets on the same East Asian punctuation cluster during wrap classification
- Keep CJK tortoise-shell brackets on the same East Asian punctuation cluster during wrap classification
- Keep CJK lenticular brackets on the same East Asian punctuation cluster during wrap classification

### Tests
- Add regressions for mixed-font split tokens in text metrics and rendered HTML wrap behavior
- Add regressions for paragraph-default font sizes affecting mixed-font autofit wrap decisions
- Add regressions for inherited text-style font sizes affecting mixed-font autofit wrap decisions
- Add regressions for `spAutoFit` long-token growth semantics versus emergency wrap fallback
- Add regressions for partial `normAutofit` inheritance when child placeholders override only `fontScale`
- Add regressions for NBSP-separated text in text metrics and rendered HTML wrap behavior
- Add regressions for soft-hyphenated text in text metrics and rendered HTML wrap behavior
- Add regressions for fullwidth text in text metrics and rendered HTML wrap behavior
- Add regressions for mixed East Asian/Latin text in text metrics, rendered HTML wrap behavior, and inherited autofit placeholder paths
- Add regressions for CJK non-starter punctuation clusters in text metrics and rendered HTML wrap behavior
- Add regressions for slash-separated text in text metrics and rendered HTML wrap behavior
- Add regressions for hyphen-separated text in text metrics and rendered HTML wrap behavior
- Add regressions for CJK opening punctuation clusters in text metrics and rendered HTML wrap behavior
- Add regressions for CJK angle-bracket punctuation clusters in text metrics and rendered HTML wrap behavior
- Add regressions for CJK white square bracket clusters in text metrics and rendered HTML wrap behavior
- Add regressions for CJK tortoise-shell bracket clusters in text metrics and rendered HTML wrap behavior
- Add regressions for CJK lenticular bracket clusters in text metrics and rendered HTML wrap behavior

### Docs / Exactness Contract
- Clarify the text/layout exactness gate around narrow-wrap, mixed-font, and autofit expectations
- Guard the documented text-layout fixture bundle against drift from `evaluate/powerpoint_evidence.py`

### Rendering — Charts
- Render clustered, stacked, and percent-stacked bar/column charts directly
- Honor OOXML `gapWidth` and `overlap` spacing for direct bar/column chart rendering
- Render first-pass bar/column chart data labels for value, category, series name, percent-stacked percentages, and basic label positions
- Render simple line charts directly
- Honor explicit line-series marker settings, including `symbol="none"`
- Render first-pass line and area point labels, including basic label positions
- Render simple scatter charts directly, including marker/line style variants and first-pass point labels with basic label positions
- Render direct chart axis titles for category and value axes
- Render simple standard area charts directly
- Render flat area3D charts through the existing area renderer
- Render simple single-series bubble charts directly with bounded bubbleScale support, while keeping width semantics on fallback
- Render bounded multi-series radar charts directly when data labels are absent
- Render bounded single-series ofPie charts directly for `ofPieType=pie` and `splitType=pos`
- Render simple single-series pie charts directly
- Render simple single-series doughnut charts directly
- Keep multi-series pie and unsupported chart families on stable preview/placeholder fallback paths, while flattening simple single-series 3D pie charts through the existing pie renderer
- Load chart-part preview images when unsupported chart families expose image relationships, so fallback rendering can use images before dropping to placeholders

### Tests
- Add chart integration coverage for clustered, stacked, percent-stacked, line, area3D, bubble, radar, ofPie, and pie direct-rendering paths
- Add regression coverage for bar/column spacing controls, direct chart data labels and positions, scatter rendering, line marker handling, axis titles, area charts, and doughnut direct rendering
- Add regression coverage for chart fallback behavior when direct rendering is not supported
- Add regression coverage for chart-part preview-image fallback before placeholder fallback
- Add installed-wheel Python smoke coverage for public conversion APIs, metadata URLs, bytes error paths, and one-based slide filtering
- Add WASM regression coverage for JSON escaping, package-root import smoke, publish contract checks, and tag/version validation

### CI / Evaluation
- Attach `powerpoint-evidence-summary.json` to tag-based GitHub Release artifacts
- Attach `powerpoint-evidence-text-layout-gate.json` and `exactness-contract-report.json` to CI/release evaluation artifacts
- Fail fast when exactness documentation drifts from CI/release workflow expectations, including the shared Python version floor for evaluate tooling
- Run Python wheel runtime smoke and WASM package validation before tag-based release publication

## [1.0.4] - 2026-04-01

### Rendering — Text Fidelity
- Preserve slide `lstStyle` precedence over layout, master, and default text styles
- Inherit placeholder `bodyPr` properties across slide/layout/master chains
  - auto-fit (`normAutofit`, `noAutofit`, `spAutoFit`)
  - vertical anchor (`anchor`)
  - wrap (`wrap`) with explicit no-wrap preservation
  - text insets (`lIns`, `tIns`, `rIns`, `bIns`)
  - vertical text direction (`vert`) including explicit `horz` override
- Add wrapped text emergency line breaking via `overflow-wrap: anywhere`
- Ensure explicit `wrap="none"` survives child run styling
- Apply hardcoded 18pt default font size when no run size is specified
- Inherit character spacing (`spc`), baseline offset (`baseline`), underline/strike, and capitalization from text defaults
- Support `anchorCtr` and bodyPr text rotation, including placeholder inheritance
- Clamp oversized `normAutofit` values before rendering

### Tests
- Add hierarchy regressions for placeholder `bodyPr` inheritance (autofit, wrap, margins, vertical anchor, vertical text, baseline, letter spacing)
- Add edge-case coverage for wrapped text line breaking, explicit nowrap preservation, `spAutoFit`, hardcoded default font size, capitalization, anchor centering, and text rotation

### npm / WASM
- Bump the WASM package to `1.0.4`
- Add a package-focused README for the public npm module
- Clarify WASM API examples and slide index conventions
- Prepare npm publish metadata in workflow inputs instead of relying on opaque inline values

### Demo / CI / Evaluation
- Harden the local WASM demo file picker and allow re-selecting the same file
- Expand the PowerPoint fidelity golden set with a bodyPr-focused text fixture
- Restore CI stability by applying rustfmt-clean output for recent text fidelity work

## [1.0.3] - 2026-03-30

### npm / WASM
- Rename the published npm package to `@briank-dev/pptx2html-turbo`

## [1.0.2] - 2026-03-30

### Open Source
- Correct repository metadata to point at `brnyxx/pptx2html-turbo`

## [1.0.1] - 2026-03-30

### npm / WASM
- Include `README.md` and `LICENSE` in the npm package payload

## [1.0.0] - 2026-03-30

### npm / WASM
- Publish WASM package to npm as `@briank-dev/pptx2html-turbo`
- Add `convert_with_options()` — full ConversionOptions support (embedImages, includeHidden, slideIndices)
- Add `convert_with_metadata()` — returns typed ConversionResult with HTML + unresolved elements
- Add `convert_with_options_metadata()` — combined options + metadata API
- Add `get_presentation_info()` — typed PresentationInfo object (replaces JSON string `get_info()`)
- Add GitHub Actions workflow for automated npm publishing on version tags
- Add WASM build verification to CI pipeline

### Open Source
- Add CONTRIBUTING.md with development setup and code style guide
- Add CODE_OF_CONDUCT.md (Contributor Covenant)
- Add GitHub issue templates (bug report, feature request) and PR template
- Add keywords and categories to all crate Cargo.toml metadata

### Performance
- Eliminate intermediate String allocations in renderer (~28% faster rendering)
- Optimize CSS style string building with direct write!() (~21% additional, ~43% cumulative)

### Rendering — Shapes & Geometry
- Expand preset shape geometries from 30 to 187 (full OOXML ECMA-376 coverage)
- Implement custom geometry (`<a:custGeom>`) DrawingML path → SVG conversion
  - Supports moveTo, lnTo, cubicBezTo, quadBezTo, arcTo, close commands
  - DrawingML arc → SVG arc mathematical transformation
- Add shape shadow (`<a:outerShdw>`) and glow (`<a:glow>`) → CSS box-shadow rendering
- Implement auto-fit fontScale and lnSpcReduction for text body sizing
- Add connector geometry paths (straightConnector1, bentConnector5)
- Default 0.75pt stroke for connectors without explicit border

### Rendering — Images
- Fix relative path resolution for image relationship targets (`../media/` → correct ZIP path)
- Handle `<a:blip>` elements with child nodes (Start event, not just Empty)
- Parse background images from master and layout slides (`<a:blipFill>` in `<p:bgPr>`)
- Load image data for shape-level blipFill (image-filled rectangles)
- Fix image crop CSS: replace extreme percentage scaling with pixel-based offsets

### Rendering — Colors & Fills
- Correct OOXML color modifier application order per ECMA-376 spec (alpha→hue→sat→lum→tint/shade)
- Fix HSL tint/shade formula to match OOXML definition
- Distinguish explicit `<a:noFill>` from unspecified fill (prevent theme fillRef overriding transparency)
- Resolve empty and unresolvable theme font references (filter `+mn-ea` → actual typeface)

### Rendering — Layout
- Fix group shape children coordinate transform (chOff/chExt → group bounding box scaling)
- Guard `<a:off>`/`<a:ext>` parsing to `<a:xfrm>` context only (prevent extLst overwriting shape size)
- Fix shape position resolution: treat (0,0) as valid position, not "unset"
- Filter master placeholder shapes through layout matching per OOXML spec
- Change `.shape` overflow from `hidden` to `visible` (prevent text clipping)
- Add word-break/overflow-wrap to text body for proper wrapping
- Remove CSS border duplication on SVG shapes (use SVG stroke only)

### Infrastructure
- Add autoresearch experiment loop (program.md, run_loop.sh, 4 phase programs)
- Add evaluation infrastructure (SSIM fidelity scorer, golden set generator, reference/candidate renderers)
- Add pptx2html-enhance LLM post-processing package (SmartArt/Math/Effects handlers)
- Add ConversionResult with unresolved_elements metadata sideband
- Python bindings: `convert_with_metadata()` API

### Tests
- Total tests: 195+ (was 145 in v0.5.0)
- 16 color modifier edge case tests
- 8 custom geometry integration tests
- 7 shadow/glow effect tests
- 7 auto-fit rendering tests
- Hierarchy/position/background fill tests

## [0.5.0] - 2026-03-28

### Added
- PPTX to HTML conversion with high-fidelity layout preservation
- 30 preset shape SVG rendering with adjust value support
- Slide master / layout inheritance chain with placeholder matching
- Table support (cell fill, borders, col/row span, merge)
- Group shape support with nested coordinate remapping
- Image embedding (base64 data URI) and external reference modes
- Image cropping via CSS clip-path
- Background image fill support
- Chart detection with preview image fallback rendering
- Theme color resolution with full 12-color scheme
- 12 color modifiers: tint, shade, alpha, lumMod/Off, satMod/Off, hueMod/Off, comp, inv, gray
- ClrMap and ClrMapOverride support per slide/layout
- Text styling: bold, italic, underline, strikethrough, superscript, subscript
- Font resolution: theme font references (+mj-lt, +mn-lt, +mj-ea, +mn-ea)
- Bullet and auto-numbering support with font, size, and color
- Vertical text rendering (vert, vert270, mongolianVert)
- Text shadow and highlight support
- Line spacing, space before/after, indent, margin
- Hyperlink support
- TxStyles inheritance (titleStyle, bodyStyle, otherStyle)
- defaultTextStyle inheritance
- FmtScheme style references (fillRef, lnRef, fontRef)
- PyO3 Python bindings with `convert()`/`info()` API
- WASM target with drag-and-drop demo page
- CLI with slide selection, multi-file output, info command
- Criterion performance benchmarks
- Graceful degradation for unsupported content (SmartArt, OLE, Math)
- Password-protected PPTX detection with clear error message
- Conversion progress logging via `log` crate
- GitHub Actions CI/CD workflows

### Architecture
- Cargo workspace: core library, CLI, Python bindings, WASM target
- SAX streaming parser for memory-efficient XML processing
- Index-based hierarchy references (no Rc/Arc)
- EMU coordinate system (914400 EMU = 1 inch = 96px)
