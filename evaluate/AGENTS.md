# evaluate

Scope: fidelity evidence and scoring tooling. Root `AGENTS.md` rules apply; not repeated here. Setup, CLI flags, and score weights live in `README.md`; read it first.

## TWO REFERENCE TRACKS, NOT ONE
For the separate PPTX `exact`-promotion track, PowerPoint is the exactness
oracle and LibreOffice is secondary regression evidence; they never substitute
for each other there. The default seven-format general conversion contract
uses the locked macOS/Linux LibreOffice/Poppler profile over frozen admitted
corpora. Optional Office/native captures may enrich a selected profile or
support exact promotion, but they are not prerequisites for the default gate
and cannot be substituted into it.

- `powerpoint_golden/<deck>/Slide*.PNG` plus `metadata.json` per deck and one root `manifest.json`. Captured on Windows through `reference_render_powerpoint.ps1`. Native export only; no LibreOffice PNG ever lands here.
- `golden_references/` holds LibreOffice PNGs from `reference_render.py`. Generated, disposable, cheap to rebuild. Use it to catch PPTX regressions during iteration, never to justify an exact-tier change.
- `golden_set/` holds generated fixture decks from `create_golden_set.py`. Also disposable; both directories ship as `.gitkeep` only.
- `completion_decks/` is a separate contract corpus with its own README. Byte-identical output is its contract: fixed ZIP order, timestamps, compression, XML bytes, JSON serialization. Output dir must not exist; the generator publishes atomically.

A PPTX family stays `approximate` for the `exact` tier until PowerPoint
captures exist. "Looks right in LibreOffice" is not evidence for that exact
promotion.

## SCORING FUNCTION IS FROZEN
`evaluate_fidelity.py` is human-owned. Don't touch the weights, the metric definitions, the SSIM path, or the `FIDELITY_SCORE:` output line. If a metric feels wrong, say so in the report and stop; changing the ruler to move the number is the one failure mode this whole directory exists to prevent.

Everything else here is editable, but the same instinct applies: gates get stricter, not looser.

## EVIDENCE GATES
`powerpoint_evidence.py` is the single entry point. `summary` and `ready` describe state; `gate --family text-layout` decides. Exit code 0 means required decks, metadata, slide exports, and manifest consistency all check out. Anything else means the promotion doesn't happen.

`EXACT_PROMOTION_FAMILIES` in `powerpoint_evidence.py` and the fixture list under "Text/Layout exact-promotion gate" in `README.md` are the same list in two places, and `check_exactness_contract.py` fails when they drift. Edit both, or the contract check will tell you which one you forgot. The same checker pins literal sentences in `README.md`, `../README.md`, and `docs/architecture/CAPABILITY_MATRIX.md`, so rewording those lines breaks CI. Change the checker in the same commit if the wording must change.

CI publishes `powerpoint-evidence-summary.json`, `powerpoint-evidence-text-layout-gate.json`, and `exactness-contract-report.json`. The gate step is advisory today (`|| true`); read the artifact rather than trusting a green job.

## AUTORESEARCH LOOP
`../autoresearch/` runs the change → `cargo check` → `cargo test` → score → keep-or-revert cycle. This directory is the referee, so agents in that loop must not write here at all. PowerPoint capture sits outside the loop; it needs Windows and a human.

Score deltas below noise aren't improvements. Twenty lines of special-casing for +0.001 is a revert.

## TESTS
Python tests run as package modules from the repo root: `python -m unittest evaluate.tests.<name>`. Relative-path invocations break the `evaluate.*` imports that every helper falls back through.

- Determinism is asserted, not assumed. Generate twice, compare bytes.
- No sleeps, no retries, no wall-clock waits. Every script here is synchronous.
- Assert exit codes and parsed JSON fields. Never assert on log prose or human-readable summaries.
- Manifests carry `sha256` digests. Regenerate the manifest when the checker complains; don't relax the comparison.
- `PPTX_COMPLETION_FIXTURE_ROOT` points the completion-deck tests at an existing read-only corpus instead of generating one.

## COMMANDS
```bash
python -m unittest evaluate.tests.test_powerpoint_evidence_cli -v
python -m unittest evaluate.tests.test_check_exactness_contract evaluate.tests.test_validate_powerpoint_golden -v
python evaluate/check_exactness_contract.py --repo-root .
python evaluate/powerpoint_evidence.py summary --golden-set-dir evaluate/golden_set --output-dir evaluate/powerpoint_golden
python evaluate/powerpoint_evidence.py gate --family text-layout --golden-set-dir evaluate/golden_set --output-dir evaluate/powerpoint_golden
python evaluate/evaluate_fidelity.py --project-root . --output-json result.json
python evaluate/create_completion_decks.py --output-dir /tmp/completion-decks
python evaluate/check_preset_adjustments.py
```

## ANTI-PATTERNS
- Editing `evaluate_fidelity.py`, or tuning any threshold to make a run pass.
- Committing generated artifacts into `golden_set/` or `golden_references/`.
- Hand-writing `metadata.json` or `manifest.json` to fake a PowerPoint batch. Scaffold it, validate it, or leave the family `approximate`.
- Adding a dependency for a helper script. The deck generators are standard library only, deliberately.
- Duplicating README setup instructions, score weights, or flag tables into this file.
- Claiming a family is gated without showing the gate's exit code and JSON.
