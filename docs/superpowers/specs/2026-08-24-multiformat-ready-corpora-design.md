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
the exact source bytes. The final external snapshot must be assembled
deterministically, validated before publication, and remain outside Git.

This work establishes source-corpus readiness only. It does not perform the
later signed aggregate admission, prove runtime security outcomes, capture
candidate or reference pixels, compute fidelity metrics, or support a 96%
claim.

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

Each execution record contains:

- source ID, format, path, and SHA-256;
- run index `1` or `2`;
- a distinct 64-lowercase-hex workspace nonce;
- the exact tool roles and argument-vector template identity;
- routing-table, tool, and font-environment SHA-256 values;
- process exit code;
- an exact environment-contract object containing the sorted allowlisted key
  names, locale, timezone, font-environment SHA-256, and booleans proving
  isolated home and temporary roots, but no volatile absolute paths;
- bounded stdout and stderr SHA-256 values;
- retained PDF and `pdfinfo.txt` paths and SHA-256 values; and
- the parsed positive unit count.

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
- an ID, path, or digest is duplicated; or
- the destination already exists.

The error identifies the failed format and source ID. There are no retries,
skips, inferred fallback counts, or partial success states.

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

