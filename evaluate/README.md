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


## Pinned PowerPoint machine provenance

Exact-promotion evidence uses pinned machine provenance, not cryptographic attestation. Every capture must identify the exact producer `Microsoft PowerPoint`, platform `Windows`, nonempty PowerPoint version and build, timezone-qualified capture timestamp, and stable batch ID. The batch manifest, per-deck metadata, canonical PPTX source SHA-256, metadata SHA-256, and each structurally valid PNG SHA-256 must cross-link exactly. PNG validation covers signature, bounded chunks and dimensions, CRC, IHDR, IDAT, and IEND. LibreOffice, browser, fabricated secondary bytes, or any other producer is rejected even if metadata labels the oracle `PowerPoint-native`.

## Generated PPTX capability registry

<!-- BEGIN GENERATED PPTX CAPABILITY MATRIX -->
<!-- manifest-sha256: f843d47cacbb6a2a2727832f5fffdc3ff0d8fafb20e0d9d730cff27d60c4b897 -->
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
| <a id="capability-handout-master"></a>`handout-master` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `15d156630f4af4daa2f61b6591fe5ea0bd2b3d4ce79fafe34c6fc5aac2bd470b` | `b32c883fe44b285780e6c5e306e0a2e550b706ebde1d244bedbe02bfc6bdbc5b` |
| <a id="capability-comments"></a>`comments` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `944fa74d1b1a1aec97d94eee1d54feb252a2b139a54939ff9388ded6595591b9` | `2ea3f2aafdfa77fd66c34f43fb85bfb4f993bf50cba40edee5eda1165d8340e9` |
| <a id="capability-comment-authors"></a>`comment-authors` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `3fdf939a544a498dda287c4cbec1ef75ccfbe8b3f5aa080ef114614b91d7900a` | `85ea90cb75643a556bd9dba65f0ce49610b7ff62b985d3ea8636f6cfbaa3ed1b` |
| <a id="capability-shape-tree"></a>`shape-tree` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `039ce2b4d821932f9c2243102b5c97dbcd41d0f4ecfc0f7e01b0fde941e7805e` | `3f86cff8d830a06e21d3779e44a9b21194756e2ad8955aefdeaba3fc9db1162a` |
| <a id="capability-preset-shape"></a>`preset-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `cb84ad4e1f0ca5b1849c7a3331a9a878a3d0b3818352f158c405e19c87a88fd2` | `5d446d085d5c42ea91cc6540d5b83bbfaca15e62afe42e6f9c20d4d59ea9a86f` |
| <a id="capability-custom-geometry"></a>`custom-geometry` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `244b8537a5f7fd49e3fafd5a462a12d5f6cf0408a8cf3235e7645b0baefea8f5` | `99c76b2c42fdf8b00e68efc337816db612d39bb09426e39028af8db8b1051083` |
| <a id="capability-connector"></a>`connector` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `7e7b1b3a0a60e49d6702574dba2a1929d3e4c82abd8f7b60a7d162a0f63fa509` | `f469f88311b3de633ad23f2d8257cd92e2faaa75299ee824ac1279ee1f00367c` |
| <a id="capability-group-shape"></a>`group-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `393472e96359637a79aa7a838f6c16db5b9d71b24cb648fefea81e3a646a41fb` | `e5f16afa6c7699ece99d11402306f0119f415730b8889499312d6be6083db36e` |
| <a id="capability-picture"></a>`picture` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `dffb48ca4b06c68069e0b407c9934ceaceb8dabf447bedac71f10b581a2ac645` | `7199c2265f56c189e0b25a8f38529f37da9174155adb2c46b2e236d3105947f8` |
| <a id="capability-text-body"></a>`text-body` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `3179eda0c97d469d36443ad0ae40f908c4d9ffaf0808c9ee704a71910119e6d4` | `083e42ef74c2c0f2c2f854b2438bea7c57c58b65f0eb7872434727244c8a54f9` |
| <a id="capability-rtl-text"></a>`rtl-text` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `fbb1190906b43337ab3fd9e33bfc218617750ab41ac88aa65e0d458a4315f542` | `00e7947e6402506a10694288ebc34504577074e467be4ff65ec677d0f7313295` |
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
| <a id="capability-extensions"></a>`extensions` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `db6e32214504a42ec3f35a9012634431e8f4d6d979ef9ba877868e45d6981f0e` | `f5fc868090d780d74bb0d8832785e45d774756186d350a8d32105b04c0215e5e` |
| <a id="capability-alternate-content"></a>`alternate-content` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `568918777892e84262c3bf521a5297a698db8831598d085a54cbf2840280c221` | `fb843b603490ab7412c7d1c34c18389bbfb9b5d8b973116d530064eec8caee18` |
| <a id="capability-bibliography"></a>`bibliography` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `cfd9306097e9322647cb5af3f1aedce173337ace3577ae5f10b58a648a509cf0` | `10ed698f73bf8cdafddd8eb3652e2afcdc3cbc83b122d989c7d95acd86f445ae` |
| <a id="capability-additional-characteristics"></a>`additional-characteristics` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `59cdef94881b0e212d15141e26b244236d905f8077986e39fc6de732cfec7967` | `87dc7c3c5f519da24bb771c23e2993ad4ad75be335bc5758475e844ce5ac38db` |
| <a id="capability-custom-xml"></a>`custom-xml` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `54a1433486fd59462089256b9d419e655cce5797045264cd979816d2b2958d6e` | `335ac0d926aeb7fd14775486967a12e89122bffbe670cabc58d11b095928ffb9` |
| <a id="capability-thumbnail"></a>`thumbnail` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `576278620fb48fa1383d38383fd0c02cf3d071cf80313089385684000c1368a3` | `44536ddbb0d638731dd84dc0a1c4486d21048cc55b445e637871946bbc1920d7` |
| <a id="capability-theme-override"></a>`theme-override` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `a9f42cffb203f72c64a5c3ace79b49f846b6972c7b83bf0d5fc175e265957de9` | `97cfde325ed38d199e01d9a6cba40f52c95af725e073889eee23551d37d731d2` |
| <a id="capability-slide-synchronization"></a>`slide-synchronization` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `1179d5e13281da76556be234550d136685903994c4b23036846fa04468ce37c2` | `778b0fe9e6e62ba9117170712b6b10e6eda50496ef3d03608112a316f8a8835a` |
| <a id="capability-content-part"></a>`content-part` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `17026cb3addec250adf2ee386c1339b61660e06f0ba59fc2b8106fb8e8f8030f` | `87542d7f904c1055387e6f5c2c16d36278f5ccfe2e46785afa6e24c100756649` |
| <a id="capability-embedded-package"></a>`embedded-package` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `0b5c84956bcda3b455fdd786bfe65bef700d73de90f4c8a2c7696901a6ec38a4` | `ca08db15b3da1ab2f3cd55ff47b6980e3cb9f15e22c1a5c2445255af0891a1e3` |
| <a id="capability-embedded-control-persistence"></a>`embedded-control-persistence` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `8be23af6eb9638f4aed07d697c5d8abf50ea26884873e15222d412df81b927fb` | `f0f169936fe00c2e295474e131cc5d56865c4b57f81ba5c93742ed47befec463` |
| <a id="capability-user-defined-tags"></a>`user-defined-tags` | unparsed/not-applicable<br>unparsed/not-applicable<br>unparsed/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `ad7c41c48ec98943c62f35c56c5700068c2c5c42b33c1b5c6c95eb7c829f8121` | `34cadc54e268123edf5ef9f028f4abeec3cd8a9d39e1be2298669b87cf336f23` |
<!-- END GENERATED PPTX CAPABILITY MATRIX -->
