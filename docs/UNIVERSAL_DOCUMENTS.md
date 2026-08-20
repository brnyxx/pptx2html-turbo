# Universal document conversion

## Supported inputs

The universal API recognizes PPTX, DOCX, DOC, XLSX, XLS, PPT, and PDF from
content. Extensions are consistency hints, not the primary detector. Legacy
DOC, XLS, and PPT share the CFBF signature, so their filename or an explicit
format hint is required.

PPTX continues through the existing pure-Rust parser, resolver, and renderer.
The other Office formats use LibreOffice PDF export followed by Poppler HTML
conversion. PDF enters the Poppler stage directly.

The request spelling `.docs` is not a Microsoft Office format; Word support is
for `.docx` and `.doc`.

## Native runtime

The initial native implementation and local QA used LibreOffice 26.2 and
Poppler 26.03. Other versions can run but remain acceptance-unverified until a
versioned oracle lock records the exact evaluator toolchain.

Executable resolution order is:

1. explicit CLI or library configuration,
2. `DOCUMENT2HTML_SOFFICE`, `DOCUMENT2HTML_PDFTOHTML`, and
   `DOCUMENT2HTML_PDFINFO`,
3. `soffice`, `pdftohtml`, and `pdfinfo` from `PATH`.

Every conversion owns a private temporary workspace and LibreOffice profile.
The runner uses argument vectors rather than a shell string, clears inherited
credentials and proxy variables, bounds input/output/log sizes, applies a
stage deadline, rejects unsafe output entries, and removes the workspace on
completion.

Strict mode blocks remote-IP output on macOS while retaining local
LibreOffice IPC. Linux strict mode requires `bwrap`. Other platforms require a
trusted explicit isolation launcher. `--allow-unisolated` is an explicit
operational escape hatch; it emits a fallback diagnostic and cannot satisfy
the release security gate.

LibreOffice command-line behavior is documented at:
https://help.libreoffice.org/latest/en-US/text/shared/guide/start_parameters.html

## CLI

Build or run the universal binary:

```bash
cargo build -p pptx2html-cli --bin document2html
cargo run -p pptx2html-cli --bin document2html -- input.docx -o output.html
```

Useful options:

- `--input-format`: disambiguate a legacy CFBF input.
- `--no-embed`: write deterministic `assets/asset-NNNN.ext` files.
- `--diagnostics PATH`: write canonical diagnostics JSON.
- `--fail-on-fallback`: return exit code 2 after outputs are written.
- `--info`: report detected format and runtime without converting.
- `--soffice`, `--pdftohtml`, `--pdfinfo`: override executable paths.
- `--allow-unisolated`: opt out of strict process isolation.

The original `pptx2html` binary retains slide filtering, per-slide output,
hidden-slide handling, and scaling.

## Rust

Use `document2html-core` for detection, common contracts, capability
inspection, and pure-Rust PPTX conversion. Use `document2html-native` when the
six native formats are required.

`DocumentInput` combines bytes, an optional source filename, and an optional
explicit format. `DocumentConversionResult` reports format, HTML, external
assets, diagnostics, visual-unit count/kind, backend provenance, and runtime
capabilities.

## Python

The `document2html-py` crate builds the `document2html` Python module. Its
typed surface is recorded in
`crates/document2html-py/document2html.pyi`:

- `detect_format`
- `convert_file`
- `convert_bytes`
- `supported_formats`
- `DocumentConversionResult`

Python returns embedded-asset HTML so the result is self-contained.

## Browser WASM

Build the browser package:

```bash
wasm-pack build crates/document2html-wasm --target web --release
```

The WASM API exports `detect_document_format`, `convert_document`, and
`runtime_capabilities_json`. All seven formats can be detected, but only PPTX
can be converted because browser WASM cannot launch LibreOffice or Poppler.
Native-only requests return a backend-unavailable error instead of silently
degrading.

## 96% acceptance gate

The contract is `evaluate/multiformat/contract.v1.json`. Every format must pass
in the same evaluation wave. Scores cannot be pooled.

Each format requires:

- exactly 100 deterministic conformance units,
- exactly 75 independently sourced blind files,
- exactly 10 negative/security cases,
- two deterministic candidate runs,
- two independent full-resolution reviewers,
- a SHA-256-bound evaluator, corpus manifest, metric evidence, and oracle lock.

Thresholds include:

- conformance and blind aggregate score at least 96.00,
- visual at least 95.00,
- content at least 98.00,
- layout at least 94.00,
- every deterministic stratum at least 94.00,
- no conformance unit below 85.00,
- no blind file below 90.00,
- zero critical defects.

Word, Excel, and PowerPoint references require pinned Windows Microsoft Office
exports. PDF references use pinned PDF renderers. LibreOffice output is local
regression evidence and never substitutes for Microsoft Office-native
evidence.

Run the gate:

```bash
uv run python -m evaluate.multiformat_gate \
  --reports-dir evaluate/multiformat/reports \
  --oracle-lock evaluate/multiformat/oracle-lock.json
```

Exit codes are 0 for `PASS`, 1 for `FAIL`, and 2 for `INCOMPLETE`.

Create a non-destructive, explicitly incomplete evidence wave before adding
corpus files:

```bash
uv run python -m evaluate.scaffold_multiformat_evidence \
  --output-dir evaluate/multiformat/wave
```

The scaffolder refuses to overlay a non-empty directory and never writes
passing scores. Every report remains `INCOMPLETE` until real corpus, metric,
security, determinism, review, and oracle artifacts replace the templates.

Before candidate execution, validate every populated corpus manifest:

```bash
uv run python -m evaluate.multiformat_corpus \
  --manifest evaluate/multiformat/wave/corpora/docx/manifest.json
```

Schema-v2 manifests bind the contract and every streamed source digest,
enforce bounded OOXML/CFBF/PDF structure and format-specific 100-unit quotas,
and assign contiguous source-relative unit ordinals. Legacy paired coverage
binds the modern counterpart and its underlying strata; binary-specific
coverage requires independent provenance. The 75 blind declarations use
canonicalized producer identities and unique hashes, source URIs, and template
families. Ten security cases must exactly match the format-specific contract.
`INCOMPLETE` templates exit 2; malformed, unsafe, or quota-invalid manifests
exit 1. The product gate runs this validation for every `READY` report.

Reports are generated, not authored. Bound raw PNGs and semantic inventories
are re-scored with the fixed five-scale MS-SSIM, active-tile SSIM, DeltaE00,
edge F1, text/cell, object, IoU, reading-order, and baseline formulas. Values
are retained to six decimals, and blind scores use unit mean within each file
before the 75-file mean. The gate independently rebuilds the report and rejects
any aggregate mismatch.

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m evaluate.assemble_multiformat_report \
  --oracle-lock evaluate/multiformat/wave/oracle-lock.json \
  --evaluator-manifest evaluate/multiformat/wave/evidence/evaluator-manifest.json \
  --corpus-manifest evaluate/multiformat/wave/corpora/docx/manifest.json \
  --metrics-evidence evaluate/multiformat/wave/metrics/docx.json \
  --evidence-root evaluate/multiformat/wave \
  --output evaluate/multiformat/wave/reports/docx.json
```

On a network-disabled Windows host with desktop Microsoft Office and Poppler,
capture the positive native references:

```powershell
pwsh -File evaluate/capture_multiformat_office_oracles.ps1 `
  -InputManifest evaluate/multiformat/wave/office-input-manifest.json `
  -OutputDir evaluate/multiformat/wave/office-oracles `
  -GoldenSetRevision <commit-sha> `
  -FontBundleSha256 <64-lowercase-hex> `
  -HostNetworkIsolation disabled
```

Ready reports bind relative evaluator, corpus-manifest, and metrics-evidence
paths to their real SHA-256 digests. The gate resolves every path under the
declared evidence root, rejects traversal, and re-hashes each file:

```bash
uv run python -m evaluate.multiformat_gate \
  --reports-dir evaluate/multiformat/wave/reports \
  --oracle-lock evaluate/multiformat/wave/oracle-lock.json \
  --evidence-root evaluate/multiformat/wave
```

Authoritative format references:

- ECMA-376 OOXML:
  https://ecma-international.org/publications-and-standards/standards/ecma-376/
- Microsoft DOC:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-doc/
- Microsoft XLS:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xls/
- Microsoft PPT:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ppt/
- ISO 32000-1 PDF:
  https://www.iso.org/standard/51502.html
