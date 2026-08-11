# Completion deck fixtures

Generate the corpus into a disposable directory:

```bash
python3 evaluate/create_completion_decks.py --output-dir /tmp/completion-decks
```

The command writes ten small PPTX packages and `manifest.json`. The generator uses only the Python standard library and fixes ZIP entry order, timestamps, compression settings, XML bytes, and JSON serialization so repeated runs are byte-identical.

The manifest maps the planned Tasks 8-21 feature ids to decks. Every feature has `powerpoint_capture_required: true` and empty `native_evidence` slots. These slots are scaffolding only: generated decks contain no PowerPoint-native screenshots or claimed pixel expectations.

Adjustment rows are deliberately non-normative until Task 2 supplies `evaluate/preset_adjustments.json`. The four case kinds (`default`, `lower`, `upper`, and `representative`) are stable join points; the generator does not invent adjustment values, constraints, or expected pixels.

For a Task 2 integration, resolve each scaffold kind against the matching official manifest row and retain `expected_pixels: null` until genuine PowerPoint capture evidence is produced.
