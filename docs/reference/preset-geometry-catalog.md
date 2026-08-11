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
- Microsoft Open Specifications accepted `upArrow` definition:
  <https://learn.microsoft.com/en-ca/answers/questions/2275994/uparrow-is-missing-in-presetshapedefinitions-xml>

The manifest records the edition, URLs, and SHA-256 checksums for the Part 1
ZIP, nested `dml-main.xsd`, and `presetShapeDefinitions.xml`. The ordered
`official_preset_names` inventory is independently bound to the checker's
canonical SHA-256 digest. Verify a downloaded Part 1 ZIP and extract its nested
`ST_ShapeType` enumeration and `presetShapeDefinitions.xml` semantics with:

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

The ECMA geometry artifact repeats `upDownArrow` and omits `upArrow`. The
checker requires the two `upDownArrow` definitions to be identical and keeps
the ECMA base audit fixed at 298 adjustments and 285 handle constraints. The
missing `upArrow` definition is supplied by the accepted Microsoft Open
Specifications answer in `evaluate/official_supplements/upArrow.xml`, whose
SHA-256 and provenance are bound by the manifest and checker. Its two
adjustments and two handle constraints produce a combined official contract of
300 adjustments and 287 constraints. This source classification does not
promote renderer fidelity beyond `non-exact` without native PowerPoint evidence.

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
python3 evaluate/check_preset_adjustments.py --repo-root . --bundle basic
python3 evaluate/check_preset_adjustments.py --repo-root . --bundle arrows
python3 evaluate/check_preset_adjustments.py --repo-root . --bundle remaining
python3 -m unittest evaluate.tests.test_check_preset_adjustments -v
```

The first command checks the 187-name dispatcher contract, traces literal
`adjust_values.get("...")` consumption through geometry-family functions with
a Rust comment/string/raw-string-aware lexical scan, and reports official
manifest keys that the current renderer never consumes. Dynamic or otherwise
unparseable adjustment lookups fail closed. The bundle commands scope consumed,
unknown, and unconsumed counts to the module groups required by Tasks 8-10. Use
`--source-root` to inspect a copied geometry-family directory, `--dispatcher`
to override the dispatcher source, and `--json` to write a stable
machine-readable report. Invalid manifests, source roots, dispatchers, and
official artifacts fail with stable error codes without Python tracebacks.
