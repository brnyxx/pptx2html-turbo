# PPTX Completeness and Honest Fidelity Plan

## TL;DR
> Summary:      Convert the open-ended "all PPTX elements and every adjustment maps 1:1" request into a finite, machine-checked coverage contract. Ship semantic preservation for every inventoried family, direct rendering only where the browser can represent the feature, deterministic fallback diagnostics everywhere else, and reserve `exact` for PowerPoint-native evidence.
> Deliverables:
> - Machine-readable PresentationML/DrawingML completeness and preset-adjustment manifests with drift gates
> - Modular model/parser/renderer seams that let independent worktrees own non-overlapping feature files
> - Test-first vertical slices for geometry adjustments, fills, bullets, tables, actions, notes/comments, effects/3D, media/timing, charts, and fallback-only domains
> - Cross-surface structured diagnostics, generated golden decks, local browser/LibreOffice evidence, and an exactness promotion gate
> Effort:       XL
> Risk:         High - OOXML is a relationship graph with renderer-, font-, codec-, Office-, and platform-dependent behavior, and this Mac cannot produce PowerPoint-native oracle renders.

## Scope
### Must have
- Define completeness as three independent dimensions: semantic preservation, static visual rendering, and behavioral playback. Each manifest row must declare one of `exact`, `approximate`, `fallback`, or `unparsed` for each applicable dimension; a feature is complete for this plan when it is directly supported or preserved with a deterministic fallback diagnostic, never when it is silently discarded.
- Inventory every supported PresentationML/DrawingML family and relationship/part kind exposed by the official format documentation, including slide/master/layout/theme, shapes/custom geometry, text/bullets, fills/effects/3D, tables, images, charts, diagrams, OLE, Math, notes, comments, media, hyperlinks/actions, timing, transitions, extensions, and `AlternateContent`.
- Preserve the existing 187/187 preset dispatch while adding a separate official-source adjustment contract. For each preset, record the official adjustment names, defaults/formulas/constraints when the official source supplies them; never infer a missing key or range from current Rust code or curated benchmarks.
- Make unknown or unsupported content observable through a typed diagnostic containing stable code, family, support tier/stage, slide index, part name, relationship id/type, qualified element name, bounds when known, raw XML/part reference, fallback kind, and reason.
- Keep `ConversionResult::unresolved_elements` backward compatible while adding the broader diagnostic contract and embedding a machine-readable diagnostic manifest in generated HTML, so even string-only conversion APIs do not silently erase off-slide parts.
- Build test-first, independently committable vertical slices. Every production change starts with a named failing test, captures RED output, implements only the bounded slice, captures GREEN output, and passes the affected package plus workspace checks before commit.
- Run independent tasks in sibling worktrees. The integration worktree exclusively owns hub files after seam extraction; feature workers own only their assigned modules/tests and must not edit another lane's files.
- Keep the existing direct chart subset `approximate`; classify every other chart type and preserve chart XML/preview fallback rather than claiming universal chart rendering.
- Treat SmartArt, OLE, complex Math, unsupported codecs, advanced 3D, and non-bounded timing as explicit fallback domains with stable metadata and static previews where the package provides them.
- Stop implementation when: (1) every completeness-manifest row is either tested direct support or tested fallback, (2) the preset/adjustment checker reports no unclassified preset or known official key, (3) unknown relationships/elements generate diagnostics, (4) all local gates pass, and (5) docs contain no `exact` promotion lacking a complete PowerPoint-native evidence bundle.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Do not promise universal semantic, pixel, or behavioral 1:1 equivalence between PowerPoint and a browser. The official package includes separate parts, Office extensions, timing state, external applications, and media behaviors that a static browser renderer cannot universally reproduce.
- Do not mark any new family or adjustment scenario `exact` on LibreOffice output alone. LibreOffice is a secondary regression oracle; PowerPoint-native capture with pinned metadata is mandatory for exact promotion.
- Do not invent undocumented adjustment keys, formulas, ranges, relationship types, API behavior, or feature support. If the official ECMA/Microsoft material does not provide a machine-readable per-shape fact, store `source_status: unavailable`, preserve the input value, and keep the item non-exact.
- Do not add or remove packages, change lockfiles, fetch proprietary fonts/codecs, or vendor third-party/AGPL implementation code without separate user approval.
- Do not attempt native OLE activation/editing, VBA/macros, Office add-ins, SmartArt auto-layout parity, advanced Office 3D lighting/material parity, or arbitrary codec/autoplay parity.
- Do not modify `evaluate/evaluate_fidelity.py`; the existing evaluation implementation is treated as a fixed downstream oracle. Add wrappers/checkers beside it.
- Do not push or force-push. Do not touch, delete, stage, or commit the existing untracked `.DS_Store`.
- Do not let parallel workers edit `crates/pptx2html-core/src/model/mod.rs`, `parser/mod.rs`, `parser/slide_parser.rs`, `renderer/mod.rs`, `lib.rs`, binding entrypoints, or capability docs after the owning seam/integration task has taken the lock.
- Do not merge implementation without its direct tests and evidence in the same commit. Do not use snapshots as a substitute for semantic assertions.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD + Rust built-in test harness for core/CLI/bindings, Python `unittest` for evaluation tooling, and real-browser QA for generated HTML
- QA policy: every task has agent-executed scenarios
- Evidence: `<attemptDir>/task-<N>-<slug>.<ext>` - under ulw-loop, `<attemptDir>` is the `currentAttemptDir` from `omo ulw-loop status --json` (`.omo/evidence/ulw/<session>/<goalId>/a<attempt>`); outside ulw-loop use `.omo/evidence/`
- RED/GREEN policy: before production edits, run the task's focused test and save the expected failure as `<attemptDir>/task-<N>-<slug>-red.txt`; after implementation, rerun the identical command and save `<attemptDir>/task-<N>-<slug>-green.txt`.
- Local-oracle policy: `/opt/homebrew/bin/soffice` and `/opt/homebrew/bin/pdftoppm` are available for secondary visual evidence; `pdf2image` and `skimage` are absent, so tasks must not require `evaluate/shape_actual_coverage.py` until the user approves dependency installation. Browser QA uses the already available Playwright/browser surface.
- Native-oracle policy: PowerPoint is unavailable on this Mac. `evaluate/reference_render_powerpoint.ps1` and the PowerPoint evidence workflow remain mandatory remote gates; local success cannot satisfy an `exact` promotion.

## Execution strategy
### Parallel execution waves
> Target 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks to maximize parallelism.

Wave 1 (no dependencies):
- Task 1: freeze the completeness contract and machine-readable inventory
- Task 2: build the official-source preset-adjustment manifest and checker
- Task 3: generate deterministic completion decks and evidence scaffolds
- Task 4: split the model into stable feature-owned modules without behavior changes
- Task 5: split reusable PPTX fixtures into feature-owned builders

Wave 2 (after Wave 1):
- Task 6: extract parser feature seams; depends [4, 5]
- Task 7: extract renderer feature seams; depends [4, 5]

Wave 3 (after Wave 2):
- Task 11: add the core preservation diagnostic envelope; depends [1, 4, 6, 7]

Wave 4 (after Wave 3):
- Task 8: close adjustment semantics for basic/rect/flowchart families; depends [2, 3, 5, 11]
- Task 9: close adjustment semantics for arrows/callouts/connectors; depends [2, 3, 5, 11]
- Task 10: close adjustment semantics for arc/star/wave/math/misc/custom geometry; depends [2, 3, 5, 11]
- Task 12: implement pattern fills; depends [1, 3, 4, 6, 7, 11]
- Task 13: implement picture bullets; depends [1, 3, 4, 6, 7, 11]
- Task 14: implement table style resolution; depends [1, 3, 4, 6, 7, 11]
- Task 15: implement typed hyperlink/action semantics; depends [1, 3, 4, 6, 7, 11]

Wave 5 (after Wave 4):
- Task 16: preserve notes and comments; depends [11]
- Task 17: implement reflection approximation and 3D preservation fallback; depends [11, 12]
- Task 18: preserve/render bounded audio and video; depends [11, 15]
- Task 19: preserve timing/transitions/animations and play only a bounded subset; depends [11, 15]
- Task 20: complete chart classification and deterministic fallback; depends [11, 14]
- Task 21: harden SmartArt/OLE/Math/AlternateContent/unknown fallback; depends [11]

Wave 6 (after Wave 5):
- Task 22: expose diagnostics on CLI/Python/WASM and run local browser/LibreOffice QA; depends [3, 8-21]

Wave 7 (after Wave 6):
- Task 23: enforce exactness/documentation drift gates and finalize the capability matrix; depends [1-22]

Critical path: Task 4 -> Task 6 -> Task 11 -> Task 16 -> Task 22 -> Task 23

Worktree/ownership protocol:
- Create an integration sibling worktree from the verified baseline: `git worktree add ../pptx2html-turbo-completeness -b feature/pptx-completeness c6e6fed`.
- For each parallel task, create `../pptx2html-turbo-task-N` on `feature/pptx-completeness-task-N` from the latest dependency commit. Workers commit only their listed files; the integration owner cherry-picks commits in task/dependency order and reruns the focused GREEN command after each pick.
- Tasks 4, 6, 7, 11, 22, and 23 are serialized integration-lock tasks. Waves 2, 3, 6, and 7 intentionally contain fewer than three tasks because each is a dependency barrier that owns shared hubs and must be cherry-picked before the next parallel wave.
- Task 11 creates compile-clean domain seams for `notes_comments`, `media`, `timing`, and `embedded` under model/parser/renderer. Tasks 16, 18, 19, and 21 own only those distinct domain files; they must not edit `model/preserved.rs`, `parser/preserved_parser.rs`, or `renderer/fallback.rs`.
- Geometry Tasks 8-10 and feature Tasks 12-21 start only after Task 11. This is required because their fallback/unknown-input acceptance criteria use the typed diagnostic codes introduced by Task 11.
- After each cherry-pick run `git diff --check`, the focused test, and `cargo test --workspace`; preserve `.DS_Store` as untracked and never include it in a pathspec.

### Dependency matrix
| Task | Depends on | Blocks | Can parallelize with |
|------|------------|--------|----------------------|
| 1 | none | 11-23 | 2, 3, 4, 5 |
| 2 | none | 8, 9, 10, 23 | 1, 3, 4, 5 |
| 3 | none | 8-10, 12-15, 22 | 1, 2, 4, 5 |
| 4 | none | 6, 7, 11-15 | 1, 2, 3, 5 |
| 5 | none | 6-10 | 1, 2, 3, 4 |
| 6 | 4, 5 | 8-15 | 7 |
| 7 | 4, 5 | 8-15 | 6 |
| 8 | 2, 3, 5, 11 | 22, 23 | 9, 10, 12, 13, 14, 15 |
| 9 | 2, 3, 5, 11 | 22, 23 | 8, 10, 12, 13, 14, 15 |
| 10 | 2, 3, 5, 11 | 22, 23 | 8, 9, 12, 13, 14, 15 |
| 11 | 1, 4, 6, 7 | 8-10, 12-23 | none |
| 12 | 1, 3, 4, 6, 7, 11 | 17, 22, 23 | 8, 9, 10, 13, 14, 15 |
| 13 | 1, 3, 4, 6, 7, 11 | 22, 23 | 8, 9, 10, 12, 14, 15 |
| 14 | 1, 3, 4, 6, 7, 11 | 20, 22, 23 | 8, 9, 10, 12, 13, 15 |
| 15 | 1, 3, 4, 6, 7, 11 | 18, 19, 22, 23 | 8, 9, 10, 12, 13, 14 |
| 16 | 11 | 22, 23 | 17, 18, 19, 20, 21 |
| 17 | 11, 12 | 22, 23 | 16, 18, 19, 20, 21 |
| 18 | 11, 15 | 22, 23 | 16, 17, 19, 20, 21 |
| 19 | 11, 15 | 22, 23 | 16, 17, 18, 20, 21 |
| 20 | 11, 14 | 22, 23 | 16, 17, 18, 19, 21 |
| 21 | 11 | 22, 23 | 16, 17, 18, 19, 20 |
| 22 | 3, 8-21 | 23 | none |
| 23 | 1-22 | final verification | none |

## Todos
> Implementation + Test = ONE task. Never separate.
> Every task MUST have: References + Acceptance Criteria + QA Scenarios + Commit.

- [ ] 1. Freeze the completeness contract and machine-readable inventory

  What to do: Add `docs/architecture/PPTX_COMPLETENESS_CONTRACT.md`, `evaluate/completeness_manifest.json`, and `evaluate/tests/test_completeness_manifest.py`. Define the three fidelity dimensions, allowed tiers/stages, stable feature ids, required fallback metadata, exact-promotion evidence fields, and the implementation stop conditions from Scope. Populate rows from official format families and current inventory; record an explicit source URL and OOXML qualified name or relationship type for every row. RED: make the test fail on an unclassified row, missing official source, or `exact` row without evidence. GREEN: make the committed manifest pass using only the Python standard library.
  Must NOT do: Do not claim current direct support merely because an XML name appears in source; do not mark anything newly exact; do not edit implementation or install a JSON-schema package.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [11-23] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/CAPABILITY_MATRIX.md:3-42` - current authoritative tier/stage rules and no-silent-loss rule
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/SUPPORTED_FEATURES.md:80-106` - detailed element inventory including picture bullets and pattern fills
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/tests/test_check_exactness_contract.py:1` - repository convention for standard-library contract tests
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document` - official part/family structure
  - External: `https://ecma-international.org/publications-and-standards/standards/ecma-376/` - official ECMA-376 edition and downloads

  Acceptance criteria (agent-executable only):
  - [ ] RED/ GREEN: `python3 -m unittest evaluate.tests.test_completeness_manifest -v` fails before the manifest/validator exists and passes afterward; both outputs are captured.
  - [ ] `python3 -c 'import json; d=json.load(open("evaluate/completeness_manifest.json")); assert d["dimensions"] == ["semantic","visual","behavioral"]; assert all(x.get("official_source") and x.get("fallback_policy") for x in d["features"])'` exits 0.
  - [ ] The validator rejects a temporary manifest row with `visual.tier=exact` and no PowerPoint evidence metadata, then the repository manifest passes unchanged.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Valid completeness inventory
    Tool:     bash
    Steps:    Run `python3 -m unittest evaluate.tests.test_completeness_manifest -v 2>&1 | tee <attemptDir>/task-1-completeness-contract.txt` from the repo root.
    Expected: Exit 0; every row has a stable id, official source, dimension tiers, fallback policy, and evidence rule.
    Evidence: <attemptDir>/task-1-completeness-contract.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: False exact claim is rejected
    Tool:     bash
    Steps:    Copy the manifest to `<attemptDir>/invalid-completeness.json`, set one row to exact with empty evidence using `python3 -c`, and run the new validator against that path.
    Expected: Nonzero exit with stable code `EXACT_REQUIRES_POWERPOINT_EVIDENCE`; the committed manifest is not modified.
    Evidence: <attemptDir>/task-1-completeness-contract-error.txt
  ```

  Commit: YES | Message: `docs: define the PPTX completeness contract` | Files: [docs/architecture/PPTX_COMPLETENESS_CONTRACT.md, evaluate/completeness_manifest.json, evaluate/tests/test_completeness_manifest.py]

- [ ] 2. Build the official-source preset-adjustment manifest and checker

  What to do: Add `evaluate/preset_adjustments.json`, `evaluate/check_preset_adjustments.py`, and a focused unit test. Start from the official ECMA-376 Part 1 download and Microsoft `a:avLst` contract, record source edition/URL/checksum, and normalize every `ST_ShapeType` preset to an ordered list of official adjustment names/default formulas/constraints. If the official material does not state a key/range, record `source_status: unavailable` and preserve/gate it instead of borrowing values from current code. Compare the 187 dispatcher names and every Rust `adjust_values` lookup against the manifest; report missing presets, unknown keys, and manifest keys never consumed. Keep custom geometry `a:avLst/a:gd` as a separate open-name formula contract.
  Must NOT do: Do not treat `docs/reference/preset-geometry-catalog.md` or `evaluate/create_adjustment_benchmark_deck.py` as authoritative; do not download from unofficial mirrors; do not claim official bounds that only appear in current implementation.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [8, 9, 10, 23] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry.rs:1-68` - 187-preset dispatcher contract and adjustment map input
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/create_adjustment_benchmark_deck.py:26-74` - current curated scenarios, useful only as non-normative fixtures
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/reference/preset-geometry-catalog.md:1-20` - existing partial catalog to supersede as source of truth
  - External: `https://ecma-international.org/publications-and-standards/standards/ecma-376/` - official Part 1 download source
  - External: `https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.adjustvaluelist?view=openxml-3.0.1` - official `a:avLst` semantics
  - External: `https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.presetgeometry?view=openxml-3.0.1` - official preset geometry contract

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `python3 -m unittest evaluate.tests.test_check_preset_adjustments -v` fails before the checker exists and passes after implementation.
  - [ ] `python3 evaluate/check_preset_adjustments.py --repo-root .` exits 0 and prints `presets=187`, `unclassified_presets=0`, and `unknown_consumed_keys=0`.
  - [ ] `shasum -a 256` of the downloaded official Part 1 artifact matches the checksum recorded in `evaluate/preset_adjustments.json`; unavailable per-shape facts are explicitly non-exact.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Dispatcher and official manifest agree
    Tool:     bash
    Steps:    Run `python3 evaluate/check_preset_adjustments.py --repo-root . --json <attemptDir>/task-2-adjustment-manifest.json`.
    Expected: Exit 0; 187 presets classified; every consumed key is official or explicitly marked unavailable/open-name for custom geometry.
    Evidence: <attemptDir>/task-2-adjustment-manifest.json   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Invented adjustment key is rejected
    Tool:     bash
    Steps:    Copy one geometry family file to `<attemptDir>`, add a lookup for `inventedAdj`, and run the checker with its source-root override against the copy.
    Expected: Nonzero exit with `UNKNOWN_ADJUSTMENT_KEY` naming preset/family/key.
    Evidence: <attemptDir>/task-2-adjustment-manifest-error.txt
  ```

  Commit: YES | Message: `test: add the official preset adjustment contract` | Files: [evaluate/preset_adjustments.json, evaluate/check_preset_adjustments.py, evaluate/tests/test_check_preset_adjustments.py, docs/reference/preset-geometry-catalog.md]

- [ ] 3. Generate deterministic completion decks and evidence scaffolds

  What to do: Add a standard-library generator `evaluate/create_completion_decks.py` and tests that emit small, deterministic PPTX ZIPs plus a manifest for every planned vertical slice. The command `python3 evaluate/create_completion_decks.py --output-dir <dir>` must produce the stable filenames `patterns.pptx`, `picture-bullets.pptx`, `table-styles.pptx`, `actions.pptx`, `notes-comments.pptx`, `reflection-3d.pptx`, `media.pptx`, `timing-transitions.pptx`, `charts.pptx`, and `fallback-domains.pptx`. Add default/lower/upper/representative adjustment cases from Task 2 without deriving expected pixels. Add a `powerpoint_capture_required` flag and scaffold metadata, but do not add fake native images. Use fixed ZIP timestamps/order so regenerated bytes are stable.
  Must NOT do: Do not use PowerPoint or LibreOffice output as embedded expected values; do not require missing `pdf2image`/`skimage`; do not commit generated bulk decks if a small source generator and checked manifest are sufficient.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [8-10, 12-15, 22] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/create_golden_set.py:1937-2006` - existing golden generation entrypoint
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/powerpoint_golden/manifest.example.json:1` - native evidence manifest shape
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/tests/test_scaffold_powerpoint_golden_batch.py:1` - scaffold test pattern
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/README.md:45-190` - exact-promotion and evaluation workflow

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `python3 -m unittest evaluate.tests.test_create_completion_decks -v` fails before implementation and passes afterward.
  - [ ] Two runs into separate temporary directories produce identical `shasum -a 256` output for every generated PPTX and manifest.
  - [ ] The generated manifest contains every feature id required by Tasks 8-21 and marks all native reference slots as absent/required rather than populated.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Deterministic fixture corpus
    Tool:     bash
    Steps:    Run the generator twice into `<attemptDir>/completion-a` and `<attemptDir>/completion-b`, then compare `find ... -type f -exec shasum -a 256 {} + | sort` after normalizing directory prefixes.
    Expected: Exit 0 and byte-identical hashes for both trees.
    Evidence: <attemptDir>/task-3-completion-decks.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Missing required feature fixture fails
    Tool:     bash
    Steps:    Delete the `media-audio` row from a copied manifest and run `python3 -m unittest evaluate.tests.test_create_completion_decks -v` with the fixture-root override.
    Expected: Nonzero exit naming `media-audio` as missing.
    Evidence: <attemptDir>/task-3-completion-decks-error.txt
  ```

  Commit: YES | Message: `test: generate deterministic PPTX completion fixtures` | Files: [evaluate/create_completion_decks.py, evaluate/tests/test_create_completion_decks.py, evaluate/completion_decks/README.md]

- [ ] 4. Split the model into stable feature-owned modules

  What to do: Behavior-preservingly extract the oversized slide/style model into `model/shape.rs`, `model/text.rs`, `model/bullet.rs`, `model/table.rs`, `model/chart.rs`, `model/fill.rs`, `model/effects.rs`, and `model/preserved.rs`; keep existing public re-exports so callers compile unchanged. Move bullet types into `model/bullet.rs` so picture-bullet work never overlaps the hyperlink/action ownership of `model/text.rs`. Move types only, preserve derives/defaults/field order and serialized names exposed by bindings. Add compile-time/public API tests before moving code. This establishes exclusive ownership: Tasks 12-21 edit only their feature model module; only the integration owner edits `model/mod.rs`.
  Must NOT do: Do not add new fields/features, rename public types, change defaults, or mix cleanup with extraction.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [6, 7, 11-15] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:11-79` - Slide/Shape/ShapeType public model
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:189-375` - chart and table types to extract
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:437-500` - unresolved and bullet types
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/style.rs:196-295` - fills and effects types
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/public_api_test.rs:1-82` - public compatibility checks

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: add public import/default tests, capture failure after temporarily removing an expected re-export, restore it, then `cargo test -p pptx2html-core --test public_api_test` passes.
  - [ ] `cargo check --workspace` and `cargo test --workspace` pass with no behavior changes.
  - [ ] `git diff --stat` shows only model moves/re-exports and direct import updates; no rendered HTML golden expectation changes.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Public model API survives extraction
    Tool:     bash
    Steps:    Run `cargo test -p pptx2html-core --test public_api_test 2>&1 | tee <attemptDir>/task-4-model-seams.txt` and `cargo check --workspace`.
    Expected: Both exit 0; downstream paths importing existing model names still compile.
    Evidence: <attemptDir>/task-4-model-seams.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Missing compatibility re-export is detected
    Tool:     bash
    Steps:    On the RED commit state, remove one test-targeted re-export and run the focused public API test; restore before GREEN.
    Expected: Compilation fails naming the missing public type; GREEN restores it.
    Evidence: <attemptDir>/task-4-model-seams-error.txt
  ```

  Commit: YES | Message: `refactor: split feature model modules` | Files: [crates/pptx2html-core/src/model/mod.rs, crates/pptx2html-core/src/model/slide.rs, crates/pptx2html-core/src/model/style.rs, crates/pptx2html-core/src/model/shape.rs, crates/pptx2html-core/src/model/text.rs, crates/pptx2html-core/src/model/bullet.rs, crates/pptx2html-core/src/model/table.rs, crates/pptx2html-core/src/model/chart.rs, crates/pptx2html-core/src/model/fill.rs, crates/pptx2html-core/src/model/effects.rs, crates/pptx2html-core/src/model/preserved.rs, crates/pptx2html-core/tests/public_api_test.rs]

- [ ] 5. Split reusable PPTX fixtures into feature-owned builders

  What to do: Extend/split `MinimalPptx` into dedicated standard-library ZIP builders under `tests/fixtures/` for relationships, extra parts, content types, slide XML, notes/comments, media, charts, and AlternateContent. Each builder must namespace temporary paths per test run and produce valid, deterministic archives. Add builder self-tests that inspect ZIP entries and relationship XML. Feature tasks must use these builders instead of duplicating large archives in integration tests.
  Must NOT do: Do not change production code or existing fixture semantics; do not use fixed shared temp paths or sleeps.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [6-10] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/fixtures/mod.rs:1-240` - current `MinimalPptx`, custom XML, relationships, and extra-file hooks
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-cli/tests/cli_integration_test.rs:10-21` - namespaced temporary path convention
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/integration_test.rs:1-120` - existing integration fixture usage

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test fixture_contract_test` fails before the new builders and passes afterward.
  - [ ] Running the fixture test twice in parallel (`... & ... & wait`) succeeds without path collisions.
  - [ ] Existing core integration, hierarchy, edge-case, and coverage tests remain green.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Feature fixture builders create valid isolated packages
    Tool:     bash
    Steps:    Run two concurrent `cargo test -p pptx2html-core --test fixture_contract_test -- --nocapture` processes and capture both outputs.
    Expected: Both exit 0; each archive contains declared content-type overrides, parts, and relationships.
    Evidence: <attemptDir>/task-5-fixture-builders.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Dangling relationship is detected by the fixture contract
    Tool:     bash
    Steps:    Use the negative builder case that emits a relationship to a missing part and run its named test.
    Expected: Test passes only when the validator returns `DANGLING_RELATIONSHIP`.
    Evidence: <attemptDir>/task-5-fixture-builders-error.txt
  ```

  Commit: YES | Message: `test: split reusable PPTX fixture builders` | Files: [crates/pptx2html-core/tests/fixtures/mod.rs, crates/pptx2html-core/tests/fixtures/package.rs, crates/pptx2html-core/tests/fixtures/parts.rs, crates/pptx2html-core/tests/fixture_contract_test.rs]

- [ ] 6. Extract parser feature seams

  What to do: Move existing fill/effect, text/bullet, table, action/hyperlink, chart-frame, and unsupported-content parsing helpers out of `slide_parser.rs` into `parser/fill_parser.rs`, `text_parser.rs`, `table_parser.rs`, `action_parser.rs`, `graphic_frame_parser.rs`, and `preserved_parser.rs`. Keep SAX event order and outputs identical. Centralize relationship records as typed `{id,type,target,target_mode}` while preserving the current target map compatibility adapter. Add parity tests before moving each helper. This task takes the exclusive parser hub lock.
  Must NOT do: Do not add new feature behavior, change namespace matching, or let feature workers edit `slide_parser.rs` after this commit.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [11-15] | Blocked by: [4, 5]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/parser/slide_parser.rs:17-6533` - monolithic parser to split by existing helper boundaries
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/parser/relationships.rs:9-42` - current id-to-target-only relationship parser
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/parser/mod.rs:180-227` - slide relationship consumption and parse order
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/coverage_regression_test.rs:1-120` - parser/renderer branch regression convention

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN parity tests compare pre/post parsed model values for a multi-feature fixture and pass with `cargo test -p pptx2html-core --test parser_seam_test`.
  - [ ] `cargo test -p pptx2html-core` passes and generated HTML for the baseline fixture is byte-identical before/after extraction.
  - [ ] New feature parser files own their domains; `slide_parser.rs` only dispatches shared SAX context and contains no moved feature implementations.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Parser extraction preserves the model
    Tool:     bash
    Steps:    Run `cargo test -p pptx2html-core --test parser_seam_test -- --nocapture 2>&1 | tee <attemptDir>/task-6-parser-seams.txt`.
    Expected: Exit 0; all parsed fields and relationship targets match the locked pre-extraction expectations.
    Evidence: <attemptDir>/task-6-parser-seams.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: External relationship target mode remains distinguishable
    Tool:     bash
    Steps:    Run the named parser seam test with one external hyperlink and one internal slide relationship sharing similar targets.
    Expected: External record has `target_mode=External`; internal record remains package-relative.
    Evidence: <attemptDir>/task-6-parser-seams-error.txt
  ```

  Commit: YES | Message: `refactor: extract parser feature modules` | Files: [crates/pptx2html-core/src/parser/mod.rs, crates/pptx2html-core/src/parser/slide_parser.rs, crates/pptx2html-core/src/parser/relationships.rs, crates/pptx2html-core/src/parser/fill_parser.rs, crates/pptx2html-core/src/parser/text_parser.rs, crates/pptx2html-core/src/parser/table_parser.rs, crates/pptx2html-core/src/parser/action_parser.rs, crates/pptx2html-core/src/parser/graphic_frame_parser.rs, crates/pptx2html-core/src/parser/preserved_parser.rs, crates/pptx2html-core/tests/parser_seam_test.rs]

- [ ] 7. Extract renderer feature seams

  What to do: Move existing fill/effect, text/bullet, table, chart, action wrapper, and unsupported placeholder rendering into `renderer/fills.rs`, `bullets.rs`, `tables.rs`, `charts.rs`, `actions.rs`, and `fallback.rs`. Preserve HTML byte output and public renderer behavior. Pass explicit render context rather than adding globals. This task takes the exclusive renderer hub lock; subsequent feature work edits only the extracted module.
  Must NOT do: Do not tune fidelity, rename CSS classes/data attributes, or change rendering order in this refactor.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [11-15] | Blocked by: [4, 5]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/mod.rs:143-238` - public render and metadata aggregation
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/mod.rs:820-930` - unsupported placeholder rendering
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/mod.rs:902-2148` - chart rendering hotspot
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/mod.rs:2140-2620` - table/text/fill rendering region
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/renderer_regression_test.rs:1-236` - public renderer regression checks

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test renderer_seam_test` locks byte-identical HTML before extraction and passes afterward.
  - [ ] `cargo test -p pptx2html-core` and `cargo clippy -p pptx2html-core --all-targets -- -D warnings` pass.
  - [ ] `renderer/mod.rs` dispatches extracted modules but contains no moved chart/table/fallback implementations.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Renderer extraction is byte-compatible
    Tool:     bash
    Steps:    Render the multi-feature fixture before/after, save both HTML files, run `cmp`, then run `cargo test -p pptx2html-core --test renderer_seam_test`.
    Expected: `cmp` and tests exit 0.
    Evidence: <attemptDir>/task-7-renderer-seams.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Unsupported placeholder metadata is not lost
    Tool:     bash
    Steps:    Run the seam test case containing SmartArt and OLE placeholders.
    Expected: Same placeholder ids, raw XML, slide indices, and HTML data attributes before/after.
    Evidence: <attemptDir>/task-7-renderer-seams-error.txt
  ```

  Commit: YES | Message: `refactor: extract renderer feature modules` | Files: [crates/pptx2html-core/src/renderer/mod.rs, crates/pptx2html-core/src/renderer/fills.rs, crates/pptx2html-core/src/renderer/bullets.rs, crates/pptx2html-core/src/renderer/tables.rs, crates/pptx2html-core/src/renderer/charts.rs, crates/pptx2html-core/src/renderer/actions.rs, crates/pptx2html-core/src/renderer/fallback.rs, crates/pptx2html-core/tests/renderer_seam_test.rs]

- [ ] 8. Close adjustment semantics for basic, rectangle, bracket, scroll, and flowchart families

  What to do: For the Task 2 manifest bundle covering `basic_shapes.rs`, `rects.rs`, `brackets_braces.rs`, `scrolls_tabs.rs`, and `flowchart.rs`, add table-driven parser-to-SVG tests for every official adjustment key at default/lower/upper/representative values. Make each key influence the intended formula/path or emit a non-exact diagnostic when the official formula is unavailable. Reject non-finite outputs and clamp only where the official constraint says to clamp.
  Must NOT do: Do not change unrelated geometry families, add visual tuning constants without source/evidence, or label finite SVG output as pixel exact.

  Parallelization: Can parallel: YES | Wave 4 | Blocks: [22, 23] | Blocked by: [2, 3, 5, 11]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry.rs:68-99` - dispatcher entries for basic/rectangle families
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry.rs:140-173` - flowchart dispatch
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry/tests.rs:1` - current geometry test conventions
  - External: `https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.adjustvaluelist?view=openxml-3.0.1` - adjustment semantics

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test geometry_adjustment_basic_test` fails for every previously ignored key and passes after implementation.
  - [ ] `python3 evaluate/check_preset_adjustments.py --repo-root . --bundle basic` exits 0 with no unconsumed official key.
  - [ ] All produced SVG paths contain only finite numbers and differ between at least two valid values for every effective adjustment key.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Every basic-family adjustment changes valid geometry
    Tool:     bash
    Steps:    Run the focused Rust test with `--nocapture`, then run the adjustment checker bundle.
    Expected: Exit 0; manifest key count equals tested key count and all effective keys alter parsed SVG geometry.
    Evidence: <attemptDir>/task-8-adjust-basic.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Out-of-contract value does not create invalid SVG
    Tool:     bash
    Steps:    Run the negative table cases for NaN-like overflow and values outside official constraints.
    Expected: Typed fallback/clamp behavior matches manifest; output contains no `NaN`/`inf`.
    Evidence: <attemptDir>/task-8-adjust-basic-error.txt
  ```

  Commit: YES | Message: `fix: complete basic shape adjustment semantics` | Files: [crates/pptx2html-core/src/renderer/geometry/basic_shapes.rs, crates/pptx2html-core/src/renderer/geometry/rects.rs, crates/pptx2html-core/src/renderer/geometry/brackets_braces.rs, crates/pptx2html-core/src/renderer/geometry/scrolls_tabs.rs, crates/pptx2html-core/src/renderer/geometry/flowchart.rs, crates/pptx2html-core/tests/geometry_adjustment_basic_test.rs]

- [ ] 9. Close adjustment semantics for arrows, callouts, and connectors

  What to do: Repeat the Task 8 TDD contract for `arrows`, `bent_u_arrows`, `curved_arrows`, `circular_arrows`, `arrow_callouts`, `callouts`, and `connectors`. Assert coupled-handle constraints, mirrored orientation, multi-path fill/stroke behavior, and connector endpoints from independent expected invariants rather than current output snapshots.
  Must NOT do: Do not reuse one preset's key meanings for a sibling unless the official source says they are shared; do not add tuple-scoped visual boosts without an evidence regression.

  Parallelization: Can parallel: YES | Wave 4 | Blocks: [22, 23] | Blocked by: [2, 3, 5, 11]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry.rs:99-139` - arrow/callout dispatch
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/create_adjustment_benchmark_deck.py:33-97` - current curved/callout scenarios, non-normative visual fixtures
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry/tests.rs:1` - path tests
  - External: `https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.presetgeometry?view=openxml-3.0.1` - rendering requirement for presets

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test geometry_adjustment_arrow_test` passes all manifest-derived keys and invariants.
  - [ ] `python3 evaluate/check_preset_adjustments.py --repo-root . --bundle arrows` reports zero unconsumed or unknown keys.
  - [ ] Existing arrow benchmark fixture generation remains byte-stable.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Arrow adjustment bundles are semantically complete
    Tool:     bash
    Steps:    Run the arrow test and checker; save the tested preset/key matrix.
    Expected: Every manifest key has default/boundary/representative coverage and finite SVG output.
    Evidence: <attemptDir>/task-9-adjust-arrows.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Coupled handles cannot invert geometry
    Tool:     bash
    Steps:    Run negative cases where head width, shaft width, and bend radius cross official constraints.
    Expected: Deterministic official clamp/fallback; no self-invalidating `NaN`, negative viewBox, or panic.
    Evidence: <attemptDir>/task-9-adjust-arrows-error.txt
  ```

  Commit: YES | Message: `fix: complete arrow adjustment semantics` | Files: [crates/pptx2html-core/src/renderer/geometry/arrows.rs, crates/pptx2html-core/src/renderer/geometry/bent_u_arrows.rs, crates/pptx2html-core/src/renderer/geometry/curved_arrows.rs, crates/pptx2html-core/src/renderer/geometry/circular_arrows.rs, crates/pptx2html-core/src/renderer/geometry/arrow_callouts.rs, crates/pptx2html-core/src/renderer/geometry/callouts.rs, crates/pptx2html-core/src/renderer/geometry/connectors.rs, crates/pptx2html-core/tests/geometry_adjustment_arrow_test.rs]

- [ ] 10. Close adjustment semantics for arc, star, wave, math, misc, action-button, chart-shape, and custom geometry

  What to do: Complete remaining preset bundles and custom `a:avLst/a:gd` formula preservation. Cover angle units/wrap, star inner radii, wave amplitude/offset, math strokes, block-arc holes, multi-path action buttons, and all currently supported custom-geometry guide operators. An unknown custom formula operator must retain its raw formula and produce a diagnostic/fallback instead of being treated as zero.
  Must NOT do: Do not silently substitute default paths for unknown formulas; do not treat a preset named `math*` as OMML equation support.

  Parallelization: Can parallel: YES | Wave 4 | Blocks: [22, 23] | Blocked by: [2, 3, 5, 11]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry.rs:174-260` - action/star/math/misc/arc/wave dispatch
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry/custom_geom.rs:1` - custom guide/path evaluation
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/geometry.rs:1-300` - custom geometry model
  - External: `https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.customgeometry?view=openxml-3.0.1` - official custom geometry contract

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test geometry_adjustment_remaining_test` passes all remaining manifest rows.
  - [ ] `python3 evaluate/check_preset_adjustments.py --repo-root . --bundle remaining` reports zero unconsumed/unknown preset keys.
  - [ ] Unknown custom guide operator test returns fallback diagnostic with original formula preserved; known guide operators yield finite paths.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Remaining preset and custom adjustments are classified
    Tool:     bash
    Steps:    Run focused test and remaining-bundle checker with JSON output.
    Expected: Exit 0; every preset/key and known custom formula operator is tested or explicitly preserved as fallback.
    Evidence: <attemptDir>/task-10-adjust-remaining.json   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Unknown custom formula is preserved
    Tool:     bash
    Steps:    Convert the generated custom-geometry fixture containing `fmla="unknownOp 1 2"` through the metadata API.
    Expected: No panic; placeholder/diagnostic contains the exact raw formula and a non-exact tier.
    Evidence: <attemptDir>/task-10-adjust-remaining-error.txt
  ```

  Commit: YES | Message: `fix: complete remaining geometry adjustment semantics` | Files: [crates/pptx2html-core/src/renderer/geometry/arcs.rs, crates/pptx2html-core/src/renderer/geometry/stars.rs, crates/pptx2html-core/src/renderer/geometry/waves_polys.rs, crates/pptx2html-core/src/renderer/geometry/math.rs, crates/pptx2html-core/src/renderer/geometry/misc.rs, crates/pptx2html-core/src/renderer/geometry/action_buttons.rs, crates/pptx2html-core/src/renderer/geometry/chart_shapes.rs, crates/pptx2html-core/src/renderer/geometry/custom_geom.rs, crates/pptx2html-core/tests/geometry_adjustment_remaining_test.rs]

- [ ] 11. Add the core preservation diagnostic envelope

  What to do: Add a typed `ConversionDiagnostic`/`DiagnosticLocation`/`FallbackKind` model in `model/preserved.rs`; include stable fields listed in Scope. Extend `ConversionResult` with `diagnostics` while keeping `unresolved_elements` as a compatibility projection. Make parser/package inventory and renderer fallback collect diagnostics deterministically, deduplicate by part/slide/element/id, and embed a JSON manifest in HTML as `<script type="application/json" id="pptx2html-diagnostics">`. Centralize JSON escaping without adding serde. Add diagnostics for every existing SmartArt/OLE/Math/custom-geometry fallback and for unknown relationship types/elements. In the same seam commit, register compile-clean domain modules `model/{notes_comments,media,timing,embedded}.rs`, `parser/{notes_comments_parser,media_parser,timing_parser,embedded_parser}.rs`, and `renderer/{media,embedded_fallback}.rs`; the shared preserved/fallback hubs expose only stable collection/dispatch interfaces, and later domain tasks fill only their assigned module.
  Must NOT do: Do not remove/change existing unresolved fields, log raw binary content, expose secrets/external file contents, or make fallback diagnostics fatal by default.

  Parallelization: Can parallel: NO | Wave 3 | Blocks: [8-10, 12-23] | Blocked by: [1, 4, 6, 7]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/lib.rs:113-188` - current `ConversionResult` and metadata APIs
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:437-474` - current unresolved model
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/mod.rs:143-238` - metadata collection
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/edge_case_test.rs:574-744` - current unresolved placeholder assertions
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/capabilities.rs:28-129` - tier/stage validity rules

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test diagnostic_contract_test` passes existing fallback, unknown relationship, off-slide part, deduplication, ordering, and escaping cases.
  - [ ] Existing `unresolved_elements` assertions remain unchanged and green.
  - [ ] String-only conversion output contains a parseable `pptx2html-diagnostics` JSON script when off-slide unsupported parts exist and `[]` when none exist.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Unsupported content is observable on every core surface
    Tool:     bash
    Steps:    Run the diagnostic contract test, extract the HTML script payload, and parse it with `python3 -m json.tool`.
    Expected: Exit 0; ordered diagnostics include stable code, tier, location, raw reference, and fallback kind.
    Evidence: <attemptDir>/task-11-diagnostic-envelope.json   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Malicious XML text is safely escaped
    Tool:     bash
    Steps:    Convert a fixture whose raw XML contains `</script><script>alert(1)</script>` and inspect HTML/parsed diagnostics.
    Expected: No executable script injection; JSON round-trips to the original text; diagnostic remains non-fatal.
    Evidence: <attemptDir>/task-11-diagnostic-envelope-error.html
  ```

  Commit: YES | Message: `feat: expose structured conversion diagnostics` | Files: [crates/pptx2html-core/src/model/mod.rs, crates/pptx2html-core/src/model/preserved.rs, crates/pptx2html-core/src/model/notes_comments.rs, crates/pptx2html-core/src/model/media.rs, crates/pptx2html-core/src/model/timing.rs, crates/pptx2html-core/src/model/embedded.rs, crates/pptx2html-core/src/lib.rs, crates/pptx2html-core/src/parser/mod.rs, crates/pptx2html-core/src/parser/preserved_parser.rs, crates/pptx2html-core/src/parser/notes_comments_parser.rs, crates/pptx2html-core/src/parser/media_parser.rs, crates/pptx2html-core/src/parser/timing_parser.rs, crates/pptx2html-core/src/parser/embedded_parser.rs, crates/pptx2html-core/src/renderer/mod.rs, crates/pptx2html-core/src/renderer/fallback.rs, crates/pptx2html-core/src/renderer/media.rs, crates/pptx2html-core/src/renderer/embedded_fallback.rs, crates/pptx2html-core/tests/diagnostic_contract_test.rs]

- [ ] 12. Implement pattern fills

  What to do: Add typed pattern fill model `{preset,foreground,background}`, parse all official preset values and theme color modifiers, and render deterministic SVG/CSS tiles for the official set. Preserve unknown future preset names with a fallback diagnostic. Apply the same fill path to shapes, table cells, and slide backgrounds where their existing fill resolution calls converge.
  Must NOT do: Do not map unknown patterns to solid fill, infer pattern colors when absent beyond official defaults, or claim Office pixel parity for browser antialiasing.

  Parallelization: Can parallel: YES | Wave 4 | Blocks: [17, 22, 23] | Blocked by: [1, 3, 4, 6, 7, 11]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/style.rs:196-229` - current Fill variants and color extraction
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/SUPPORTED_FEATURES.md:96-106` - pattern fill currently unparsed
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/REMAINING_WORK_PLAN.md:90-103` - fill fidelity gap
  - External: `https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.patternfill?view=openxml-3.0.1` - official pattern fill element

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test pattern_fill_test` passes parser, theme resolution, every official preset classification, and renderer tests.
  - [ ] Generated HTML has deterministic pattern tile ids and foreground/background colors; no unsupported pattern disappears.
  - [ ] Unknown preset test emits `DRAWINGML_PATTERN_UNSUPPORTED` with raw preset/XML.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Pattern deck renders all classified presets
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/patterns.pptx -o <attemptDir>/task-12-pattern-fill.html`; start `python3 -m http.server 4212 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-12-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4212/task-12-pattern-fill.html >/dev/null`; open that URL, capture a full-page screenshot to `<attemptDir>/task-12-pattern-fill.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Every labeled shape has a non-solid repeated pattern using the expected two resolved colors; no blank shape.
    Evidence: <attemptDir>/task-12-pattern-fill.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Unknown pattern preserves semantics
    Tool:     bash
    Steps:    Convert unknown-pattern fixture and parse embedded diagnostics.
    Expected: Stable fallback diagnostic includes raw preset and colors; conversion succeeds.
    Evidence: <attemptDir>/task-12-pattern-fill-error.json
  ```

  Commit: YES | Message: `feat: render DrawingML pattern fills` | Files: [crates/pptx2html-core/src/model/fill.rs, crates/pptx2html-core/src/parser/fill_parser.rs, crates/pptx2html-core/src/renderer/fills.rs, crates/pptx2html-core/tests/pattern_fill_test.rs]

- [ ] 13. Implement picture bullets

  What to do: Model `Bullet::Picture` with relationship id, image bytes/content type, and bullet sizing; parse `a:buBlip` through typed relationships; render an inline image marker aligned to paragraph metrics. Reuse current image embedding/external-asset policy. Missing/dangling/unsupported image relationships produce a diagnostic and deterministic marker fallback.
  Must NOT do: Do not hardcode a glyph substitute without diagnostic; do not duplicate image MIME detection or external asset writing.

  Parallelization: Can parallel: YES | Wave 4 | Blocks: [22, 23] | Blocked by: [1, 3, 4, 6, 7, 11]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:476-500` - current bullet variants
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/SUPPORTED_FEATURES.md:84-95` - picture bullet gap
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:171-187` - picture data/crop pattern
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/integration_test.rs:1-120` - parser-to-renderer feature tests

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test picture_bullet_test` passes embed, external asset, sizing, and missing relationship cases.
  - [ ] Same fixture with embed true/false produces equivalent marker semantics and correct asset ownership.
  - [ ] Dangling relationship emits `PICTURE_BULLET_IMAGE_MISSING`; text remains visible.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Picture bullets align with text
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/picture-bullets.pptx -o <attemptDir>/task-13-picture-bullets.html`; start `python3 -m http.server 4213 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-13-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4213/task-13-picture-bullets.html >/dev/null`; open that URL, inspect bullet image count, capture 1440x900 and 390x844 screenshots to `<attemptDir>/task-13-picture-bullets.png` and `<attemptDir>/task-13-picture-bullets-mobile.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: One bullet image per labeled paragraph, no overlap with text, and stable scaling at page zoom.
    Evidence: <attemptDir>/task-13-picture-bullets.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Missing image degrades visibly
    Tool:     bash
    Steps:    Convert the dangling-relationship fixture and parse diagnostics/HTML text.
    Expected: Conversion succeeds, paragraph text remains, marker fallback is visible, and diagnostic code is exact.
    Evidence: <attemptDir>/task-13-picture-bullets-error.json
  ```

  Commit: YES | Message: `feat: render DrawingML picture bullets` | Files: [crates/pptx2html-core/src/model/bullet.rs, crates/pptx2html-core/src/parser/text_parser.rs, crates/pptx2html-core/src/renderer/bullets.rs, crates/pptx2html-core/tests/picture_bullet_test.rs]

- [ ] 14. Implement table style resolution

  What to do: Parse `ppt/tableStyles.xml`, table style id, whole-table/first-last/banded row/column/corner regions, and theme references into a typed table-style model. Resolve precedence `explicit cell > applicable style region > whole table > existing default` and render current HTML table output. Built-in or extension style ids without available definitions remain approximate and emit a stable diagnostic while preserving ids/flags.
  Must NOT do: Do not hardcode Office built-in GUID appearances without an official definition; do not break explicit cell fill/border precedence.

  Parallelization: Can parallel: YES | Wave 4 | Blocks: [20, 22, 23] | Blocked by: [1, 3, 4, 6, 7, 11]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:320-375` - table data/cell model
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/REMAINING_WORK_PLAN.md:104-117` - table style/banding gaps
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/CAPABILITY_MATRIX.md:30` - current approximate tier
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document` - official DrawingML table context

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test table_style_test` passes style-part parsing and all precedence/flag combinations.
  - [ ] Explicit cell formatting wins over every table-style region; corner/first/last and band intersections follow one deterministic documented precedence.
  - [ ] Unknown style id produces `TABLE_STYLE_DEFINITION_UNAVAILABLE` and preserves id/flags.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Table style regions render with correct precedence
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/table-styles.pptx -o <attemptDir>/task-14-table-styles.html`; start `python3 -m http.server 4214 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-14-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4214/task-14-table-styles.html >/dev/null`; open that URL, inspect computed backgrounds/borders for header, bands, edge columns, corners, and explicit overrides, capture `<attemptDir>/task-14-table-styles.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Computed values match the test matrix and labels; merged cells remain aligned.
    Evidence: <attemptDir>/task-14-table-styles.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Missing style definition is not silently flattened
    Tool:     bash
    Steps:    Convert fixture with unknown table style GUID and parse diagnostics.
    Expected: Table content/grid renders; diagnostic preserves GUID/flags and tier remains approximate/fallback.
    Evidence: <attemptDir>/task-14-table-styles-error.json
  ```

  Commit: YES | Message: `feat: resolve DrawingML table styles` | Files: [crates/pptx2html-core/src/model/table.rs, crates/pptx2html-core/src/parser/table_parser.rs, crates/pptx2html-core/src/renderer/tables.rs, crates/pptx2html-core/tests/table_style_test.rs]

- [ ] 15. Implement typed hyperlink and action semantics

  What to do: Replace string-only run hyperlinks with a typed action target covering external URI, internal slide, next/previous/first/last slide, no-op, media play, and preserved custom action. Parse click and hover relationships plus `ppaction://` semantics; render safe external links and deterministic slide navigation data attributes. Reject active schemes such as `javascript:` as non-clickable diagnostics. Keep hover/media/custom actions represented even when the browser runtime cannot execute them.
  Must NOT do: Do not map every action to `<a href>`, execute files/programs/macros, or allow unsafe URI schemes.

  Parallelization: Can parallel: YES | Wave 4 | Blocks: [18, 19, 22, 23] | Blocked by: [1, 3, 4, 6, 7, 11]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:163-169` - current string hyperlink field
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/parser/relationships.rs:9-42` - relationship parser before typed target/mode extraction
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/geometry.rs:174-186` - action-button shapes currently visual only
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/how-to-get-all-the-external-hyperlinks-in-a-presentation` - official external hyperlink relationships
  - External: `https://learn.microsoft.com/en-us/office/vba/api/powerpoint.hyperlink` - official click/hover/action distinction

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test action_semantics_test` passes external/internal/navigation/hover/media/custom/unsafe cases.
  - [ ] External HTTP(S)/mailto links have safe attributes; internal actions reference stable one-based slide ids; unsafe schemes have no executable href.
  - [ ] Unsupported action strings remain in diagnostics with code `ACTION_UNSUPPORTED`, not dropped.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Internal action buttons navigate deterministically
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/actions.pptx -o <attemptDir>/task-15-actions.html`; start `python3 -m http.server 4215 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-15-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4215/task-15-actions.html >/dev/null`; open that URL, click next/previous/first/last buttons, inspect active slide/hash after each click, capture `<attemptDir>/task-15-actions.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Navigation matches action type; external link attributes are correct without actually leaving the local page.
    Evidence: <attemptDir>/task-15-actions.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Unsafe/custom action cannot execute
    Tool:     browser:control-in-app-browser
    Steps:    Inspect/click the `javascript:` and custom-program action fixtures.
    Expected: No script/process executes; elements are non-clickable or blocked and diagnostics preserve original action.
    Evidence: <attemptDir>/task-15-actions-error.json
  ```

  Commit: YES | Message: `feat: preserve hyperlink and action semantics` | Files: [crates/pptx2html-core/src/model/text.rs, crates/pptx2html-core/src/parser/action_parser.rs, crates/pptx2html-core/src/renderer/actions.rs, crates/pptx2html-core/tests/action_semantics_test.rs]

- [ ] 16. Preserve notes and comments

  What to do: Fill only the Task 11-provisioned `model/notes_comments.rs` and `parser/notes_comments_parser.rs` seams. Parse slide-to-notes, notes master, comment authors, legacy comments, and modern comment relationship parts into preserved typed records. Keep notes/comments out of the visible slide by default; expose them in diagnostics/HTML metadata with author/time/text/part location and stable slide association. Missing authors/targets become diagnostics without losing text.
  Must NOT do: Do not edit `model/preserved.rs` or `parser/preserved_parser.rs`; do not render comments on the slideshow canvas by default or conflate notes with slide body text.

  Parallelization: Can parallel: YES | Wave 5 | Blocks: [22, 23] | Blocked by: [11]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/CAPABILITY_MATRIX.md:35-42` - currently unparsed and required fallback
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/parser/mod.rs:180-227` - package/slide relationship loading point
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-notes-slides` - official notes parts
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-comments` - official comments/authors behavior

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test notes_comments_test` passes notes/master/comments/authors/missing relation cases.
  - [ ] Visible slide HTML excludes note/comment text; embedded diagnostic metadata includes exact text and slide/part association.
  - [ ] Unknown modern comment extension is preserved as raw XML with non-exact tier.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Notes/comments are preserved off-canvas
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/notes-comments.pptx -o <attemptDir>/task-16-notes-comments.html`; start `python3 -m http.server 4216 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-16-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4216/task-16-notes-comments.html >/dev/null`; open that URL, verify the sentinel is absent from the visible slide subtree and present once in `#pptx2html-diagnostics`, capture `<attemptDir>/task-16-notes-comments.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Sentinel absent from slide subtree and present once in typed metadata with correct slide/author.
    Evidence: <attemptDir>/task-16-notes-comments.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Missing author relation degrades gracefully
    Tool:     bash
    Steps:    Convert comment fixture without author part.
    Expected: Comment text remains preserved and diagnostic code is `COMMENT_AUTHOR_UNRESOLVED`.
    Evidence: <attemptDir>/task-16-notes-comments-error.txt
  ```

  Commit: YES | Message: `feat: preserve slide notes and comments` | Files: [crates/pptx2html-core/src/model/notes_comments.rs, crates/pptx2html-core/src/parser/notes_comments_parser.rs, crates/pptx2html-core/tests/notes_comments_test.rs]

- [ ] 17. Implement reflection approximation and 3D preservation fallback

  What to do: Parse reflection parameters and 3D scene/shape properties into typed effects. Render bounded reflection using a clipped mirrored SVG/CSS layer with transparency/blur where browser primitives correspond; label it approximate. Preserve `scene3d`, `sp3d`, materials, lights, camera, bevel/extrusion, and effect DAG raw semantics with a fallback diagnostic when not directly represented.
  Must NOT do: Do not flatten 3D to arbitrary shadows, claim Office lighting parity, or discard effect DAG order.

  Parallelization: Can parallel: YES | Wave 5 | Blocks: [22, 23] | Blocked by: [11, 12]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/style.rs:272-295` - current outer-shadow/glow-only effect model
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/REMAINING_WORK_PLAN.md:174-185` - deferred advanced effects/3D boundary
  - External: `https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.pictures.shapeproperties?view=openxml-3.0.1` - official fill/effect/3D child model

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test effects_3d_test` passes reflection parsing/render and full 3D preservation cases.
  - [ ] Reflection output is deterministic and explicitly approximate in diagnostics/provenance.
  - [ ] Every 3D/effect DAG fixture retains raw XML/typed known fields and emits no silent loss.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Reflection renders as a bounded approximation
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/reflection-3d.pptx -o <attemptDir>/task-17-effects-3d.html`; start `python3 -m http.server 4217 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-17-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4217/task-17-effects-3d.html >/dev/null`; open that URL, inspect the source and mirrored clipped layer, capture `<attemptDir>/task-17-effects-3d.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Reflection is positioned below the source, clipped/faded, and diagnostic tier is approximate.
    Evidence: <attemptDir>/task-17-effects-3d.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Advanced 3D never disappears
    Tool:     bash
    Steps:    Convert camera/light/material/extrusion fixture and parse diagnostics.
    Expected: Static fallback remains visible and metadata preserves all related XML with `DRAWINGML_3D_FALLBACK`.
    Evidence: <attemptDir>/task-17-effects-3d-error.json
  ```

  Commit: YES | Message: `feat: preserve reflection and 3D effects` | Files: [crates/pptx2html-core/src/model/effects.rs, crates/pptx2html-core/src/parser/fill_parser.rs, crates/pptx2html-core/src/renderer/fills.rs, crates/pptx2html-core/tests/effects_3d_test.rs]

- [ ] 18. Preserve and render bounded audio/video

  What to do: Fill only the Task 11-provisioned `model/media.rs`, `parser/media_parser.rs`, and `renderer/media.rs` seams. Parse media relationships, embedded/linked audio/video, poster frames, trim/loop/volume flags, and action links. Emit HTML `<audio>/<video>` only for safe MIME types already embeddable/externalizable by the asset pipeline; otherwise render poster/placeholder and diagnostic. Never force autoplay; represent requested autoplay as metadata blocked by browser policy.
  Must NOT do: Do not edit `model/preserved.rs`, `parser/preserved_parser.rs`, or `renderer/fallback.rs`; do not transcode media, fetch external media, bundle codecs, or claim playback equivalence.

  Parallelization: Can parallel: YES | Wave 5 | Blocks: [22, 23] | Blocked by: [11, 15]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/CAPABILITY_MATRIX.md:35` - media currently unparsed
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/lib.rs:121-135` - external asset contract
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/how-to-add-an-audio-to-a-slide-in-a-presentation` - official media relationship/timing semantics
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document` - official audio/video feature family

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test media_test` passes embedded/linked/poster/missing/unsupported MIME cases.
  - [ ] Supported media uses deterministic data URI or external asset path according to options; unsupported/linked media is never fetched.
  - [ ] Autoplay request is preserved but output does not add browser autoplay without a user gesture policy.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Supported local media exposes browser controls
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/media.pptx -o <attemptDir>/task-18-media.html`; start `python3 -m http.server 4218 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-18-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4218/task-18-media.html >/dev/null`; open that URL, inspect `<audio>/<video>` controls/source/poster, click play, record media state, capture `<attemptDir>/task-18-media.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Controls exist; source resolves; user-initiated playback starts or reports a codec diagnostic without page failure.
    Evidence: <attemptDir>/task-18-media.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Unsupported codec/link is explicit
    Tool:     bash
    Steps:    Convert unsupported-codec and external-link fixtures without network access.
    Expected: No fetch/transcode; poster/placeholder visible; diagnostics identify MIME/target and fallback reason.
    Evidence: <attemptDir>/task-18-media-error.json
  ```

  Commit: YES | Message: `feat: preserve PPTX audio and video` | Files: [crates/pptx2html-core/src/model/media.rs, crates/pptx2html-core/src/parser/media_parser.rs, crates/pptx2html-core/src/renderer/media.rs, crates/pptx2html-core/tests/media_test.rs]

- [ ] 19. Preserve timing, transitions, and animations and play only a bounded subset

  What to do: Fill only the Task 11-provisioned `model/timing.rs` and `parser/timing_parser.rs` seams, plus the already extracted `renderer/actions.rs` after Task 15 is integrated. Parse slide transition and timing trees into a typed preserved event graph with ids, triggers, duration/delay, target shape ids, ordering, and raw unknown nodes. Implement only `cut`/`fade` slide transitions and simple click/with-previous/after-previous appear/disappear/fade actions whose targets and durations are fully resolved. All other nodes stay behavioral fallback with diagnostics.
  Must NOT do: Do not edit `model/preserved.rs` or `parser/preserved_parser.rs`; do not approximate motion paths, morph, media synchronization, build sequences, or unknown extension timing as generic fades.

  Parallelization: Can parallel: YES | Wave 5 | Blocks: [22, 23] | Blocked by: [11, 15]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/CAPABILITY_MATRIX.md:35` - animation currently unparsed
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/REMAINING_WORK_PLAN.md:161-170` - required timing metadata sideband
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-animation` - official timing element
  - External: `https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-presentation-slides` - official slide/transition context

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test timing_transition_test` passes event graph, supported subset, missing target, and unknown timing node cases.
  - [ ] Supported sequences produce deterministic data attributes/runtime order; unsupported nodes preserve raw XML and never alter static visibility irreversibly.
  - [ ] Behavioral tier remains approximate/fallback; no exact promotion.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Bounded fade sequence follows trigger order
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/timing-transitions.pptx -o <attemptDir>/task-19-timing.html`; start `python3 -m http.server 4219 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-19-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4219/task-19-timing.html >/dev/null`; open that URL, click advance exactly three times, record target visibility after each event, navigate the slide transition, capture `<attemptDir>/task-19-timing.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Click/with/after ordering matches fixture and fade/cut duration metadata; no extra event fires.
    Evidence: <attemptDir>/task-19-timing.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Unknown motion path stays static and explicit
    Tool:     browser:control-in-app-browser
    Steps:    Open motion-path fixture and trigger playback.
    Expected: Shape remains statically visible, no invented motion occurs, and diagnostics contain raw timing node/target.
    Evidence: <attemptDir>/task-19-timing-error.json
  ```

  Commit: YES | Message: `feat: preserve PPTX timing and transitions` | Files: [crates/pptx2html-core/src/model/timing.rs, crates/pptx2html-core/src/parser/timing_parser.rs, crates/pptx2html-core/src/renderer/actions.rs, crates/pptx2html-core/tests/timing_transition_test.rs]

- [ ] 20. Complete chart classification and deterministic fallback

  What to do: Inventory all official chart-space child types and extension variants. Keep current bounded `ChartType` direct renderer approximate; add an explicit unsupported chart classification carrying qualified type/raw chart XML/series metadata/preview relationship. For every non-direct or structurally incompatible chart, prefer package preview image, otherwise a semantic placeholder; never flatten unsupported structures into a misleading supported type.
  Must NOT do: Do not implement an omnibus chart renderer, infer 3D/stock/surface/combination semantics from series alone, or upgrade charts beyond approximate.

  Parallelization: Can parallel: YES | Wave 5 | Blocks: [22, 23] | Blocked by: [11, 14]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:189-318` - current chart types/spec
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/parser/chart_parser.rs:22-430` - current chart parser boundary
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/mod.rs:902-2148` - current direct/fallback renderer
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/integration_test.rs:5580-5970` - bounded direct/fallback chart tests
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/CAPABILITY_MATRIX.md:33` - current approximate contract

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test chart_completeness_test` covers every chart manifest family and compatible/incompatible structure.
  - [ ] Every official chart type is direct, preview fallback, or semantic placeholder; parser returns no unclassified `None` path.
  - [ ] Unsupported chart diagnostics preserve type/XML/series summary and direct chart tests remain green.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Mixed chart deck uses truthful direct/fallback paths
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/charts.pptx -o <attemptDir>/task-20-chart-completeness.html`; start `python3 -m http.server 4220 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-20-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4220/task-20-chart-completeness.html >/dev/null`; open that URL, inspect labeled direct/preview/placeholder cases, capture `<attemptDir>/task-20-chart-completeness.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Supported bounded charts render; unsupported charts show preview/placeholder and corresponding diagnostic, never an empty frame.
    Evidence: <attemptDir>/task-20-chart-completeness.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Structurally incompatible known type does not misrender
    Tool:     bash
    Steps:    Convert multi-series pie or unsupported combination fixture.
    Expected: Preview/placeholder fallback with `CHART_STRUCTURE_UNSUPPORTED`, not a lossy direct pie.
    Evidence: <attemptDir>/task-20-chart-completeness-error.json
  ```

  Commit: YES | Message: `fix: classify unsupported chart spaces` | Files: [crates/pptx2html-core/src/model/chart.rs, crates/pptx2html-core/src/parser/chart_parser.rs, crates/pptx2html-core/src/renderer/charts.rs, crates/pptx2html-core/tests/chart_completeness_test.rs]

- [ ] 21. Harden SmartArt, OLE, Math, AlternateContent, and unknown fallback

  What to do: Fill only the Task 11-provisioned `model/embedded.rs`, `parser/embedded_parser.rs`, and `renderer/embedded_fallback.rs` seams. Preserve the full relationship closure and static preview for SmartArt diagram parts, OLE embedded objects, and OMML equations. Keep direct native activation/layout out of scope. Parse `mc:AlternateContent` by selecting a supported choice/fallback for rendering while recording every branch and requirement token. Add a package-level unknown relationship/element inventory so new extensions generate diagnostics rather than disappear.
  Must NOT do: Do not edit `model/preserved.rs`, `parser/preserved_parser.rs`, or `renderer/fallback.rs`; do not execute OLE payloads, call LLM providers, copy AGPL implementations, or treat the optional enhancer as deterministic exact rendering.

  Parallelization: Can parallel: YES | Wave 5 | Blocks: [22, 23] | Blocked by: [11]

  References (executor has NO interview context - be exhaustive):
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/model/slide.rs:437-474` - existing fallback types/metadata
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/src/renderer/mod.rs:859-901` - existing deterministic placeholder
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-core/tests/edge_case_test.rs:574-744` - current fallback tests
  - External: `https://learn.microsoft.com/en-us/openspecs/office_standards/ms-odrawxml/ec96c9e0-757a-4f24-9725-c52d3c34e310` - official SmartArt/diagram layout part
  - External: `https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/4a70eb6c-75a7-4927-b786-f1dd7b84c3db` - official OLE element behavior
  - External: `https://learn.microsoft.com/en-us/office/math/mathml` - official Office Math/MathML boundary
  - External: `https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/b9ff79b4-5e24-4c85-b567-e5f43d498375` - official PPTX extensions

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: `cargo test -p pptx2html-core --test fallback_domains_test` passes relationship closure, preview, AlternateContent, and unknown extension cases.
  - [ ] No embedded OLE bytes are executed or emitted into HTML; only metadata/reference and safe preview are exposed.
  - [ ] Unknown part/relationship/element fixture produces stable diagnostics and non-empty fallback output.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Fallback-only domains remain visible and inspectable
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `cargo run -p pptx2html-cli -- <attemptDir>/completion-decks/fallback-domains.pptx -o <attemptDir>/task-21-fallback-domains.html`; start `python3 -m http.server 4221 --bind 127.0.0.1 --directory <attemptDir> > <attemptDir>/task-21-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4221/task-21-fallback-domains.html >/dev/null`; open that URL, inspect placeholders/previews and `#pptx2html-diagnostics`, capture `<attemptDir>/task-21-fallback-domains.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: Each object occupies its bounds, has a unique id/type label, and preserves relationship closure metadata.
    Evidence: <attemptDir>/task-21-fallback-domains.png   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Unknown extension and OLE payload remain inert
    Tool:     bash
    Steps:    Convert fixture containing unknown extension and executable-looking OLE bytes; grep HTML/assets and parse diagnostics.
    Expected: Payload bytes are not embedded/executed; diagnostic references part safely and records unknown namespace/relationship.
    Evidence: <attemptDir>/task-21-fallback-domains-error.json
  ```

  Commit: YES | Message: `fix: preserve unsupported OOXML domains` | Files: [crates/pptx2html-core/src/model/embedded.rs, crates/pptx2html-core/src/parser/embedded_parser.rs, crates/pptx2html-core/src/renderer/embedded_fallback.rs, crates/pptx2html-core/tests/fallback_domains_test.rs]

- [ ] 22. Expose diagnostics on CLI, Python, and WASM and run local visual evidence

  What to do: Project core diagnostics onto all public surfaces. CLI: add `--diagnostics <PATH>` for JSON sidecar and `--fail-on-fallback` that writes requested outputs then exits 2 when any fallback/unparsed diagnostic exists; without flags, print a stable count/code summary to stderr when diagnostics exist. Python: add typed diagnostic objects to `ConversionResult`. WASM: add a `diagnostics` JSON getter while keeping `unresolvedElements`. Update stubs/package contract tests. Generate all completion decks, convert them, serve HTML, capture browser screenshots, and render LibreOffice PDFs/PNGs using `/opt/homebrew/bin/soffice` plus `/opt/homebrew/bin/pdftoppm` as secondary evidence.
  Must NOT do: Do not change default successful exit for approximate-only content, remove old metadata fields, install missing image packages, or call local evidence PowerPoint-native.

  Parallelization: Can parallel: NO | Wave 6 | Blocks: [23] | Blocked by: [3, 8-21]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-cli/src/main.rs:8-42` - CLI options
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-cli/src/main.rs:142-227` - conversion result handling
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-py/src/lib.rs:50-149` - Python metadata mapping
  - API/Type: `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-wasm/src/lib.rs:106-135` - WASM result getters
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-wasm/src/lib.rs:238-260` - existing manual unresolved JSON serialization
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/crates/pptx2html-cli/tests/cli_integration_test.rs:24-352` - CLI success/error test pattern
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/reference_render_powerpoint.ps1:1` - remote native oracle entrypoint

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: focused CLI, Python, and WASM tests fail before fields/flags and pass afterward.
  - [ ] `cargo test --workspace`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets -- -D warnings` pass.
  - [ ] CLI diagnostic JSON parses; `--fail-on-fallback` returns 2 for fallback fixture and 0 for fully direct fixture; old APIs remain compatible.
  - [ ] Every generated deck has HTML screenshot and LibreOffice PNG evidence; artifacts are labeled secondary/non-native.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: Cross-surface diagnostics agree
    Tool:     bash
    Steps:    Convert the same fallback fixture with CLI JSON sidecar, Python metadata API, and WASM Node smoke; normalize/order JSON and run `cmp` on code/family/tier/location fields.
    Expected: All surfaces expose the same ordered diagnostics; existing unresolved projection remains present.
    Evidence: <attemptDir>/task-22-cross-surface.json   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: Local visual evidence is not promoted as native exactness
    Tool:     browser:control-in-app-browser
    Steps:    Run `python3 evaluate/create_completion_decks.py --output-dir <attemptDir>/completion-decks`; run `mkdir -p <attemptDir>/task-22-html <attemptDir>/lo`; run `for deck in <attemptDir>/completion-decks/*.pptx; do name="$(basename "$deck" .pptx)"; cargo run -p pptx2html-cli -- "$deck" -o "<attemptDir>/task-22-html/$name.html"; /opt/homebrew/bin/soffice --headless --convert-to pdf --outdir <attemptDir>/lo "$deck"; /opt/homebrew/bin/pdftoppm -png "<attemptDir>/lo/$name.pdf" "<attemptDir>/lo/$name"; done`; start `python3 -m http.server 4222 --bind 127.0.0.1 --directory <attemptDir>/task-22-html > <attemptDir>/task-22-http.log 2>&1 & server_pid=$!`; wait with `curl --retry 20 --retry-connrefused --retry-delay 0 http://127.0.0.1:4222/patterns.html >/dev/null`; open `http://127.0.0.1:4222/patterns.html`, `http://127.0.0.1:4222/picture-bullets.html`, `http://127.0.0.1:4222/table-styles.html`, `http://127.0.0.1:4222/actions.html`, `http://127.0.0.1:4222/notes-comments.html`, `http://127.0.0.1:4222/reflection-3d.html`, `http://127.0.0.1:4222/media.html`, `http://127.0.0.1:4222/timing-transitions.html`, `http://127.0.0.1:4222/charts.html`, and `http://127.0.0.1:4222/fallback-domains.html`; capture full-page screenshots to `<attemptDir>/task-22-local-visual-evidence-patterns.png`, `<attemptDir>/task-22-local-visual-evidence-picture-bullets.png`, `<attemptDir>/task-22-local-visual-evidence-table-styles.png`, `<attemptDir>/task-22-local-visual-evidence-actions.png`, `<attemptDir>/task-22-local-visual-evidence-notes-comments.png`, `<attemptDir>/task-22-local-visual-evidence-reflection-3d.png`, `<attemptDir>/task-22-local-visual-evidence-media.png`, `<attemptDir>/task-22-local-visual-evidence-timing-transitions.png`, `<attemptDir>/task-22-local-visual-evidence-charts.png`, and `<attemptDir>/task-22-local-visual-evidence-fallback-domains.png`, then run `kill "$server_pid"; wait "$server_pid" 2>/dev/null || true`.
    Expected: All local artifacts exist and metadata says `oracle=browser` or `oracle=libreoffice`, never `oracle=powerpoint`; exact gate remains blocked.
    Evidence: <attemptDir>/task-22-local-visual-evidence-patterns.png
  ```

  Commit: YES | Message: `feat: expose fidelity diagnostics across public surfaces` | Files: [crates/pptx2html-cli/src/main.rs, crates/pptx2html-cli/tests/cli_integration_test.rs, crates/pptx2html-py/src/lib.rs, crates/pptx2html-py/pptx2html.pyi, crates/pptx2html-py/tests/test_runtime.py, crates/pptx2html-wasm/src/lib.rs, crates/pptx2html-wasm/tests/node-smoke.mjs, crates/pptx2html-wasm/tests/check-package-contract.mjs]

- [ ] 23. Enforce exactness/documentation drift gates and finalize the capability matrix

  What to do: Extend `evaluate/check_exactness_contract.py` to cross-check completeness manifest, adjustment manifest, Rust direct/fallback registration, tests, evidence manifests, and all support docs. Update capability/support/README docs only from actual task results. Keep every locally verified new visual/behavioral feature `approximate` or `fallback`; exact promotion is allowed only after `powerpoint_evidence.py gate` reports ready with pinned PowerPoint capture metadata. Document the explicit Mac blockers and remote handoff commands.
  Must NOT do: Do not alter evaluation thresholds to make results pass, claim 100% fidelity, report current test counts as timeless documentation, or add native evidence placeholders that look complete.

  Parallelization: Can parallel: NO | Wave 7 | Blocks: [final verification] | Blocked by: [1-22]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/check_exactness_contract.py:26-136` - existing doc/evidence drift checks
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/powerpoint_evidence.py:1-360` - native evidence summary/gate CLI
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/evaluate/README.md:45-190` - exact-promotion workflow
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/docs/architecture/CAPABILITY_MATRIX.md:23-42` - matrix to update truthfully
  - Pattern:  `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/README.md:175-191` - public feature claims
  - Test:     `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/.github/workflows/ci.yml:32-99` - current Rust/Python/exactness commands

  Acceptance criteria (agent-executable only):
  - [ ] RED/GREEN: the checker fails after intentionally changing one manifest tier without docs/evidence, then passes on the committed tree.
  - [ ] `python3 evaluate/check_exactness_contract.py --repo-root .` and all evaluation unit tests pass.
  - [ ] `python3 evaluate/powerpoint_evidence.py gate --family text-layout --golden-set-dir evaluate/golden_set --output-dir evaluate/powerpoint_golden --output-json <attemptDir>/task-23-powerpoint-text-layout-gate.json` remains nonzero on this Mac with a clear missing-native-evidence report; `text-layout` is the only family currently accepted by `EXACT_PROMOTION_FAMILIES`, and no exact tier is promoted.
  - [ ] Full Rust, Python evaluation, format, clippy, and doc checks pass; `git diff --check` is clean and `.DS_Store` remains untracked/unmodified.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: All local completeness gates pass honestly
    Tool:     bash
    Steps:    Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `python3 -m unittest evaluate.tests.test_validate_powerpoint_golden evaluate.tests.test_scaffold_powerpoint_golden_batch evaluate.tests.test_summarize_powerpoint_golden evaluate.tests.test_powerpoint_evidence_cli evaluate.tests.test_check_exactness_contract evaluate.tests.test_completeness_manifest evaluate.tests.test_check_preset_adjustments evaluate.tests.test_create_completion_decks -v`, `python3 evaluate/check_exactness_contract.py --repo-root .`, and `git diff --check`; tee the complete output to `<attemptDir>/task-23-final-local-gates.txt`.
    Expected: Every local gate exits 0 and docs/manifests agree on exact/approximate/fallback status.
    Evidence: <attemptDir>/task-23-final-local-gates.txt   (attemptDir = currentAttemptDir from `omo ulw-loop status --json`, .omo/evidence/ulw/<session>/<goalId>/a<attempt>)

  Scenario: PowerPoint-native text-layout exact gate stays blocked on this Mac
    Tool:     bash
    Steps:    Run `set +e; python3 evaluate/powerpoint_evidence.py gate --family text-layout --golden-set-dir evaluate/golden_set --output-dir evaluate/powerpoint_golden --output-json <attemptDir>/task-23-powerpoint-text-layout-gate.json > <attemptDir>/task-23-powerpoint-text-layout-gate.txt 2>&1; gate_rc=$?; set -e; test "$gate_rc" -ne 0`; do not create fake images.
    Expected: Nonzero exit identifies absent native images/metadata and lists the Windows/PowerPoint capture handoff; no implementation/test failure is conflated with the environmental blocker.
    Evidence: <attemptDir>/task-23-powerpoint-text-layout-gate.json
  ```

  Commit: YES | Message: `docs: align support claims with completeness evidence` | Files: [evaluate/check_exactness_contract.py, evaluate/tests/test_check_exactness_contract.py, evaluate/README.md, evaluate/powerpoint_golden/README.md, docs/architecture/PPTX_COMPLETENESS_CONTRACT.md, docs/architecture/CAPABILITY_MATRIX.md, docs/architecture/REMAINING_WORK_PLAN.md, SUPPORTED_FEATURES.md, README.md]

## Final verification wave (MANDATORY - after all implementation tasks)
> Runs in PARALLEL. ALL must APPROVE. Surface results to the caller and wait for an explicit "okay" before declaring complete.
- [ ] F1. Plan compliance audit - every task done, every acceptance criterion met
- [ ] F2. Code quality review - diagnostics clean, idioms match, no dead code
- [ ] F3. Real manual QA - every QA scenario executed with evidence captured
- [ ] F4. Scope fidelity - nothing extra shipped beyond Must-Have, nothing Must-NOT-Have introduced

## Commit strategy
- One logical change per commit. Conventional Commits (`<type>(<scope>): <subject>` body + footer).
- Atomic: every commit builds and passes tests on its own.
- No "WIP" / "fix typo squash later" commits on the final branch - clean up before merge.
- Use the repository's observed English conventional-prefix style (`feat:`, `fix:`, `test:`, `refactor:`, `docs:`); never introduce phase/step titles or Co-Authored-By lines.
- Before each commit, inspect the latest 20 non-merge commits, stage only the task's owned files, and verify the staged diff plus focused GREEN command. Cherry-pick task commits into the integration worktree in dependency order.
- No push. Preserve the untracked `.DS_Store`. Update root README only in Task 23 after implementation state is final.
- Reference the plan file path in the final commit footer: `Plan: .omo/plans/pptx-completeness-heavy.md`.

## Success criteria
- All Must-Have shipped; all QA scenarios pass with captured evidence; F1-F4 approved; commit history clean.
- Every manifest feature and official preset/adjustment fact is directly supported or preserved with a tested diagnostic; no relationship, part, or known element is silently lost.
- The 187/187 dispatch claim is separated from adjustment semantic coverage, and the checker reports zero unclassified preset or known official key.
- Local browser and LibreOffice evidence exists for every completion deck, but no feature is called exact without a complete PowerPoint-native evidence bundle.
- The runtime clearly exposes the features that cannot be truthfully completed on this Mac: native OLE/Office behavior, SmartArt auto-layout parity, complex Math typography parity, advanced 3D/effect parity, arbitrary media/autoplay behavior, complete animation semantics, and PowerPoint pixel identity.
