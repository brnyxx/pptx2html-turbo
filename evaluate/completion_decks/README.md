# Completion deck fixtures

Generate the corpus into a disposable directory:

```bash
python3 evaluate/create_completion_decks.py \
  --output-dir /tmp/completion-decks

# Equivalent package entry point
python3 -m evaluate.create_completion_decks \
  --output-dir /tmp/completion-decks-module
```

Both commands use the committed `evaluate/preset_adjustments.json` by default and write ten small PPTX packages plus `manifest.json`. The output path must not exist; every pre-existing file, directory, or symlink is refused. Artifacts are written to a temporary directory beside the target and the completed directory is published only after every write succeeds. The generator uses only the Python standard library and fixes ZIP entry order, timestamps, compression settings, XML bytes, and JSON serialization so repeated runs are byte-identical.

The manifest maps every planned Tasks 8-21 scenario to an observable ZIP part and byte token and to a `completeness_feature_id` from the committed Task 1 canonical inventory. Tests reject duplicate, missing, extra, or unknown scenario mappings before converting rows to keyed lookups. Every feature has `powerpoint_capture_required: true` and empty `native_evidence` slots. These slots are scaffolding only: generated decks contain no PowerPoint-native screenshots or claimed pixel expectations. `pattern-fill-unknown` is deliberately marked `schema_expectation: negative` with the stable `DRAWINGML_PATTERN_UNSUPPORTED` diagnostic; it is not claimed as a schema-valid positive, while `pattern-fill-known` remains positive.

Set `PPTX_COMPLETION_FIXTURE_ROOT` when running `evaluate.tests.test_create_completion_decks` to validate an existing read-only corpus instead of generating one. The directory must already contain the ten decks and `manifest.json`; missing scenario IDs such as `media-audio` make the focused unittest fail and name the missing ID.

The generator runs the Task 2 checker against the 187-preset inventory, source digest, dispatcher, and consumed adjustment keys. An explicit `--adjustment-manifest` is accepted only when its parsed content exactly matches that canonical artifact. It then emits `default`, `lower`, `upper`, and `representative` cases as XML-escaped `a:prstGeom/a:avLst/a:gd` stimuli. Every case records its preset, adjustment key, original value or formula, source field, and source status. `representative` deliberately reuses the official default formula because Task 2 has no representative-value field; `expected_pixels` remains null.

The implementation is split by contract: `create_completion_decks.py` owns CLI orchestration and output policy; package/common modules own deterministic OPC and theme construction; notes, chart, table, and fallback modules own their schema-bounded OOXML; inventory and feature modules own canonical IDs and locators; and `completion_deck_manifest.py` owns the Task 2 join and XML-safe adjustment scaffolds.

The table deck pairs one package-defined custom style with a loadable missing-definition fallback. The fallback preserves the official built-in `Medium Style 2 - Accent 1` GUID plus `firstCol` and `bandCol` flags while deliberately omitting that style from `ppt/tableStyles.xml`, which lets later work verify ID/flag preservation and fallback diagnostics independently from the positive row-style case.

The OOXML mappings follow Microsoft's documentation for [PresentationML structure](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document), [notes slides](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-notes-slides), [table styles](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.tablestyle?view=openxml-3.0.1), [table style IDs](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.tablestyleid?view=openxml-3.0.1), [Office built-in table style IDs](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/d1f27e91-0523-459b-bc14-ba61b29e95e6), [diagram relationship IDs](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.diagrams.relationshipids?view=openxml-3.0.1), [OLE objects](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.oleobject?view=openxml-3.0.1), [adjustment value lists](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.adjustvaluelist?view=openxml-3.0.1), [custom geometry](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.customgeometry?view=openxml-3.0.1), and [Office Math](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.math.officemathargumenttype?view=openxml-3.0.1). Modern comment structure and relationships follow [MS-PPTX CT_Comment](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/161bc2c9-98fc-46b7-852b-ba7ee77e2e54) and the [Comment Part specification](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/b85a9293-bdca-4c6b-a554-8f3918db9791).
