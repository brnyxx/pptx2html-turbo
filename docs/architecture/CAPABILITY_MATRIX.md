# Capability Matrix

This document is the source of truth for implementation maturity and fidelity expectations.

## Support Tiers

| Tier | Meaning |
|------|---------|
| `exact` | Intended to match the supported PowerPoint behavior in a controlled evaluation environment |
| `approximate` | Rendered directly, but known to diverge in layout, metrics, or visual details |
| `fallback` | Not fully rendered; emitted as deterministic fallback HTML/metadata |
| `unparsed` | Not yet parsed or not preserved well enough for reliable downstream handling |

## Capability Stages

| Stage | Meaning |
|-------|---------|
| `parsed` | OOXML is captured into the internal model |
| `resolved` | Inheritance/theme/style resolution is applied |
| `rendered` | Direct HTML/CSS/SVG output exists |
| `fidelity-tested` | Compared against a reference workflow in a pinned environment |

## Current High-Level Matrix

| Family | Current Tier | Highest Stage | Target Tier | Owner Chunk | Notes |
|--------|--------------|---------------|-------------|-------------|-------|
| Shapes | `approximate` | `rendered` | `exact` | Chunk 2 | Broad preset/custom SVG coverage exists; PowerPoint-reference validation still needs expansion |
| Text | `approximate` | `rendered` | `exact` | Chunk 2 | Text layout works, adjacent-run unbreakable tokens plus paragraph-level and inherited text-style font sizes now participate in emergency-wrap detection, `spAutoFit` long-token growth paths no longer force emergency wrapping, partial `normAutofit` overrides now preserve inherited line-spacing reduction, NBSP-separated text now follows non-breaking wrap classification, soft-hyphenated text now follows normal discretionary break opportunities, Devanagari combining-mark clusters now stay on the normal wrap path during emergency-wrap classification, fullwidth/ideographic forms now follow East Asian-style natural breaks, CJK non-starter punctuation now stays attached to the preceding glyph, slash-/hyphen-separated text now follows ordinary break opportunities, CJK opening punctuation now stays attached to the following glyph, CJK closing angle-bracket punctuation now stays attached to the preceding glyph, white square bracket pairs now stay on a single East Asian punctuation cluster, tortoise-shell bracket pairs now stay on the same East Asian punctuation cluster, and lenticular bracket pairs now stay on the same East Asian punctuation cluster, but font metrics, broader line breaking, and autofit still need a dedicated fidelity pass; exact promotion requires the text/layout gate in `evaluate/README.md` |
| Colors and fills | `approximate` | `rendered` | `exact` | Chunk 2 | Theme/styleRef/color modifier stack is implemented, but needs stronger fidelity-test coverage |
| Effects and 3D | `fallback` | `rendered` | `fallback` | Chunk 2 | Namespace-validated direct reflection has a bounded deterministic browser approximation; private sideband metadata types unqualified reflection attributes and exact-DrawingML, owner-path-validated 3D camera/light/material/depth/extrusion/contour/bevel properties while retaining raw XML. Other contexts remain truthful fallbacks. Encounter order is numeric; raw XML is bounded to 65,536 UTF-8 bytes and typed strings to 1,024 bytes, with original length/FNV-1a hash and explicit truncation reason. Public `ShapeEffects` remains source-compatible and Office lighting/material fidelity is not claimed |
| Tables | `approximate` | `rendered` | `exact` | Chunk 2 | Package-defined DrawingML table styles resolve in Office region order with explicit-cell and logical-merge precedence; unavailable built-ins remain diagnostic fallbacks, and header/footer-relative band origin is `[교차검증 필요]` |
| Images | `approximate` | `rendered` | `exact` | Chunk 1 | Crop/render paths and direct slide/table picture bullets exist with identical embedded/external-asset semantics; picture bullets inherited from master/layout/default text styles remain an explicit diagnostic fallback |
| Layout and inheritance | `approximate` | `resolved` | `exact` | Chunk 1 | Placeholder matching and ClrMap work, but layout `lstStyle` and template-style carry-over still need closing work; exact promotion requires the text/layout gate in `evaluate/README.md` |
| Charts | `approximate` | `rendered` | `approximate` | Chunk 3 | Clustered, stacked, and percent-stacked bar/column charts now honor gap/overlap spacing and first-pass data labels; simple line, standard area, flat area3D, scatter, single-series bubble (non-negative sizes, area semantics only, width semantics still fallback), multi-series radar, and single-series ofPie (`ofPieType=pie`, `splitType=pos`) charts honor their basic direct-render paths; direct charts render category/value axis titles; single-series pie, doughnut, and pie3D charts render directly via the flat pie path; when unsupported chart parts expose image relationships they fall back to preview images, otherwise multi-series pie and other chart families still fall back to placeholders |
| SmartArt / OLE / Math | `fallback` | `rendered` | `fallback` | Chunk 3 | Deterministic unresolved placeholders + metadata sideband are emitted |
| Hyperlinks and actions | `approximate` | `rendered` | `approximate` | Chunk 3 | Typed click/mouse-over metadata, safe external links, and deterministic internal navigation render directly; media is preserved without playback, while program/macro/file/custom actions remain inert diagnostic fallbacks; PowerPoint boundary and hidden-slide traversal remain `[교차검증 필요]` |
| Notes and comments | `fallback` | `parsed` | `fallback` | Chunk 3 | Slide notes, notes-master relationships, legacy and modern comments, authors, and unknown modern extensions are preserved as deterministic off-canvas metadata; they are not rendered on the slide canvas |
| Media / animation | `unparsed` | - | `fallback` | Chunk 3 | Action metadata can identify media intent, but playback and the remaining domain-specific fallback contracts still need to be introduced |

## Operating Rules

1. A feature must not be marked `exact` until it has a PowerPoint-reference verification path.
2. Unsupported domains must never silently disappear; they must land in `fallback` or `unparsed` with stable metadata.
3. The repository-root `SUPPORTED_FEATURES.md` remains the detailed element inventory, but this matrix defines the authoritative support contract.
4. Text and layout families must cite the fixture bundle and capture metadata defined in `evaluate/README.md` and `evaluate/powerpoint_golden/README.md` before promotion to `exact`.
