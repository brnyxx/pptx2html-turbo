# RENDERER

Last stage: resolved model in, self-contained HTML string out. Read root `AGENTS.md` and `resolver/AGENTS.md` first; this file covers presentation-layer rules only.

## RENDERCTX
- `RenderCtx<'a>` is borrow-only: `pres`, `slide`, `scheme`, `clr_map`, `embed_images`, plus `&RefCell<UnresolvedCollector>`. Never add owned model data or a second mutable path.
- `for_slide` rebuilds the context per slide: theme from `master.theme_idx`, ClrMap from `resolve_clr_map`, each falling back to the presentation-level value. Never read `masters.first()` theme for a slide that has its own master.
- All shared mutation goes through the collector: diagnostics, unresolved elements, external assets, font entries, provenance, and the id counters. Borrow it briefly and `drop` before writing more HTML; a live borrow across a nested render call panics.
- `next_gradient_id` / `next_pattern_id` / `next_marker_id` / `register_external_asset` are the only id sources. Ids must stay deterministic across runs, so counters advance in document order and never hash-derive.
- `color_to_css` goes through `resolve_color` (theme + ClrMap) and only then `Color::to_css`. Don't hand-map scheme names.

## DISPATCH
- `render_with_options_diagnostics` builds head/CSS/container, loops slides honoring `should_include_slide` and `hidden`, then appends the diagnostics JSON script and the actions runtime.
- Per shape, `render_shape_resolved` picks a lane and returns: table → `tables`, group → `render_group`, `Unsupported` → `fallback::render_unsupported`, chart → `charts`, preset name → SVG, otherwise a plain `<div class="shape">`.
- Positioned CSS is the default surface. SVG is for preset and custom geometry paths, markers, gradient/pattern defs. Text is HTML inside the shape div, never `<text>`.
- `uses_svg` also switches the effect surface: SVG shapes get a `filter: drop-shadow(...)` attribute, CSS shapes get `box-shadow`. Emitting both double-draws the shadow.
- Line and connector presets carry special handling: zero-dimension viewBox floor, anchored-endpoint paths, rotation-swap path variants. Extend that block, don't fork a new renderer.
- Preset paths come only from `geometry::preset_shape_svg` / `preset_shape_multi_svg`. New shapes belong in `geometry/` family modules, keyed by the OOXML preset name.

## INHERITANCE INTEGRATION
- The renderer calls `inheritance::*` and `placeholder::find_matching_placeholder`; it never re-walks layout and master itself. Layout and master matches arrive as `Option<&Shape>` arguments and stay that way down the call chain.
- `build_text_style_ctx` collects the five list-style levels (slide, layout, master placeholder, master `txStyles`, presentation default) and walks them; picking the `txStyles` list is `placeholder::text_style_source`'s job.
- `inherited_geometry_source` only borrows geometry from a placeholder match when it's a real shape, meaning non-rectangle/textbox, or carries adjust values, rotation, or a flip. Otherwise slide geometry wins.
- Effects resolve through `resolve_shape_effects` (explicit, then `effectRef`); `explicit_shape_effects` skips the theme branch for presets that shouldn't inherit it.
- New cascade logic goes in `resolver/`. The renderer may choose between resolved answers, never invent one.

## DIAGNOSTICS AND PROVENANCE
- Unsupported content renders a bounded placeholder div plus a `ConversionDiagnostic` and an `UnresolvedElement`. Both, always. No silent drops, no invented appearance.
- Diagnostic emitters live beside what they describe: `fallback`, `custom_geometry_diagnostic`, `table_style_diagnostics`, `action_diagnostics`, `media`, `embedded_fallback`. `media.rs` and `embedded_fallback.rs` are intentional no-op hooks; keep the signature when filling them in.
- `fallback::sort_and_deduplicate` runs once at the end, so ordering inside the render loop doesn't leak into output. Codes and `FallbackKind` strings are consumed by `evaluate/`; renaming one is a contract change.
- `RenderedProvenanceEntry` is pushed for slide backgrounds and for shapes, sourced from the resolver's `*_source` twins. Add a provenance field here only after the matching `*_source` exists.
- `FontResolutionEntry` records requested vs resolved typeface and whether fallback kicked in; push it wherever a run's font is decided, not just on failure.

## MASTER PLACEHOLDERS
- Master shapes render only when `slide.show_master_sp` and the layout agree.
- Every master shape with `placeholder.is_some()` is skipped. Placeholders are property templates; rendering them duplicates shapes and leaks "Click to edit Master title style" into output.
- Layout placeholders are never rendered directly either. They reach output solely through slide shape inheritance.

## EXTERNAL ASSETS
- `embed_images` true means base64 data URLs inline; false means `register_external_asset` returns `images/slide-N/prefix-N.ext` and the bytes ride out on `ConversionResult::external_assets`.
- Both paths exist for pictures (`mod.rs`), background/blip fills (`fills.rs`), chart previews (`charts.rs`), picture bullets (`picture_bullets.rs`). Adding a fifth image source means wiring both.
- Extension comes from the MIME map in `register_external_asset`; unknown types fall back to `png`. Pattern tiles stay inline SVG data URLs regardless of the flag.

## WHERE TO LOOK
| Need | File |
|---|---|
| Entry point, slide/shape/group dispatch, global CSS | `mod.rs` |
| Preset path math by shape family | `geometry.rs`, `geometry/` |
| Fills, gradients, blips, effect resolution | `fills.rs` |
| Pattern CSS/SVG and tile motifs | `patterns.rs`, `pattern_tiles.rs` |
| Paragraphs, runs, bullets, autofit, font resolution | `bullets.rs` |
| Wrap policy, script segmentation, metrics | `text_metrics.rs` |
| Tables, cell style regions | `tables.rs`, `table_styles.rs` |
| Charts and chart previews | `charts.rs` |
| Hyperlinks, click actions, runtime JS/CSS | `actions.rs`, `action_diagnostics.rs` |
| Placeholder output for unsupported content | `fallback.rs`, `custom_geometry_diagnostic.rs` |
| Provenance types | `provenance.rs` |

## TESTS
- Unit tests sit at the bottom of each file and build a `Presentation` plus `RefCell<UnresolvedCollector>` through the local `test_ctx` helper.
- Assert on emitted markup substrings and on collector contents, both. HTML that looks right while diagnostics are empty is still a regression.
- Geometry has its own suites: `geometry/tests.rs` for path output, `official_presets_review_tests.rs` for the schema-derived presets.
- Cross-layer behavior (real layout/master XML, external asset payloads, diagnostic contracts) belongs in `crates/pptx2html-core/tests/`, notably `renderer_seam_test.rs`, `renderer_regression_test.rs`, `diagnostic_contract_test.rs`.
- Pin structural output (class names, `data-*` attributes, diagnostic codes), never prose or comment text.

## ANTI-PATTERNS
- Don't re-implement inheritance, placeholder matching, or color resolution here.
- Don't hold a `collector` borrow across a nested render call, and don't hand out ids from anywhere but the `RenderCtx` helpers.
- Don't render master or layout placeholder shapes as visible content.
- Don't invent appearance for unsupported OOXML or swallow a fallback without a diagnostic.
- Don't apply both `box-shadow` and an SVG filter to the same shape.
- Don't emit non-deterministic output: no `HashMap` iteration into markup, no time or random-derived ids.
- Don't scatter magic pixel constants; EMU converts once via `to_px` and `914400 EMU = 96 px`.
- Don't add rendering branches in CLI, Python, or WASM adapters.
