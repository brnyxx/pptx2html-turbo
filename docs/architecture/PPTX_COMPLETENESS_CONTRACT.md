# PPTX Completeness Contract

## Purpose

This contract defines finite, observable completeness for PPTX conversion. It does not promise universal PowerPoint-to-browser equivalence. A feature is complete only when it is directly supported with evidence or preserved through the declared deterministic fallback policy. Content must not silently disappear.

`evaluate/completeness_manifest.json` is the machine-readable inventory. Every feature row has a stable id, an official source URL, and an OOXML qualified name or relationship type. The source is format authority, not evidence that the current converter supports the feature.

## Fidelity dimensions

Each feature declares these independent dimensions:

- `semantic`: semantic preservation of package content and relationships.
- `visual`: static visual rendering in generated HTML, CSS, or SVG.
- `behavioral`: behavioral playback, including actions, media, timing, animation, and transitions.

Each dimension has one tier and one stage. The allowed tiers are `exact`, `approximate`, `fallback`, and `unparsed`. The allowed stages are `parsed`, `resolved`, `rendered`, `fidelity-tested`, and `not-applicable`.

- `exact` means a controlled PowerPoint-native comparison supports the claimed dimension.
- `approximate` means there is direct output with known fidelity limits.
- `fallback` means the feature is preserved or represented by deterministic fallback output and diagnostic metadata.
- `unparsed` means the converter has not preserved the feature sufficiently for reliable downstream handling.

The manifest describes target dispositions, not a claim of current direct support merely because an XML name appears in source code. This initial inventory promotes no feature to `exact`.

## Stable feature inventory

The manifest inventories the PresentationML, DrawingML, Office Math, markup-compatibility, and Open Packaging Convention relationship families that are in scope. It includes presentation, master, layout, theme, shapes, custom geometry, text and bullets, fills, effects and 3D, tables, images, charts, diagrams, OLE, Math, notes, comments, media, hyperlinks/actions, timing, transitions, extensions, and `mc:AlternateContent`.

Each feature's `id` is stable and lowercase kebab case. Later tasks may add facts to an existing row, but must not rename the id. Every row must retain an `official_source`, `ooxml`, `fallback_policy`, and all three dimension declarations.

## Fallback diagnostic envelope

Any fallback must emit the stable code declared by its feature row and retain this metadata whenever it is known:

- `code`, `family`, `tier`, `stage`, and `slide_index`
- `part_name`, `relationship_id`, and `relationship_type`
- `qualified_name`, `bounds`, and `raw_reference`
- `fallback_kind` and `reason`

`raw_reference` identifies preserved XML or a package part without requiring raw XML to be embedded in HTML. A missing value is allowed only when the source package does not provide it. The fallback policy is still required for an `unparsed` row so downstream work has a deterministic, non-silent destination.

## Exact-promotion gate

No row or dimension may use `exact` without a complete PowerPoint-native evidence bundle. The validator rejects a missing or incomplete bundle with `EXACT_REQUIRES_POWERPOINT_EVIDENCE`.

The evidence bundle must identify:

- `oracle` set to `PowerPoint-native`
- `powerpoint_version` and `windows_version`
- `capture_metadata` and the matching `fixture_bundle`
- `artifact_paths` for the exported native result and comparison artifacts

LibreOffice and browser output are useful regression evidence but cannot satisfy this gate. The native evidence workflow is documented in `evaluate/README.md` and `evaluate/powerpoint_golden/README.md`.

## Implementation stop conditions

Implementation stops only when all of these are true:

1. Every manifest row has tested direct support or tested deterministic fallback.
2. Unknown relationships and elements produce typed diagnostics instead of silent loss.
3. The preset-adjustment contract has no unclassified preset or undocumented consumed key.
4. The affected local test and workspace gates pass.
5. No `exact` promotion lacks its complete PowerPoint-native evidence bundle.

The current Mac environment cannot create a PowerPoint-native oracle. That limitation is an external gate, not permission to weaken or bypass `EXACT_REQUIRES_POWERPOINT_EVIDENCE`.
