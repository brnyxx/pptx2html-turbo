# PPTX Completeness Delivery Status and Roadmap

Status date: 2026-08-12

This document is the execution ledger for the PPTX completeness program. It separates code delivery from full acceptance so that an implemented feature is not reported as complete while required browser or PowerPoint-reference evidence is still missing.

## Progress summary

| Measure | Result | Meaning |
|---|---:|---|
| Code delivered to `main` | 15 / 23 tasks (65.2%) | Tasks 1-15 are included in the publication batch represented by this document. |
| Fully accepted | 11 / 23 tasks (47.8%) | Tasks 1-11 passed implementation, regression, manual non-browser QA, and independent review gates. |
| Evidence-only work remaining | 4 tasks | Tasks 12-15 have accepted code but still require the mandatory browser screenshots or click checks. |
| Implementation remaining | 8 tasks (34.8%) | Tasks 16-23 still require implementation, review, and their matching manual QA surfaces. |
| Remaining to full program acceptance | 12 / 23 tasks (52.2%) | Evidence work for Tasks 12-15 plus implementation and evidence for Tasks 16-23. |

No universal PowerPoint-to-browser pixel or behavioral 1:1 claim is made. Microsoft documents PresentationML as a relationship-based package containing separate slide, notes, comments, media, animation, and transition parts. Browser rendering, codec policy, fonts, and event behavior can differ from PowerPoint. Native PowerPoint pixel equality remains `[교차검증 필요]` unless a pinned reference capture is attached to the relevant feature family.

## Delivered task ledger

| Task | Delivery | Acceptance | Concrete result |
|---:|---|---|---|
| 1 | Delivered | Accepted | Machine-readable completeness inventory is bound to evidence and drift checks. |
| 2 | Delivered | Accepted | Official-source preset adjustment manifest and semantic checker cover all declared bundles. |
| 3 | Delivered | Accepted | Deterministic completion-deck corpus, hostile fixtures, locators, and package contracts are generated reproducibly. |
| 4 | Delivered | Accepted | Public model is split into feature-owned modules with compatibility tests. |
| 5 | Delivered | Accepted | Reusable PPTX fixture builders enforce package, namespace, path, and relationship invariants. |
| 6 | Delivered | Accepted | Parser feature seams own their SAX state and typed relationships. |
| 7 | Delivered | Accepted | Renderer feature seams isolate text, fills, tables, charts, actions, and preserved fallbacks. |
| 8 | Delivered | Accepted | Basic, rectangle, bracket, scroll, and flowchart adjustment semantics consume the official keys and handle degenerate/extreme inputs deterministically. |
| 9 | Delivered | Accepted | Arrow, callout, and connector presets use strict official definitions, typed invalid routing, deterministic arcs, and atomic benchmarks. |
| 10 | Delivered | Accepted | Remaining preset families and custom geometry preserve strict formula failures and exact raw fallback metadata. |
| 11 | Delivered | Accepted | Conversion diagnostics are typed, deterministic, script-safe, namespace-aware, and available with conversion metadata. |
| 12 | Delivered | Browser evidence pending | All 54 DrawingML pattern presets render as deterministic approximate tiles; unknown patterns remain typed fallbacks. Mandatory full-page screenshot is still missing because the browser runtime returned `No browser is available`. |
| 13 | Delivered | Browser evidence pending | Picture bullets support slide paragraphs, slide-owned list styles, and table cells with bounded sizing and visible fallback. Mandatory desktop/mobile screenshots remain. |
| 14 | Delivered | Browser evidence pending | Package and built-in DrawingML table styles resolve with region, merge, explicit-cell, and typed fallback contracts. Mandatory browser screenshot remains. |
| 15 | Delivered | Browser evidence pending | Click and hover actions preserve external links, actual-order slide navigation, shape/group/table/run ownership, legacy-link precedence, inert unsafe actions, and stable diagnostics. Mandatory browser click proof and screenshot remain. |

## Active work that is not in `main`

Task 16, notes and comments preservation, is intentionally not included in this publication. Its worktree and branch are preserved because the implementation is uncommitted and has not completed the workspace, Clippy, manual QA, code-review, or specification-review gates. The current direction is:

- parse slide-to-notes, notes-master, legacy comments/authors, and modern comments/authors through exact internal relationships;
- keep notes and comments outside the visible slide canvas;
- preserve exact text, author, time, part, slide association, and unknown modern extension XML in typed metadata;
- retain text when an author or target is missing and emit stable diagnostics such as `COMMENT_AUTHOR_UNRESOLVED`;
- reject external, spoofed, malformed, duplicate, and package-root-escaping relationships without exposing unrelated targets.

The Task 16 worktree must be resumed and completed or explicitly discarded in a later change. It must not be inferred as shipped from the presence of local files.

## Remaining execution plan

| Task | Deliverable | Required exit conditions |
|---:|---|---|
| 16 | Preserve notes and comments off-canvas | Typed package records, missing-author diagnostics, modern raw extension preservation, focused/core/workspace gates, deterministic CLI output, browser metadata screenshot, code and specification reviews. |
| 17 | Reflection approximation and 3D fallback | Bounded deterministic reflection output; raw scene3d/sp3d/effect-DAG preservation; explicit approximate/fallback provenance; reference screenshot and reviews. |
| 18 | Bounded audio/video | Exact media relationships and MIME/assets; controls and user-gesture-safe playback only where supported; no autoplay invention; unsupported codecs remain typed fallbacks; browser playback evidence and reviews. |
| 19 | Timing, transitions, and animations | Preserve the full timing/transition graph; execute only a documented bounded subset; unknown nodes remain ordered raw metadata; deterministic time-based browser evidence and reviews. |
| 20 | Chart classification and deterministic fallback | Close chart-family classification, validate series/axis compatibility, render only supported bounded cases, and preserve preview/placeholder fallback with no silent data loss. |
| 21 | SmartArt, OLE, Math, AlternateContent, and unknown fallback | Harden relationship graphs, Markup Compatibility selection, raw payload identity, redaction, and deterministic placeholder metadata without claiming native Office behavior. |
| 22 | Diagnostics on CLI, Python, and WASM | Expose one stable diagnostic schema on every public surface; verify ordering, redaction, JSON round-trip, exit behavior, and local visual evidence. |
| 23 | Drift gates and capability finalization | Enforce manifest, documentation, fixture, official-source, and reference-evidence drift checks; synchronize the capability matrix and release checklist; run the final repository-wide gate. |

## Cross-cutting evidence backlog

The following evidence remains mandatory even though the corresponding code is already in `main`:

- Task 12: pattern-fill full-page screenshot.
- Task 13: picture-bullet desktop and mobile screenshots.
- Task 14: table-style screenshot plus already-generated machine-readable fallback JSON.
- Task 15: real browser click/hover/navigation proof, unsafe/custom action non-execution proof, and screenshot.
- Tasks 16-22: each task's planned browser or live-surface artifact.
- Exact promotion of any visual family: pinned PowerPoint reference images and environment metadata.

Static HTML parsing, HTTP byte equality, LibreOffice loadability, and deterministic hashes are useful supporting evidence, but they do not replace a QA scenario that explicitly requires browser interaction or a PowerPoint reference.

## Definition of done for each remaining task

1. Verify the OOXML contract against official Microsoft documentation. Mark unresolved native behavior `[교차검증 필요]`.
2. Add a failing public regression before the production fix.
3. Keep package relationships namespace-aware, type-aware, mode-aware, owner-relative, and package-root bounded.
4. Pass the focused test, full workspace test, `cargo clippy --workspace --all-targets -- -D warnings`, formatting, evaluate tests, and relevant Python lint.
5. Build the exact final SHA and exercise the real CLI or public library surface twice from clean output directories.
6. Capture deterministic output, typed diagnostic, redaction, script-safety, and non-finite-value evidence.
7. Complete independent code review and specification/evidence review at the exact final SHA.
8. Capture the required browser or PowerPoint-reference artifact. If the runtime is unavailable, keep the task in evidence-pending state.
9. Cherry-pick only accepted commits into the integration branch, verify the union, then publish the linear batch to `main`.

## Official references

- [Structure of a PresentationML document](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document)
- [Working with notes slides](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-notes-slides)
- [Working with comments](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-comments)
- [Working with animation](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-animation)
- [Add an audio file to a slide](https://learn.microsoft.com/en-us/office/open-xml/presentation/how-to-add-an-audio-to-a-slide-in-a-presentation)
- [MS-PPTX overview](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/b9ff79b4-5e24-4c85-b567-e5f43d498375)
