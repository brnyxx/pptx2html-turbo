# PPTX Completeness Contract

## Purpose

This contract makes completeness finite and observable. It does not promise universal PowerPoint-to-browser 1:1 rendering. Every inventoried feature must have direct output with an honest tier or a deterministic fallback diagnostic; content must not silently disappear.

`evaluate/completeness_manifest.json` is schema version `2.0`. Its root `contract_scope` must exactly equal `current and target dispositions; no exact claim without feature evidence`. The checker reports `INVALID_CONTRACT_SCOPE` for drift.

## Disposition schema

Every feature has independent `current` and `target` objects. Each contains exactly the `semantic`, `visual`, and `behavioral` dimensions, and each dimension has one tier and one stage.

- `current` records observed implementation maturity from `docs/architecture/CAPABILITY_MATRIX.md` and `SUPPORTED_FEATURES.md`.
- `target` records the required destination: direct output is `approximate` until native evidence permits `exact`; otherwise it is deterministic `fallback`.
- `approximate` is direct-but-nonexact output, `fallback` is an explicit placeholder or sideband, and `unparsed` is not yet preserved sufficiently.

The initial inventory has no `exact` row. `EXACT_REQUIRES_POWERPOINT_EVIDENCE` remains mandatory for all three dimensions in either disposition.

## Stable inventory and sources

Feature IDs are lowercase kebab case and the checker requires exact equality with its frozen inventory. Missing, extra, and duplicate IDs are rejected. Rows split different states rather than combining them: character/number bullets and picture bullets; ordinary fills and pattern fills; shadow effects and reflection/3D; direct chart subset, preview fallback, and placeholder fallback; run/cell hyperlinks and shape/action hyperlinks; and audio and video.

The inventory includes the PresentationML parts exposed by Microsoft's structure document: notes master, handout master, comment authors, picture, additional characteristics, bibliography, custom XML, thumbnail, theme override, slide synchronization, content part, embedded control persistence, embedded package, and user-defined tags. It also isolates table styles and RTL text from their directly rendered parent families. A verified row has an approved Microsoft or ECMA source and an allowlisted QName or relationship type. `source_status: unavailable` means the official structure source confirmed the part family but this contract did not verify its QName or relationship URI; its empty `ooxml` field prevents an invented value and its target is deterministic fallback.

The checker accepts sources only from `learn.microsoft.com` and `ecma-international.org`, rejects malformed or unapproved URLs with `UNOFFICIAL_SOURCE`, and rejects a QName or relationship type outside the known official inventory.

## Fallback diagnostic envelope

Any fallback keeps the row's stable diagnostic code and, when known, the metadata `code`, `family`, `tier`, `stage`, `slide_index`, `part_name`, `relationship_id`, `relationship_type`, `qualified_name`, `bounds`, `raw_reference`, `fallback_kind`, and `reason`.

## Exact-promotion gate

No row or dimension may use `exact` without a complete PowerPoint-native evidence bundle: `oracle` set to `PowerPoint-native`, `powerpoint_version`, `windows_version`, `capture_metadata`, `fixture_bundle`, and nonempty `artifact_paths`. Browser and LibreOffice output are useful regression evidence but cannot satisfy this gate.
