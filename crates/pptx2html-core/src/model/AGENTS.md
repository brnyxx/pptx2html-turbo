# MODEL LAYER

Scope: `crates/pptx2html-core/src/model/`. ECMA-376 PresentationML types only. Read the root `AGENTS.md` first; this file covers rules specific to the data model.

## OWNERSHIP
- Model is a plain data layer. Types describe what the PPTX says, not what HTML needs.
- Parsing lives in `parser/`, cascade lives in `resolver/`, appearance lives in `renderer/`. No XML reading, no inheritance walking, no CSS strings here.
- Small pure helpers on model types are fine when they belong to the format itself: `ColorScheme::get`, `FontScheme::resolve_typeface`, `ClrMap::get` alias normalization, `FmtScheme::get_fill_style` idx offset, `PlaceholderType::from_ooxml`, `GradientType::from_path_attr`, `Fill::color_ref`.
- `Color::resolve()` in `color.rs` is the single color resolution point. Do not spawn a second one.

## RE-EXPORTS
- `mod.rs` is the public face. Every consumer should reach types via `crate::model::X`, and `renderer/mod.rs` does `use crate::model::*`.
- `geometry`, `pattern`, `style` are private modules; their types are public only through `mod.rs` re-exports. Adding a type there means adding it to the `pub use` list, else it's invisible.
- `slide.rs` and `style.rs` carry compatibility re-export blocks (shape, table, text, chart, bullet, preserved, fill, effects). Keep them; older paths like `model::slide::Shape` and `model::style::Fill` are still used across the workspace.
- Add a new type: define in the narrow module, re-export from `mod.rs`, and add the shim only when an existing path already implies it.

## SEMANTIC DISTINCTIONS
- `Fill::None` = nothing specified, inheritance and theme fallback still apply. `Fill::NoFill` = explicit `<a:noFill/>`, transparent, fallback suppressed. Collapsing them silently repaints backgrounds that PowerPoint leaves empty.
- `Border::no_fill` is the same idea for `<a:ln><a:noFill/>`: suppress the stroke, don't inherit theme `lnRef`.
- `ColorKind::None` means absent, not black. `Color::none()` is the neutral sentinel that `Fill::color_ref` returns for image or empty-stop fills.
- `Option<T>` on `ParagraphDefaults` and `RunDefaults` means "not set at this level" so the resolver can keep walking. Never default-fill in the model.
- `SpacingValue::Percent` vs `Points`, `ClrMapOverride::UseMaster` vs `Override`: keep both arms distinct; the difference decides the cascade.

## INDEX REFERENCES
- Hierarchy links are `Vec` positions into `Presentation`: `Slide::layout_idx` (`Option<usize>`, absent means no layout), `SlideLayout::master_idx`, `SlideMaster::theme_idx`. No `Rc`, `Arc`, or back-pointers, so `Presentation` stays `Clone` and cheap to move across the FFI adapters.
- Indices are only valid against the `Presentation` they came from. Never pass a bare index across presentations, and never reorder `slides`, `layouts`, `masters`, or `themes` after parse.
- Theme `idx` values are 1-based OOXML values, not slice offsets. `FmtScheme::get_fill_style` maps 1..999 to `fill_style_lst` and 1001+ to `bg_fill_style_lst`; do the subtraction there, not at call sites.

## DIAGNOSTICS AND FALLBACK
- `preserved.rs` holds the record of what wasn't rendered: `ConversionDiagnostic`, `DiagnosticLocation`, `FallbackKind`, plus `UnsupportedData` / `UnresolvedElement` / `UnresolvedType`.
- `ShapeType::Unsupported(UnsupportedData)` keeps the shape in the tree with its label, typed kind, bounded raw XML, and optional custom geometry. Unsupported content never disappears and never gets invented appearance.
- `FallbackKind::as_str` values are stable strings consumed downstream. Rename one and you break evidence tooling, not just Rust code.
- `DiagnosticLocation` implements `PartialEq` by hand because `Emu` positions are float-free but `Option`-wrapped; extend both the struct and that impl together or equality silently ignores the new field.
- `UnresolvedElement::slide_index` is 0-based. Adapters that expose 1-based indices convert at their own boundary.

## PUBLIC ENUM COMPATIBILITY
- Model enums are exhaustive on purpose; downstream `match` arms are the compile-time checklist that a new variant got handled in resolver, renderer, and adapters. Do not add `#[non_exhaustive]` to dodge that.
- Adding a variant to `ShapeType`, `Fill`, `ColorKind`, `ColorModifier`, `FallbackKind`, `FeatureFamily`, `SupportTier`, or `CapabilityStage` is a workspace-wide change. Compile the workspace and fix every arm rather than adding a `_ =>` catch-all.
- `as_str` / `Display` output on `capabilities.rs` and `FallbackKind` is a contract with `evaluate/` and the Python and WASM surfaces. Tests pin those strings; treat a change as a breaking release, and mirror it in stubs and TypeScript types.
- `FeatureCapability::validate` encodes the support policy: `Exact` requires `FidelityTested`, `Unparsed` forbids a stage, `Approximate` and `Fallback` require one. Construct through `new()` so the check runs.

## WHERE TO LOOK
| Need | File |
|---|---|
| Root container, theme, ColorScheme, ClrMap | `presentation.rs` |
| Slide, layout, master, placeholder, tx styles | `slide.rs`, `hierarchy.rs` |
| Shape tree, pictures, groups, connections | `shape.rs` |
| Fills, gradients, patterns, borders, effects | `fill.rs`, `pattern.rs`, `style.rs`, `effects.rs` |
| Text bodies, runs, paragraphs, autofit | `text.rs`, `bullet.rs` |
| Tables and table style refs | `table.rs`, `table_style.rs` |
| EMU, Position, Size, custom geometry paths | `geometry.rs` |
| Color kinds, modifiers, resolution | `color.rs` |
| Charts, hyperlinks and actions | `chart.rs`, `action.rs` |
| Support tiers and stage policy | `capabilities.rs` |
| Fallbacks and diagnostics | `preserved.rs` |
| Inventory stubs awaiting real types | `media.rs`, `embedded.rs`, `timing.rs`, `notes_comments.rs` |

## ANTI-PATTERNS
- No renderer-shaped fields: no CSS strings, no pixel values, no pre-serialized SVG. EMU in, conversion at render time.
- No `Rc`/`Arc`/lifetime-borrowed parent pointers to fake a tree.
- No lossy defaults that erase "unspecified"; that's what `Option` and `Fill::None` are for.
- No parser-only scratch state parked on model structs.
- No renaming or reordering of public enum variants and `as_str` strings without updating Python stubs, WASM types, and `evaluate/` expectations.
- No new stringly-typed field where an enum already exists, and no enum split that leaves a `_ =>` arm swallowing future variants.
