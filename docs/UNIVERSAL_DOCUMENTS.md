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

## Reference profiles

The default required profile is `libreoffice-poppler`. Its signed production
capture currently runs on supported macOS hosts: the six Office formats use
locked LibreOffice PDF export followed by locked Poppler rendering/text
extraction, while PDF uses locked Poppler directly. Linux supports native
conversion with the same tools, but cannot complete the signed profile until a
Linux process-sandbox backend is implemented. The profile runs over the seven
frozen format corpora and requires a schema-2 portable lock, signed receipt,
and SHA-256 binding for every admitted source, tool, runtime, and output.
Missing, stale, substituted, or tampered evidence remains `INCOMPLETE` or
`FAIL`.

Signed Microsoft Office/Windows oracle evidence remains supported as the
optional `microsoft-office` profile. It is not a prerequisite for the default
portable acceptance path. If selected, that profile still requires its signed
schema-1 lock, verifier-bound capture, provenance, and artifact hashes; an
incomplete Office profile cannot pass and cannot be substituted for portable
evidence.

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

## General conversion evaluation contract

The approved general claim wording is
`96% under the documented general conversion evaluation contract`. The contract
is `evaluate/multiformat/contract.v1.json`; every format must pass in the same
evaluation wave, and scores cannot be pooled. This is not a Microsoft Office
pixel-accuracy, PowerPoint pixel-match, byte-identical-output, or PPTX
`exact`-promotion claim.

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

For the standardized claim, 96.00 applies to the conformance and blind
aggregate scores only. Structural validity, exact unit/file/security quotas,
review outcomes, determinism, and SHA-256 evidence bindings are hard gates;
they are not averaged into a 96% promise. Textual/content similarity is `C`
(minimum 98.00), layout is `L` (minimum 94.00), and visual similarity is `V`
(minimum 95.00). Two clean runs must produce identical HTML, inventories, and
screenshot hashes. Frozen source bytes and signed hashes bind the evaluator,
corpora, tools, runtimes, and admitted outputs; they establish evidence
identity, not byte-identical output across separate capture environments.
Microsoft Office pixel-accuracy wording is prohibited for this general claim.

The default `libreoffice-poppler` profile uses the locked macOS
LibreOffice + Poppler route for the six Office formats and locked Poppler
directly for PDF. Signed Microsoft Office/Windows exports remain supported
only as the optional `microsoft-office` profile; they are not required for the
default path. Each selected profile must satisfy the same signed, hash-bound,
fail-closed evidence contract.

The native converter may run on Linux, but signed portable reference capture
currently requires the macOS `sandbox-exec` backend. A Linux capture attempt is
`INCOMPLETE` until an equivalent process-sandbox implementation is available.

Run the gate:

```bash
uv run python -m evaluate.multiformat_gate \
  --reports-dir evaluate/multiformat/reports \
  --oracle-lock pptx=evaluate/multiformat/oracle-locks/pptx.json \
  --oracle-lock docx=evaluate/multiformat/oracle-locks/docx.json \
  --oracle-lock doc=evaluate/multiformat/oracle-locks/doc.json \
  --oracle-lock xlsx=evaluate/multiformat/oracle-locks/xlsx.json \
  --oracle-lock xls=evaluate/multiformat/oracle-locks/xls.json \
  --oracle-lock ppt=evaluate/multiformat/oracle-locks/ppt.json \
  --oracle-lock pdf=evaluate/multiformat/oracle-locks/pdf.json
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

Candidate evidence is captured by `evaluate.capture_multiformat_candidates`.
It requires a release converter, exact Chromium/Playwright/font/runtime hashes,
a clean Git revision, a network-disabled/no-golden sandbox attestation, and a
READY frozen corpus. The host attestation is Ed25519-signed by a public key
bound in the oracle lock; converter/native runtime hashes, versions, build
revision, Chromium, Playwright, OpenSSL, and the isolated font environment are
also lock-checked. Executed binary bytes are materialized into evidence and
re-hashed by the gate; the signed host claim carries a per-run nonce. It runs
every conformance and blind source twice in fresh
converter and Chromium workspaces. Candidate HTML is served from an intercepted
synthetic origin with scripts and external resources blocked; unit discovery,
native dimensions, DOM inventories, and all run-2 artifacts are fail-closed and
digest-bound.
The trusted host receipt signer runs only after capture and signs the runtime,
execution, determinism, and complete artifact root; replayed preflight
attestations cannot authorize a different evidence set. Portable receipt
schema 2 keeps the canonical runtime nonce field but makes it a signer-derived
SHA-256 claim identity over a fixed domain, complete scope, batch identity,
artifact root, and canonical receipt path. Request schema 2 contains no nonce.
Exact replay is byte-identical; any batch, artifact, scope, or path variant has
a different nonce unless SHA-256 collides. No mutable replay state is used.

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

For the optional `microsoft-office` profile, a network-disabled Windows host
with desktop Microsoft Office and Poppler can capture the positive native
references. Schema 2 also records `pdftotext -bbox-layout` output so the
finalizer can build page-bound text geometry and worksheet/cell inventories:

```powershell
pwsh -File evaluate/capture_multiformat_office_oracles.ps1 `
  -InputManifest evaluate/multiformat/wave/office-input-manifest.json `
  -OutputDir evaluate/multiformat/wave/office-oracles `
  -GoldenSetRevision <commit-sha> `
  -FontBundleSha256 <64-lowercase-hex> `
  -HostNetworkIsolation disabled
```

Finalize one format into a signed product-gate capture:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m evaluate.finalize_multiformat_office_oracles \
  --batch-manifest evaluate/multiformat/wave/office-oracles/manifest.json \
  --contract evaluate/multiformat/wave/contract.json \
  --corpus-manifest evaluate/multiformat/wave/corpora/docx/manifest.json \
  --evaluator-manifest evaluate/multiformat/wave/evidence/evaluator-manifest.json \
  --oracle-lock evaluate/multiformat/wave/oracle-lock.json \
  --output-dir evaluate/multiformat/wave/oracle/docx \
  --receipt-signer <trusted-signer> \
  --public-key <office-oracle-public-key> \
  --openssl <locked-openssl> \
  --project-revision <commit-sha> \
  --run-nonce <64-lowercase-hex>
```

For the optional `microsoft-office` profile, remote execution can dispatch
`.github/workflows/capture-office-oracles.yml`. It deliberately requires a
dedicated self-hosted runner labeled `office-oracle`; the host-owned
`OFFICE_ORACLE_CAPTURE_WRAPPER` must enforce network isolation around both raw
capture and finalization. The lock uses a separate `office_oracle_verifier`
key, and pins the Office channel, Word, Excel, PowerPoint, `pdfinfo`,
`pdftoppm`, and `pdftotext` identities. See GitHub's official
[self-hosted runner workflow documentation](https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/use-in-a-workflow)
for label routing. No self-hosted Office runner is currently registered in this
repository, so optional Office batches remain external prerequisites rather
than claimed evidence. The default portable profile does not depend on this
runner.

Ready reports bind relative evaluator, corpus-manifest, and metrics-evidence
paths to their real SHA-256 digests. The gate resolves every path under the
declared evidence root, rejects traversal, and re-hashes each file:

```bash
uv run python -m evaluate.multiformat_gate \
  --reports-dir evaluate/multiformat/wave/reports \
  --oracle-lock pptx=evaluate/multiformat/wave/oracle-locks/pptx.json \
  --oracle-lock docx=evaluate/multiformat/wave/oracle-locks/docx.json \
  --oracle-lock doc=evaluate/multiformat/wave/oracle-locks/doc.json \
  --oracle-lock xlsx=evaluate/multiformat/wave/oracle-locks/xlsx.json \
  --oracle-lock xls=evaluate/multiformat/wave/oracle-locks/xls.json \
  --oracle-lock ppt=evaluate/multiformat/wave/oracle-locks/ppt.json \
  --oracle-lock pdf=evaluate/multiformat/wave/oracle-locks/pdf.json \
  --evidence-root evaluate/multiformat/wave
```

For every `READY` security track, corpus validation derives the declared attack
family from the source bytes. It checks OOXML package relationships and
payloads, CFBF FAT/DIFAT/directory/mini-stream structure and Office storages,
and PDF cross-reference/object/page/action structure. This proves that a
fixture contains the stated precondition; signed execution evidence remains
separately required to prove rejection, safe conversion, isolation, and
bounded resource use.

Authoritative format references:

- ECMA-376 OOXML:
  https://ecma-international.org/publications-and-standards/standards/ecma-376/
- Microsoft DOC:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-doc/
- Microsoft XLS:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xls/
- Microsoft PPT:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ppt/
- Microsoft Compound File Binary Format:
  https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
- Microsoft Office VBA File Format:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ovba/
- ISO 32000-1 PDF:
  https://www.iso.org/standard/51502.html
