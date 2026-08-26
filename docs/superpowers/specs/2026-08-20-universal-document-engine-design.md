# Universal Document-to-HTML Engine Design

## Goal

Evolve the PPTX-specific workspace into a document-to-HTML tool that accepts
PPTX, DOCX, DOC, XLSX, XLS, PPT, and PDF while preserving every existing
PPTX API and fidelity contract.

The project succeeds only when every format independently satisfies the
approved claim
`96% under the documented general conversion evaluation contract`. Scores are
never pooled across formats, modern and legacy variants, or metric families.
This is a general conversion-evaluation claim, not Microsoft Office pixel
accuracy, PowerPoint pixel matching, byte-identical output, or the separate
PPTX `exact`-promotion tier.

`.docs` is not a Microsoft Office file format. The requested Word formats are
therefore interpreted as `.docx` and `.doc`.

## Constraints

- Existing `pptx2html-*` crate names, public functions, diagnostics, slide
  indexing, Python bindings, and npm/WASM exports remain compatible.
- The existing pure-Rust PPTX implementation remains the preferred PPTX
  backend.
- `evaluate/evaluate_fidelity.py` remains human-owned and unchanged.
- The locked LibreOffice/Poppler portable profile supplies the reference
  evidence required by the general conversion-evaluation contract. Optional
  Office/PDF native workflows may provide enrichment or promotion evidence;
  Microsoft Office pixel output is not a requirement for the general claim.
  Native PowerPoint pixel evidence remains reserved for the separate `exact`
  capability claim.
- Browser WASM cannot launch native processes. Unsupported runtime/format
  combinations must be reported explicitly rather than silently degraded.
- New format support must preserve unsupported content through typed
  diagnostics; content may not disappear silently.
- No new third-party Rust dependency is required for the first implementation
  slice. Existing `zip`, `quick-xml`, and standard-library process APIs are
  sufficient for routing and native orchestration.

## Considered approaches

### 1. Reimplement every format in pure Rust

This gives the best theoretical portability, including browser WASM. It is not
the selected first path because Word pagination, Excel print layout, legacy
CFBF formats, and PDF painting each require a separate rendering engine. A
small implementation could parse content but could not honestly satisfy the
claim `96% under the documented general conversion evaluation contract`.

### 2. Delegate every format directly to LibreOffice

This provides broad native coverage quickly. It is not selected as the sole
path because it would regress the existing PPTX behavior, remove browser
support, make output dependent on an external process, and replace the current
PowerPoint-specific diagnostics and preservation model.

### 3. Hybrid common engine with isolated backends

This is the selected approach.

- Keep the current PPTX parser, resolver, and renderer unchanged.
- Add a format-neutral `document2html-core` crate for detection, routing,
  results, capabilities, and the PPTX adapter.
- Add a native-only `document2html-native` crate for Office-to-PDF and
  PDF-to-HTML orchestration.
- Route the CLI and Python bindings through the native engine.
- Route WASM through the common engine and expose runtime support explicitly.

This keeps the proven PPTX path intact while making the broad native path
replaceable. Future pure-Rust DOCX, XLSX, or PDF adapters can be added behind
the same interface without changing callers.

## Workspace architecture

```text
crates/
├── document2html-core/       # format-neutral API, detection, routing
├── document2html-native/     # native Office/PDF process backends
├── pptx2html-core/           # existing PresentationML implementation
├── pptx2html-cli/            # compatibility binary + generic CLI surface
├── pptx2html-py/             # compatibility module + generic Python surface
└── pptx2html-wasm/           # compatibility exports + runtime capabilities
```

Dependency direction:

```text
pptx2html-cli ─────► document2html-native ─────► document2html-core
pptx2html-py  ─────► document2html-native ─────► document2html-core
pptx2html-wasm ────────────────────────────────► document2html-core
                                                        │
                                                        ▼
                                                 pptx2html-core
```

No dependency points from `pptx2html-core` back to the generic engine.
`document2html-native` is never a dependency of `document2html-core` or
`pptx2html-wasm`. It is compiled only on non-WASM targets through the CLI and
Python dependency edges. Process APIs and native executable configuration do
not exist in `document2html-core`, so a feature toggle cannot accidentally
pull them into a browser build.

## Common API

### Document format

`DocumentFormat` is an exhaustive enum:

- `Pptx`
- `Docx`
- `Doc`
- `Xlsx`
- `Xls`
- `Ppt`
- `Pdf`

Its stable string values are lowercase file extensions.

### Input identity

`DocumentInput<'a>` contains `data: &'a [u8]`,
`source_name: Option<&'a str>`, and
`format_hint: Option<DocumentFormat>`. `DocumentInput::detect` leaves
`format_hint` empty. `DocumentInput::with_format` sets it explicitly.
Detection uses content first and the filename only to disambiguate formats with
the same container.

- `%PDF-` identifies PDF.
- OPC ZIP content types and the office-document relationship identify DOCX,
  XLSX, or PPTX.
- CFBF magic identifies a legacy Office container. A `.doc`, `.xls`, or `.ppt`
  source hint is required until a bounded stream-directory classifier is
  implemented.
- A source hint that conflicts with a conclusive signature is rejected.
- Ambiguous or unsupported input returns a typed error.

Extension-only detection is forbidden for ZIP, PDF, or arbitrary bytes.
When `format_hint` is absent, a recognized `.doc`, `.xls`, or `.ppt`
extension in `source_name` supplies the required CFBF disambiguation hint.
Other recognized extensions participate only as consistency checks against a
conclusive signature. Unknown extensions are ignored rather than treated as
formats.
An explicit format hint is accepted only when it agrees with every conclusive
signature and any recognized source extension. For CFBF input it is the
authoritative disambiguator when the bounded stream classifier is unavailable.
Any conflict returns `ConflictingFormatHint`.
Native Office conversion additionally requires either `source_name` or a
conclusive `DocumentFormat` supplied by the caller. Its temporary input is
always named `input.<DocumentFormat::extension()>`; the caller's original
basename is never copied into the temporary root.

### Conversion request

`DocumentConversionOptions` has one field,
`asset_mode: AssetMode`. `AssetMode` is an exhaustive enum with `Embed` as the
default and `External` as the other variant.

There is no generic backend preference and no nullable union of unrelated
format controls. `document2html-core` always selects the pure-Rust PPTX adapter
for PPTX. `document2html-native::NativeDocumentConverter` accepts all seven
formats: PPTX delegates back to the common core and the other six select the
native Office/PDF route. CLI and Python always call
`NativeDocumentConverter`, so their PPTX requests still reach the existing
pure-Rust implementation. Asking the core-only runtime to convert a
native-only format returns `BackendUnavailable`.

PPTX slide filtering and scale remain in the existing
`pptx2html_core::ConversionOptions` and a distinct
`convert_pptx_with_options` entry point. New format-specific controls require
their own typed request and are not added until a real consumer needs them.

`NativeBackendConfig` belongs only to `document2html-native` and contains:

- optional explicit `soffice_path`,
- optional explicit `pdftohtml_path`,
- optional explicit `pdfinfo_path`,
- `stage_timeout` with a 120-second default,
- `max_input_bytes` with a 512 MiB default,
- `max_output_bytes` with a 1 GiB default,
- `max_log_bytes` with a 1 MiB per-stream default,
- `process_isolation: ProcessIsolation` with `StrictAuto` as the default.

`ProcessIsolation` has `StrictAuto`, `Explicit(IsolationLauncher)`, and
`AllowUnisolated`. The unisolated variant is an explicit unsafe operational
choice, emits `NATIVE_NETWORK_ISOLATION_DISABLED`, and can never pass the
security or release gate.

### Conversion result

`DocumentConversionResult` contains:

- detected `DocumentFormat`,
- generated HTML,
- external assets,
- ordered generic diagnostics,
- unit count and unit kind (`page`, `sheet-page`, `slide`, or `slide-page`),
- backend identity and version,
- runtime capability evidence.

PPTX compatibility wrappers continue returning the existing
`pptx2html_core::ConversionResult`.

For native results, unit counting is derived from the PDF page count before
HTML conversion and cross-checked against Poppler page containers:

- DOCX, DOC, and PDF report `page`.
- XLSX and XLS report `sheet-page`.
- PPT reports `slide-page`.

The native backend emits one ordered `NATIVE_BACKEND_OPAQUE` diagnostic that
records backend identity and states that unsupported source constructs cannot
be classified element-by-element by this adapter. It never claims exact
preservation. Missing content is a conformance-gate failure, not a silently
promoted capability.

### Errors

`DocumentError` is a typed library error with separate variants for:

- ambiguous or unsupported format,
- conflicting format hint,
- unavailable backend,
- backend launch failure,
- backend nonzero exit,
- encrypted or password-protected input,
- timeout,
- input or output resource limit,
- partial backend output,
- missing or malformed backend output,
- I/O and PPTX adapter failures.

Errors include bounded command identity and exit status but never document
contents, credentials, external targets, or temporary paths containing secrets.
Known password/encryption exits are classified only by version-pinned stderr
signatures in the evaluator lock. Unknown text remains `BackendFailed`; it is
never guessed to be encryption. A timeout kills the child and discards every
partial output. Crossing an input/output/log bound returns
`ResourceLimitExceeded` and likewise discards partial output.

## Backend routing

### Pure-Rust PPTX backend

PPTX continues to call
`pptx2html_core::convert_bytes_with_options_metadata`. The generic result is a
projection over the existing HTML, assets, diagnostics, and slide count.

### Native Office backend

DOCX, DOC, XLSX, XLS, and PPT use a two-stage native pipeline:

```text
Office input
  -> LibreOffice headless PDF export
  -> Poppler pdftohtml complex single-document export
  -> normalized self-contained HTML + assets
```

The verified LibreOffice command surface is:

```text
soffice --headless -env:UserInstallation=file://<isolated-profile>
  --convert-to pdf --outdir <output-dir> <input-file>
```

The verified Poppler command surface is:

```text
pdftohtml -c -s -noframes -hidden -enc UTF-8 -fmt png
  <input.pdf> <output.html>
```

Every conversion gets an isolated temporary root and LibreOffice profile.
Output discovery is bounded to that root. Assets are collected deterministically
by normalized relative path. Cleanup is RAII-owned.

The temporary root is
`<system-temp>/document2html-<pid>-<atomic-counter>`. It is created with one
exclusive directory-creation call, never follows an existing entry, and has
owner-only permissions on Unix. A collision advances the counter and retries
at most 128 times; exhaustion is an I/O error. Separate `input`, `office`,
`poppler`, `profile`, `home`, and `tmp` children are created before launch.
Temporary absolute paths are never serialized into the result.

The Office input is `input/input.<extension>`. LibreOffice must create exactly
one regular `office/input.pdf`; a missing file, symlink, directory, differently
named PDF, or any extra regular file in `office` is malformed output.

Executable discovery is deterministic:

1. an explicit path in `NativeBackendConfig`,
2. `DOCUMENT2HTML_SOFFICE`, `DOCUMENT2HTML_PDFTOHTML`, or
   `DOCUMENT2HTML_PDFINFO`,
3. `soffice`, `pdftohtml`, or `pdfinfo` from `PATH`.

The process runner invokes `--version` or `-v` once per converter instance.
`pdfinfo -v` and `pdftohtml -v` must report the same Poppler release. The first
acceptance wave is locked to LibreOffice 26.2 and Poppler 26.03.
Other versions may run but are marked `unverified-runtime` and cannot produce a
passing acceptance report until the evaluator lock is intentionally revised.

Each stage runs with a 120-second default deadline, bounded captured
stdout/stderr, the conversion-owned working directory, and no inherited Office
profile. A timed-out child is killed before cleanup. Parallel conversions are
allowed because every invocation has a separate profile and output root; no
global daemon or shared output filename is used. An existing destination is
never overwritten by the library because conversion results stay in memory
until the caller writes them.

Child environments are cleared. The runner restores only the resolved `PATH`
needed by executable launch, required Windows `SystemRoot`/`WINDIR` values,
and controlled `HOME`, `TMPDIR`/`TEMP`/`TMP`, `LANG=C.UTF-8`,
`LC_ALL=C.UTF-8`, and `TZ=UTC`. Proxy, credential, Office-profile, and user
configuration variables are not inherited. Executables are canonicalized
before the environment is cleared.

Stdout and stderr are drained concurrently into separate bounded buffers. If
the next read would make either stream exceed `max_log_bytes`, the controller
kills the child and returns `ResourceLimitExceeded`; output at exactly the
limit is accepted. The controller always joins both drainers before cleanup.

`max_output_bytes` is a cumulative bound over every regular file in the
conversion root, including the intermediate PDF, HTML, and assets. It excludes
the input copy, which is governed by `max_input_bytes`, and captured logs,
which are governed by `max_log_bytes`. While a child is running, the process
runner scans the bounded root every 25 milliseconds, rejects symlinks and
non-regular entries, sums file lengths with checked arithmetic, and kills the
child immediately when the cumulative bound is crossed. A final scan runs
before any output is read.

Poppler output normalization has a closed contract:

1. exactly one requested `<stem>.html` regular file must exist;
2. every other accepted regular file must start with `<stem>` and have a
   recognized image extension (`png`, `jpg`, or `jpeg`);
3. symlinks, nested directories, device files, and unexpected sibling files
   fail conversion;
4. local image references must resolve to an accepted sibling asset;
5. `Embed` rewrites each local image reference to a MIME-correct data URI and
   returns no external assets;
6. `External` rewrites each local image reference to
   `assets/<deterministic-name>` and returns assets sorted by that path;
7. missing references, unreferenced emitted assets, absolute filesystem URLs,
   network image URLs, scripts, forms, plugins, and executable URL schemes fail
   normalization;
8. ordinary `http`, `https`, and `mailto` document links are retained only
   under the existing safe-link policy and receive `noopener noreferrer`;
9. the normalized HTML entry point is the sole `html` field in the result.

The locked Poppler grammar may contain inline `<style>` elements and `style`
attributes but no external stylesheet or font file. `<link>`, `<script>`,
`<form>`, `<object>`, `<embed>`, `<iframe>`, `@import`, and `@font-face` are
rejected. Every CSS `url(...)` token is parsed; only a data URI or an accepted
local image asset is allowed and rewritten under the same asset policy.
Unquoted, single-quoted, and double-quoted URL forms are all covered. The
normalizer rejects unknown URL-bearing attributes rather than copying them.

Accepted source assets are sorted by normalized original filename bytes and
renamed `asset-0001.<lowercase-extension>`, `asset-0002...`, and so on.
Sequence position, not content hash, resolves equal-byte assets; references to
the same source file reuse one name, so collisions are impossible.

Canonical HTML keeps Poppler's tag and attribute order, rewrites only validated
resource/link tokens, converts CRLF and CR to LF, removes trailing ASCII
spaces from each line, and emits exactly one final LF. The locked Poppler
grammar permits its generated `<title>` to equal the conversion-owned output
prefix and permits one generated date meta element. The normalizer replaces
that exact title with `document` and removes that exact date element. Any other
timestamp or temporary absolute path is a grammar mismatch. These rules and
deterministic asset names make clean-run HTML hashes comparable without hiding
backend drift.

The normalizer parses only the bounded Poppler HTML grammar produced by the
locked runtime. A grammar mismatch is `MalformedBackendOutput`, not a best-effort
rewrite.

### Native PDF backend

PDF skips LibreOffice and enters the same bounded `pdftohtml` stage. The
backend rejects missing output and unexpected paths. The PDF page count is
read by the required `pdfinfo` executable from the same locked Poppler
installation:

```text
pdfinfo <input.pdf>
```

It uses the same executable discovery, version probe, timeout, log, and output
bounds as the other stages. The parser accepts exactly one ASCII
`Pages: <positive-decimal>` line, rejects duplicates, overflow, zero, or a
missing field, and ignores only other bounded `Key: Value` lines. The parsed
count must equal the number of Poppler page containers. A mismatch is partial
output.

The locked page-container grammar is one opening `div` per page whose double-
quoted `id` is exactly `page<positive-decimal>-div`. IDs must be unique,
strictly sequential from `page1-div`, and closed within the document body.
Substring matches in comments, scripts, style text, or attribute values do not
count. The final sequential count is cross-checked against `pdfinfo`.

For PPT, LibreOffice's PDF page count is reported as `slide-page`; the runtime
does not claim it is the source slide inventory. The acceptance oracle records
the native source slide count and requires exact equality with candidate
slide-page count before scoring.

### Browser WASM

The generic WASM API exposes format detection and a support matrix. PPTX uses
the existing pure-Rust backend. Native-only formats return
`backend-unavailable` with the requested format and runtime. This is an honest
capability boundary, not a successful conversion.

Future browser adapters must pass the same format gate before they are added to
the support matrix.

## Security boundaries

- Strict conversion never fetches external relationships or links.
- Inputs and outputs stay inside one conversion-owned temporary root.
- Native commands receive an explicit argument vector; no shell command string
  is constructed.
- LibreOffice uses a fresh profile per conversion.
- Active content is never surfaced as executable HTML.
- Generated HTML is untrusted active content and retains the existing sandbox
  guidance.
- Password-protected, encrypted, malformed, and resource-exhaustion fixtures
  must produce the typed errors above.
- A release cannot claim the security gate until the native process is run in a
  network-disabled, resource-bounded evaluator sandbox.

`StrictAuto` wraps every native process in a platform isolation launcher:

- macOS uses `/usr/bin/sandbox-exec` with an inline profile that allows local
  LibreOffice IPC but denies outbound remote-IP connections;
- Linux uses `bwrap` with `--unshare-net`, a read-only bind of `/`, and a
  writable bind only for the conversion root;
- Windows requires an `Explicit(IsolationLauncher)` supplied by the
  distribution until a built-in restricted-token/AppContainer launcher is
  implemented.

If the required launcher is absent or its startup probe fails, strict
conversion returns `BackendUnavailable`. `IsolationLauncher` is an executable
plus fixed argument prefix with one typed placeholder for the child argument
vector; it is configured by trusted operators, never from document content.
The evaluator additionally applies host-level CPU and memory limits. Runtime
code enforces input, cumulative output, log, and time bounds on every platform.

## General conversion evaluation contract

The standardized claim is
`96% under the documented general conversion evaluation contract`. The gate is
an acceptance gate, not an `exact` promotion. Its default required reference
profile is `libreoffice-poppler`: supported macOS hosts use
locked LibreOffice and Poppler over the seven frozen format corpora, with PDF
entering Poppler directly. Existing PPTX exactness remains a separate,
stricter PowerPoint-native zero-RGBA-difference contract. A signed
`microsoft-office`/Windows profile is supported as optional evidence and is not
a prerequisite for the default acceptance path.

### Corpus

Each format has three independent tracks:

| Track | Required population |
|---|---|
| Deterministic conformance | exactly 100 distinct positive visual units |
| Blinded generalization | exactly 75 independently sourced valid files |
| Negative/security | exactly 10 format-appropriate hostile or invalid files |

Each deterministic manifest lists files whose complete native unit inventories
sum to exactly 100. Every unit in those files is evaluated; there is no
selection, truncation, or sampling after export. The positive corpus is
stratified across text, layout, images, tables, drawings, charts,
international content, and format-specific edge cases.
Modern and legacy Office formats are evaluated separately. The actual DOC,
XLS, or PPT file is the oracle input; a converted OOXML counterpart may not
substitute for it.

Every deterministic unit has exactly one primary stratum in the manifest.
The required quotas are:

| Format | Required primary strata, summing to 100 |
|---|---|
| DOCX | text/typography 25; sections/headers/footers 20; tables/images/shapes 20; lists/fields/references 15; international 10; mixed/stress 10 |
| DOC | paired legacy coverage 60 across the DOCX strata; independently authored binary-specific coverage 40 |
| XLSX | values/formulas 25; styles/conditional formats 20; print layout 20; charts/images/shapes 15; international formats 10; mixed/stress 10 |
| XLS | paired legacy coverage 60 across the XLSX strata; independently authored BIFF-specific coverage 40 |
| PPTX | text 20; shapes/connectors 20; images/effects 15; tables/charts 15; masters/layouts/groups 15; international 10; visible fallback/edge 5 |
| PPT | paired legacy coverage 60 across the PPTX strata; independently authored binary-specific coverage 40 |
| PDF | text/fonts 20; vector/transparency 20; raster/color-space 15; page geometry 15; forms/annotations/links 10; international 10; mixed/edge 10 |

Criterion 4 is calculated from these primary memberships. Secondary feature
labels are reported but do not change stratum weighting.

The 75-file blind set is fixed before candidate execution, contains at least
five independent producers, and has no duplicate source hash or template
family. All pages, printable sheet pages, or slides are evaluated.
Each blind file score is the arithmetic mean of all its unit scores. The blind
format score is the arithmetic mean of exactly 75 file scores, so a long file
cannot dominate the result.

### Reference profiles and evidence identity

The default required profile is `libreoffice-poppler`:

- DOCX, DOC, XLSX, XLS, PPTX, and PPT use locked LibreOffice PDF export plus
  locked Poppler metadata, rendering, and text extraction on supported macOS or
  Linux hosts.
- PDF uses the same locked Poppler stages directly, without LibreOffice.

Its schema-2 portable lock records the platform, routing-table and
canonicalizer identities, tool/font/browser/runtime hashes, frozen corpus and
evaluator scope, signed executor, network-isolated runtime attestation, and
project revision. Every selected reference and candidate artifact is
SHA-256-bound, and missing, stale, substituted, or tampered evidence remains
`INCOMPLETE` or `FAIL`.

The optional `microsoft-office` profile remains supported for signed Windows
Word, Excel, and PowerPoint oracle evidence. Its schema-1 lock, distinct
verifier, Office provenance, and artifact bindings remain fail-closed when that
profile is selected, but its absence does not block the default portable
profile. Profiles are selected as a complete evidence wave and are not mixed.

A selected Office profile may use:

- DOCX and DOC: pinned Microsoft Word PDF export on Windows;
- XLSX and XLS: pinned Microsoft Excel PDF export on Windows;
- PPTX and PPT: pinned Microsoft PowerPoint slide export on Windows;
- PDF: source PDF rendered by pinned PDF renderers.

The lock is populated only from real evaluator hosts. Until the concrete
values and hashes for a selected profile exist, that profile is `INCOMPLETE`;
placeholder or invented versions are forbidden.

PowerPoint references in the optional Office profile are exported through the
documented `Slide.Export(path, "PNG", 960, 540)` operation. The gate verifies
every PNG IHDR is exactly 960 by 540. Decoding RGB PNG data into an RGBA pixel
buffer is allowed; geometric resize, crop, padding, or alignment is not.

### Metrics

Every visual unit receives:

```text
V = 0.35 * full-image MS-SSIM
  + 0.25 * active-tile SSIM
  + 0.20 * color similarity
  + 0.20 * edge F1

C = 0.70 * text-or-cell similarity
  + 0.30 * object F1

L = 0.70 * matched-box IoU
  + 0.30 * reading-order and baseline similarity

unit score = 0.60 * V + 0.25 * C + 0.15 * L
```

Tests, builds, security, determinism, and performance are hard gates and never
contribute points.

The machine implementation is fixed as follows:

- Candidate and reference images use their native dimensions, linearized sRGB,
  and the manifest page background; no resize or alignment search is allowed.
- Full-image MS-SSIM uses five scales, an 11x11 Gaussian window, sigma 1.5, and
  standard `K1=0.01`, `K2=0.03`.
- Active tiles use a fixed 32x32 grid. A tile is active when either image has
  luminance variance above 16, a Canny edge, or an oracle semantic box. Active
  pixels are all pixels in the union of active tiles.
- Color similarity is
  `100 * (1 - mean(min(deltaE00, 20)) / 20)` over active pixels.
- Edge F1 uses 8-bit grayscale Canny edges after Gaussian sigma 1.0, thresholds
  100 and 200, and one-output-pixel bipartite tolerance.
- Text uses NFC-normalized grapheme edit similarity in oracle reading order.
  Spreadsheet content instead compares ordered
  `(worksheet, cell-coordinate, displayed-value)` tuples.
- Object F1 uses manifest object identity and type. Boxes are matched by
  semantic identity, then minimum-cost assignment on center distance.
- Layout IoU is the mean IoU of matched boxes with unmatched boxes scored zero.
  Reading-order similarity uses normalized Kendall tau; baseline similarity is
  one minus clamped mean baseline error divided by text height.
- The `reading-order and baseline similarity` term is the fixed arithmetic
  mean of reading-order similarity and baseline similarity.
- Each component is scaled to `[0, 100]`, retained to six decimal places, and
  compared to thresholds without pre-comparison rounding.
- Conformance score is the arithmetic mean of exactly 100 unit scores. Blind
  aggregation follows the file-then-format rule above.

Reference semantic inventories for the default required profile come only
from its locked LibreOffice/Poppler reference outputs and documented extraction
contract. Optional Office/native enrichment may add pinned Word, Excel, and
PowerPoint COM/Microsoft Office reading-order captures, MuPDF extraction, or
other locked PDF-renderer evidence for a separately selected profile or exact
promotion; none is a prerequisite for the default general gate. Candidate
inventories come from a pinned Chromium script that walks visible DOM text
nodes, images, links, form controls, SVG graphics, and page containers and
records bounding boxes and text baselines. Spreadsheet reference tuples retain
their cell coordinates; candidate nodes are assigned to tuples by displayed
value, page, and minimum-cost box matching. Duplicate values use global
minimum-cost assignment, never first-match order.

Object identity is `(unit, type, semantic-value-or-content-hash, occurrence)`.
Reading order is the selected profile's locked extraction order and candidate
DOM order after page-container grouping. The default profile uses Poppler
extraction; optional Office/native enrichment may use COM/Microsoft Office or
MuPDF reading-order evidence. Baselines use the selected profile's bound PDF
text geometry and Chromium `Range.getClientRects()` plus computed font metrics.
These optional native readings never become prerequisites for the default
`96% under the documented general conversion evaluation contract` claim.

If a metric family is declared not applicable by the frozen stratum manifest,
both inventories must be empty and that component scores 100. If either side
contains an undeclared item, or applicable evidence is missing, it scores zero.
Weights are never renormalized. Track-level `V`, `C`, and `L` use the same
macro aggregation as the track score: unit mean for conformance, and unit mean
within file followed by the 75-file mean for blind evidence.

The evaluator dependency versions and every algorithm parameter are duplicated
in the SHA-256-bound evaluator lock; drift fails before scoring.

### Pass criteria

A format passes only when all of these hold:

1. Conformance score is at least 96.00.
2. Blind-format score is at least 96.00.
3. Both tracks have `V >= 95.00`, `C >= 98.00`, and `L >= 94.00`.
4. Every deterministic feature stratum is at least 94.00.
5. No visual unit is below 85.00.
6. No blind file is below 90.00.
7. All 75 blind files are acceptable with no critical defect.
8. All 10 negative/security cases pass.
9. Two clean runs produce identical HTML, inventory, and screenshot hashes.
10. Two independent reviewers pass every full-resolution pair.
11. Relevant tests, release builds, diagnostics, and contract checks pass.

For the standardized claim, 96.00 applies only to the conformance and blind
aggregate scores. Structural validity, exact unit/file/security quotas,
review outcomes, determinism, and SHA-256 evidence bindings are hard gates;
they are not averaged into a 96% promise. Textual/content similarity is `C`
(minimum 98.00), layout is `L` (minimum 94.00), and visual similarity is `V`
(minimum 95.00). Two clean runs must produce identical HTML, inventories, and
screenshot hashes. Frozen source bytes and signed hashes bind the evaluator,
corpora, tools, runtimes, and admitted outputs; they establish evidence
identity, not byte-identical output across separate capture environments.
Microsoft Office pixel-accuracy wording is prohibited for this general claim.

The product passes only when all seven format results are `PASS` in the same
evaluation wave.

An acceptable blind file has a score of at least 90.00 and no critical defect.
A critical defect is a crash, blank output, missing/extra/reordered unit, wrong
displayed spreadsheet value, missing text line or cell, missing visible object
occupying at least 0.25% of a unit, clipping or overlap obscuring more than 10%
of a text box, changed link target, active-content execution, external fetch,
or silent unsupported-content loss.

Two reviewers independently inspect every full-resolution pair using the
frozen checklist for missing content, clipping, overlap, wrong order, wrong
color, and unsafe behavior. Both must return `PASS`. A disagreement or a
reference defect fails the wave. For PDF transparency/color-space strata,
MuPDF agreement with a secondary locked renderer is optional native/reference
or exact-promotion evidence. If that enrichment or promotion profile selects
it, the agreement threshold is stored before candidate execution and
invalidation remains fail-closed; its absence never blocks the default
LibreOffice/Poppler general gate.

### Anti-gaming rules

- Corpus manifests, evaluator code, and thresholds are SHA-256-bound before
  candidate execution.
- Candidate conversion runs without network access and cannot read goldens.
- Candidate/reference inventory must match exactly.
- Resizing, cropping, alignment optimization, screenshot substitution, and
  source-hash or filename special cases are prohibited.
- Results are reported per format, generation, stratum, file, and visual unit.
- Failed samples cannot be removed without a new versioned corpus.

## Migration and commit slices

1. Add this design and acceptance contract.
2. Add `document2html-core`, detection tests, generic types, and the PPTX
   adapter.
3. Add `document2html-native` and the PDF backend.
4. Add DOCX and DOC native adapters.
5. Add XLSX and XLS native adapters.
6. Add PPT native adapter.
7. Add generic CLI behavior without removing the `pptx2html` binary.
8. Add Python and WASM generic surfaces while preserving compatibility exports.
9. Add the machine-consumed multi-format acceptance manifest and validators.
10. Update architecture, support, release, and README documentation.

Each slice is independently formatted, linted, tested, manually exercised
through its public surface, and committed before the next slice begins.

## Verification

Minimum local gates:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v
```

Required target matrix:

```text
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
cargo check --target wasm32-unknown-unknown -p document2html-core
cargo check --target wasm32-unknown-unknown -p pptx2html-wasm
wasm-pack build crates/pptx2html-wasm --target web --release
cargo tree -p pptx2html-wasm
```

The dependency-tree gate fails if `document2html-native` appears under
`pptx2html-wasm`.

Manual QA covers each CLI format with a valid input, invalid input, and
`--help`. Native QA records the actual LibreOffice and Poppler versions.

The default general acceptance result requires the complete portable
`libreoffice-poppler` profile and all seven machine-readable format reports at
the same project revision. Optional Office/native evidence may enrich a
separate selected profile or support exact promotion, but it cannot replace the
portable profile or become a prerequisite for the general claim.

## Authoritative references

- ECMA-376 Office Open XML:
  https://ecma-international.org/publications-and-standards/standards/ecma-376/
- Microsoft DOCX extensions:
  https://learn.microsoft.com/en-us/openspecs/office_standards/ms-docx/b839fe1f-e1ca-4fa6-8c26-5954d0abbccd
- Microsoft XLSX extensions:
  https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/2c5dee00-eff2-4b22-92b6-0738acd4475e
- Microsoft PPTX extensions:
  https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/efd8bb2d-d888-4e2e-af25-cad476730c9f
- Microsoft DOC binary format:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-doc/
- Microsoft XLS binary format:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xls/
- Microsoft PPT binary format:
  https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ppt/
- Microsoft PowerPoint `Slide.Export`:
  https://learn.microsoft.com/en-us/office/vba/api/powerpoint.slide.export
- ISO 32000-1 PDF:
  https://www.iso.org/standard/51502.html
- LibreOffice command-line parameters:
  https://help.libreoffice.org/latest/en-US/text/shared/guide/start_parameters.html
