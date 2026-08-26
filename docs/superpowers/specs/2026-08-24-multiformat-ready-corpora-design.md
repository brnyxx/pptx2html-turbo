# Seven-Format READY Corpus Assembly Design

## Goal

Produce and independently validate one schema-v2 `READY` source manifest for
each required format:

- PPTX
- DOCX
- DOC
- XLSX
- XLS
- PPT
- PDF

Each manifest must bind exactly 100 conformance units, 75 blind files, and 10
security cases from the already frozen source snapshots. Blind `unit_count`
values must come from two clean LibreOffice/Poppler native inventory runs over
the exact source bytes. This inventory is the native observation stage for the
default macOS `libreoffice-poppler` acceptance profile; signed Windows
Office oracle evidence is a separate optional profile. The final external
snapshot must be assembled deterministically, validated before publication, and
remain outside Git.

This work establishes source-corpus readiness only. It does not perform the
later signed aggregate admission, prove runtime security outcomes, capture
candidate or reference pixels, compute fidelity metrics, or support the
claim `96% under the documented general conversion evaluation contract`.

## Existing Inputs

The assembly consumes these frozen source families:

| Track | Formats | Count | Current source |
|---|---|---:|---|
| Modern/PDF conformance | PPTX, DOCX, XLSX, PDF | 400 | Four validated generation snapshots |
| Paired legacy conformance | DOC, XLS, PPT | 180 | Validated legacy-pair snapshot |
| Binary-specific conformance | DOC, XLS, PPT | 120 | Validated independent legacy pool |
| Blind | All seven | 525 | Validated public blind pool |
| Security | All seven | 70 | Validated deterministic security snapshot |
| Paired modern support | DOCX, XLSX, PPTX | 180 | Immutable support bytes in the legacy-pair snapshot |

The contract-derived conformance plan is not preserved as an external input.
It is regenerated from `contract.v1.json`; its canonical SHA-256 must equal
`609762e81c90f4d2185f7078fad699aa1ea65c76b2aa2c48680b7b001e6df94a`,
which is already bound by all five conformance snapshots.

The current blind pool has five producers per format and no digest overlap
with the conformance or security tracks. Its `background` value is the source
catalog token `light`; assembly maps only that exact token to the corpus
contract color `#ffffff`. Unknown background tokens fail closed.

## Status Vocabulary

Three states must remain distinct:

1. `CAPTURED`: the native unit inventory has two matching observations for
   every blind source.
2. `READY`: one per-format schema-v2 source manifest passes
   `validate_corpus_manifest`.
3. `VALIDATED`: the root assembly snapshot contains exactly seven `READY`
   manifests and an independently verified complete tree.

The root assembly does not contain a `READY` marker. That marker remains
reserved for the separately qualified aggregate produced by
`admit_multiformat_corpus`.

## Selected Approach

Use two stages:

1. Capture a reusable, provenance-bound native unit inventory.
2. Assemble the seven manifests without invoking external converters.

This separation keeps slow native execution out of deterministic publication,
makes unit-count disagreements explicit, allows assembly tests to use injected
inventory fixtures, and prevents a manifest rebuild from silently rerunning a
different LibreOffice or font environment.

### Rejected: structural unit inference

Counting OOXML slides, sheets, or PDF pages directly is insufficient. DOC,
XLS, and PPT do not expose portable page counts, and spreadsheet sheet count
is not the paged unit count used by the reference routing contract.

### Rejected: conversion inside the assembler

A monolithic assembler would couple long-running native conversion to source
copying and publication. It would be difficult to retry, independently
validate, or prove that the same inventory was consumed by two assemblies.

## External Artifact Layout

Generated artifacts remain outside Git:

```text
artifacts/
├── multiformat-font-bundle/
│   ├── font-bundle.json
│   └── fonts/
│       └── <digest-prefixed font files>
├── multiformat-native-units/
│   ├── native-unit-inventory.json
│   └── observations/
│       └── <format>/<source-id>/<run-1|run-2>/
│           ├── execution.json
│           ├── reference.pdf
│           └── pdfinfo.txt
└── multiformat-ready-corpora/
    ├── assembly-manifest.json
    ├── conformance-plan.json
    ├── native-unit-inventory.json
    └── corpora/
        ├── pptx/
        │   ├── manifest.json
        │   └── sources/
        ├── docx/
        ├── doc/
        ├── xlsx/
        ├── xls/
        ├── ppt/
        └── pdf/
```

The READY corpus snapshot contains:

- 1,295 primary track source files:
  1,295 = 7 x (100 conformance + 75 blind + 10 security);
- 180 paired modern support files;
- seven per-format manifests;
- one assembly manifest;
- one regenerated conformance plan; and
- one immutable copy of the native unit inventory.

No source is linked from another snapshot. Every input byte is copied into the
staged corpus tree and rehashed before publication.

Every new JSON artifact uses a closed schema with exact field sets, schema
version `1`, UTF-8, JCS canonical bytes, and one trailing LF. IDs use the
existing corpus identifier grammar. Relative paths use forward-slash POSIX
syntax, stay below the artifact root, and contain no empty, current-directory,
or parent-directory component.

## Deterministic Font Bundle

The native inventory requires an explicit schema-v1 font bundle accepted by
`prepare_font_environment`.

A font-bundle materializer receives one or more explicit font directories. It:

1. Resolves each root strictly, rejects a symlinked root, and walks without
   following directory or file symlinks.
2. Accepts only contained regular `.ttf` and `.otf` files whose `st_nlink` is
   exactly one.
3. Rejects duplicate `(st_dev, st_ino)` identities, hard links, duplicate
   SHA-256 digests, special files, and traversal outside an input root. The
   identity and digest sets span the union of every supplied font root.
4. Copies files under deterministic digest-prefixed names.
5. Writes canonical JCS `font-bundle.json`.
6. Reopens the output with `validate_font_bundle`.
7. Publishes the complete snapshot with no partial destination on failure.

The real macOS capture uses the fonts distributed with the locked LibreOffice
application. Copying the font bytes into the external artifact makes the same
bundle usable on Linux and prevents ambient user fonts from becoming corpus
identity.

Independent font validation requires the canonical manifest path and the
snapshot root. It repeats the exact regular-file, containment, link-count,
inode, digest, and file-set checks before calling `validate_font_bundle`.

## Native Unit Inventory

### Scope

The inventory contains exactly 525 blind records: 75 for each required format.
Conformance units are already fixed by the contract-derived plan, and each
security case contributes one source-level security unit. They are not
recaptured here.

### Runtime

For DOC, DOCX, XLS, XLSX, PPT, and PPTX, each observation:

1. Copies the exact blind source into an isolated workspace.
2. Creates a unique LibreOffice user profile and clean home directory.
3. Applies the locked font configuration.
4. Runs the contract routing command to export PDF.
5. Runs locked `pdfinfo` over the exported PDF.
6. Parses one positive page count from bounded output.

For PDF, each observation runs locked `pdfinfo` directly over the source.

Every source is observed twice in clean workspaces. Counts must be identical
and positive. The two observation directories retain the exact generated PDF,
bounded `pdfinfo` stdout, and a canonical execution record. For PDF inputs,
`reference.pdf` is an immutable copy of the source. LibreOffice PDF bytes do
not need to match across runs; each retained PDF is independently bound and
recounted. The manifest records the source identity, two observation bindings,
the accepted `unit_count`, and the exact runtime identities.

### Closed execution schema

Every `execution.json` uses UTF-8 JCS bytes plus one LF and has exactly these
root fields:

```text
schema_version, source, run, workspace_nonce, routing_sha256,
tools, processes, environment, evidence, unit_count
```

Their exact nested field sets are:

- `source`: `id`, `format`, `path`, and `sha256`;
- each tool: `name`, `sha256`, and `version`;
- each process: `role`, `arguments`, `timeout_seconds`, and `exit_code`;
- `evidence`: `reference_pdf` and `pdfinfo`; and
- each evidence binding: `path` and `sha256`.

`processes` is a JSON array in the listed order. `arguments` and
`environment.keys` are JSON arrays of strings. `schema_version`, `run`,
`unit_count`, `timeout_seconds`, and `exit_code` are JSON integers. Isolation
fields are JSON booleans rather than numeric substitutes. All other scalar
schema values are JSON strings.

`schema_version` is `1`. `run` is `1` or `2`; `workspace_nonce` is a
64-lowercase-hex value supplied by the capture orchestrator; `unit_count` is
positive. Every SHA-256 is 64 lowercase hexadecimal characters. Tool names and
versions are non-empty single-line strings. Every accepted process has exit
code `0` and a positive bounded timeout.

The six Office formats have exactly the `libreoffice` and `pdfinfo` tool keys.
Their process roles and order are:

```text
libreoffice_version, pdfinfo_version, libreoffice, poppler_metadata
```

PDF has exactly the `pdfinfo` tool key and these process roles:

```text
pdfinfo_version, poppler_metadata
```

Version arguments are exactly `["--version"]` and `["-v"]`. The
`libreoffice` and `poppler_metadata` process arguments are the exact
unrendered argument templates from the locked routing table. No rendered
workspace path is persisted.

Version-command `timeout_seconds` is the literal integer `120`. Routed process
timeouts equal the positive integer `timeout_seconds` from the validated
routing table. Every accepted `exit_code` is the integer `0`.

The runtime selects only `libreoffice` plus `poppler_metadata` from an Office
route, and only `poppler_metadata` from a PDF route. It never invokes or
records the routing table's `poppler_render` or `poppler_text` commands for
native unit inventory capture.

The common `environment` field set is:

```text
keys, locale, lang, lc_all, timezone,
home_isolated, temporary_root_isolated, profile_isolated
```

Office adds exactly `font_environment_sha256`; PDF omits that field.
`profile_isolated` is `true` for Office and `false` for PDF. The other two
isolation booleans are always `true`.

`source.path` is relative to the validated public-pool snapshot.
`evidence.reference_pdf.path` and `evidence.pdfinfo.path` are relative to the
inventory root and name the two retained files in the observation directory.
For observation root `observations/{format}/{id}/run-{run}`, those paths are
exactly `{observation-root}/reference.pdf` and
`{observation-root}/pdfinfo.txt`.
PDF observations neither resolve nor record LibreOffice or font identities.
The batch-level inventory still binds them because the same inventory contains
the six Office formats.

For every observation, `source`, `run`, `workspace_nonce`, `unit_count`, and
both evidence bindings in `execution.json` must exactly equal the corresponding
parent inventory source/observation fields. No repeated field may disagree.

Converter/version stdout and stderr are bounded temporary capture mechanics.
They are discarded and are not persisted as hashes or independent evidence.
The retained `pdfinfo.txt` is the bounded raw metadata stdout and is fully
bound by the evidence object. Tool versions are independently re-read during
inventory validation.

The validator requires exactly run indexes `{1, 2}`, distinct nonces and
evidence paths, complete artifact bindings, and matching accepted counts. This
is auditable local execution evidence, not a substitute for the later signed
portable-reference receipt.

### Runtime identity

`native-unit-inventory.json` binds:

- schema version and `CAPTURED` status;
- public-pool manifest SHA-256;
- reference-routing table SHA-256;
- LibreOffice executable SHA-256 and version;
- `pdfinfo` executable SHA-256 and version;
- font-bundle manifest SHA-256;
- validated font-environment SHA-256;
- OS, architecture, locale, timezone, and worker count;
- 525 source records ordered by format and source ID.

### Closed inventory schema

`native-unit-inventory.json` has the following exact field sets. No object may
contain an additional field:

- root: `schema_version`, `status`, `contract_sha256`, `public_pool`,
  `routing`, `tools`, `font`, `runtime`, and `sources`;
- `public_pool`: `config_sha256` and `manifest_sha256`;
- `routing`: `sha256`;
- `tools`: `libreoffice` and `pdfinfo`;
- each tool: `name`, `sha256`, and `version`;
- `font`: `manifest_sha256` and `environment_sha256`;
- `runtime`: `os`, `architecture`, `locale`, `lang`, `lc_all`, `timezone`,
  `worker_count`, and `environment_keys`;
- `environment_keys`: `office` and `pdf`;
- each source: `id`, `format`, `path`, `sha256`, `unit_count`, and
  `observations`;
- each observation: `run`, `workspace_nonce`, `path`, `execution`,
  `reference_pdf`, and `pdfinfo`; and
- each execution/evidence binding: `path` and `sha256`.

The root has `schema_version` `1` and status `CAPTURED`. Every SHA-256 is 64
lowercase hexadecimal characters. Tool versions are non-empty single-line
strings. `worker_count` is in `1..8`.

`sources` is sorted by `(format, id)`. Every source has exactly two
observations sorted by run, with run indexes `{1, 2}`. Observation nonces are
preallocated by the capture orchestrator before worker submission and passed
into the runtime; the runtime does not generate hidden randomness. Nonces are
globally unique across all 1,050 observations and are 64-lowercase-hex values.

Source identity is keyed by `(format, id)`, not by bare ID. All 525 keys are
unique. Source paths and source SHA-256 values are each globally unique across
all formats. All 1,050 observation paths are globally unique. The 3,150
execution/evidence binding paths are globally unique and their set equals the
physical observation file set exactly.

Source paths are safe POSIX paths relative to the validated public-pool
snapshot. Observation and binding paths are safe POSIX paths relative to the
inventory root. An observation path is exactly
`observations/{format}/{id}/run-{run}`. Its three bindings name
`execution.json`, `reference.pdf`, and `pdfinfo.txt` beneath that path.

Native inventory capture for this acceptance profile supports only macOS and
Linux. Every other platform fails with a typed unsupported-platform error
before a tool is invoked. The exact sorted environment key sets are:

```text
office = [FONTCONFIG_FILE, HOME, LANG, LC_ALL, PATH, TMPDIR, TZ]
pdf    = [HOME, LANG, LC_ALL, PATH, TMPDIR, TZ]
```

`locale` is `en-US`, `lang` and `lc_all` are `en_US.UTF-8`, and `timezone` is
`UTC`. `PATH` comes only from the clean subprocess environment. Office
observations require `FONTCONFIG_FILE` and the locked font environment. PDF
observations omit both and must not require a LibreOffice executable or font
bundle at observation time.

`runtime.os` is exactly `macos` or `linux`. `runtime.architecture` is normalized
to `arm64` or `x86_64`; every other architecture fails typed before capture.

The shared failure enum contains
`NativeUnitFailure.UNSUPPORTED_PLATFORM = "unsupported-platform"`.
`NativeUnitError` has exact fields `failure`, `document_format`, `source_id`,
and `detail`; the middle two are optional only for non-source-scoped failures.
Capture preflight and independent validation raise
`NativeUnitError(UNSUPPORTED_PLATFORM, None, None, detail)` before resolving,
hashing, versioning, or invoking a tool on an unsupported OS or architecture.

The acceptance contract does not separately pin semantic version numbers.
Each supplied tool's trusted identity is the pair of its exact executable
SHA-256 and normalized first non-empty version-output line. Normalization
strips surrounding ASCII whitespace and rejects embedded CR, LF, NUL, or an
empty result. Capture and validation invoke exactly LibreOffice `--version`
and `pdfinfo -v` and require both the hash and normalized line to match the
inventory. Font-snapshot identity is independent and need not come from the
same installation root as the LibreOffice executable.

Task 4 exposes one typed capture boundary:

```python
@dataclass(frozen=True, slots=True)
class NativeUnitCaptureInputs:
    contract: Path
    public_config: Path
    public_pool_manifest: Path
    routing: Path
    font_manifest: Path
    libreoffice: Path
    pdfinfo: Path
    output_dir: Path
    workers: int

NonceFactory = Callable[[], str]

def capture_native_unit_inventory(
    inputs: NativeUnitCaptureInputs,
    *,
    runner: NativeProcessRunner = run_native_process,
    nonce_factory: NonceFactory = generate_native_nonce,
) -> NativeUnitInventorySummary: ...
```

`font_manifest` is the canonical font-bundle manifest path. The runtime derives
the bundle root from its parent; capture has no separate font-root field.
`output_dir` is the no-replace publication destination and is part of the
capture inputs. The orchestrator calls `nonce_factory` exactly 1,050 times in
sorted `(format.value, source_id, run)` order before submitting any worker.
The keyword-only factory and runner are the only public capture test seams.

Unsupported platform or architecture raises
`NativeUnitError(UNSUPPORTED_PLATFORM, None, None, detail)` before any input
tool is resolved, hashed, versioned, or invoked. Capture validates the complete
staged inventory through `validate_native_unit_inventory` before publication
and returns that independent validator's `NativeUnitInventorySummary`. There
is no alternate overload and no public per-source summary.

The validator returns a separate aggregate value:

```python
@dataclass(frozen=True, slots=True)
class NativeUnitInventorySummary:
    files: int
    sources: int
    observations: int
    total_units: int
    manifest_sha256: str
```

`files` is exactly `3,151`, `sources` is `525`, and `observations` is `1,050`.
`total_units` is the sum of the accepted positive source `unit_count` values.
There is no public `NativeUnitSummary`. Task 3 returns one
`NativeObservation` per call. Task 4 compares exactly two observations and
constructs `NativeUnitCount`; aggregate validation returns
`NativeUnitInventorySummary`.

The manifest digest is SHA-256 over the exact canonical
`native-unit-inventory.json` bytes including the trailing LF. Task 4 exposes:

```python
@dataclass(frozen=True, slots=True)
class NativeUnitCount:
    document_format: DocumentFormat
    source_id: str
    relative_path: str
    source_sha256: str
    unit_count: int

@dataclass(frozen=True, slots=True)
class NativeUnitInventory:
    summary: NativeUnitInventorySummary
    sources: tuple[NativeUnitCount, ...]

def load_native_unit_inventory(
    inputs: NativeUnitValidationInputs,
) -> NativeUnitInventory: ...
```

`NativeUnitValidationInputs` has exactly these `Path` fields:

```text
contract, public_config, public_pool_manifest, routing,
font_manifest, libreoffice, pdfinfo, inventory_root
```

The source tuple is sorted by `(document_format.value, source_id)`.
`validate_native_unit_inventory` delegates to this loader and returns only its
summary.

Public-pool validation exposes a matching typed source boundary:

```python
@dataclass(frozen=True, slots=True)
class ValidatedPublicPoolSource:
    document_format: DocumentFormat
    source_id: str
    relative_path: str
    source_sha256: str

def load_validated_public_pool_sources(
    config_path: Path,
    manifest_path: Path,
) -> tuple[ValidatedPublicPoolSource, ...]: ...
```

The loader performs the complete existing validation, enforces unique
`(format, id)` keys plus globally unique paths and digests, and returns sources
in that key order. The existing `validate_public_pool` delegates to the loader
and discards the tuple.

Every public-pool, native-inventory, and Task 5 join record uses
`evaluate.multiformat_corpus_types.DocumentFormat`. The separate routing enum
is converted by `.value` only at the routing boundary. Sorting and joins use
`(document_format.value, source_id)`.

Locale is `en-US`, timezone is `UTC`, and the subprocess environment is
allowlisted. Native conversion uses bounded output, a per-command timeout, and
unique profiles. Bounded parallel workers may be used because workspaces and
profiles share no writable state. The configured worker count is recorded but
does not alter manifest ordering.

### Failure behavior

The capture fails without publication when:

- any input source or source manifest changes;
- a tool, routing table, or font binding is malformed;
- native conversion or `pdfinfo` fails;
- output exceeds its bound;
- a page count is zero or missing;
- the two clean runs disagree;
- a `(format, id)` key, source path, source digest, nonce, observation path, or
  binding path is duplicated; or
- the destination already exists.

Every source-scoped error identifies the failed format and source ID.
Unsupported-platform preflight is the sole non-source-scoped failure and
carries `None` for both fields. There are no retries, skips, inferred fallback
counts, or partial success states.

Independent inventory validation receives these trusted inputs explicitly:

```text
contract
public-pool source catalog
public-pool manifest
reference-routing table
font-bundle manifest
LibreOffice executable
pdfinfo executable
inventory root
```

It revalidates the public pool, rehashes every source and retained observation
artifact, validates the font bundle, rehashes and re-versions both tools,
revalidates routing identity and runtime fields, and reruns locked `pdfinfo`
over every retained `reference.pdf`. Merely recording a global identity is
insufficient; any supplied input or runtime identity drift fails validation.
Each execution record's environment-contract object must equal the
inventory-level runtime values and the fixed allowlist.

Task 5 joins the typed public-pool and native-inventory source tuples only by
`(document_format, source_id)` and additionally requires exact relative-path
and SHA-256 equality. Missing, extra, or duplicate keys fail before source
copying.

## Per-Format Manifest Assembly

### Conformance records

The assembler regenerates and validates the canonical conformance plan.

For PPTX, DOCX, XLSX, and PDF, each frozen generation record becomes one
conformance source with one unit. The plan supplies:

- unit ID;
- primary stratum;
- paired stratum;
- feature seed identity; and
- global plan ordinal.

The source-relative unit ordinal is `1`, as required by the existing corpus
validator.

For DOC, XLS, and PPT:

- the first 60 plan cases use the paired legacy snapshot and a uniquely named
  paired modern support copy;
- the final 40 cases use the independent binary pool and preserve its
  `producer`, `source_uri`, and `independently_authored` provenance.

Support IDs and basenames include the legacy owner format so they cannot
collide with the modern format's primary conformance records during later
aggregate admission. A support source appears only as the owning legacy
conformance item's closed-schema `paired_source` object; it is not a fourth
top-level track and contributes no additional conformance unit.

`paired_source` uses the already-supported exact schema
`{"id", "path", "sha256"}`. Its format is implied by the contract's
`paired_format` for the owning legacy format; no `format`, provenance, or
relationship field is added. During input validation, the upstream support ID
and digest must match the plan-selected modern conformance case. During
assembly, the final support ID is derived as
`{owner_format}-support-{modern_case_id}` and its exact filename is
`{support_id}.{support_format}`. This prevents later aggregate admission from
colliding with the modern format's primary `(format, id)` identity while
satisfying format-extension validation. The root assembly's support relation
record has exactly `owner_format`,
`owner_source_id`, `support_format`, `modern_case_id`, `support_id`, `path`,
and `sha256`.

Digest uniqueness is mandatory across conformance, blind, and security tracks
within each per-format manifest. Paired support bytes intentionally equal the
corresponding modern conformance bytes across different manifests; the root
assembly records that explicit support relation. No other cross-manifest
digest reuse is accepted.

### Blind records

Each public-pool source byte stream is copied unchanged. Its assembled blind
record preserves:

- ID;
- SHA-256;
- canonical producer;
- source URI;
- template family;
- applicable metrics; and
- normalized background.

Its `unit_count` comes only from the matching validated native inventory
record. Source ID, format, and SHA-256 must all agree.

The public-pool token `light` is replaced by `#ffffff` in the closed schema-v2
blind record. The original token is not added as an extra per-format field;
its provenance remains bound by the public-pool manifest SHA-256 in
`assembly-manifest.json`. Any token other than `light` fails closed.

### Security records

Each deterministic security source preserves:

- ID;
- SHA-256;
- case family; and
- expected outcome.

The existing corpus validator re-derives the declared semantic family from the
copied bytes.

### Publication

Assembly writes a sibling staging tree, constructs all seven manifests, and
runs `validate_corpus_manifest` on every staged manifest. It then writes a
candidate final canonical `assembly-manifest.json` with status `VALIDATED`
that binds:

- all upstream manifest SHA-256 values;
- contract and plan identities;
- native inventory identity;
- seven per-format manifest identities;
- exact per-format track counts;
- exact physical file count; and
- a canonical relative-path/tree digest.

`VALIDATED` is not trusted merely because the assembler wrote it. The
independent validator runs over the complete immutable candidate bytes,
including the candidate final manifest, and publication occurs only if that
call succeeds. There is no mutation after validation and before rename.

An independent assembly validator repeats exact-tree, regular-file, link,
hash, plan, inventory, and per-format corpus validation before one no-marker
atomic rename publishes the destination.

The tree digest excludes only `assembly-manifest.json`. Lock files and staging
paths are outside the candidate root and can never be entries. The validator
builds a list of every other regular file as:

```json
{"path":"UTF-8 POSIX relative path","sha256":"64 lowercase hex","size":123}
```

It sorts entries by UTF-8 path bytes, JCS-canonicalizes
`{"schema_version":1,"files":[...]}`, and hashes those canonical bytes with
SHA-256. The manifest binds this digest, the entry count, and the sum of file
sizes. This removes self-reference and gives independent implementations one
exact framing.

The publisher uses a cooperative sibling lock and inode-owned staging cleanup.
It refuses destination replacement and preserves a primary error if cleanup
also fails. The existing security snapshot publisher is generalized behind a
format-neutral no-marker publisher, while the security API remains a
compatibility wrapper with unchanged behavior and tests.

## Independent Validation

The assembly validator:

1. Requires the canonical basenames and directory layout.
2. Rejects symlinks, hard links, special files, extras, and missing files.
3. Requires canonical JCS bytes for all new root manifests.
4. Rehashes the copied plan, inventory, seven manifests, and every source.
5. Validates the copied plan against the contract.
6. Validates the copied native inventory with explicit contract, catalog,
   pool, routing, font, and executable inputs.
7. Runs `validate_corpus_manifest` for all seven formats.
8. For every support relation, resolves the owning legacy source and selected
   modern plan case, then requires exact `modern_case_id`, derived
   `support_id`, `support_format`, extension-bearing path, digest, and matching
   closed-schema `paired_source` values.
9. Recomputes track counts, source counts, support counts, and tree digest.
10. Requires root status `VALIDATED`.
11. Rejects a root `READY` marker to preserve the aggregate-admission boundary.

Validation does not regenerate source documents or rerun LibreOffice.

## Public CLIs

Four machine-facing entry points are added:

```text
python -m evaluate.generate_multiformat_font_bundle
python -m evaluate.capture_multiformat_native_units
python -m evaluate.assemble_multiformat_ready_corpora
python -m evaluate.validate_multiformat_ready_corpora
```

Success and domain failures use compact canonical ASCII JSON. Exit codes are:

- `0`: success;
- `1`: domain or validation failure; and
- `2`: argument usage error.

No CLI emits a traceback for a typed boundary failure.

The native capture and validator use the established version invocations:
LibreOffice `--version` and Poppler `pdfinfo -v`. Each command runs in its own
process group with a bounded timeout; timeout termination kills the complete
group. Captured stdout and stderr are each limited to 1 MiB. The child
environment starts from `clean_subprocess_environment` and adds only the
isolated `HOME`, `TMPDIR`, `LANG`, `LC_ALL`, `TZ`, font configuration, and
LibreOffice user-profile URI required by the routing contract.

## Tests

Tests are written before production changes and cover:

1. Exact deterministic font-bundle materialization and independent validation.
2. Font symlink, hard-link, duplicate, mutation, extra-file, and existing
   destination rejection.
3. Exact 7 x 75 native inventory records with two matching observations.
4. Count disagreement, conversion failure, malformed `pdfinfo`, tool drift,
   source drift, and partial-publication rejection.
5. Stable inventory bytes regardless of worker completion order.
6. Exact modern, legacy-paired, legacy-binary, blind, security, and support
   source mapping.
7. Seven independently passing schema-v2 READY manifests.
8. Plan, upstream manifest, inventory, source, hash, provenance, quota,
   background, and unit-count tampering.
9. Cross-track source identity uniqueness and support basename isolation.
10. Assembly symlink, hard-link, special-file, extra-file, staging
    substitution, and destination-race rejection.
11. Root `VALIDATED` status with no aggregate `READY` marker.
12. CLI argument binding and stable exit behavior.
13. Evaluator allowlist coverage for every new production and test module.

Tests contain no sleeps or timing-based polling. Native process tests use an
injected runner; the acceptance run uses real LibreOffice and Poppler.

The inventory capture itself contains distinct execution nonces and therefore
is not claimed byte-identical across separate capture waves. Determinism means
both observations in one accepted wave agree on every unit count. The
assembler is deterministic: two assemblies consuming the same accepted
inventory and frozen inputs must be byte-identical, with no timestamps,
absolute paths, hostnames, or other volatile fields in assembled artifacts.

## Real Acceptance

The task is complete only after:

1. The exact LibreOffice-distributed font bundle is externally materialized
   and independently validated.
2. All 525 blind sources complete two clean native unit observations.
3. No source is skipped or assigned an inferred count.
4. The native inventory is independently validated.
5. The corpus assembler publishes all seven source-level READY manifests.
6. Each manifest independently passes `multiformat_corpus`.
7. The root assembly validator reports `VALIDATED`.
8. A second clean assembly from the same frozen inputs is byte-identical.
9. Focused, full multiformat, static, LSP, and Rust workspace gates pass.
10. Independent goal, QA, code, security, and context reviews pass.
11. Verified atomic commits are cherry-picked to `main`.
12. Main repeats the relevant artifact and quality gates.

Generated font, inventory, and corpus artifacts are not committed or pushed.

