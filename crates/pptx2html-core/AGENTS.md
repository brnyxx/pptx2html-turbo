# pptx2html-core

Scope: this crate only. Root `AGENTS.md` owns workspace rules; they apply here unqualified and aren't repeated.

## OVERVIEW
The whole conversion pipeline lives here. Adapters (`cli`, `py`, `wasm`) only pass bytes in and take `ConversionResult` out.
Four layers, strict one-way flow: `parser` builds `model`, `renderer` reads `model` and calls `resolver` on demand, `resolver` never touches XML or HTML.
Dependencies are deliberately thin: `quick-xml`, `zip`, `base64`, `thiserror`, `log`. Adding a crate here is an architecture decision, not a convenience.

## STRUCTURE
```text
src/
├── lib.rs          # public API surface: convert_*, get_info*, ConversionOptions, ConversionResult
├── error.rs        # PptxError / PptxResult
├── model/          # ECMA-376 data types; mod.rs re-exports the public subset
├── parser/         # ZIP + SAX; all submodules private except master_parser, relationships
├── resolver/       # inheritance.rs, placeholder.rs, style_ref.rs (pure functions)
└── renderer/       # mod.rs coordinates; provenance + text_metrics public, rest private
tests/              # 19 integration targets + fixtures/ builders
benches/pipeline.rs # criterion, harness = false
```

## WHERE TO LOOK
| Task | Location |
|---|---|
| Entry point every path funnels through | `src/lib.rs::convert_bytes_with_options_metadata` |
| Package-level diagnostics collection | `src/parser/preserved_parser.rs` (`collect_package_diagnostics`, crate-internal) |
| Slide → layout → master cascade | `src/resolver/inheritance.rs` |
| `lnRef`/`fillRef`/`effectRef`/`fontRef` lookups | `src/resolver/style_ref.rs` |
| Placeholder matching by type and idx | `src/resolver/placeholder.rs` |
| Render context, per-slide loop | `src/renderer/mod.rs` (`RenderCtx`, `render_with_options_diagnostics`) |
| Preset geometry dispatch | `src/renderer/geometry.rs` then `geometry/<family>.rs` |
| Test archive builders | `tests/fixtures/mod.rs`, `tests/fixtures/package.rs` |

## MINIMALPPTX FLOW
`MinimalPptx::new(slide_body)` wraps a `<p:spTree>` fragment in a full `p:sld` document, then `.with_theme()`, `.with_clr_map()`, `.with_master_shapes()`, `.with_layout()`, `.with_extra_file()` swap individual parts. `.build()` returns PPTX bytes; feed them straight to `convert_bytes*` or `PptxParser::parse_bytes`. Defaults fill `[Content_Types].xml`, rels, presentation, master, layout, theme, so a test only declares what it asserts on.
`PackageBuilder` is the stricter sibling: it validates before writing and returns `FixtureError` with a stable `code()` (`DANGLING_RELATIONSHIP`, `RESERVED_PART_PATH`, ...). Use it when relationship wiring, part paths, or determinism is the thing under test; use `MinimalPptx` for content behavior. Both are `mod fixtures;` copies per test target, so a fixture change recompiles many targets.

## CONVENTIONS
- Inheritance resolves at render time, not parse time. The parsed `Presentation` stays a faithful record of the file; `Fill::None` and `None` fields must survive parsing intact so the resolver can distinguish "inherit" from "explicitly set".
- Parser submodules stay private. Widening one to `pub` is an API commitment; prefer re-exporting the resulting model type.
- New model types need a `pub use` in `model/mod.rs` and coverage in `tests/public_api_test.rs`, which pins both module paths and the flat re-export.
- Seam tests split by layer: `parser_seam_test.rs` builds archives and asserts on the model, `renderer_seam_test.rs` builds a `Presentation` in memory and asserts on HTML. Keep them on their own side.
- Unsupported content produces a `ConversionDiagnostic` plus an `UnresolvedElement`, never a silent drop. `diagnostic_contract_test.rs` guards the shape.

## ANTI-PATTERNS
- Don't resolve theme colors, style refs, or placeholder inheritance inside `parser/`, and don't reach into `zip`/`quick-xml` from `renderer/`.
- Don't hand-write ZIP bytes in a test. Extend the fixture builders instead.
- Don't add `Rc`, `Arc`, or back-pointers to the model. Cross-layer links are `Vec` indices (`layout_idx`, `master_idx`, `theme_idx`).
- Don't assert on whole HTML documents. Match the specific attribute, path data, or CSS declaration under test; broad snapshots break on every unrelated renderer change.
- Don't add a preset shape to `geometry.rs` directly. Route through the family module and its formula helpers.

## TEST COMMANDS
```bash
cargo test -p pptx2html-core                              # crate only
cargo test -p pptx2html-core --lib                        # unit tests beside source
cargo test -p pptx2html-core --test hierarchy_test         # inheritance cascade
cargo test -p pptx2html-core --test public_api_test        # API surface + re-exports
cargo test -p pptx2html-core --test fixture_contract_test  # fixture determinism/validation
cargo test -p pptx2html-core --test renderer_seam_test     # model -> HTML
cargo test -p pptx2html-core --test parser_seam_test       # archive -> model
cargo test -p pptx2html-core --test geometry_adjustment_basic_test  # preset adjust values
cargo bench -p pptx2html-core --bench pipeline
```
