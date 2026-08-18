# Visual Acceptance Contract

The visual-fidelity goal is complete only when every requirement below passes
in the same evaluation wave.

## Corpus

- Exactly 10 deterministic PPTX decks.
- Exactly 10 slides per deck.
- All 100 slides have distinct visible text and geometry after removing the
  synthetic evidence badge.
- Badge-only, color-only, or metadata-only variants do not count as independent
  visual scenarios.

## Candidate Rendering

- Every deck converts through the released Rust CLI surface.
- Chromium captures exactly 100 candidate PNGs at 960x540.
- Missing, duplicate, extra, or incorrectly sized captures fail the wave.

## Vision Review

- A primary vision reviewer opens every full-resolution reference/candidate
  pair.
- An independent reviewer opens all 100 pairs again.
- Every pair must receive `PASS` from both reviewers.
- Any `REVISE`, `FAIL`, unresolved disagreement, clipping, overlap, alignment
  drift, missing semantic content, or reference defect fails the wave.
- Reference-renderer defects must be replaced by PowerPoint-native evidence;
  they cannot be waived into a pass.

## Native PowerPoint Oracle

- References come from Windows 11 Microsoft PowerPoint `Slide.Export`.
- Output resolution is exactly 960x540.
- Provenance, deck hashes, slide hashes, and dimensions pass validation.
- Strict RGBA comparison reports:
  - `ok: true`
  - `mismatched_pixels: 0`
  - `max_channel_delta: 0`

LibreOffice, browser, PDF, and manually labeled images are proxy evidence only.
The goal remains incomplete while native PowerPoint evidence is unavailable.

## Quality Gates

- Rust formatting, Clippy, workspace tests, and documentation tests pass.
- Python evaluation tests pass.
- Exactness and capability contracts pass.
- Language-server diagnostics contain no errors.
- `git diff --check` passes.
