# pptx2html-turbo

Convert PPTX slides to high-fidelity HTML in pure Rust with direct rendering and structured fallbacks.

Built on the ECMA-376 open standard — no Microsoft dependencies, no C/C++ bindings, just Rust.

**[Live Demo](https://kim62210.github.io/pptx2html-turbo/)** — try it in your browser, no installation needed.

## Features

- High-fidelity layout preservation using absolute positioning
- Theme color resolution with 12 color modifiers (tint, shade, lumMod, etc.)
- Slide master / layout inheritance chain with placeholder matching
- 187 preset shape SVG rendering with broad adjust value support; known custom guide formulas render directly, while unknown, non-finite, or unresolved formulas preserve their exact raw formula and emit `DRAWINGML_CUSTOM_GEOMETRY_FALLBACK` instead of substituting zero or a default
- SVG stroke dash styles (solid, dash, dot, dashDot, etc.)
- Line ending markers (arrow, triangle, stealth, diamond, oval)
- Table, group shape, and connector support
- Image embedding (base64) or external references, with cropping
- Text styling: bold, italic, underline, strikethrough, super/subscript, bullets, vertical text, shadows, highlights, letter spacing
- Layout-preserving whole-slide scale/zoom across Rust, CLI, Python, and WASM surfaces
- Approximate direct chart rendering for clustered, stacked, and percent-stacked bar/column charts plus simple line, flat area3D, single-series/no-dLbls bubble (non-negative sizes, area semantics, bubbleScale support), multi-series/no-dLbls radar (approximate marker handling), single-series/no-dLbls ofPie (`ofPieType=pie`, `splitType=pos`), pie, doughnut, and flat pie3D charts
- Graceful placeholders for unsupported content (SmartArt, OLE, Math)
- Self-contained HTML output (single file, no external dependencies)

## Install

```bash
# npm (WASM — browser)
npm install @briank-dev/pptx2html-turbo

# Rust library
cargo add pptx2html-core

# CLI
cargo install --path crates/pptx2html-cli

# Python (requires maturin)
cd crates/pptx2html-py && maturin develop

# WASM (build from source)
cd crates/pptx2html-wasm && wasm-pack build --target web
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
```

### Rust Library

```rust
use std::path::Path;
use pptx2html_core::{convert_file, convert_file_with_options, ConversionOptions, get_info};

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
let html = convert_file_with_options(Path::new("presentation.pptx"), &opts)?;

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

# Conversion with metadata (SmartArt/OLE/Math sideband)
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

// Conversion with metadata sideband (SmartArt/OLE/Math)
const result = convert_with_metadata(data);
console.log(`HTML: ${result.html.length}, Unresolved: ${result.unresolvedElements}`);
</script>
```

A drag-and-drop demo page is included at `crates/pptx2html-wasm/demo/index.html`.
The included demo exposes image-like whole-slide zoom controls that keep the original slide coordinates and text flow intact.

## Supported Features

See [SUPPORTED_FEATURES.md](SUPPORTED_FEATURES.md) for the full ECMA-376 element inventory and [docs/architecture/CAPABILITY_MATRIX.md](docs/architecture/CAPABILITY_MATRIX.md) for the authoritative support-stage matrix.

| Category | Highlights |
|----------|-----------|
| Shapes | 187 preset shapes with broad adjust value coverage + custom geometry SVG rendering, guide formulas, and text rectangles |
| Text | Bold, italic, underline, strikethrough, super/subscript, vertical text, highlights, shadows, letter spacing, default 18pt fallback |
| Colors | RGB, theme, system, preset with 12 modifiers (tint, shade, lumMod, satMod, etc.) |
| Fills | Solid, gradient, image, noFill, and all 54 DrawingML pattern presets with approximate repeated SVG tiles; style references (fillRef/lnRef). Unknown or unresolved patterns emit `DRAWINGML_PATTERN_UNSUPPORTED` without an invented solid color. |
| Tables | Cell fill, borders, column/row spans, vertical merge |
| Images | Base64 embedding, deterministic external assets under `images/slide-N/`, cropping, MIME auto-detection |
| Layout | Master/layout inheritance, ClrMap overrides, placeholder matching, TxStyles, and bodyPr property carry-over (wrap, margins, vertical anchor, vertical text, autofit) |
| Bullets | Character and auto-numbered bullets with font, size, color |
| Charts | Direct clustered, stacked, and percent-stacked bar/column rendering with gap/overlap and first-pass data labels, simple line/standard area/scatter rendering with point labels and explicit marker handling, simple single-series radar rendering, axis titles, and single-series pie/doughnut plus flat-rendered single-series pie3D and area3D rendering, with chart-part preview-image fallback when available and placeholder fallback for unsupported chart families and complex variants |
| Unsupported | SmartArt, OLE, Math — structured placeholders with metadata sideband (raw XML, type, position) |
| LLM Enhance | Post-processing layer: SmartArt→HTML/CSS, OMML→MathML, DrawingML→CSS via LLM (pptx2html-enhance) |

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

The Rust core converts PPTX to HTML with high fidelity. Elements it cannot fully render (SmartArt, Math, OLE, and custom geometry) are emitted as structured placeholders with an ordered diagnostic sideband containing a safe source reference; custom-geometry formula fallbacks preserve the exact raw formula. Package-level unsupported parts and relationships are reported even when they do not produce a visible shape; relationship diagnostics identify only the source part and relationship ID, never the target. The optional Python `pptx2html-enhance` package uses placeholder metadata to transform supported fallback types into semantic HTML.

Rust consumers that construct `ConversionResult` should use `ConversionResult::new(html, slide_count)` and then populate any required metadata fields. The Task 11 `diagnostics` field necessarily makes legacy external struct literals source-incompatible, while the existing `unresolved_elements` field and its returned projection remain unchanged. `ConversionResult::diagnostics()` provides a stable ordered slice accessor.

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
│   ├── create_golden_set.py   # Generate 50 golden PPTX test files
│   ├── golden_set/            # Golden PPTX files (generated)
│   └── golden_references/     # Reference PNG renders (generated)
├── pptx2html-enhance/         # LLM post-processing for unresolved elements (Python)
│   ├── src/pptx2html_enhance/ # Enhancer, handlers (SmartArt/Math/Effects), providers
│   └── tests/                 # 32 tests with mock LLM provider
└── Cargo.toml                 # Workspace root
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full pipeline diagram and module responsibilities.

## Testing

```bash
# Rust tests (296 tests)
cargo test --workspace

# Python tests (32 tests)
cd pptx2html-enhance && .venv/bin/python -m pytest tests/ -v

# Benchmarks
cargo bench --package pptx2html-core
```

328 tests total, all passing:
- **Rust (296):** 109 unit tests + 178 integration tests + 7 CLI tests + 2 doc-tests
- **Python (32):** Enhancer pipeline, SmartArt/Math/Effects handlers, HTML patching (mock LLM provider)

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

# 1. Generate golden test set (50 PPTX files, 10 categories)
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
