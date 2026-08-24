# Deterministic Multi-Format Security Sources

## Context

The seven-format acceptance contract requires ten security cases for each of
PPTX, DOCX, DOC, XLSX, XLS, PPT, and PDF. The repository already has strict
semantic detectors for every contract family and deterministic writers under
`evaluate/tests/`, but it does not have a production materializer or a frozen
70-source snapshot.

This design promotes the existing deterministic writers into evaluator
production modules and materializes an external snapshot under `artifacts/`.
The snapshot is input to later corpus assembly. It does not make a corpus
`READY`, prove runtime security outcomes, or support a 96% claim.

## Decision

Generate the security sources as an external, reproducible snapshot:

```text
artifacts/multiformat-security-sources/
├── security-sources.json
└── sources/
    ├── doc/{family}.doc
    ├── docx/{family}.docx
    ├── pdf/{family}.pdf
    ├── ppt/{family}.ppt
    ├── pptx/{family}.pptx
    ├── xls/{family}.xls
    └── xlsx/{family}.xlsx
```

Repository commits contain the generator, validator, tests, and workflow
documentation. They do not contain the generated binary files.

### Alternatives rejected

1. **Commit all 70 binary fixtures.** This makes checkout and review heavier,
   duplicates generated evidence, and does not prove that regeneration is
   deterministic.
2. **Commit both generator and binaries.** This adds a second source of truth
   without helping later corpus admission, which already binds admitted source
   bytes and hashes.
3. **Import writers directly from `evaluate.tests`.** This is the smallest
   patch, but it makes production tooling depend on a test package and leaves
   the public evaluator boundary unclear.

## Components

### Production source writers

Move the deterministic positive-source and OOXML, CFBF, and PDF security
writers from `evaluate/tests/` into focused modules under `evaluate/`. Update
tests to consume those production writers so the tested bytes and materialized
bytes use one implementation.

Each writer accepts a destination path, document format where applicable, and
one typed family. Fixed ZIP timestamps, stable archive order, stable PDF object
order, and fixed CFBF layout preserve byte determinism.

### Snapshot materializer

`evaluate/generate_multiformat_security_sources.py` is the CLI entry point. A
small orchestration module:

1. Requires `--contract PATH` and `--output-dir PATH`. The production command
   passes `evaluate/multiformat/contract.v1.json`; there is no implicit default.
2. Captures the exact contract bytes once, rejects duplicate JSON object keys,
   and requires schema version `1`.
3. Requires `required_formats` to contain each supported format exactly once,
   `corpus.security_cases` to equal `10`, and
   `security_case_outcomes` to be an object with exactly one ten-member object
   for each required format. Family object keys are unique by strict JSON
   parsing; repeated outcome values are allowed.
4. Derives families only from those per-format object keys, validates the ASCII
   lowercase grammar `[a-z0-9][a-z0-9._-]{0,127}`, parses every outcome through
   the closed `SecurityOutcome` enum, and iterates formats and families in
   lexicographic order.
5. Writes each source into a sibling staging directory.
6. Writes one canonical JSON manifest.
7. Runs the independent snapshot validator.
8. Re-hashes the contract path and requires it to match the captured bytes.
9. Publishes only when every check passes.

Formats are not required to share family keys: OOXML, CFBF, and PDF have
different closed family vocabularies. Exactly ten contract keys per format
produce ten records per format and 70 total records. Global ID, path, and digest
uniqueness does not depend on equal family names.

The output parent is created, resolved to its real directory, and used for both
staging and destination so the final rename is same-filesystem. Destination
existence uses `lexists`, rejecting files, directories, and symlinks. Staging
uses an exclusive random `mkdtemp`; its random name is never serialized.

The lock path is exactly
`output.parent / f".{output.name}.security-sources.lock"`. The publisher opens
it with `O_CREAT | O_EXCL | O_NOFOLLOW` before staging and records its descriptor
device/inode. Any existing lock, including a stale one, is a typed contention
failure; the tool never steals it. On every exit it closes the descriptor and
unlinks the lock only when `lstat` still matches the recorded identity.

The lock prevents two cooperating materializers from racing. The publisher
records the staging device/inode and checks destination absence immediately
before `os.rename`. A destination present at either check is a typed
destination-exists failure. Any rename error is a typed publication failure.
All such failures remove only staging whose `lstat` still matches its recorded
identity and release the owned lock. Under the cooperative threat model,
`os.rename` cannot replace another publisher's output.

An adversarial local process that ignores the lock and mutates filesystem names
concurrently is outside this local generator's threat model. This design does
not claim no-replace publication or hostile-filesystem TOCTOU resistance.
Later signed corpus admission owns descriptor-stable hashing and hostile
replacement protection. Crash durability beyond the namespace rename is also
out of scope.

### Snapshot manifest

`security-sources.json` has:

- `schema_version`: `1`
- `status`: `GENERATED`
- `contract_sha256`
- `formats`: an object keyed by all seven canonical format names

Each format entry has `expected_count: 10` and ten source records:

- `id`: `security-{format}-{family}`
- `path`
- `sha256`
- `case_family`
- `expected_outcome`

No timestamp, hostname, random identifier, or absolute path is serialized.
Records are ordered by family name. The manifest is UTF-8 encoded from
`json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True)`, followed by
exactly one LF. Unknown fields at every manifest object level are rejected.

Every record path is exactly
`sources/{format}/{family}.{extension}` with ASCII lowercase POSIX components.
For all seven formats, `extension == format`. No escaping, alternate separator,
uppercase alias, URL encoding, or normalization is accepted.

### Writer API

The only public writer API is:

```python
write_security_source(
    path: Path,
    document_format: DocumentFormat,
    family: str,
) -> None
```

The caller creates the parent directory. The destination must not exist; the
writer opens it exclusively and never overwrites. The family must satisfy the
identifier grammar and the selected format's closed writer enum. Success
returns `None`. A typed writer error preserves an invalid format/family or I/O
cause. A partial file is removed only when its device and inode still match the
file created by that invocation. Snapshot-level failure removes owned staging.

### Snapshot validator

The independent validator requires `--contract PATH --manifest PATH`, captures
and strictly parses the contract bytes itself, derives all 70 expected
`(format, family, outcome, id, path)` tuples, and treats every manifest field as
untrusted. It rejects:

- contract or manifest schema drift;
- missing or extra formats, families, outcomes, files, or fields;
- path traversal, symlinks, noncanonical paths, or extension drift;
- source hash mismatch or duplicate source bytes;
- invalid safe-convert source containers;
- a fixture that proves no family, multiple families, or the wrong family;
- pre-existing or extra files in the snapshot.

The manifest basename must be exactly `security-sources.json`; alternate names
are rejected. The snapshot root is exactly `manifest.parent`, and every record
path resolves relative to that root.

The snapshot root and all expected directories must be real directories, not
symlinks. The manifest and sources must be regular files with link count one;
special files and hard links are rejected. Relative POSIX paths must exactly
match the contract-derived paths, and the recursively enumerated file set must
equal the manifest plus 70 sources. This also rejects case aliases on
case-insensitive filesystems because no two expected paths case-fold alike.

IDs, paths, and SHA-256 digests are each globally unique across all 70 records.
Family names are unique within a format; expected outcomes need not be unique.

For every source, `validate_security_fixture` must observe exactly
`{case_family}`. Empty, wrong, or multiple detector results fail. The manifest
outcome must equal the contract enum. A `safe-convert` source additionally
passes `validate_source(..., require_valid_format=True)`; a `reject` source is
accepted only through its exact hostile-family proof.

The generator re-hashes the contract path immediately before rename. A
replacement or mutation that changes its bytes fails and removes owned staging.
The validator uses one captured contract byte string for its expected-set
derivation and digest comparison.

Generation and validation run in a trusted local worktree without concurrent
hostile filesystem mutation. Within that boundary, each source is read once
for the shared size, digest, and uniqueness pass. Semantic and safe-container
validators may reopen the same generator-owned staging path. Hostile concurrent
substitution is deferred to signed corpus admission rather than claimed here.

After strict parsing, the validator reserializes the manifest with the exact
canonical JSON algorithm and rejects unless those bytes, including the single
terminal LF, equal the captured raw manifest bytes.

### CLI contracts

`evaluate/generate_multiformat_security_sources.py` and
`evaluate/validate_multiformat_security_sources.py` are the generator and
validator entry points. `SecuritySnapshotError` is their typed boundary. Both
return `0` on success, `1` for a domain or I/O failure, and leave argparse usage
errors at `2`. Failure writes one compact, `sort_keys=True`, ASCII JSON line to
stderr:

```json
{"error":"security-snapshot","message":"..."}
```

Success writes one canonical compact JSON line to stdout:

```json
{"counts":{"doc":10,"docx":10,"pdf":10,"ppt":10,"pptx":10,"xls":10,"xlsx":10},"files":71,"manifest_sha256":"<64 lowercase hex>","schema_version":1,"status":"GENERATED"}
```

No progress lines are written to stdout. Each JSON line ends with one LF.
The validator computes `files`, counts, status, and manifest digest from the
validated snapshot and emits the same success schema as the generator.

## Module Boundaries

Every Python production and test module remains below 250 nonblank,
non-comment lines, matching the repository no-excuse counter:

- `multiformat_source_fixture.py`: deterministic positive containers
- `multiformat_security_source_ooxml.py`: OOXML writers
- `multiformat_security_source_cfb.py`: CFBF writers
- `multiformat_security_source_pdf.py`: PDF writers
- `multiformat_security_source.py`: format dispatcher
- `multiformat_security_snapshot.py`: generation orchestration
- `multiformat_security_snapshot_validation.py`: independent validation
- `generate_multiformat_security_sources.py`: CLI adapter
- separate generator, validator, and CLI test modules

Existing tests are updated to import the promoted production writers. No
production module imports `evaluate.tests`.

## Data Flow

```text
contract.v1.json
      |
      v
strict contract rules
      |
      v
deterministic container writers
      |
      v
sibling staging tree
      |
      v
hash + format + semantic + exact-set validation
      |
      v
single rename -> GENERATED snapshot
```

Later READY-manifest assembly copies or links these exact frozen bytes and uses
their SHA-256 values. Runtime candidate capture separately proves reject,
safe-convert, network-isolation, active-content, and resource-bound outcomes.

## Error Handling

The generator and validator expose one typed security-snapshot error at their
boundary and preserve the originating exception as its cause. The CLI maps
that error to a stable nonzero exit without a traceback. It never retries,
silently skips a family, or publishes a partial snapshot.

## Tests

Tests are written before implementation and cover:

1. Exact seven-format, ten-family, contract-outcome materialization.
2. Two clean output directories with byte-identical complete trees.
3. Every generated source proving exactly its declared semantic family.
4. Safe-convert sources remaining valid input containers.
5. Contract, manifest, source, hash, family, outcome, and extra-file tampering.
6. Existing-output refusal and injected mid-write failure with no publication.
7. CLI argument binding and typed failure exit.
8. Evaluator allowlist coverage for every new source and test module.
9. Duplicate contract keys, unsupported formats/families, canonical manifest
   bytes, path grammar, symlinks, hard links, special files, and extra files.
10. Cooperative lock contention, staging ownership mismatch, and cleanup that
    refuses to delete a substituted path.

## Acceptance

Snapshot acceptance requires a real generation with exactly 71 files, 70
globally unique IDs, paths, and SHA-256 digests, independent validation, and a
second clean generation whose relative paths and bytes are identical. The
validator CLI emits machine-readable counts and the manifest SHA-256; the
execution evidence records that digest without committing generated files.

The executable workflow is:

1. Generate snapshot A under `artifacts/multiformat-security-sources`.
2. Validate A and capture its success JSON in the task execution record.
3. Generate snapshot B under `/tmp` from the same contract.
4. Compare recursively enumerated relative file sets and every file byte, then
   remove B.
5. Run focused, evaluator-boundary, static, and full multiformat gates.

There is deliberately no product evidence file beyond
`security-sources.json`; the validator's canonical stdout is captured by CI or
the task log. Snapshot A remains `GENERATED`. READY-manifest assembly, signed
admission, runtime outcome capture, conversion determinism, metrics, and 96%
reporting are later phases.

Focused and full multiformat tests are implementation quality gates. Atomic
commits and cherry-pick integration are separate delivery gates, not properties
of the generated snapshot.
