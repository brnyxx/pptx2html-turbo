# Evaluation Infrastructure

Objective scoring pipeline for the pptx2html-rs autoresearch loop.

The evaluation strategy now has three tracks:

1. **PowerPoint-first fidelity validation** for features that claim `exact` support.
2. **LibreOffice-backed regression detection** for fast, broad visual comparison during iteration.
3. **Independent synthetic exactness** for deterministic parser and renderer contracts.

The existing composite score remains useful for regression control, but it is no longer the only fidelity signal.

## Seven-format acceptance gate

The universal document engine has a separate fail-closed product gate for
PPTX, DOCX, DOC, XLSX, XLS, PPT, and PDF:

```bash
uv run python -m evaluate.multiformat_gate \
  --reports-dir evaluate/multiformat/reports \
  --oracle-lock evaluate/multiformat/oracle-lock.json
```

The machine-consumed contract is
`evaluate/multiformat/contract.v1.json`. Every format must independently pass
the conformance, blind, component, stratum, minimum-unit, security,
determinism, review, and SHA-256 evidence-binding checks in the same wave.
Missing native Microsoft Office evidence is `INCOMPLETE`, never `PASS`.

Scaffold one new fail-closed evidence wave:

```bash
uv run python -m evaluate.scaffold_multiformat_evidence \
  --output-dir evaluate/multiformat/wave
```

Populate each schema-v2 corpus manifest, set its status to `READY`, and validate
it before any candidate conversion:

```bash
uv run python -m evaluate.multiformat_corpus \
  --manifest evaluate/multiformat/wave/corpora/docx/manifest.json
```

The validator streams every source hash below the manifest directory, rejects
path traversal and symlinks, bounds OOXML/CFBF/PDF structure, enforces exact
stratum quotas and contiguous unit ordinals, and requires canonicalized blind
producer identities with unique source hashes, source URIs, and template
families. Legacy sources bind their modern counterpart and independently
authored binary coverage. Security cases must exactly match the format-specific
families and expected outcomes in the contract. The validator also derives the
declared security precondition from the fixture bytes: OOXML package and
relationship structure, CFBF allocation and storage structure, or PDF
cross-reference, object graph, page tree, action, and payload structure. A
renamed, relabeled, arbitrary, or trigger-free fixture is rejected even when
its path and SHA-256 are updated. This source-level proof does not replace the
signed runtime evidence for actual rejection, network isolation, active-content
suppression, or resource bounds. Exit codes are 0 for `READY`, 1 for an invalid
corpus, and 2 for an untouched `INCOMPLETE` scaffold.

The product gate invokes this validator for every `READY` report. Corpus
validation alone does not prove that native Office inventories and metric
records are complete; those artifacts remain separately required and
fail-closed.

Capture candidate artifacts only in a clean, externally sandboxed checkout.
The sandbox attestation must prove disabled network access and that oracle
directories are absent from the candidate namespace. The runner performs two
fresh converter/browser runs and publishes `READY` only when HTML, every
inventory, and every PNG are byte-identical:

```bash
cargo build --release -p pptx2html-cli --bin document2html
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m evaluate.capture_multiformat_candidates \
  --contract evaluate/multiformat/contract.v1.json \
  --corpus-manifest evaluate/multiformat/wave/corpora/docx/manifest.json \
  --evaluator-manifest evaluate/multiformat/wave/evidence/evaluator-manifest.json \
  --oracle-lock evaluate/multiformat/wave/oracle-lock.json \
  --evidence-root evaluate/multiformat/wave \
  --output-dir evaluate/multiformat/wave/candidate/docx \
  --converter target/release/document2html \
  --soffice /locked/bin/soffice \
  --pdftohtml /locked/bin/pdftohtml \
  --pdfinfo /locked/bin/pdfinfo \
  --chromium /locked/bin/chromium \
  --font-bundle evaluate/multiformat/wave/font-bundle-manifest.json \
  --sandbox-attestation evaluate/multiformat/wave/candidate-sandbox-attestation.json \
  --sandbox-public-key evaluate/multiformat/wave/sandbox-verifier.pub \
  --openssl /locked/bin/openssl \
  --receipt-signer /locked/bin/candidate-receipt-signer
```

Browser loading uses one intercepted synthetic HTTP document and rejects every
other resource request. The sandbox attestation must carry a lock-bound Ed25519
signature from the trusted host verifier; self-authored JSON is rejected.
The signature carries a per-run nonce, and the evidence package hardlinks or
copies every executed converter/native/Chromium/OpenSSL binary so the product
gate re-hashes the exact bytes instead of trusting reported versions.
After both runs, the lock-bound host signer emits a second signed receipt that
binds the nonce, runtime identity, execution log, determinism manifest, and the
canonical root of every HTML, inventory, PNG, runtime binary, and package file.
Scripts, active objects, service workers, popups,
downloads, animations, unstable geometry, broken images, unpinned runtime
bytes, and noncanonical presentation dimensions fail closed. Paged native HTML
is generated at 144 DPI; 16:9 PPT/PPTX evidence is exactly 960x540. Spreadsheet
Chromium is launched with a generated fontconfig that contains only the
hash-locked font bundle, and the signed host attestation binds the effective
font-environment digest. Cells are emitted only when the DOM carries independently derived worksheet
and coordinate metadata—coordinates are never invented from visual position.

Metric evidence contains bound candidate/reference PNG and inventory paths,
never caller-supplied scores. The evaluator re-hashes and parses every artifact,
checks exact corpus unit/file/case coverage, computes the frozen visual,
content, and layout formulas, applies file-then-format blind aggregation, and
retains six decimals with `ROUND_HALF_EVEN`. It also recomputes security,
per-file determinism, reviewer coverage, quality, and performance hard gates.

After both capture manifests and all raw artifacts are present, assemble the
only gate-accepted report:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m evaluate.assemble_multiformat_report \
  --oracle-lock evaluate/multiformat/wave/oracle-lock.json \
  --evaluator-manifest evaluate/multiformat/wave/evidence/evaluator-manifest.json \
  --corpus-manifest evaluate/multiformat/wave/corpora/docx/manifest.json \
  --metrics-evidence evaluate/multiformat/wave/metrics/docx.json \
  --evidence-root evaluate/multiformat/wave \
  --output evaluate/multiformat/wave/reports/docx.json
```

The evaluator manifest binds exact NumPy, SciPy, scikit-image, Pillow, and
Playwright versions, Python 3.11, its Unicode database, and every scoring, schema,
aggregation, capture, test, and gate source file. A
manually edited aggregate report, stale dependency, missing unit, pooled blind
score, changed raw artifact, incomplete reviewer, or unequal clean run fails.

On a network-disabled Windows Office host, populate the positive native oracle
batch with `evaluate/capture_multiformat_office_oracles.ps1`. The capture
script opens the actual modern or binary source file read-only, disables
macros and link updates, exports native Office PDFs, exports PowerPoint slides
directly at 960x540, rasterizes page formats at 144 DPI, records semantic
inventories, and SHA-256-binds every artifact.

When evaluating a scaffolded wave, pass
`--evidence-root evaluate/multiformat/wave`. Report evidence paths must remain
under that root and their bytes must match the recorded digests.

See `docs/UNIVERSAL_DOCUMENTS.md` and
`docs/superpowers/specs/2026-08-20-universal-document-engine-design.md` for the
runtime architecture and full evidence rationale.

## Strict PowerPoint Pixel Gate

The strict gate is intentionally binary. It validates the PowerPoint-native
batch provenance, compares every browser candidate against its corresponding
PowerPoint PNG at the same dimensions, and fails when any RGBA pixel differs.
It never resizes images and has no tolerance threshold.

Generate the complete 16:9 corpus and export it on the pinned Windows host:

```bash
python evaluate/create_golden_set.py --output evaluate/golden_set
```

```powershell
pwsh -File ./evaluate/reference_render_powerpoint.ps1 `
  -InputDir ./evaluate/golden_set `
  -OutputDir ./evaluate/powerpoint_golden `
  -PowerPointChannel "Current Channel" `
  -WindowsVersion "Windows 11 23H2" `
  -OutputResolution "960x540" `
  -GoldenSetRevision <commit-sha>
```

After converting the same decks and rendering each HTML slide into
`evaluate/candidates/<deck>/slide_<zero-based-index>.png`, run:

```bash
python evaluate/strict_pixel_compare.py \
  --golden-set-dir evaluate/golden_set \
  --reference-dir evaluate/powerpoint_golden \
  --candidate-dir evaluate/candidates \
  --diff-dir artifacts/evaluate/pixel-diffs \
  --output-json artifacts/evaluate/strict-pixel-report.json
```

An `ok: true` result means zero mismatched pixels across the complete captured
batch. Absence of a PowerPoint-native batch is not a pass.

## Synthetic Scene Exact Gate

The synthetic gate compiles one immutable scene specification through two
independent paths:

1. scene specification -> PPTX -> pptx2html -> Chromium candidate PNG;
2. scene specification -> standalone HTML -> Chromium reference PNG.

The reference emitter does not import the PPTX emitter, converter, candidate
renderer, or strict comparator. The initial 100-slide vocabulary covers
whole-pixel slide backgrounds, opaque rectangles, overlap order, coordinates,
sizes, and solid colors. This gate proves those controlled contracts exactly;
it does not replace PowerPoint-native evidence.

Generate and compare the 10-deck, 100-slide corpus from the repository root:

```bash
rm -rf /tmp/pptx-synthetic-oracle
python -m evaluate.synthetic_pptx \
  --output /tmp/pptx-synthetic-oracle/decks
python -m evaluate.synthetic_reference \
  --output /tmp/pptx-synthetic-oracle/reference-html
python evaluate/candidate_render.py \
  --html-dir /tmp/pptx-synthetic-oracle/reference-html \
  --output /tmp/pptx-synthetic-oracle/references \
  --width 960 --height 540

mkdir -p /tmp/pptx-synthetic-oracle/candidate-html
for deck in /tmp/pptx-synthetic-oracle/decks/synthetic_*.pptx; do
  stem="$(basename "${deck}" .pptx)"
  target/release/pptx2html "${deck}" \
    -o "/tmp/pptx-synthetic-oracle/candidate-html/${stem}.html"
done
python evaluate/candidate_render.py \
  --html-dir /tmp/pptx-synthetic-oracle/candidate-html \
  --output /tmp/pptx-synthetic-oracle/candidates \
  --width 960 --height 540
python -m evaluate.synthetic_pixel_compare \
  --reference-dir /tmp/pptx-synthetic-oracle/references \
  --candidate-dir /tmp/pptx-synthetic-oracle/candidates \
  --diff-dir /tmp/pptx-synthetic-oracle/diffs \
  --output-json /tmp/pptx-synthetic-oracle/report.json
```

An `ok: true` report requires all 51,840,000 RGBA pixels to match exactly.

## Visual Element Coverage Matrix

`visual_element_coverage.json` is the machine-consumed inventory for visual
elements. It maps every `ShapeType` variant plus renderer-level features to at
least one evidence source:

- `synthetic-exact`: independent scene reference with exact RGBA equality;
- `challenge-proxy`: authored PPTX rendered by LibreOffice and Chromium;
- `rust-regression`: parser, resolver, geometry, or renderer contract test;
- `fallback-contract`: preserved unsupported content and diagnostics.

The challenging 10-deck corpus explicitly includes arrows, diamonds, curved
and straight connectors, charts, tables, text, cropped pictures, gradients,
groups, borders, rotations, and preset shapes. The coverage test generates a
real deck, inspects its OOXML parts, validates every manifest source, and fails
when a `ShapeType` variant or required element loses evidence:

```bash
python -m unittest evaluate.tests.test_visual_element_coverage -v
```

After producing the challenge proxy report, link every manifest element to all
ten corresponding deck/slide pairs:

```bash
python -m evaluate.visual_element_evidence \
  --manifest evaluate/visual_element_coverage.json \
  --deck-root artifacts/evaluate/decks \
  --proxy-report artifacts/evaluate/proxy-pixel-report.json \
  --output-json artifacts/evaluate/visual-element-evidence.json
```

Synthetic exactness and challenge proxy fidelity are intentionally separate.
An exact rectangle result cannot promote charts, tables, text, or complex
geometry to PowerPoint-native exactness.

## Exhaustive Preset Adjustment Matrix

The adjustment corpus is generated directly from `preset_adjustments.json`.
Its contract covers all 187 preset names, all 300 official preset/adjustment
pairs, and three deterministic cases per pair (`low`, `default`, and `high`).
The resulting deck contains 900 named shapes across 75 slides. Connector
presets use native `p:cxnSp` elements, and the corpus manifest records whether
each probe uses numeric bounds, default-to-bound interpolation, or an
unverified/unavailable official range.

```bash
python evaluate/check_preset_adjustments.py --repo-root .

python -m evaluate.create_exhaustive_adjustment_deck \
  --adjustment-manifest evaluate/preset_adjustments.json \
  --output-dir artifacts/evaluate/all-adjustments/decks

python evaluate/reference_render.py \
  --input artifacts/evaluate/all-adjustments/decks \
  --output artifacts/evaluate/all-adjustments/references \
  --dpi 150 --force

target/release/pptx2html \
  artifacts/evaluate/all-adjustments/decks/all_adjustments.pptx \
  -o artifacts/evaluate/all-adjustments/all_adjustments.html

python evaluate/candidate_render.py \
  --html-dir artifacts/evaluate/all-adjustments \
  --output artifacts/evaluate/all-adjustments/candidates \
  --width 960 --height 540
```

Generate `proxy.json` with `proxy_pixel_report.py` and `shapes.json` with
`shape_actual_coverage.py`, then enforce complete slide/shape linkage plus the
95% per-slide and 0.75 per-shape foreground SSIM gates:

```bash
python -m evaluate.adjustment_visual_evidence \
  --corpus-manifest artifacts/evaluate/all-adjustments/decks/manifest.json \
  --proxy-report artifacts/evaluate/all-adjustments/reports/proxy.json \
  --shape-report artifacts/evaluate/all-adjustments/reports/shapes.json \
  --output-json artifacts/evaluate/all-adjustments/reports/evidence.json
```

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

If that environment is not available, keep the contract files in place and treat PowerPoint capture as a required external verification step. In particular, macOS without PowerPoint must produce a nonzero text/layout gate with missing native decks or metadata; secondary-renderer evidence is not accepted for promotion.

After an intentional `evaluate/completeness_manifest.json` status or verification change, regenerate every capability block and digest marker, then run the gate:

```bash
python3 evaluate/check_exactness_contract.py --repo-root . --update-generated-docs
python3 evaluate/check_exactness_contract.py --repo-root .
```

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

The deterministic completion `actions.pptx` is a separate contract fixture. Its presentation order is `slide1.xml`, `slide42.xml`, then `slide7.xml`; the specific-slide action on the first slide targets the third presentation slide. It also carries safe HTTPS/mailto, blocked JavaScript, click/mouse-over, four relative navigation actions, no-op, media, program/macro, shape, picture, connector, shape-run, and table-cell-run stimuli. This fixture verifies package structure and converter behavior; boundary and hidden-slide traversal remain approximate and are not claimed as PowerPoint-equivalent.

## Autoresearch Integration

This evaluation infrastructure is the regression loop in the autoresearch pattern. The LLM agent:

1. Makes a code change to pptx2html-rs
2. Runs `evaluate_fidelity.py` to get a score
3. If score improved -> keep the change
4. If score regressed -> revert the change

The `evaluate_fidelity.py` file must never be modified by the LLM agent.
Only humans may change the scoring weights or metric definitions.

PowerPoint-reference capture is intentionally outside the autoresearch loop unless the environment is explicitly prepared for it.


## Pinned PowerPoint machine provenance

Exact-promotion evidence uses pinned machine provenance, not cryptographic attestation. Every capture must identify the exact producer `Microsoft PowerPoint`, platform `Windows`, nonempty PowerPoint version and build, timezone-qualified capture timestamp, and stable batch ID. The batch manifest, per-deck metadata, canonical PPTX source SHA-256, metadata SHA-256, and each structurally valid PNG SHA-256 must cross-link exactly. PNG validation covers signature, bounded chunks and dimensions, CRC, IHDR, IDAT, and IEND. LibreOffice, browser, fabricated secondary bytes, or any other producer is rejected even if metadata labels the oracle `PowerPoint-native`.

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
