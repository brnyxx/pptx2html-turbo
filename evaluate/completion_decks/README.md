# Completion deck fixtures

Generate the corpus into a disposable directory:

```bash
python3 evaluate/create_completion_decks.py \
  --output-dir /tmp/completion-decks \
  --adjustment-manifest evaluate/preset_adjustments.json
```

The command writes ten small PPTX packages and `manifest.json`. The generator uses only the Python standard library and fixes ZIP entry order, timestamps, compression settings, XML bytes, and JSON serialization so repeated runs are byte-identical.

The manifest maps every planned Tasks 8-21 feature id to an observable ZIP part and byte token. Tests open each package, validate the common PresentationML relationship graph, and require every locator to resolve to real OOXML. Every feature has `powerpoint_capture_required: true` and empty `native_evidence` slots. These slots are scaffolding only: generated decks contain no PowerPoint-native screenshots or claimed pixel expectations.

The default adjustment manifest is the repository-local `evaluate/preset_adjustments.json`. The generator rejects any inventory other than 187 unique official preset names with a one-to-one preset row join. It reads representative basic, arrow, and remaining-family presets from that input, then emits `default`, `lower`, `upper`, and `representative` cases as real `a:prstGeom/a:avLst/a:gd` stimuli. Every case records its preset, adjustment key, original value or formula, source field, and source status. `representative` deliberately reuses the official default formula because Task 2 has no representative-value field; `expected_pixels` remains null.

The implementation is split by contract: `create_completion_decks.py` owns CLI orchestration, `completion_deck_package.py` owns deterministic OPC ZIP/common package construction, `completion_deck_specs.py` owns the ten deck stimuli, and `completion_deck_manifest.py` owns the Task 2 join and feature locators.

The OOXML mappings used by the fixtures follow the Microsoft Open XML documentation for [PresentationML structure](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document), [slide comments](https://learn.microsoft.com/en-us/office/open-xml/presentation/how-to-add-a-comment-to-a-slide-in-a-presentation), [slide audio](https://learn.microsoft.com/en-us/office/open-xml/presentation/how-to-add-an-audio-to-a-slide-in-a-presentation), [adjustment value lists](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.adjustvaluelist?view=openxml-3.0.1), [custom geometry](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.customgeometry?view=openxml-3.0.1), and [Office Math](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.math.officemathargumenttype?view=openxml-3.0.1). Modern comment relationship and content types follow the Microsoft PowerPoint extension specifications [MS-PPTX 2.1.7](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/b85a9293-bdca-4c6b-a554-8f3918db9791) and [MS-PPTX 2.1.9](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/4071f53f-9509-405f-a76b-594b865e177a).
