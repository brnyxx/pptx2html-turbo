# RESOLVER

Answers one question: what is the effective value of a property for this slide shape? Parse already happened, render hasn't started.

## FILES
| File | Role |
|---|---|
| `inheritance.rs` | Cascade functions plus `*_source` provenance twins |
| `placeholder.rs` | `(type, idx)` matching, `text_style_source` routing |
| `style_ref.rs` | `<p:style>` refs into theme `fmtScheme` |

## CASCADE
- Order: slide value → layout placeholder match → master placeholder match → style ref → hardcoded default.
- Background skips `Fill::None` at each level, falls back to white.
- `Fill::NoFill` stops the walk; explicit transparent is a real answer, not a gap.
- Border stops on `border.no_fill`. A border counts as "present" when width > 0 or `has_border_properties` is true (color, dash, cap, join, compound, alignment, miter, head/tail end).
- `lnRef` fallback merges: the theme line wins, shape-local end markers/dash/cap/join survive when the theme leaves them at their default variant.
- Geometry: `has_own_geometry` treats any non-zero x/y/w/h as "xfrm was present". Position (0,0) with real size is valid placement, don't fall through it.
- Every `resolve_*` that has a `*_source` twin must branch identically. Change one, change both, assert both.

## PLACEHOLDER MATCHING
- Priority 1 both type and idx, priority 2 type alone, priority 3 idx alone but only when the source type is `None`.
- Priority 1 returns immediately; 2 and 3 record the first hit and lose to any later exact match.
- Type equality is by discriminant, not payload.
- Shapes with no `placeholder` are skipped, never treated as wildcards.
- `text_style_source` maps Title/CtrTitle → titleStyle, Body/SubTitle/Obj → bodyStyle, everything else → otherStyle. The renderer walks the `txStyles` levels itself; this module only picks the list.

## STYLE REFS
- `fillRef` idx 1..=3 indexes `fill_style_lst[idx-1]`, idx >= 1001 indexes `bg_fill_style_lst[idx-1001]`. idx 0 means no reference, return `None`.
- `lnRef` / `effectRef` are plain `list[idx-1]`. `fontRef` keys on the strings `"major"` / `"minor"`.
- `phClr` is a marker, not a color. Substitute the ref's color and keep the base entry's modifiers.
- Out-of-range idx returns `None` and lets the caller keep cascading. Never clamp to index 0.
- `effectRef` returns `None` when the theme entry carries neither shadow nor glow, so empty refs don't shadow inherited effects.

## THEME AND CLRMAP
- `resolve_clr_map` checks slide `clr_map_ovr`, then layout, then master. `ClrMapOverride::UseMaster` means keep walking, not stop.
- Theme context arrives as four separate optionals (`fmt_scheme`, `scheme`, `clr_map`, plus the shape's `style_ref`). Missing any one means skip the theme branch, not invent a scheme.
- Resolve colors only through `Color::resolve()`. Never hand-map `accent1` to a hex string here.
- Theme lookups go through `theme_idx`; don't chase parent pointers, they don't exist.

## PURITY
- Free functions over borrowed model data. No `&mut`, no cache, no interior mutability, no I/O, no logging.
- Same inputs, same output, every run. Iterate `Vec` order only, never a `HashMap`.
- No allocation-order or float-accumulation dependence in the branch conditions.
- Return owned clones of model values. The resolver hands back answers, it doesn't patch the model in place.

## TESTS
- Unit tests live at the bottom of each file. Build fixtures with `Shape::default()` / `Slide::default()` and `..Default::default()`.
- Per cascade function cover: value at each level, skip-on-`None`, explicit-stop (`NoFill` / `no_fill`), and the final default.
- Pair every resolve test with its provenance test; `ProvenanceSource` drift is the usual silent regression.
- Placeholder tests must pin priority ordering, including the case where a later candidate outranks an earlier one.
- Cross-layer behavior (real layout/master XML) belongs in core integration tests, not here.

## ANTI-PATTERNS
- Don't treat `Fill::None` and `Fill::NoFill` as the same thing. One inherits, one terminates.
- Don't collapse the three placeholder priorities into a single equality check.
- Don't apply a style ref before the layout and master matches have been tried.
- Don't resolve theme colors to CSS or emit any HTML/px here; the renderer owns presentation.
- Don't read the master's `clr_map` directly when a slide or layout override may exist.
- Don't add a `resolve_*` without its `*_source` counterpart when the renderer reports provenance for that property.
- Don't fall back to hardcoded appearance for unsupported OOXML. Return the neutral default and let the renderer diagnose.
