# PROJECT KNOWLEDGE BASE

**Generated:** 2026-08-12T22:58:26Z
**Commit:** dd3bafb
**Branch:** main

## OVERVIEW
Pure Rust PPTX-to-HTML converter built on ECMA-376. A four-crate Cargo workspace exposes one core conversion pipeline through CLI, Python, and browser WASM adapters.

## STRUCTURE
```text
.
├── crates/
│   ├── pptx2html-core/       # Parse, model, resolve, render
│   ├── pptx2html-cli/        # clap/file-system adapter
│   ├── pptx2html-py/         # PyO3/maturin adapter
│   └── pptx2html-wasm/       # wasm-bindgen/npm adapter
├── evaluate/                 # Fidelity scoring and evidence gates
├── pptx2html-enhance/        # Optional Python LLM fallback layer
├── scripts/                  # Release and evidence utilities
├── docs/                     # ADRs, architecture, release records
└── autoresearch/             # Controlled fidelity experiments
```

## WHERE TO LOOK
| Task | Location | Notes |
|---|---|---|
| Public conversion API | `crates/pptx2html-core/src/lib.rs` | `ConversionOptions`, metadata, file/byte wrappers |
| Parse PPTX/OOXML | `crates/pptx2html-core/src/parser/` | ZIP traversal and SAX state machines |
| Add document data | `crates/pptx2html-core/src/model/` | ECMA-376-aligned types and public re-exports |
| Fix inheritance | `crates/pptx2html-core/src/resolver/` | Slide → layout → master → defaults |
| Change HTML/SVG | `crates/pptx2html-core/src/renderer/` | Rendering, diagnostics, provenance |
| Add preset shape | `crates/pptx2html-core/src/renderer/geometry/` | Family routing and adjustment math |
| Integration fixture | `crates/pptx2html-core/tests/fixtures/` | `MinimalPptx`, `PackageBuilder` |
| CLI behavior | `crates/pptx2html-cli/src/main.rs` | Keep conversion logic in core |
| Python API | `crates/pptx2html-py/src/lib.rs` | Keep `pptx2html.pyi` synchronized |
| Browser/npm API | `crates/pptx2html-wasm/` | Dual compatibility API and package checks |
| Fidelity evidence | `evaluate/` | PowerPoint-first exactness, LibreOffice regression |
| Architecture reference | `ARCHITECTURE.md`, `docs/architecture/` | Pipeline and feature-extension flow |

## CODE MAP
| Symbol | Type | Location | Role |
|---|---|---|---|
| `convert_bytes_with_options_metadata` | Function | `core/src/lib.rs` | Full conversion entry point |
| `ConversionOptions` | Struct | `core/src/lib.rs` | Slide filtering, scale, image policy |
| `PptxParser` | Struct | `core/src/parser/mod.rs` | Archive and OOXML orchestration |
| `Presentation` | Struct | `core/src/model/presentation.rs` | Root hierarchy container |
| `resolve_*` | Functions | `core/src/resolver/inheritance.rs` | Property cascade |
| `HtmlRenderer` | Struct | `core/src/renderer/mod.rs` | HTML/CSS/SVG coordinator |
| `RenderCtx` | Struct | `core/src/renderer/mod.rs` | Theme, hierarchy, diagnostics context |
| `preset_shape_svg` | Function | `core/src/renderer/geometry.rs` | Preset geometry dispatcher |
| `MinimalPptx` | Builder | `core/tests/fixtures/mod.rs` | Deterministic PPTX test archives |

## CONVENTIONS
- Pipeline order is model → parser → resolver when inherited → renderer → tests.
- Use SAX streaming with `quick-xml`; do not introduce a DOM parser.
- Hierarchy links are `Vec` indices (`layout_idx`, `master_idx`, `theme_idx`), never `Rc`/`Arc`.
- Resolve colors only through `Color::resolve()` with theme and `ClrMap` context.
- `Fill::None` means inherit; `Fill::NoFill` means explicitly transparent.
- Library output uses `log`; user-facing output belongs to adapters.
- All code comments, documentation, CLI text, and commit messages are English.
- Public binding changes must remain synchronized across Rust, Python stubs, WASM types, and package docs.

## ANTI-PATTERNS
- No `unsafe`, library `unwrap()`, bare fallback swallowing, or undocumented lint suppression.
- No parsing, inheritance, or rendering logic in CLI/Python/WASM adapters.
- Do not render master placeholder templates as visible slide shapes.
- Do not invent appearance for unsupported OOXML; preserve bounded metadata and emit diagnostics.
- Do not copy ONLYOFFICE AGPL implementation; use it only to understand behavior.
- Do not modify `evaluate/evaluate_fidelity.py`; its scoring contract is human-owned.
- Do not call support `exact` without PowerPoint-native evidence.

## COMMANDS
```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build --workspace
cargo bench --package pptx2html-core

cd crates/pptx2html-py && maturin develop
cd crates/pptx2html-wasm && wasm-pack build --target web
python3 evaluate/evaluate_fidelity.py --project-root .
python3 evaluate/powerpoint_evidence.py gate --family text-layout
```

## NOTES
- `914400 EMU = 1 inch = 96 CSS px`; scale the slide uniformly rather than reflowing text.
- `convert_slides` in legacy WASM uses zero-based indices; enhanced APIs use one-based indices.
- Self-contained HTML is default; external assets are an explicit conversion option.
- Unit tests live beside source; cross-layer behavior belongs in core integration tests using generated PPTX archives.
