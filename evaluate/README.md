# Evaluation Infrastructure

Objective scoring pipeline for the pptx2html-rs autoresearch loop.

The evaluation strategy now has two tracks:

1. **PowerPoint-first fidelity validation** for features that claim `exact` support.
2. **LibreOffice-backed regression detection** for fast, broad visual comparison during iteration.

The existing composite score remains useful for regression control, but it is no longer the only fidelity signal.

## Composite Score

```
fidelity_score = 0.40 * ssim + 0.25 * text_match + 0.25 * test_pass + 0.10 * perf
```

| Weight | Metric         | Description                             |
|--------|----------------|-----------------------------------------|
| 0.40   | SSIM           | Structural similarity vs LibreOffice    |
| 0.25   | Text Match     | Token-level Jaccard on extracted text    |
| 0.25   | Test Pass Rate | `cargo test --workspace` pass ratio     |
| 0.10   | Performance    | Slides/sec normalized to 50 sps baseline|

## Prerequisites

- Python 3.11+ (matches the CI and release workflows)
- LibreOffice (for reference rendering)
- Poppler (`pdftoimage` — `brew install poppler` on macOS)
- Chromium (installed automatically by Playwright)
- Rust toolchain with `cargo`

## Setup

```bash
cd evaluate
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
playwright install chromium
```

## Usage

### 0. Understand the two reference tracks

- **Primary:** PowerPoint-native exports in `evaluate/powerpoint_golden/`
- **Secondary:** LibreOffice-generated PNGs in `evaluate/golden_references/`

Use PowerPoint references before promoting any feature to `exact` in the capability matrix.

### Text/Layout exact-promotion gate

Before upgrading **Text** or **Layout / inheritance** to `exact`, keep the following evidence bundle together:

1. **Fixture coverage** from `create_golden_set.py` for all of these families:
   - `basic_text_08_narrow_box_autofit.pptx`
   - `basic_text_09_mixed_font_paragraph.pptx`
   - `basic_text_10_bodypr_fidelity.pptx`
   - `basic_text_11_wrap_gate_sentence.pptx`
   - `basic_text_12_wrap_gate_unbreakable.pptx`
   - `basic_text_13_autofit_modes.pptx`
   - `basic_text_14_complex_script_fonts.pptx`
   - `basic_text_15_mixed_script_single_run.pptx`
   - `basic_text_16_cjk_autofit_wrap_gate.pptx`
   - `basic_text_17_indic_complex_script_fonts.pptx`
   - `basic_text_18_emoji_cluster_segments.pptx`
2. **PowerPoint-native captures** for each deck under `evaluate/powerpoint_golden/<deck-name>/Slide*.PNG`.
3. **Local converter verification** with `cargo test --workspace` on the same revision.
4. **Capability-doc update** that records which fixture set and PowerPoint capture batch justified the tier change.

Behavior expectations for this gate:

- narrow-box wrapping should stay on normal wrapping paths unless content remains effectively unbreakable after ordinary break opportunities are considered,
- mixed-font and mixed-script segmentation should preserve intended run-level font resolution through the text/layout gate,
- mixed East Asian/Latin script boundaries should stay on natural wrap paths before emergency wrapping is considered,
- `normAutofit` / `spAutoFit` behavior should be evaluated together with wrapping decisions before exact promotion.

If any item above is missing, keep the family at `approximate`.

### 1. Generate golden PPTX test set

```bash
python create_golden_set.py
# -> evaluate/golden_set/*.pptx  (generated fixture set; category counts vary by coverage depth)
```

Filter by category:

```bash
python create_golden_set.py --categories basic_text shapes tables
```

### 2. Render reference PNGs (LibreOffice)

```bash
python reference_render.py --input golden_set/ --output golden_references/
```

### 2b. Render reference PNGs with PowerPoint (primary oracle)

On Windows with Microsoft PowerPoint installed:

```powershell
pwsh -File ./reference_render_powerpoint.ps1 `
  -InputDir ./golden_set `
  -OutputDir ./powerpoint_golden `
  -PowerPointChannel "Current Channel" `
  -WindowsVersion "Windows 11 23H2" `
  -OutputResolution "960x540" `
  -GoldenSetRevision <commit-sha>
```

The PowerShell export now scaffolds `metadata.json` in each deck directory and a root `manifest.json`. Validate the batch afterward with:

```bash
python validate_powerpoint_golden.py --golden-set-dir golden_set --output-dir powerpoint_golden
```

Summarize exact-evidence readiness in a human-readable JSON report with:

```bash
python summarize_powerpoint_golden.py --golden-set-dir golden_set --output-dir powerpoint_golden
```

The summary reports missing decks, missing metadata, incomplete slide exports, manifest consistency, batch identity, and an `evidence_ready_for_exact_promotion` boolean.

For a single entrypoint over scaffold / validate / summary / ready, use:

```bash
python powerpoint_evidence.py summary --golden-set-dir golden_set --output-dir powerpoint_golden
python powerpoint_evidence.py ready --golden-set-dir golden_set --output-dir powerpoint_golden
python powerpoint_evidence.py gate --family text-layout --golden-set-dir golden_set --output-dir powerpoint_golden
```

`gate --family text-layout` checks the exact-promotion fixture bundle from the Text/Layout gate and returns exit code 0 only when the required decks, metadata, slide exports, and manifest consistency are all satisfied.

The CI `evaluate-tools` job exports `powerpoint-evidence-summary.json` and `powerpoint-evidence-text-layout-gate.json` as artifacts so exact-evidence status stays visible even when the gate is advisory.

The tag-based `release.yml` workflow also attaches `powerpoint-evidence-summary.json` and `powerpoint-evidence-text-layout-gate.json` to GitHub Release artifacts so release consumers can inspect both the current exact-evidence summary and the text/layout promotion gate state.

The CI `evaluate-tools` job also runs `evaluate/check_exactness_contract.py` and exports `exactness-contract-report.json` so contract drift between docs and workflows fails fast.

If that environment is not available, keep the contract files in place and treat PowerPoint capture as a required external verification step.

For release preparation, pair these artifact checks with [`docs/release-notes/pre-release-checklist.md`](../docs/release-notes/pre-release-checklist.md) so exactness evidence is reviewed as part of the pre-tag checklist.

### 3. Run fidelity evaluation

```bash
python evaluate_fidelity.py --project-root /path/to/pptx2html-rs
```

Options:

```bash
# Evaluate specific phase only
python evaluate_fidelity.py --project-root . --phase theme_colors

# Verbose per-slide scores
python evaluate_fidelity.py --project-root . --verbose

# JSON output for automation
python evaluate_fidelity.py --project-root . --output-json result.json
```

### 4. Render candidate screenshots (standalone)

```bash
python candidate_render.py --html-dir output/ --output candidates/
```

## Directory Structure

```
evaluate/
├── evaluate_fidelity.py       # Immutable scoring function (DO NOT MODIFY)
├── reference_render.py        # LibreOffice headless -> PNG
├── reference_render_powerpoint.ps1 # PowerPoint COM export bootstrap
├── validate_powerpoint_golden.py   # Validate PowerPoint evidence batches
├── summarize_powerpoint_golden.py  # Summarize evidence readiness and gaps
├── scaffold_powerpoint_golden_batch.py # Scaffold metadata.json and manifest.json
├── powerpoint_evidence.py          # Unified CLI for scaffold/validate/summary/ready
├── candidate_render.py        # Playwright HTML -> PNG
├── create_golden_set.py       # Generate golden PPTX test files
├── requirements.txt           # Python dependencies
├── README.md                  # This file
├── golden_set/                # Golden PPTX files (generated)
│   └── .gitkeep
├── powerpoint_golden/         # PowerPoint-native reference renders
│   └── README.md
└── golden_references/         # LibreOffice reference PNGs (generated)
    └── .gitkeep
```

## Golden Set Categories

| Category     | Count | Tests                                    |
|--------------|-------|------------------------------------------|
| basic_text   | 18    | Bold, italic, sizes, colors, alignment, font fallback, vertical text, narrow autofit, mixed fonts, bodyPr fidelity, sentence-wrap gate, unbreakable-wrap gate, autofit comparison, complex-script fonts, mixed-script single-run segmentation, CJK autofit wrap gate, Indic and Thai complex-script fonts, emoji cluster segmentation |
| shapes       | 5     | Rectangles, ellipses, arrows, stars      |
| theme_colors | 5     | 12 theme colors, tint, shade, dark bg    |
| tables       | 5     | Headers, merge, colors, alignment, large |
| images       | 5     | Centered, tiled, overlay, bordered       |
| gradients    | 5     | Two-color, three-color, oval, dark bg    |
| groups       | 5     | Overlapping, rotated, concentric, z-order|
| layouts      | 5     | Title, content, two-column, section      |
| bullets      | 5     | Simple, nested, bold labels, colored     |
| mixed        | 5     | Dashboard, comparison, architecture      |

The deterministic completion `actions.pptx` is a separate contract fixture. Its presentation order is `slide1.xml`, `slide42.xml`, then `slide7.xml`; the specific-slide action on the first slide targets the third presentation slide. It also carries safe HTTPS/mailto, blocked JavaScript, click/mouse-over, four relative navigation actions, no-op, media, program/macro, shape, picture, connector, shape-run, and table-cell-run stimuli. This fixture verifies package structure and converter behavior, not exact PowerPoint boundary or hidden-slide traversal semantics (`[교차검증 필요]`).

## Autoresearch Integration

This evaluation infrastructure is the regression loop in the autoresearch pattern. The LLM agent:

1. Makes a code change to pptx2html-rs
2. Runs `evaluate_fidelity.py` to get a score
3. If score improved -> keep the change
4. If score regressed -> revert the change

The `evaluate_fidelity.py` file must never be modified by the LLM agent.
Only humans may change the scoring weights or metric definitions.

PowerPoint-reference capture is intentionally outside the autoresearch loop unless the environment is explicitly prepared for it.
