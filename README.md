# pptx2html-turbo

Convert PPTX slides to HTML in pure Rust with direct rendering and structured fallbacks.

Built on the ECMA-376 open standard — no Microsoft dependencies, no C/C++ bindings, just Rust.

**[Live Demo](https://kim62210.github.io/pptx2html-turbo/)** — try it in your browser, no installation needed.
**[Releases](https://github.com/kim62210/pptx2html-turbo/releases)** — download CLI artifacts and read versioned release notes.

## Features

- Approximate browser layout preservation using absolute positioning
- Theme color resolution with 12 color modifiers (tint, shade, lumMod, etc.)
- Slide master / layout inheritance chain with placeholder matching
- 187 preset shape SVG rendering with broad adjust value support; known custom guide formulas render directly, while unknown, non-finite, or unresolved formulas preserve their exact raw formula and emit `DRAWINGML_CUSTOM_GEOMETRY_FALLBACK` instead of substituting zero or a default
- SVG stroke dash styles (solid, dash, dot, dashDot, etc.)
- Line ending markers (arrow, triangle, stealth, diamond, oval)
- Table, group shape, and connector support
- Image embedding (base64) or external references, with cropping
- Text styling: bold, italic, underline, strikethrough, super/subscript, bullets, vertical text, shadows, highlights, letter spacing
- Typed click and mouse-over action preservation for shape, group, picture, connector, table graphic-frame, and text-run surfaces
- Slide notes, notes-master links, and legacy/modern comments and authors preserved as typed off-canvas fallback metadata
- Layout-preserving whole-slide scale/zoom across Rust, CLI, Python, and WASM surfaces
- Approximate direct chart rendering for a bounded compatible subset, with exact namespace/ancestry-aware classic/ChartEx classification, signature-validated bounded preview-or-placeholder fallback, and one typed diagnostic for every rejected chart; ChartEx remains fallback-only with qualified inventory metadata
- Graceful placeholders for unsupported content (SmartArt, OLE, Math)
- Self-contained HTML output (single file, no external dependencies)

## Install

```bash
# npm (WASM — browser)
npm install @briank-dev/pptx2html-turbo@2.0.0

# CLI (from a checked-out v2.0.0 source tree)
cargo install --path crates/pptx2html-cli

# Python (requires maturin)
cd crates/pptx2html-py && maturin develop

# WASM (build from source)
cd crates/pptx2html-wasm && wasm-pack build --target web
```

The Rust crates and Python binding are source distributions in v2.0.0; this release does not publish them to crates.io or PyPI.
Rust library consumers can depend on the release tag directly:

```toml
[dependencies]
pptx2html-core = { git = "https://github.com/kim62210/pptx2html-turbo", tag = "v2.0.0" }
```

## Usage

### CLI

```bash
# Basic conversion
pptx2html input.pptx -o output.html

# Default output: input.html
pptx2html input.pptx

# Select specific slides
pptx2html input.pptx --slides 1,3,5-8

# Per-slide output files
pptx2html input.pptx --format multi -o output_dir/

# External images (not embedded; writes assets under images/slide-N/)
pptx2html input.pptx --no-embed

# Include hidden slides
pptx2html input.pptx --include-hidden

# Image-like whole-slide zoom without reflow
pptx2html input.pptx --scale 2.0

# Print presentation info as JSON
pptx2html input.pptx --info

# Write the canonical ordered diagnostics JSON
pptx2html input.pptx --diagnostics diagnostics.json

# Still write outputs, but exit 2 when fallback diagnostics are present
pptx2html input.pptx --fail-on-fallback
```

### Rust Library

```rust
use std::{fs, path::Path};
use pptx2html_core::{
    convert_file, convert_file_with_options_metadata, get_info, ConversionOptions,
};

// Simple conversion
let html = convert_file(Path::new("presentation.pptx"))?;

// From bytes
let html = pptx2html_core::convert_bytes(&pptx_data)?;

// With options
let opts = ConversionOptions {
    embed_images: false,
    include_hidden: true,
    slide_indices: Some(vec![1, 3, 5]),
    scale: 2.0,
    ..Default::default()
};
let result = convert_file_with_options_metadata(Path::new("presentation.pptx"), &opts)?;
let output_dir = Path::new("output");
fs::create_dir_all(output_dir)?;
fs::write(output_dir.join("presentation.html"), &result.html)?;
for asset in &result.external_assets {
    let asset_path = output_dir.join(&asset.relative_path);
    if let Some(parent) = asset_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(asset_path, &asset.data)?;
}

// Get metadata
let info = get_info(Path::new("presentation.pptx"))?;
println!("Slides: {}, Size: {}x{}", info.slide_count, info.width_px, info.height_px);

// Conversion with ordered preservation diagnostics and the legacy unresolved sideband
let result = pptx2html_core::convert_file_with_metadata(Path::new("presentation.pptx"))?;
println!("HTML length: {}", result.html.len());
let diagnostic_codes = result.diagnostics.iter().map(|item| item.code.as_str()).collect::<Vec<_>>();
for elem in &result.unresolved_elements {
    println!("Unresolved: {:?} at slide {}", elem.element_type, elem.slide_index);
}
```

Every generated HTML document also embeds the same ordered diagnostics as a JSON array in
`<script type="application/json" id="pptx2html-diagnostics">`. The script contains `[]` when
conversion requires no fallback. `unresolved_elements` remains available as the compatibility
projection for placeholder-based SmartArt, OLE, Math, and custom-geometry handling.

### Python

```python
import pptx2html

# Simple conversion
html = pptx2html.convert_file("presentation.pptx")

# From bytes
html = pptx2html.convert_bytes(pptx_data)

# With options
html = pptx2html.convert(
    "presentation.pptx",
    embed_images=False,
    include_hidden=True,
    slides=[1, 3, 5],
    scale=2.0,
)

# Get metadata
info = pptx2html.get_info("presentation.pptx")
print(f"Slides: {info.slide_count}, Size: {info.width_px}x{info.height_px}")

# Conversion with metadata (SmartArt/OLE/Math/custom-geometry sideband)
result = pptx2html.convert_with_metadata("presentation.pptx")
print(f"HTML: {len(result.html)} chars, Unresolved: {len(result.unresolved_elements)}")
for elem in result.unresolved_elements:
    print(f"  {elem.element_type} at slide {elem.slide_index}: {elem.placeholder_id}")
```

### WASM / Browser

```html
<script type="module">
import init, {
  convert,
  convert_with_options,
  convert_with_metadata,
  get_presentation_info,
} from '@briank-dev/pptx2html-turbo';

await init();

const response = await fetch('presentation.pptx');
const data = new Uint8Array(await response.arrayBuffer());

// Simple conversion
const html = convert(data);
document.getElementById('output').srcdoc = html;

// With options (embedImages, includeHidden, slideIndices, scale)
const html2 = convert_with_options(data, false, true, new Uint32Array([1, 3]), 1.5);

// Typed metadata
const info = get_presentation_info(data);
console.log(`Slides: ${info.slideCount}, Size: ${info.widthPx}x${info.heightPx}`);

// Conversion with metadata sideband (SmartArt/OLE/Math/custom geometry)
const result = convert_with_metadata(data);
console.log(`HTML: ${result.html.length}, Unresolved: ${result.unresolvedElements}`);
</script>
```

A drag-and-drop demo page is included at `crates/pptx2html-wasm/demo/index.html`.
The included demo displays ordered diagnostic counts, runs renderer-owned actions and timing in an opaque-origin frame, and initializes image-like whole-slide zoom to the available width while keeping slide coordinates and text flow intact.

## Supported Features

See [SUPPORTED_FEATURES.md](SUPPORTED_FEATURES.md) for the full ECMA-376 element inventory, [docs/architecture/CAPABILITY_MATRIX.md](docs/architecture/CAPABILITY_MATRIX.md) for the authoritative support-stage matrix, and [docs/architecture/PPTX_COMPLETENESS_PROGRESS.md](docs/architecture/PPTX_COMPLETENESS_PROGRESS.md) for the current capability ledger and remaining exactness work.

| Category | Highlights |
|----------|-----------|
| Shapes | 187 preset shapes with broad adjust value coverage + custom geometry SVG rendering, guide formulas, and text rectangles |
| Text | Bold, italic, underline, strikethrough, super/subscript, vertical text, highlights, shadows, letter spacing, default 18pt fallback |
| Colors | RGB, theme, system, preset with 12 modifiers (tint, shade, lumMod, satMod, etc.) |
| Fills | Solid, gradient, image, noFill, and all 54 DrawingML pattern presets with approximate repeated SVG tiles; style references (fillRef/lnRef). Unknown or unresolved patterns emit `DRAWINGML_PATTERN_UNSUPPORTED` without an invented solid color. |
| Tables | Package-defined DrawingML table styles, official region precedence, parsed theme-format cell fills, text/borders, column/row spans, and horizontal/vertical merge |
| Images | Base64 embedding, deterministic external assets under `images/slide-N/`, cropping, MIME auto-detection |
| Layout | Master/layout inheritance, ClrMap overrides, placeholder matching, TxStyles, and bodyPr property carry-over (wrap, margins, vertical anchor, vertical text, autofit) |
| Bullets | Character and auto-numbered bullets plus embedded picture bullets in slide paragraphs, slide-owned list styles, and table cells; unavailable images render a visible marker with diagnostics |
| Charts | Direct clustered, stacked, and percent-stacked bar/column rendering with gap/overlap and first-pass data labels, simple line/standard area/scatter rendering with point labels and explicit marker handling, simple single-series radar rendering, axis titles, and single-series pie/doughnut plus flat-rendered single-series pie3D and area3D rendering, with chart-part preview-image fallback when available and placeholder fallback for unsupported chart families and complex variants |
| Media | Approximate bounded playback for shape-owned internal PCM WAV and the deterministic one-frame Constrained Baseline AVC MP4 subset with native controls, no autoplay, and user-gesture media actions; all external, oversized, malformed, missing, or unsupported media uses typed poster/placeholder fallback without fetching |
| Notes and comments | Slide notes, notes-master association, legacy and modern comments/authors preserved as typed off-canvas diagnostics metadata; unresolved authors use `COMMENT_AUTHOR_UNRESOLVED` and unknown modern extensions retain raw XML as fallback |
| Unsupported | SmartArt, OLE, Math — structured placeholders with metadata sideband (raw XML, type, position) |
| LLM Enhance | Post-processing layer: SmartArt→HTML/CSS, OMML→MathML, DrawingML→CSS via LLM (pptx2html-enhance) |

### v2.0.0 API compatibility

Rust consumers upgrading from v1.x must account for these public API changes:

- `Bullet::Picture` adds a new exhaustive-match arm.
- `Shape::actions`, `TextRun::actions`, typed action enums, and `FallbackKind::ActionMetadata` extend action handling.
- `TableStyleReference::definition` is now `Option<Box<TableStyle>>`, and the public table-style structs expose additional typed fields.
- `Presentation::embedded_inventory` adds a public field; use `..Default::default()` where appropriate.
- `ConversionResult::diagnostics` adds a public field; construct results with `ConversionResult::new(html, slide_count)`.

DrawingML preset names beginning with `math`, such as `mathPlus`, are geometric shapes only and do not imply OMML equation support.

## Architecture

### Pipeline

```
PPTX → pptx2html-turbo (Rust) → HTML + Metadata
                                    │
                                    ├─→ Direct HTML output (existing, zero dependencies)
                                    └─→ pptx2html-enhance (Python, LLM) → Enhanced HTML
                                              │
                                              ├── SmartArt XML  → HTML/CSS layout
                                              ├── OMML equations → MathML
                                              └── DrawingML effects → CSS (shadow, glow, blur)
```

The Rust core preserves ordered slide transition/timing XML and approximately executes a bounded interaction-driven subset: cut/fade transitions and click/with-previous/after-previous appear, disappear, or fade effects on resolved slide shapes, including finite start-condition delays up to 10000 ms. It never autoplays or loops; unsupported timing remains typed fallback metadata with exact raw node XML, and unsupported targets stay statically visible. Timing inventory remains private to conversion, preserving the pre-v2.0 public `Slide`, capability enums, and `TimingInventory` API.

The Rust core converts PPTX to HTML with high fidelity. Elements it cannot fully render (SmartArt, Math, OLE, and custom geometry) are emitted as structured placeholders with an ordered diagnostic sideband containing a safe source reference; custom-geometry formula fallbacks preserve the exact raw formula. Package-level unsupported parts and relationships are reported even when they do not produce a visible shape; relationship diagnostics identify only the source part and relationship ID, never the target. The optional Python `pptx2html-enhance` package uses placeholder metadata to transform supported fallback types into semantic HTML.

Rust consumers upgrading to v2.0.0 and constructing `ConversionResult` should use `ConversionResult::new(html, slide_count)` and then populate any required metadata fields. The `diagnostics` field makes legacy external struct literals source-incompatible, while the existing `unresolved_elements` field and its returned projection remain unchanged. `ConversionResult::diagnostics()` provides a stable ordered slice accessor.

Slide notes and legacy/modern comments are parsed from internal relationship parts and remain outside the visible `.slide` subtree. Their paragraph-aware text, one-based presentation slide association, author records, timestamp, relationship ID, part name, and validated notes-master relationship are carried by the existing ordered diagnostics JSON as `fallback/parsed` metadata. Missing authors do not discard comment text and emit `COMMENT_AUTHOR_UNRESOLVED`; duplicate author IDs remain unresolved instead of selecting an arbitrary record. Unsafe, external, malformed, duplicate, or type-spoofed annotation relationships never select unrelated package parts. Each unknown modern comment extension subtree is retained independently with `MODERN_COMMENT_EXTENSION_FALLBACK` and is not claimed as exact interpretation. These bounds follow Microsoft's [Notes Slide](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-notes-slides), [legacy comments](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-comments), [PresentationML structure](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document), [modern CT_Comment](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/161bc2c9-98fc-46b7-852b-ba7ee77e2e54), [modern Comment Part](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/b85a9293-bdca-4c6b-a554-8f3918db9791), and [modern Author Part](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/4071f53f-9509-405f-a76b-594b865e177a) documentation.

The v2.0.0 table-style model adds fields to the public `TableData`, `TableCell`, `TableCellStyle`, and `TableStyle` structs, including typed unsupported reference primitives. External Rust code upgrading from v1.x and using struct literals must migrate by using `..Default::default()` or the new fields explicitly. Package-defined styles follow the documented Office region order. A valid Office built-in whose converter definition is unavailable, or an invalid ID, preserves its ID and six flags and emits `TABLE_STYLE_DEFINITION_UNAVAILABLE` without inventing an appearance. Table-style `fillRef` preserves its index, reference color, and modifiers and resolves only when the referenced fill is present in the parsed theme format scheme. A non-solid theme fill that the current theme parser cannot carry is left unapplied with `TABLE_STYLE_PRIMITIVE_UNSUPPORTED`; flattening it to a solid fill is forbidden and the converter does not claim exact non-solid resolution. Scoped `tblBg/effectRef` and border-side `lnRef` index/color/modifiers are also preserved and diagnosed as unsupported without discarding sibling styles or inventing effects/lines. Header/footer-relative row and column band origins remain approximate and are not claimed as PowerPoint-equivalent.

`Shape::actions` and `TextRun::actions` are the authoritative typed action contract. They distinguish click from mouse-over and preserve external URI, actual presentation-order slide target, next/previous/first/last, no-op, media, and unsupported raw action semantics. `TextRun::hyperlink` remains as a compatibility projection, but both typed and legacy links pass the same product security policy: only ASCII-control/whitespace-free `http`, `https`, and `mailto` URIs are executable. HTTP(S) credentials and malformed, relative, protocol-relative, file, program, macro, and custom targets are inert. Executable external links open in a new browsing context with `rel="noopener noreferrer"`; mouse-over metadata never navigates. Boundary and hidden-slide traversal remain approximate and are not claimed as PowerPoint-equivalent.

Shape-owned `a:audioFile` and `a:videoFile` references are supported only for official internal relationships that resolve safely into `ppt/media/`, stay within 16 MiB, and have namespace-valid content types. Audio is limited to PCM WAV. Video is limited to one structurally parsed IDR I-slice of 8-bit 4:2:0 progressive Constrained Baseline AVC (profile 66, compatibility `0xc0`, level 30), 16x16 through 256x256 macroblock-aligned dimensions, with raster-ordered I_PCM macroblocks, canonical emulation prevention and trailing bits, matching `avc1`/SPS dimensions, and sample-table ranges wholly inside `mdat`. Extra parameter sets, slices, NAL units, or unsupported AVC syntax fall back; no fixture-byte or pixel-byte whitelist is accepted. Supported assets use native controls without autoplay, and external relationships are never fetched. Browser codec behavior and native PowerPoint fidelity remain approximate and are not claimed as exact.

Group and table graphic-frame actions retain their own `cNvPr` identity without overriding descendant actions. Typed runs and safe legacy `TextRun::hyperlink` anchors remain pointer-reachable above enclosing shape, group, and table action surfaces; plain runs and blocked unsafe legacy links do not intercept the owner action. Run diagnostics use stable slide/shape/paragraph/run coordinates, with table row/column coordinates where applicable; exact duplicate emissions collapse while distinct occurrences remain separate. Action parsing requires the exact PresentationML owner/nonvisual/`cNvPr` stack and DrawingML action namespace.

The v2.0.0 line adds `actions` to the public `Shape` and `TextRun` structs, public typed action enums, and `FallbackKind::ActionMetadata`. External Rust consumers upgrading from v1.x must migrate exhaustive matches and struct literals, use `..Default::default()` where appropriate, and treat `actions` as authoritative.

### Project Layout

```
├── autoresearch/               # Autoresearch experiment loop
│   ├── program.md              # Master protocol
│   ├── run_loop.sh             # Experiment runner
│   ├── phases/                 # Phase-scoped programs (4 phases)
│   └── results.tsv             # Experiment audit log
├── crates/
│   ├── pptx2html-core/        # Core library (model, parser, resolver, renderer)
│   ├── pptx2html-cli/         # CLI binary (clap)
│   ├── pptx2html-py/          # PyO3 Python bindings (maturin)
│   └── pptx2html-wasm/        # WASM bindings (wasm-bindgen) + demo page
├── evaluate/                   # Fidelity evaluation (sacred — do not modify)
│   ├── evaluate_fidelity.py   # Composite scoring (SSIM + text + tests + perf)
│   ├── reference_render.py    # LibreOffice headless → reference PNGs
│   ├── candidate_render.py    # Playwright HTML → candidate PNGs
│   ├── create_golden_set.py   # Generate the golden PPTX fixture set
│   ├── golden_set/            # Golden PPTX files (generated)
│   └── golden_references/     # Reference PNG renders (generated)
├── pptx2html-enhance/         # LLM post-processing for unresolved elements (Python)
│   ├── src/pptx2html_enhance/ # Enhancer, handlers (SmartArt/Math/Effects), providers
│   └── tests/                 # Enhancer tests with a mock LLM provider
└── Cargo.toml                 # Workspace root
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full pipeline diagram and module responsibilities.

## Testing

```bash
# Rust tests
cargo test --workspace

# Python tests
cd pptx2html-enhance && .venv/bin/python -m pytest tests/ -v

# Evaluation contract tests
python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v

# Benchmarks
cargo bench --package pptx2html-core
```

Tag-based CI and release validation also now replays:
- installed-wheel Python runtime smoke for the published binding surface
- WASM package contract + package-root/runtime smoke for the npm/browser distribution
- exactness contract checks plus exported evaluation artifacts (`powerpoint-evidence-summary.json`, `powerpoint-evidence-text-layout-gate.json`, `exactness-contract-report.json`)
- text/layout fixture-bundle drift checks so the gate documented in `evaluate/README.md` stays aligned with `evaluate/powerpoint_evidence.py`
- text-wrap fidelity checks that now cover explicit runs, paragraph defaults, and inherited text-style font sizes under autofit
- `spAutoFit` growth semantics that keep long unbreakable tokens on the grow-to-fit path instead of forcing emergency word breaking
- partial `normAutofit` inheritance that preserves inherited line-spacing reduction when a child placeholder only overrides `fontScale`
- non-breaking-space-aware wrap classification so NBSP-separated text follows browser non-breaking behavior during emergency-wrap decisions
- soft-hyphen-aware wrap classification so discretionary hyphenation stays on the normal line-breaking path instead of forcing emergency wrapping
- Devanagari combining-mark clusters so sequences like `क़` stay on the normal wrap path during emergency-wrap classification instead of being measured like separate glyphs
- fullwidth and ideographic-form-aware wrap classification so East Asian-width forms stay on the natural break path instead of being measured like one Latin token
- CJK non-starter punctuation clustering so characters like `、` stay attached to the preceding glyph during emergency-wrap classification
- slash-aware wrap classification so `alpha/beta` style text stays on the normal break path instead of being treated as one unbreakable token
- hyphen-aware wrap classification so `alpha-beta` style text also stays on the normal break path instead of falling back to emergency wrapping
- CJK opening-punctuation clustering so characters like `（` stay attached to the following glyph during emergency-wrap classification
- CJK closing angle-bracket clustering so characters like `》` stay attached to the preceding glyph during emergency-wrap classification
- CJK white square bracket clustering so bracket pairs like `〚漢〛` stay on a single East Asian punctuation cluster during emergency-wrap classification
- CJK tortoise-shell bracket clustering so bracket pairs like `〔漢〕` stay on a single East Asian punctuation cluster during emergency-wrap classification
- CJK lenticular bracket clustering so bracket pairs like `〘漢〙` stay on a single East Asian punctuation cluster during emergency-wrap classification

## Autoresearch

Automated experiment loop inspired by the [Karpathy autoresearch](https://x.com/karpathy/status/1886192184808149383) pattern. An LLM agent modifies source code, runs build/test/evaluation, and keeps the change only if the fidelity score improves — otherwise it reverts.

```bash
# Run a specific phase
./autoresearch/run_loop.sh --phase 01_color_fidelity

# Limit iterations
./autoresearch/run_loop.sh --phase 02_performance --max-iterations 50
```

| Phase | Target |
|-------|--------|
| `01_color_fidelity` | Theme color modifier accuracy (12 modifier types) |
| `02_performance` | Rendering throughput optimization |
| `03_effect_rendering` | Shadow/glow DrawingML → CSS conversion |
| `04_geometry_coverage` | Preset shape expansion (30 → 187) |

Results are logged to `autoresearch/results.tsv`. See `autoresearch/program.md` for the full protocol.

## Evaluation

The project now treats PowerPoint-native references as the primary fidelity oracle and LibreOffice references as a secondary regression signal.

```bash
cd evaluate
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt && playwright install chromium

# 1. Generate the deterministic golden fixture set
python create_golden_set.py

# 2. Render references via LibreOffice headless (secondary regression signal)
python reference_render.py --input golden_set/ --output golden_references/

# 2b. Render PowerPoint-native references when a Windows/PowerPoint environment is available
pwsh -File ./reference_render_powerpoint.ps1 -InputDir ./golden_set -OutputDir ./powerpoint_golden

# 3. Compute composite fidelity score
python evaluate_fidelity.py --project-root ..
```

Composite score: `0.40*SSIM + 0.25*TextMatch + 0.25*TestPassRate + 0.10*Performance`

Use the composite score for regression control, but require a PowerPoint-reference check before labeling a feature `exact`.

See [`evaluate/README.md`](evaluate/README.md) for details, including the exactness contract checker and the shared Python 3.11+ floor used by the CI/release evaluation workflows.

## pptx2html-enhance (LLM Post-Processing)

Optional Python package that uses LLM providers to enhance the Rust converter's output. Replaces structured placeholders (SmartArt, Math, OLE) with semantic HTML generated by an LLM.

### Install

```bash
pip install ./pptx2html-enhance[anthropic]   # or [openai] or [all]
```

### Quick Usage

```python
import pptx2html
from pptx2html_enhance import enhance

# Step 1: Convert with metadata sideband
result = pptx2html.convert_with_metadata("presentation.pptx")

# Step 2: Enhance placeholders via LLM
enhanced_html = await enhance(
    result.html,
    [e.__dict__ for e in result.unresolved_elements],
    provider="anthropic",       # or "openai"
    timeout=30.0,
    max_concurrent=5,
)
```

### Supported Element Types

| Type | Handler | Strategy |
|------|---------|----------|
| SmartArt | `SmartArtHandler` | LLM converts raw DrawingML XML to HTML/CSS layout |
| Math (OMML) | `MathHandler` | Rule-based for simple formulas (fractions, scripts, roots); LLM fallback for complex equations |
| Effects | `EffectsHandler` | Rule-based: outer shadow → `box-shadow`, glow → `box-shadow`, soft edge → `filter: blur()` |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and submission guidelines.

## License

MIT - see [LICENSE](LICENSE)


## Generated PPTX capability registry

<!-- BEGIN GENERATED PPTX CAPABILITY MATRIX -->
<!-- manifest-sha256: dd24142f66dbd737b6ef27f77ac4bc433053bc1249e86965c34033a19b32da47 -->
| Feature | Current S/V/B | Target S/V/B | Verification SHA256 | Status SHA256 |
|---|---|---|---|---|
| <a id="capability-presentation"></a>`presentation` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `c07e2810b8d5e13a63436f7b11c3ee961e11b15f61bdc50a1ca260c0738e4a4f` | `29665c44b1b28428449e05099e8b3f5d22f1e577d8eaaf700a7f1c9a1b347de5` |
| <a id="capability-presentation-properties"></a>`presentation-properties` | approximate/parsed<br>fallback/not-applicable<br>fallback/not-applicable | approximate/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `03b3697960c6db57bc2d101452d5e8abc0a9ecd7ed2048d867a97032ccb94e5b` | `cf3d3cadc4899f4321326655a859005131cd42d60dc1e24accad86220543b42d` |
| <a id="capability-slide-master"></a>`slide-master` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `d26c42cad024a240ba42584139d32b0485d45f86a946ebc65d2cf2c2d9c920eb` | `2fcbe53ce1225a110400f235335397da53ab763ef52d242204931561cf098958` |
| <a id="capability-slide-layout"></a>`slide-layout` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `80a9fec92635d749ef0271cfb91e56a7c2b642a42f42a3719badde4160d0e329` | `fd2002a3e42946c1a1212cdb072c36fdc16f6aa2f56c1c6ae6920649413f4792` |
| <a id="capability-slide"></a>`slide` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `d7216600198cf446aa21948013131473434b57228017fdd7c2eea16a3aee2ed7` | `9ed1789d738b9c6f29e7712866cb1b72ef0b9798f5f78d5e3210d92d59eeaf4c` |
| <a id="capability-theme"></a>`theme` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `a1050e25c09f1b3687932cd923ac2c5e9ac8b8bd04ea694e1af75f7ff6397807` | `70df65b760e43407d76fcadcbc3fb5e52fe68c9cd94624c584352cc2bffb0921` |
| <a id="capability-notes-master"></a>`notes-master` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `7c0f6c034617ee80dfedda6fad705b98bd052084f09a7878d8f44c0b8637b507` | `f2dcd5a888468034bfcb5e696a84f70f017ab138c1727937b79cbbd743f21e3a` |
| <a id="capability-notes"></a>`notes` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `1b5af7f5ec83a70268e65aa5017a47d559c69452cea72f455c343edd4ac94e51` | `1e0e297d3d1c8e823ed852c6eb690944605bbf290c62c24bf39300d901642b7f` |
| <a id="capability-handout-master"></a>`handout-master` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `891c69a9b73e211f98dba58561eff7c132fccbe56f73cc738a94d39aa81c3b4a` | `9d44cff55da2c0159e8c5dcc8ead0ff6e9769ead1dd7e6e0c3efaabb2b811497` |
| <a id="capability-comments"></a>`comments` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `944fa74d1b1a1aec97d94eee1d54feb252a2b139a54939ff9388ded6595591b9` | `2ea3f2aafdfa77fd66c34f43fb85bfb4f993bf50cba40edee5eda1165d8340e9` |
| <a id="capability-comment-authors"></a>`comment-authors` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `3fdf939a544a498dda287c4cbec1ef75ccfbe8b3f5aa080ef114614b91d7900a` | `85ea90cb75643a556bd9dba65f0ce49610b7ff62b985d3ea8636f6cfbaa3ed1b` |
| <a id="capability-shape-tree"></a>`shape-tree` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `039ce2b4d821932f9c2243102b5c97dbcd41d0f4ecfc0f7e01b0fde941e7805e` | `3f86cff8d830a06e21d3779e44a9b21194756e2ad8955aefdeaba3fc9db1162a` |
| <a id="capability-preset-shape"></a>`preset-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `cb84ad4e1f0ca5b1849c7a3331a9a878a3d0b3818352f158c405e19c87a88fd2` | `5d446d085d5c42ea91cc6540d5b83bbfaca15e62afe42e6f9c20d4d59ea9a86f` |
| <a id="capability-custom-geometry"></a>`custom-geometry` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `244b8537a5f7fd49e3fafd5a462a12d5f6cf0408a8cf3235e7645b0baefea8f5` | `99c76b2c42fdf8b00e68efc337816db612d39bb09426e39028af8db8b1051083` |
| <a id="capability-connector"></a>`connector` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `7e7b1b3a0a60e49d6702574dba2a1929d3e4c82abd8f7b60a7d162a0f63fa509` | `f469f88311b3de633ad23f2d8257cd92e2faaa75299ee824ac1279ee1f00367c` |
| <a id="capability-group-shape"></a>`group-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `393472e96359637a79aa7a838f6c16db5b9d71b24cb648fefea81e3a646a41fb` | `e5f16afa6c7699ece99d11402306f0119f415730b8889499312d6be6083db36e` |
| <a id="capability-picture"></a>`picture` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `dffb48ca4b06c68069e0b407c9934ceaceb8dabf447bedac71f10b581a2ac645` | `7199c2265f56c189e0b25a8f38529f37da9174155adb2c46b2e236d3105947f8` |
| <a id="capability-text-body"></a>`text-body` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `262f4fb2d080594a9c78a70b702253e646af04a1e7e86f2d9b8debfe18f15e8b` | `bbbb778196c659c4ba3931d9f51c8383575a005812fde7c4f92a85d90cf53e89` |
| <a id="capability-rtl-text"></a>`rtl-text` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `c503fb10524fa65e82d1d4ea5d4de2579f51949547d1de8ad5cb1b496f0070e5` | `85173066116d7250da3058a7f80b43b147cfeef918f4cf802bbf94dff3613c65` |
| <a id="capability-bullets"></a>`bullets` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `ed157a688196eea774392c88ae5db59cb6cfc0f7167532360488ca899ebdff3d` | `7083d9593322381b21f9ac938277da2637c57b8e9663fe7baf886efe289ff341` |
| <a id="capability-picture-bullets"></a>`picture-bullets` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `7d1c21ce2540da7b56a5a48196f9f4d69d56c985e23afd6772a5b96d1de5508f` | `d4d97387d415bb350ee62522151319c7190d7a60f9fc6a33ad16fd2953d680d0` |
| <a id="capability-fills"></a>`fills` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `f7a7e6203cadf6138eda6a0262ea7f8413a200044cbaa8be71445d6ee0d08e7b` | `27f0d1439c068d3dcdc802df5c98749ad63a526753bf4e411cd97c0a5025cac2` |
| <a id="capability-pattern-fill"></a>`pattern-fill` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `aa65f7e14d906cfa690b48408c5e59168b09e5ec7f29366695a335778beb8fab` | `e7687dc0b1523f4d8d835a27538507091664a2af8daf52c7cefa2253b28a7171` |
| <a id="capability-effects"></a>`effects` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `768bd8e0e131deaf5a963f37a66952f4287ebff4a860cf8d2fda726f2f67968d` | `7e79f784844b8576e35fa68dce69588d336125fab6a6e84caf40373b91880b73` |
| <a id="capability-reflection-and-3d"></a>`reflection-and-3d` | fallback/parsed<br>approximate/rendered<br>fallback/not-applicable | fallback/parsed<br>approximate/rendered<br>fallback/not-applicable | `05625623d02d2afb0f7c3529951fd70e1f3611f7ba5acacb447b5e512abac08d` | `22d06b2ad85fb0a25de923ea582f347b688efbb1330a7e110660f45afea9c183` |
| <a id="capability-table"></a>`table` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `4e5951bb9a4549790b7adc79890517a1225009b40688d246c11850c66101d192` | `7ae399ecfa572df16f042587cb995cdb8c754fbf48cc584ff6c28c79083e8d3b` |
| <a id="capability-table-style"></a>`table-style` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `e87d1531fdab2c0c063de4a617627c411454f05c6359e2b93c499fed5638617e` | `8507e8b5258344ccbf42786395cd9e9c1305007d9abc67292710353c91254cce` |
| <a id="capability-image"></a>`image` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `66ee0ff62f62adf90b2cb61bd3298f76d6db7d7e54e03632ffd5ff38e026714a` | `cf16268eadaa17f2829467c88b11c2858d7c58fd445c2c45d803d7b38ac8c213` |
| <a id="capability-chart-direct-subset"></a>`chart-direct-subset` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `9f1b331a89dc0443e3f4a31837f1ab9da612a9570c789fd9dd8e0503e9600643` | `377a904a5d76d39a2ba0164bfcaa24fe1b451c01555b70940225ffd655df7287` |
| <a id="capability-chart-preview-fallback"></a>`chart-preview-fallback` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `9b15b4f2cefcc9a46086fd4b54264d753f9e874554b4e153c0e4f8f5fb15ea29` | `587a7fd372d58f5da936b784d45cbdfa7536d5c3a5a95d31d5274264c8dc0c73` |
| <a id="capability-chart-placeholder-fallback"></a>`chart-placeholder-fallback` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `5f0b4fdecb60710becd532d16762d18734667afea2cda8d28449a5f25da1f9ad` | `4d57460e0f8ebae9e2e593c40d9876782b2e0bb6cfd1dfb8eb6d8e9730b8d49b` |
| <a id="capability-diagram"></a>`diagram` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `0cecd24eb6161f5bad365f66ddf4877732436c3ca3e0e67dfb2a76475572cf3b` | `2bb9eca9b9fd5342b7090b50836f0832acfe59b7d877dd77a8a172efcd3d2e0b` |
| <a id="capability-diagram-data"></a>`diagram-data` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `33d058f921ab4bf96eb875079f95b5c6a103dfd9fbb60ecb5c6b54684882aa19` | `e63c3b734b25079b0df064d2f74f4f085d4d8e6b345afb3b04b45c6f639625fb` |
| <a id="capability-diagram-layout"></a>`diagram-layout` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `39729d3ac8e6afc2d55966c8170f8fbc9412921364b9c031faf980945f9e08fb` | `5c71485b56affe554eccbc54e7c24d5f8d267033dc34b95065fe6ddab4da9427` |
| <a id="capability-diagram-styles"></a>`diagram-styles` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `0dce3209140ee3800b43953d43b6d77dd727cbe19ea1699d088bf2ffccee8725` | `e5cfd249fd43693753b370d54b8846c9eb397e0583fb734ef665c394db77ee19` |
| <a id="capability-diagram-colors"></a>`diagram-colors` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `835995fb45ba39bcffc948c66a1714647a4dde4f45b860bbf04c6e32918dc681` | `c8048fc748ccbf5216d5c9b3e55fcef0ac3fcd062ca75ee75f10378a49429032` |
| <a id="capability-ole-embedded-object"></a>`ole-embedded-object` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `9a1ea008d8a2422d170624f54c315e1e1ff435dee7a9f7528ab130827840486b` | `03972ca8681ad5adfff52f278be1c4c35b0ebaf19251d75f88b1f4eed8a04cc6` |
| <a id="capability-math"></a>`math` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `f902b654054d7ac1aaea679b5832d73bbb121c6d14a593df127bb97a77df9dbd` | `98676a3ba2f695ae7b3fc77b29d51d0b65cc21c8cdbe976aa61777b5637c29c6` |
| <a id="capability-media-audio"></a>`media-audio` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `72f9f2545ef7b485e028296680e9943b5b679f55ec7bfc267a4659fa459c2bdb` | `115a7ac4ad92809c52144bca695530c20c42876eb4cd62a92903a793721370ef` |
| <a id="capability-media-video"></a>`media-video` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `2de9f9aa1ac20fdda24dff34d3317856b28bc00dcaee216df808cee57158ae08` | `55c5b1bd4d7d05b9e7f5297572607be5fd9e1607eb98bec331ea411c041b83db` |
| <a id="capability-hyperlink-run-and-cell"></a>`hyperlink-run-and-cell` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `8078ebcf0df8602a6acf21547e7e42a8ade526d127fd7e921d249ae07b88d993` | `57dc2d2d733cbce264d1b225496048d1d95072ddb15fce3c38f4b8728124983b` |
| <a id="capability-shape-hyperlink-and-action"></a>`shape-hyperlink-and-action` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `d240b3956c52dba4526750cddd4d9c7a2690f59c295075997f6e3bb46b71664f` | `db947d63b09b3d26c18ff44bc685731344501a9beb14ad530512501e04230603` |
| <a id="capability-timing-and-animation"></a>`timing-and-animation` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `ee976c5f050029d337e0ea3a1ff5cfe3351b9aa59f3da5042e507eeaecfa521f` | `30e10705c96190b94004219490a97a7116fcf5f49a9c0b45ca5730fe39f1ce35` |
| <a id="capability-transitions"></a>`transitions` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `e06c8a2724ec2b5c11b4f4fbea9c88c66d4957fa756d15c2d27a543f6cf6719c` | `bdc5bc99fe9a448365a3d9721e6e67ca4df2fea07b674856df476f9aedfaef1c` |
| <a id="capability-extensions"></a>`extensions` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `4894bba77c5de06b7102b327fc78201befcc59a7b37cf9aa2f85c1f8e6ac0305` | `b36f463983b9b6f31f21ee7624b8179f3c336069e97235efd07f4c6933e6ad25` |
| <a id="capability-alternate-content"></a>`alternate-content` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `568918777892e84262c3bf521a5297a698db8831598d085a54cbf2840280c221` | `fb843b603490ab7412c7d1c34c18389bbfb9b5d8b973116d530064eec8caee18` |
| <a id="capability-bibliography"></a>`bibliography` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `e23a461817a9ded877cb7eb1e4979501178769765e246971ab74578a4ffe4ebb` | `30ae0425dec8aa78fc8c534d721be0277cce56b84761cfbdc4562175005a5f25` |
| <a id="capability-additional-characteristics"></a>`additional-characteristics` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `f10ed7446d28df9e489140d5c04044a23d86d782cdcfad33eaf6fb000fc8aaf2` | `3be632f7c8a7c60cff5633dd014bdf1f7e036a8c6431adc7bec1e6b8ec3ab2af` |
| <a id="capability-custom-xml"></a>`custom-xml` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `6328918018db2ff76d4aa2d8c8b27bbdace4bc71d46fba6b764209026b2c94c6` | `0e27bb416ec6d01d306d50e4976418e0743916f7531705fd88f99aa855983008` |
| <a id="capability-thumbnail"></a>`thumbnail` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `eea1202e0937556ba322f690e25073981337ab75cb3f640432aed42981fb1a83` | `ac63bbea2b37bedfb131e943838539e1d7373e7a3686fea14f95ee8dfed820c3` |
| <a id="capability-theme-override"></a>`theme-override` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `a6c3565ab75b88f7bcd341876512c4752c266a017d1d6d5ba08aa37b5cda995d` | `8dbd8139836a153e1e69009efcb939bd980708c67e19e790aafba14bb2c71dc9` |
| <a id="capability-slide-synchronization"></a>`slide-synchronization` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `d251deca6b42414d070751e4e079abd3c75abebf6fb296bb9c61d48be6e604d1` | `7108f8d030277f501eccd5e01cfef2389496178cf6128c7ae5248a8b067d1d42` |
| <a id="capability-content-part"></a>`content-part` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `2f7dce33c2e6808355a43fe0820855450ade1abd9fffe83bf6989965dc3da5d9` | `a6dc798a71b64907ffa02c9c93548a78f91ec78b0fca9852ffa861abd11f649e` |
| <a id="capability-embedded-package"></a>`embedded-package` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `9c9a08d8fb4442f66df36bc3de23ca6a0d0448bab2260996ed41c262cca6d5c0` | `1027870090ccee53b686f31b5098514211c5799b534f317f604406e734c57627` |
| <a id="capability-embedded-control-persistence"></a>`embedded-control-persistence` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `8cf43f357e46ee3defd6250fa099d6f88a37f4ac976b58cb6e5c6898c1785ce2` | `9353e1d1789f94b67689440757d4617fa6f283426188298e1914fb12f0922f82` |
| <a id="capability-user-defined-tags"></a>`user-defined-tags` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `d2bd311c48e46b4ba4449d05eb1b99762d2cc782adb325ec275a07b84c29a6d7` | `c5d90044021cd20e3c67fe72a821ca0073e55a3dde89af7916abfc57ce31f26c` |
<!-- END GENERATED PPTX CAPABILITY MATRIX -->
