# GEOMETRY

Scope: preset and custom shape path generation. Input is a preset name, px width/height, and an adjust map. Output is SVG `d` data. Nothing here reads XML parts, colors, or the model hierarchy.

## DISPATCH ORDER
- `geometry.rs` is routing only. Two entry points: `preset_shape_svg` (single `d` string), `preset_shape_multi_svg` (`CustomGeomSvg`, multi-subpath with per-path `PathFill` and stroke flag).
- Both call `official_presets::route` first. `Rendered`/`Invalid` return immediately; only `NotOfficial` falls through to the hand-written family `match`.
- `route` owns two name tables: `OFFICIAL_NAMES` (55, `official_arrow_presets.xml`) and `official_remaining_presets::NAMES` (63, `official_remaining_presets.xml`). A name in either table never reaches a family module, even on failure.
- New preset goes in a family file plus one dispatch arm. Adding a name to an official table without adding the definition XML makes the shape render as a bare rect forever.
- `misc.rs` is the leftovers bucket, not a default. Pick the family that matches the shape's math.

## OFFICIAL PIPELINE
- `official_presets_xml` parses the bundled asset once into `OnceLock`; `official_presets_schema` validates every attribute against an allow list; unknown attribute, stray text, CDATA, PI, DOCTYPE, or a second XML declaration is a hard parse error.
- Per render: seed `GuideEnvironment::new(w, h)`, apply `<avLst>` defaults, override with caller adjustments that are finite, then evaluate `<gdLst>` in document order. Guides are order-dependent, don't sort them.
- Environment presets `w h l t r b hc vc ss ls`, the `cd`/`3cd4` angle constants, and the `wd*`/`hd*`/`ssd*` divisor families. Angles are 60000ths of a degree everywhere.
- `evaluate` implements the DrawingML operator set literally (`*/ +- +/ ?: abs at2 cat2 cos max min mod pin sat2 sin sqrt tan`). `?:` branches on `> 0.0`, `mod` is a 3D hypot, division by ~0 yields 0.0.
- Non-finite is a failure, not a clamp: `insert` and `evaluate` reject NaN/inf, `route` logs one `warn!` and returns `Invalid` with the viewBox rect `M0,0 L w,0 L w,h L 0,h Z`. Same input, same fallback, every run.
- `render_path` scales by `shape/path` per axis when `<path w=/h=>` is present, splits arcs at half-turn boundaries (cap 2048 segments), and skips arcs whose radius or swing is under 0.001.

## FAMILY MODULES
- Signature is `(w, h, &HashMap<String, f64>) -> String`, `pub(super)`, no `&mut`, no I/O, no logging.
- Adjustment units: OOXML thousandths of a percent. Divide by 100_000 before use. `plus_numeric_path` and `scaled()` show the pattern.
- Read defaults with `finite(adj.get("adj1").copied(), DEFAULT)` so NaN/inf falls back instead of poisoning the format string. Clamp to the range the spec gives (`roundRect` 0..50_000), never to a value you invented.
- Some families keep a captured normalized path constant for the default adjust set and only compute when the caller supplies one. Keep both branches producing the same silhouette at default values.
- Curved arrow multi-path variants match on exact adjust profiles via `matches_curved_arrow_profile` (0.5 tolerance) and interpolate between tight/wide captures. Token counts of the two normalized paths must stay equal or interpolation silently returns the start path.
- Shared helpers live in `shared.rs`: `polygon_path`, `ellipse_point`, `scale_unit_point`, `scale_normalized_path` (M/L pairs), `scale_normalized_svg_d` (arc-aware). Don't hand-roll a fourth scaler.

## COORDINATES
- Width and height arrive already in CSS px; the renderer did the EMU conversion. No `Emu`, no 914400 here.
- Output is absolute px in the shape's own local box, origin top-left, matching the caller's `viewBox="0 0 w h"` with `preserveAspectRatio="none"`.
- Format coordinates `{:.1}` in family modules, `{:.2}` in the official and custom pipelines. Both are deterministic; keep the existing precision per file so path assertions hold.
- `custom_geom` divides shape px by the path's declared `w`/`h`, falling back to shape extent when either is 0, and converts `ArcTo` from start/swing angles to an SVG endpoint arc.

## EDGE CASES
- Zero or non-finite extent must still yield a closed path. Use the `degenerate_path`/`fallback_path` pair: the viewBox rect is the agreed answer, `None` and `""` are not.
- Extreme adjustments that overflow to inf collapse to the same rect fallback via `finite_composition`. Tests pin that equality against `rect`.
- Connectors take `extent()`/`finite()`/`coordinate()` from `connectors.rs` and clamp adjustments to i32 range. They render as unfilled polylines, so an unclosed `d` is correct there.
- The renderer, not this module, handles 90/270-degree connector rotation: it swaps w/h and substitutes a rotated `d`. Adding a bent/curved connector means updating those `matches!` lists in `renderer/mod.rs` too.
- Sub-half-pixel line shapes get bumped to 2.0 px by the caller before dispatch. Don't re-add a minimum here.
- `needs_evenodd_fill` is the only hole contract. A new shape whose inner subpath cuts a hole must be listed there or it fills solid; a shape whose subpaths don't overlap must stay off the list.

## TESTS
- `tests.rs` covers the dispatcher and families; the official pipeline has its own `#[cfg(test)]` modules plus `official_presets_review_tests.rs` for malformed-asset behavior.
- Every adjustable preset gets the hostile matrix: adjust values `-1, MAX, -MAX, NaN, inf, -inf` crossed with extents `0x100, 100x0, 0x0, MAX`. Assert starts with `M`, closes with `Z` on degenerate input, contains no `NaN`/`inf`, and repeats identically.
- Assert on topology (subpath count, arc count, endpoint coordinates), not whole `d` strings, unless you're pinning a default against a captured official path.
- `test_total_supported_shapes_at_least_187` is the coverage floor. New preset, new entry.
- Fault tests must show `Invalid`, never `NotOfficial`, and must show the same bytes twice.

## ANTI-PATTERNS
- Don't add a preset arm to `geometry.rs` with inline math. Route to a family module.
- Don't `unwrap`, panic, divide without a zero guard, or let NaN reach a `format!`. Fallback is the rect, not a crash.
- Don't approximate an official preset by hand when its definition already ships in the XML asset.
- Don't reorder or "simplify" `<gdLst>` guides, or precompute them outside `GuideEnvironment`.
- Don't emit color, CSS, `fill=`, or `stroke=` attributes. This module returns path data plus a `PathFill` role; the renderer resolves appearance.
- Don't clamp an adjustment to a value the spec doesn't state just to make one deck look right.
- Don't touch the bundled preset XML to fix a single shape; it's the ECMA reference copy.
