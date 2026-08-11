# Preset Geometry Adjustment Catalog

`evaluate/preset_adjustments.json` is the machine-readable contract for preset
geometry adjustments. It classifies all 187 values in ECMA-376 Part 1
`ST_ShapeType` and records each official `a:avLst/a:gd` name, default formula,
and adjustment-handle constraint in source order.

## Sources

- ECMA-376 Part 1, 5th edition, December 2016:
  <https://ecma-international.org/publications-and-standards/standards/ecma-376/>
- Official Part 1 download:
  <https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip>
- Microsoft `a:avLst` contract:
  <https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.adjustvaluelist?view=openxml-3.0.1>
- Microsoft `a:prstGeom` contract:
  <https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.presetgeometry?view=openxml-3.0.1>

The manifest records the edition, URLs, and SHA-256 checksums for the Part 1
ZIP, `dml-main.xsd`, and `presetShapeDefinitions.xml`. Verify a downloaded Part
1 ZIP with:

```bash
python3 evaluate/check_preset_adjustments.py \
  --repo-root . \
  --official-artifact /path/to/ECMA-376-1_5th_edition_december_2016.zip
```

## Contract boundaries

Dispatcher completeness and adjustment-semantics completeness are independent.
The checker requires all 187 official preset names to be dispatched, but that
does not make any renderer implementation exact. Every manifest row remains
`non-exact` until native PowerPoint evidence satisfies the repository exactness
gate.

The official geometry artifact repeats `upDownArrow` and omits `upArrow`.
Consequently, `upArrow.source_status` is `unavailable`; its current Rust keys
are recorded only as non-normative preservation data. They are not official
defaults or ranges.

The checker also records implementation-only keys where current Rust consumes a
key that the official definition does not assign to that preset. These entries
are explicit adjustment-semantics gaps with non-exact preservation. They are not
promoted into the official `adjustments` list.

Custom geometry is a separate open-name contract. Names in
`a:custGeom/a:avLst/a:gd` are document-defined formula guides and are not
validated against the closed preset adjustment names.

## Static verification

```bash
python3 evaluate/check_preset_adjustments.py --repo-root .
python3 -m unittest evaluate.tests.test_check_preset_adjustments -v
```

The first command checks the 187-name dispatcher contract, traces literal
`adjust_values.get("...")` consumption through geometry-family functions, and
reports official manifest keys that the current renderer never consumes. Use
`--source-root` to inspect a copied geometry-family directory and `--json` to
write a stable machine-readable report.
