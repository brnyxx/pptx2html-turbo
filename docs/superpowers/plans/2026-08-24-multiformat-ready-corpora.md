# Seven-Format READY Corpus Assembly Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture native unit counts for all 525 frozen blind sources and deterministically assemble seven independently valid schema-v2 READY corpus manifests.

**Architecture:** A provenance-bound LibreOffice/Poppler inventory stage runs twice per blind source and publishes retained observation evidence. A separate pure assembler consumes that inventory plus the frozen conformance, legacy, blind, and security snapshots, validates every input, builds seven source-level READY manifests, and publishes a root `VALIDATED` snapshot without an aggregate READY marker.

**Tech Stack:** Python 3.11 standard library, existing strict JSON/JCS, corpus validators, bounded subprocess runtime, SHA-256 plus normalized-version identity-locked LibreOffice and Poppler, Ruff, basedpyright/LSP, unittest, Cargo workspace gates.

**Design:** `docs/superpowers/specs/2026-08-24-multiformat-ready-corpora-design.md`

---

## File Map

### Format-neutral publication

- Create `evaluate/multiformat_snapshot_publish.py`: cooperative no-marker staging, lock ownership, rename, and cleanup.
- Modify `evaluate/multiformat_security_publish.py`: compatibility wrapper over the format-neutral publisher.
- Create `evaluate/tests/test_multiformat_snapshot_publish.py`: generic publisher contract.
- Modify `evaluate/tests/test_multiformat_security_publish.py`: compatibility and delegated cleanup regressions.

### Font snapshot

- Create `evaluate/multiformat_font_snapshot.py`: typed font enumeration, copy, canonical manifest, and independent validation.
- Create `evaluate/generate_multiformat_font_bundle.py`: public generator CLI.
- Create `evaluate/tests/test_multiformat_font_snapshot.py`: filesystem and determinism tests.
- Create `evaluate/tests/test_generate_multiformat_font_bundle.py`: CLI boundary tests.

### Native unit inventory

- Modify `evaluate/multiformat_candidate_process.py`: expose typed bounded-process failure reasons.
- Create `evaluate/multiformat_native_unit_types.py`: immutable runtime, source, observation, error, and runner types.
- Create `evaluate/multiformat_native_unit_runtime.py`: tool locking, isolated conversion, retained PDF, bounded `pdfinfo`, and execution records.
- Create `evaluate/multiformat_native_unit_observation.py`: one-observation conversion and retained-evidence orchestration.
- Create `evaluate/multiformat_native_unit_files.py`: stable descriptor/file operations.
- Create `evaluate/multiformat_native_unit_process.py`: typed bounded-process construction and parsing.
- Create `evaluate/multiformat_native_unit_capture.py`: exact 525-source orchestration and atomic publication.
- Create `evaluate/multiformat_native_unit_validation.py`: independent trusted-input and exact-tree validation.
- Create `evaluate/capture_multiformat_native_units.py`: `capture` and `validate` CLI subcommands.
- Create `evaluate/tests/multiformat_native_unit_fixture.py`: production-schema pool/font/tool and injected-runner fixture.
- Create `evaluate/tests/test_multiformat_native_unit_runtime.py`: process request and evidence tests.
- Create `evaluate/tests/test_multiformat_native_unit_schema.py`: exact Office/PDF execution schema tests.
- Modify `evaluate/tests/test_multiformat_snapshot_publish.py`: preserve publisher regressions across the typed process seam.
- Modify `evaluate/tests/test_multiformat_subprocess.py`: preserve clean-environment regressions.
- Create `evaluate/tests/test_multiformat_native_unit_capture.py`: exact-set, determinism, disagreement, and atomicity tests.
- Create `evaluate/tests/test_multiformat_native_unit_validation.py`: tamper and trusted-input tests.
- Create `evaluate/tests/test_capture_multiformat_native_units.py`: CLI tests.

### READY assembly

- Create `evaluate/multiformat_ready_types.py`: typed upstream paths, source records, summary, and errors.
- Create `evaluate/multiformat_ready_inputs.py`: exact upstream snapshot parsing and validation.
- Create `evaluate/multiformat_ready_manifest.py`: per-format conformance/blind/security manifest construction.
- Create `evaluate/multiformat_ready_tree.py`: closed tree inventory and non-self-referential digest.
- Create `evaluate/multiformat_ready_assembly.py`: staging, copying, validation, and no-marker publication.
- Create `evaluate/multiformat_ready_validation.py`: independent complete-snapshot validator.
- Create `evaluate/assemble_multiformat_ready_corpora.py`: assembler CLI.
- Create `evaluate/validate_multiformat_ready_corpora.py`: validator CLI.
- Create `evaluate/tests/multiformat_ready_fixture.py`: realistic upstream snapshot fixture.
- Create `evaluate/tests/test_multiformat_ready_inputs.py`: upstream adapter tests.
- Create `evaluate/tests/test_multiformat_ready_manifest.py`: exact schema-v2 manifest tests.
- Create `evaluate/tests/test_multiformat_ready_assembly.py`: atomic assembly and byte determinism tests.
- Create `evaluate/tests/test_multiformat_ready_validation.py`: exact-tree and tamper tests.
- Create `evaluate/tests/test_multiformat_ready_corpora_cli.py`: public CLI tests.

### Evaluator and docs

- Modify `evaluate/multiformat_evaluator_files.py`: bind all production and test modules.
- Modify `evaluate/tests/test_multiformat_evaluator_manifest.py`: assert the new boundary.
- Modify `evaluate/README.md`: document font, inventory, assembly, validation, and status boundaries.

## Common Python Quality Protocol

For each task, set `FILES` to every Python file created or modified by that
task and run:

```bash
uv run --python 3.11 --with ruff ruff check "${FILES[@]}"
uv run --python 3.11 --with ruff ruff format --check "${FILES[@]}"
uv run --python 3.11 \
  /opt/homebrew/lib/node_modules/omo-ai/plugin/skills/programming/scripts/python/check-no-excuse-rules.py \
  "${FILES[@]}"
git diff --check
```

The no-excuse gate enforces the pure-LOC ceiling; every new production and test
module must stay below 250 pure LOC. Run `lsp_diagnostics(severity="all")` on
each changed Python file and require zero findings. The final gate additionally
runs the evaluator allowlist test and the complete commands in Task 10.

---

## Chunk 1: Publication and Font Foundation

### Task 1: Generalize no-marker atomic publication

**Files:**
- Create: `evaluate/multiformat_snapshot_publish.py`
- Modify: `evaluate/multiformat_security_publish.py`
- Create: `evaluate/tests/test_multiformat_snapshot_publish.py`
- Modify: `evaluate/tests/test_multiformat_security_publish.py`

- [ ] **Step 1: Write the generic publisher contract tests**

Cover:

```python
def test_complete_tree_is_renamed_without_ready_marker() -> None: ...
def test_existing_destination_and_lock_are_preserved() -> None: ...
def test_substituted_staging_inode_is_never_published_or_deleted() -> None: ...
def test_cleanup_error_adds_note_without_masking_primary_error() -> None: ...
def test_standalone_cleanup_error_is_typed() -> None: ...
```

Use real temporary directories. Patch only the narrow cleanup helper for the
injected cleanup error.

- [ ] **Step 2: Run the new tests and verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_snapshot_publish -v
```

Expected: import failure for `evaluate.multiformat_snapshot_publish`.

- [ ] **Step 3: Implement the format-neutral API**

The public seam is:

```python
class SnapshotPublishFailure(StrEnum):
    DESTINATION_EXISTS = "destination-exists"
    LOCKED = "locked"
    PUBLICATION_FAILED = "publication-failed"

@dataclass(frozen=True, slots=True)
class SnapshotPublishError(Exception):
    path: Path
    failure: SnapshotPublishFailure

def publish_snapshot(
    destination: Path,
    writer: Callable[[Path], None],
    *,
    lock_namespace: str = "snapshot",
) -> None: ...
```

Move the existing lock, inode, no-follow, staging, rename, primary-error note,
and all-cleanup-attempt semantics without changing behavior. The generic lock
basename is `.{destination.name}.{lock_namespace}.lock`. Validate the namespace
against a lowercase ASCII token grammar before touching the filesystem.

- [ ] **Step 4: Make the security API a compatibility wrapper**

Keep `SecurityPublishFailure`, `SecurityPublishError`, and
`publish_security_snapshot` import-compatible. Map generic failures to the
security error while preserving causes and notes. Pass
`lock_namespace="security-snapshot"` so the exact existing
`.{destination.name}.security-snapshot.lock` pathname remains unchanged.
Update private-helper patch targets in existing tests; do not weaken their
assertions. Add an exact legacy-lock-path regression.

- [ ] **Step 5: Run GREEN and the existing security suites**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest \
  evaluate.tests.test_multiformat_snapshot_publish \
  evaluate.tests.test_multiformat_security_publish \
  evaluate.tests.test_generate_multiformat_security_sources -v
```

Expected: all tests pass once.

- [ ] **Step 6: Run static and LSP gates**

Run Ruff check/format and the Python no-excuse checker on the four changed
files. Run LSP diagnostics on each file. No suppression comments are allowed.

- [ ] **Step 7: Commit**

```bash
git add evaluate/multiformat_snapshot_publish.py \
  evaluate/multiformat_security_publish.py \
  evaluate/tests/test_multiformat_snapshot_publish.py \
  evaluate/tests/test_multiformat_security_publish.py
git commit -m "refactor: generalize atomic snapshot publication" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Task 2: Materialize and validate deterministic font bundles

**Files:**
- Create: `evaluate/multiformat_font_snapshot.py`
- Create: `evaluate/generate_multiformat_font_bundle.py`
- Create: `evaluate/tests/test_multiformat_font_snapshot.py`
- Create: `evaluate/tests/test_generate_multiformat_font_bundle.py`

- [ ] **Step 1: Write font snapshot RED tests**

Use two small real test font byte fixtures with `.ttf`/`.otf` suffixes. Assert:

- exact copied file set and canonical `font-bundle.json`;
- two source-directory orderings produce byte-identical trees;
- symlink roots and entries fail;
- `st_nlink != 1`, repeated inode, repeated digest, unsupported suffix,
  special file, extra output, and changed bytes fail;
- writer failure leaves no destination;
- existing destination and cooperative lock are untouched.

- [ ] **Step 2: Verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_font_snapshot -v
```

Expected: import failure for `evaluate.multiformat_font_snapshot`.

- [ ] **Step 3: Implement strict input discovery**

Create immutable records:

```python
@dataclass(frozen=True, slots=True)
class FontSource:
    source: Path
    digest: str
    suffix: str

@dataclass(frozen=True, slots=True)
class FontSnapshotSummary:
    files: int
    fonts: int
    manifest_sha256: str
    environment_sha256: str
```

Walk each strict root with `os.scandir`, never follow links, require regular
files, `st_nlink == 1`, containment, unique `(st_dev, st_ino)`, and unique
digest across all roots. Sort by `(digest, suffix)`.

- [ ] **Step 4: Implement staged generation and independent validation**

Copy fonts to `fonts/{ordinal:04d}-{digest}{suffix}`. Write the existing
schema-v1 font bundle with relative paths and SHA-256 values using JCS plus LF.
Validate:

- exact directories and regular files;
- no links or special files;
- canonical bytes;
- safe POSIX-relative manifest paths with no empty, `.`, or `..` component;
- lexical rejection before any path resolution;
- containment below the explicit snapshot root;
- exact paths, hashes, inodes, and count;
- `validate_font_bundle` returns the manifest's environment hash.

The independent API is:

```python
def validate_font_snapshot(
    manifest_path: Path,
    snapshot_root: Path,
) -> FontSnapshotSummary: ...
```

Require `manifest_path == snapshot_root / "font-bundle.json"` after lexical
and strict containment checks. Use `publish_snapshot`; no READY marker.

- [ ] **Step 5: Add CLI tests and verify RED**

The same module exposes `generate` and `validate` subcommands:

CLI:

```text
python -m evaluate.generate_multiformat_font_bundle generate \
  --font-dir <absolute-dir> [--font-dir <absolute-dir> ...] \
  --output-dir <path>
python -m evaluate.generate_multiformat_font_bundle validate \
  --manifest <snapshot-root>/font-bundle.json \
  --snapshot-root <snapshot-root>
```

Assert canonical compact ASCII JSON, exit `0` on success, `1` on typed domain
failure, and argparse `2` on usage errors.

Run only `test_generate_multiformat_font_bundle` and require failure because
the CLI module is missing.

- [ ] **Step 6: Implement the CLI and run GREEN/static/LSP gates**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest \
  evaluate.tests.test_multiformat_font_snapshot \
  evaluate.tests.test_generate_multiformat_font_bundle -v
```

Then run Ruff, format, no-excuse, LOC, and LSP on the four files.

- [ ] **Step 7: Commit**

```bash
git add evaluate/multiformat_font_snapshot.py \
  evaluate/generate_multiformat_font_bundle.py \
  evaluate/tests/test_multiformat_font_snapshot.py \
  evaluate/tests/test_generate_multiformat_font_bundle.py
git commit -m "feat: materialize deterministic font bundles" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Chunk 1 Review Gate

- [ ] Run the Task 1-2 focused suites.
- [ ] Dispatch the plan chunk reviewer with the spec path and changed files.
- [ ] Fix blockers test-first and repeat until approved.

---

## Chunk 2: Native Unit Inventory

### Task 3: Implement bounded native observation runtime

**Files:**
- Modify: `evaluate/multiformat_candidate_process.py`
- Create: `evaluate/multiformat_native_unit_types.py`
- Create: `evaluate/multiformat_native_unit_runtime.py`
- Create: `evaluate/multiformat_native_unit_observation.py`
- Create: `evaluate/multiformat_native_unit_files.py`
- Create: `evaluate/multiformat_native_unit_process.py`
- Create: `evaluate/tests/multiformat_native_unit_fixture.py`
- Create: `evaluate/tests/test_multiformat_native_unit_runtime.py`
- Create: `evaluate/tests/test_multiformat_native_unit_schema.py`
- Modify: `evaluate/tests/test_multiformat_snapshot_publish.py`
- Modify: `evaluate/tests/test_multiformat_subprocess.py`

- [ ] **Step 1: Write runtime RED tests**

Define an injected low-level `NativeProcessRunner` protocol. Tests must prove:

- PDF observations copy the source and invoke only `pdfinfo`;
- six Office formats use exact routing arguments, unique profile/home/tmp
  roots, and the locked font config;
- output PDF and `pdfinfo.txt` are retained and hashed;
- process stdout/stderr are bounded temporary files and are neither retained
  nor persisted as unverifiable hashes;
- version commands are exactly LibreOffice `--version` and `pdfinfo -v`;
- zero/missing/multiple page fields, nonzero exit, timeout, missing PDF, and
  outputs over 1 MiB raise typed errors;
- execution records contain no absolute workspace path;
- array, integer, and boolean fields have the exact JSON types frozen in the
  design; and
- every repeated execution field equals its parent observation binding.

- [ ] **Step 2: Verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_native_unit_runtime -v
```

Expected: missing production module.

- [ ] **Step 3: Implement immutable runtime types**

Use frozen, slotted records and a protocol:

```python
class NativeUnitFailure(StrEnum): ...

@dataclass(frozen=True, slots=True)
class NativeUnitError(Exception):
    failure: NativeUnitFailure
    document_format: DocumentFormat | None
    source_id: str | None
    detail: str

@dataclass(frozen=True, slots=True)
class NativeUnitRequest: ...

@dataclass(frozen=True, slots=True)
class NativeObservation: ...

class NativeProcessRunner(Protocol):
    def __call__(self, request: NativeProcessRequest) -> int: ...

def capture_native_observation(
    request: NativeUnitRequest,
    runner: NativeProcessRunner,
) -> NativeObservation: ...
```

`NativeUnitRequest` carries the preallocated 64-lowercase-hex
`workspace_nonce`. The runtime records that value and never generates its own
random nonce.

All public source/runtime types use
`evaluate.multiformat_corpus_types.DocumentFormat`. Routing converts from its
separate enum by `.value` only at selection time.

Define `NativeUnitFailure.UNSUPPORTED_PLATFORM = "unsupported-platform"`.
Unsupported OS/architecture capture preflight and validation raise
`NativeUnitError(UNSUPPORTED_PLATFORM, None, None, detail)` before any tool is
resolved or invoked. Source-scoped failures retain format and source ID.

The capture layer owns the observation directory and supplies its staged
destination to `NativeUnitRequest`. The runtime owns isolated workspaces,
process requests, retained `reference.pdf`/`pdfinfo.txt`, temporary log
cleanup, and the typed observation result. The low-level runner only executes
one bounded process request. Do not pass raw JSON dictionaries across either
boundary.

Task 3 exposes no `NativeUnitSummary`. Each call returns one
`NativeObservation`; Task 4 compares two observations and constructs
`NativeUnitCount`.

- [ ] **Step 4: Implement real bounded process execution**

Reuse `run_bounded_process` for process-group timeout and log bounds. Resolve
tool paths strictly, hash them, capture versions, use
`clean_subprocess_environment`, and add only isolated runtime keys. Apply
`prepare_font_environment` only for the six Office formats on supported
macOS/Linux hosts. PDF must not resolve LibreOffice or the font bundle. Parse
exactly one positive `Pages:` record from `pdfinfo`.

LibreOffice and pdfinfo stdout/stderr live only in isolated temporary files
and are discarded without persisted hash claims. The only retained files are
`execution.json`, `reference.pdf`, and `pdfinfo.txt`; the latter is the bounded
raw pdfinfo stdout.

For the six Office formats, invoke only the exact version commands plus the
route's `libreoffice` and `poppler_metadata` commands. For PDF, invoke only
`pdfinfo -v` plus `poppler_metadata`. Never invoke `poppler_render` or
`poppler_text` in this runtime.

The exact root/nested execution field sets, process-role order, tool variants,
path bases, and Office/PDF environment variants are normative in the approved
design's **Closed execution schema** section. Add tests proving timeout kills
the full process group and there are no retries/skips/fallback counts. Every
source-scoped failure carries format and source ID; unsupported-platform
preflight carries neither.

Tool identity is exactly executable SHA-256 plus the normalized first non-empty
line from LibreOffice `--version` or `pdfinfo -v`. Do not separately compare a
semantic-version constant. Font-manifest identity remains independent from the
LibreOffice executable installation root.

This source-level inventory records execution but does not claim the signed
network-isolation proof owned by the later portable-reference receipt.

- [ ] **Step 5: Run GREEN/static/LSP**

Run the runtime suite, Ruff, format, no-excuse, LOC, and LSP. Keep every module
below 250 pure LOC.

- [ ] **Step 6: Commit**

```bash
git add evaluate/multiformat_native_unit_types.py \
  evaluate/multiformat_candidate_process.py \
  evaluate/multiformat_native_unit_runtime.py \
  evaluate/multiformat_native_unit_observation.py \
  evaluate/multiformat_native_unit_files.py \
  evaluate/multiformat_native_unit_process.py \
  evaluate/tests/multiformat_native_unit_fixture.py \
  evaluate/tests/test_multiformat_native_unit_runtime.py \
  evaluate/tests/test_multiformat_native_unit_schema.py \
  evaluate/tests/test_multiformat_snapshot_publish.py \
  evaluate/tests/test_multiformat_subprocess.py
git commit -m "feat: capture bounded native unit observations" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Task 4: Capture and independently validate all blind units

**Files:**
- Modify: `evaluate/multiformat_public_pool.py`
- Modify: `evaluate/multiformat_public_pool_types.py`
- Modify: `evaluate/tests/test_collect_multiformat_public_pool.py`
- Create: `evaluate/multiformat_native_unit_capture.py`
- Create: `evaluate/multiformat_native_unit_validation.py`
- Create: `evaluate/capture_multiformat_native_units.py`
- Create: `evaluate/tests/test_multiformat_native_unit_capture.py`
- Create: `evaluate/tests/test_multiformat_native_unit_validation.py`
- Create: `evaluate/tests/test_capture_multiformat_native_units.py`

- [ ] **Step 1: Write exact 525-record capture tests**

Reuse `multiformat_public_pool_fixture` and inject a runner that emits two
retained one-page observations per source. Assert:

- seven formats x 75 records;
- exactly runs `{1, 2}` and distinct injected nonces;
- sorted canonical manifest records despite reversed worker completion;
- source/config/pool/routing/tool/font/runtime identities;
- the exact closed inventory field sets frozen in the approved design;
- typed public-pool source loading sorted by `(format, id)`;
- global path/digest/nonce/binding uniqueness;
- disagreement, duplicate nonce/path, source mutation, runner error, and
  writer error leave no output.

- [ ] **Step 2: Verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_native_unit_capture -v
```

Expected: missing capture module.

- [ ] **Step 3: Implement capture orchestration**

Freeze the public typed capture boundary exactly as:

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

`font_manifest` maps from CLI `--font-bundle`; the runtime derives the bundle
root from its parent. `output_dir` is the no-replace publication destination.
The nonce factory is a keyword-only public test seam and is called exactly
1,050 times in sorted `(format.value, source_id, run)` order before workers are
submitted. Unsupported platform or architecture raises the frozen unscoped
`NativeUnitError` before any tool is resolved or invoked. The function
independently validates the complete staged tree and returns that validator's
`NativeUnitInventorySummary`.

Validate and enumerate the public pool with:

```python
sources = load_validated_public_pool_sources(
    public_config_path,
    blind_manifest_path,
)
```

Preallocate two nonces per sorted source before submitting work. Use
`ThreadPoolExecutor` with a validated worker range `1..8`. Each task has a
unique workspace and receives its preallocated nonce through
`NativeUnitRequest`. Sort completed observations by
`(format, source_id, run)`. Write `native-unit-inventory.json` only after all
counts agree.

All inventory and execution JSON has a closed exact schema, schema version
`1`, duplicate-key rejection on read, UTF-8 JCS canonical bytes, and one
trailing LF. The exact inventory object field sets are normative in the
approved design's **Closed inventory schema** section. Execution records use
the design's exact **Closed execution schema** and contain no discarded-log
hash claims. Build the complete observation tree inside the generic
`publish_snapshot` staging directory. Run independent validation on immutable
final staged bytes before the one rename. Destination existence, lock
contention, any worker failure, disagreement, or cleanup failure publishes
nothing.

- [ ] **Step 4: Write independent validator RED tests**

Start from a complete captured fixture and mutate one concern per test:

- contract/config/pool/routing/tool/font binding;
- execution environment;
- source hash;
- run index/nonce/evidence path;
- retained PDF or `pdfinfo.txt` bytes;
- parsed or accepted count;
- symlink, hard link, special file, extra, or missing file;
- noncanonical or duplicate-key JSON.

Assert the validator reruns the supplied locked `pdfinfo` over both retained
PDFs for every source.

- [ ] **Step 5: Implement independent validation**

Expose:

```python
def validate_native_unit_inventory(
    inputs: NativeUnitValidationInputs,
) -> NativeUnitInventorySummary: ...

def load_native_unit_inventory(
    inputs: NativeUnitValidationInputs,
) -> NativeUnitInventory: ...
```

The typed input has exactly the `Path` fields `contract`, `public_config`,
`public_pool_manifest`, `routing`, `font_manifest`, `libreoffice`, `pdfinfo`,
and `inventory_root`. Rehash and re-version all trusted inputs. Validate the
exact 3,151-file tree:

- 1 inventory manifest;
- 525 x 2 x (`execution.json`, `reference.pdf`, `pdfinfo.txt`).

`NativeUnitInventorySummary` has exactly `files`, `sources`, `observations`,
`total_units`, and `manifest_sha256`. There is no per-source
`NativeUnitSummary`; `manifest_sha256` hashes the exact manifest bytes
including the trailing LF.

`NativeUnitInventory` contains that summary plus a sorted tuple of
`NativeUnitCount` records carrying exact format, source ID, relative path,
source SHA-256, and accepted unit count. The validator delegates to this typed
loader and returns its summary.

Every public-pool, native-inventory, and Task 5 join type uses
`evaluate.multiformat_corpus_types.DocumentFormat`; routing converts by
`.value` only at route selection.

Use the retained file bindings and rerun `pdfinfo` for count confirmation.

- [ ] **Step 6: Add `capture` and `validate` CLI subcommands**

The approved public module remains
`python -m evaluate.capture_multiformat_native_units`; `capture` and
`validate` are explicit modes beneath that one entry point, not separate
public modules. Both subcommands require explicit trusted paths. `capture` additionally
requires `--output-dir` and `--workers`; `validate` requires
`--inventory-root`. Emit compact ASCII JSON and preserve exit `0/1/2`.

- [ ] **Step 7: Run GREEN, adversarial, static, and LSP gates**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest \
  evaluate.tests.test_multiformat_native_unit_runtime \
  evaluate.tests.test_collect_multiformat_public_pool \
  evaluate.tests.test_multiformat_native_unit_capture \
  evaluate.tests.test_multiformat_native_unit_validation \
  evaluate.tests.test_capture_multiformat_native_units -v
```

Then run Ruff, format, no-excuse, LOC, and LSP on all native-unit files.

- [ ] **Step 8: Commit**

```bash
git add evaluate/multiformat_native_unit_capture.py \
  evaluate/multiformat_native_unit_validation.py \
  evaluate/capture_multiformat_native_units.py \
  evaluate/multiformat_public_pool.py \
  evaluate/multiformat_public_pool_types.py \
  evaluate/multiformat_native_unit_types.py \
  evaluate/multiformat_native_unit_runtime.py \
  evaluate/tests/multiformat_native_unit_fixture.py \
  evaluate/tests/test_collect_multiformat_public_pool.py \
  evaluate/tests/test_multiformat_native_unit_capture.py \
  evaluate/tests/test_multiformat_native_unit_validation.py \
  evaluate/tests/test_capture_multiformat_native_units.py
git commit -m "feat: bind native blind unit inventories" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Chunk 2 Review Gate

- [ ] Run every native-unit and public-pool suite.
- [ ] Dispatch spec and code reviewers with only Chunk 2 files and evidence.
- [ ] Fix blockers test-first and repeat until approved.

---

## Chunk 3: Seven READY Manifests

### Task 5: Parse and validate every frozen upstream snapshot

**Files:**
- Create: `evaluate/multiformat_ready_types.py`
- Create: `evaluate/multiformat_ready_inputs.py`
- Create: `evaluate/tests/multiformat_ready_fixture.py`
- Create: `evaluate/tests/test_multiformat_ready_inputs.py`

- [ ] **Step 1: Build a production-schema READY input fixture**

The fixture must materialize:

- a regenerated canonical 700-case plan;
- four 100-record modern/PDF manifests;
- one 3 x 60 paired legacy manifest with support;
- one validated 3 x 40 independent binary pool;
- one validated 7 x 75 public pool;
- one validated 7 x 10 security snapshot; and
- one validated 7 x 75 native inventory.

Use production source writers and canonical JSON, not hand-waved placeholder
paths.

- [ ] **Step 2: Write RED input-adapter tests**

Assert exact record counts, IDs, plan joins, stratum joins, provenance, and
support relationships. Mutate:

- plan/contract digest;
- snapshot status or format;
- source path/hash/format;
- ordinal/stratum/paired case;
- binary provenance;
- exact file set;
- blind/security upstream identity.

For each legacy support, validate the upstream ID and digest against the
selected modern plan case, then return a typed relation containing both
`modern_case_id` and the final collision-free support ID
`{owner_format}-support-{modern_case_id}`. Its exact final filename is
`{support_id}.{support_format}`. Add a regression proving that reusing the
modern case ID as the final support ID would collide in
`load_admission_sources`.

Each mutation must fail before copying bytes.

- [ ] **Step 3: Verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_ready_inputs -v
```

- [ ] **Step 4: Implement typed upstream adapters**

Expose:

```python
@dataclass(frozen=True, slots=True)
class ReadyInputPaths: ...

@dataclass(frozen=True, slots=True)
class ReadySourceSet:
    sources: tuple[ReadySource, ...]
    supports: tuple[ReadySupport, ...]

def load_ready_inputs(paths: ReadyInputPaths) -> ReadySourceSet: ...
```

Regenerate and validate the plan. Reuse:

- `validate_public_pool`;
- `load_validated_public_pool_sources`;
- `validate_legacy_binary_pool`;
- `validate_security_snapshot`;
- `validate_native_unit_inventory`;
- `load_native_unit_inventory`;
- existing OOXML, CFBF, PDF, conformance package, and source validators.

For conformance snapshots, require their exact closed manifests, statuses,
contract/plan bindings, source hashes, exact file sets, and format-specific
100/60 counts. Do not invoke any source generator.

Join blind public-pool records to native unit counts only by
`(document_format, source_id)` and require exact relative-path and SHA-256
equality. Missing, extra, or duplicate keys fail before copying bytes.

- [ ] **Step 5: Run GREEN/static/LSP and commit**

Run the input suite plus all upstream pool/snapshot validator suites. Then
Ruff, format, no-excuse, LOC, and LSP.

```bash
git add evaluate/multiformat_ready_types.py \
  evaluate/multiformat_ready_inputs.py \
  evaluate/tests/multiformat_ready_fixture.py \
  evaluate/tests/test_multiformat_ready_inputs.py
git commit -m "feat: validate READY corpus source inputs" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Task 6: Build exact per-format manifests and tree identity

**Files:**
- Create: `evaluate/multiformat_ready_manifest.py`
- Create: `evaluate/multiformat_ready_tree.py`
- Create: `evaluate/tests/test_multiformat_ready_manifest.py`

- [ ] **Step 1: Write schema-v2 manifest RED tests**

For all seven formats assert:

- status `READY`, contract hash, exact quotas;
- 100 conformance units with source-relative ordinal `1`;
- 60 paired + 40 binary legacy unit mappings;
- closed `paired_source` `{id,path,sha256}` shape;
- owner-prefixed final support IDs/basenames derived from the preserved
  `modern_case_id`;
- binary provenance;
- 75 blind records with `#ffffff` and inventory-derived counts;
- 10 security families/outcomes;
- unique IDs, paths, and track digests;
- support names owned by the legacy format.

Pass each generated manifest to the existing `validate_corpus_manifest`.
Also pass all seven valid manifests to `load_admission_sources` and require
exactly 180 support records with the owner-prefixed IDs and extension-bearing
paths, in addition to all 1,295 primary track records.

- [ ] **Step 2: Verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_ready_manifest -v
```

- [ ] **Step 3: Implement manifest construction**

Expose pure builders:

```python
def build_format_manifest(
    contract_digest: str,
    document_format: DocumentFormat,
    sources: ReadySourceSet,
) -> dict[str, JsonValue]: ...
```

Use exhaustive `match` for modern/PDF versus three legacy variants. Copy only
machine-consumed fields. Replace `light` with `#ffffff`; reject every other
token. Keep plan feature seeds as `secondary_features` only if they conform to
the existing string-list schema; do not add extra fields.

- [ ] **Step 4: Write and implement tree digest tests**

Test sorted UTF-8 paths, JCS framing, size, digest, exclusion of only
`assembly-manifest.json`, and rejection of links/special files. Two differently
created but byte-identical trees must have the same digest.

Expose:

```python
def tree_identity(root: Path) -> TreeIdentity: ...
```

- [ ] **Step 5: Run GREEN/static/LSP and commit**

```bash
git add evaluate/multiformat_ready_manifest.py \
  evaluate/multiformat_ready_tree.py \
  evaluate/tests/test_multiformat_ready_manifest.py
git commit -m "feat: construct seven READY corpus manifests" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Task 7: Atomically assemble and independently validate the snapshot

**Files:**
- Create: `evaluate/multiformat_ready_assembly.py`
- Create: `evaluate/multiformat_ready_validation.py`
- Create: `evaluate/assemble_multiformat_ready_corpora.py`
- Create: `evaluate/validate_multiformat_ready_corpora.py`
- Create: `evaluate/tests/test_multiformat_ready_assembly.py`
- Create: `evaluate/tests/test_multiformat_ready_validation.py`
- Create: `evaluate/tests/test_multiformat_ready_corpora_cli.py`

- [ ] **Step 1: Write assembly RED tests**

Assert exact 1,485-file output:

- 1,475 source/support files = 1,295 primary track sources + 180 paired
  support files;
- seven per-format manifests;
- plan copy;
- inventory copy;
- assembly manifest.

Assert no root READY marker, root status `VALIDATED`, all source bytes copied,
and two clean assemblies from the same inputs are byte-identical. Inject
copy/validation/rename failures and destination races; no partial output may
survive.

- [ ] **Step 2: Verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_ready_assembly -v
```

- [ ] **Step 3: Implement staged assembly**

Copy sources to collision-free canonical track paths. Write seven JCS
schema-v2 manifests, plan, and inventory. Validate each per-format manifest.
Compute the tree identity excluding `assembly-manifest.json`; then write the
candidate final root manifest with:

```json
{
  "schema_version": 1,
  "status": "VALIDATED",
  "contract_sha256": "...",
  "plan": {"path": "conformance-plan.json", "sha256": "..."},
  "native_inventory": {"path": "native-unit-inventory.json", "sha256": "..."},
  "upstream_manifests": [],
  "corpora": {},
  "support_relations": [],
  "tree": {"files": 1484, "bytes": 0, "sha256": "..."}
}
```

The tree count excludes the assembly manifest itself, so it is `1,484`.
Invoke the independent validator on final staged bytes, then call
`publish_snapshot(lock_namespace="ready-corpora")`. Do not mutate staging
after validation. The generic publisher from Task 1 must use a cooperative
sibling lock, place lock/staging paths outside the candidate root, refuse an
existing destination, compare owned staging/lock inodes before rename or
cleanup, attempt every cleanup, and preserve a primary failure with a cleanup
note.

- [ ] **Step 4: Write independent validator RED tests**

Mutate one concern per test:

- manifest canonical bytes/status/field set;
- plan, inventory, upstream, corpus, support, or tree binding;
- source bytes and source record hashes;
- missing/extra path or directory;
- symlink, hard link, special file;
- root READY marker;
- cross-track duplicate digest;
- support relationship mismatch;
- final support ID reuse of a modern primary `(format, id)` identity;
- per-format manifest or unit-count tampering.

For every support relation, require the validator to resolve the owning legacy
source and selected modern plan case, then compare exact `modern_case_id`,
derived `support_id`, `support_format`, extension-bearing path, SHA-256, and
the owning manifest's closed-schema `paired_source`. Mutate each field
independently.

Also inject lock contention, staging inode substitution, lock substitution,
destination appearance before rename, cleanup failure with a primary error,
and standalone cleanup failure. Assert substituted paths are neither published
nor deleted and the primary error is never masked. Reuse the exact generic
publisher semantics rather than implementing assembly-local lock or cleanup
code.

- [ ] **Step 5: Implement independent validation**

Expose:

```python
def validate_ready_corpora(
    inputs: ReadyValidationInputs,
) -> ReadyAssemblySummary: ...
```

Require explicit trusted upstream and runtime paths, validate the copied
inventory, validate all upstream manifests, recompute the exact tree identity,
run all seven existing corpus validations, and require exactly 1,485 physical
files.

- [ ] **Step 6: Add both CLIs**

`assemble_multiformat_ready_corpora` accepts every upstream path and
`--output-dir`. `validate_multiformat_ready_corpora` accepts the same trusted
inputs plus `--corpus-root`. Both emit compact ASCII JSON and exit `0/1/2`.

- [ ] **Step 7: Run focused/full/static/LSP gates**

Run all READY, corpus, upstream, security, and native inventory suites. Then
run full `test_multiformat*.py`, Ruff, format, no-excuse, LOC, and LSP.

- [ ] **Step 8: Commit**

```bash
git add evaluate/multiformat_ready_assembly.py \
  evaluate/multiformat_ready_validation.py \
  evaluate/assemble_multiformat_ready_corpora.py \
  evaluate/validate_multiformat_ready_corpora.py \
  evaluate/tests/test_multiformat_ready_assembly.py \
  evaluate/tests/test_multiformat_ready_validation.py \
  evaluate/tests/test_multiformat_ready_corpora_cli.py
git commit -m "feat: assemble validated seven-format corpora" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Chunk 3 Review Gate

- [ ] Run all Task 5-7 focused suites.
- [ ] Dispatch goal, source-schema, determinism, and security reviewers.
- [ ] Fix every blocker test-first and repeat until approved.
- [ ] Confirm Task 9 contains the real frozen-input assembly, independent
  validation, and second byte-identical assembly commands; fixture gates alone
  are not acceptance.

---

## Chunk 4: Boundary, Real Evidence, and Integration

### Task 8: Bind the evaluator and document the workflow

**Files:**
- Modify: `evaluate/multiformat_evaluator_files.py`
- Modify: `evaluate/tests/test_multiformat_evaluator_manifest.py`
- Modify: `evaluate/README.md`

- [ ] **Step 1: Write evaluator boundary RED assertions**

Require every new production, CLI, fixture, and test module. Assert no missing
path and no duplicate evaluator path.

- [ ] **Step 2: Verify RED**

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest evaluate.tests.test_multiformat_evaluator_manifest -v
```

- [ ] **Step 3: Add focused grouped file tuples**

If the main evaluator tuple would make the module exceed 250 pure LOC, create
`evaluate/multiformat_evaluator_ready_files.py` and import grouped tuples.
Keep every actual file path bound exactly once.

- [ ] **Step 4: Document exact external commands and statuses**

Document:

- font snapshot generation;
- native `capture` and `validate`;
- READY assembly and independent validation;
- external artifact locations;
- `CAPTURED` / per-format `READY` / root `VALIDATED` / later aggregate READY;
- no runtime security, metric, or
  `96% under the documented general conversion evaluation contract` claim at
  this stage.

Do not add prose-pinning tests.

- [ ] **Step 5: Run tests/static and commit**

```bash
git add evaluate/multiformat_evaluator_files.py \
  evaluate/tests/test_multiformat_evaluator_manifest.py evaluate/README.md
test ! -f evaluate/multiformat_evaluator_ready_files.py || \
  git add evaluate/multiformat_evaluator_ready_files.py
git commit -m "docs: document READY corpus materialization" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

Only add the grouped file path if it exists.

### Task 9: Materialize real macOS LibreOffice/Poppler evidence

**External outputs only:**
- `artifacts/multiformat-font-bundle/`
- `artifacts/multiformat-native-units/`
- `artifacts/multiformat-ready-corpora/`
- one second assembly below `/tmp`

- [ ] **Step 0: Bind the external artifact root**

The isolated worktree does not contain ignored artifacts. Run real evidence
commands from the feature worktree with:

```bash
EVIDENCE_ROOT=/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/artifacts
```

Never create a worktree symlink to the root artifact directory.

- [ ] **Step 1: Generate and validate the font bundle**

Run:

```bash
uv run --python 3.11 python -m evaluate.generate_multiformat_font_bundle generate \
  --font-dir \
  /Applications/LibreOffice.app/Contents/Resources/fonts/truetype \
  --output-dir "$EVIDENCE_ROOT/multiformat-font-bundle"
uv run --python 3.11 python -m evaluate.generate_multiformat_font_bundle validate \
  --manifest \
  "$EVIDENCE_ROOT/multiformat-font-bundle/font-bundle.json" \
  --snapshot-root "$EVIDENCE_ROOT/multiformat-font-bundle"
```

Require both commands to exit `0`. Record font count, manifest SHA-256, and
environment SHA-256 from the independent validator output.

- [ ] **Step 2: Capture all 525 blind sources twice**

Run:

```bash
uv run --python 3.11 python -m evaluate.capture_multiformat_native_units capture \
  --contract evaluate/multiformat/contract.v1.json \
  --public-config evaluate/multiformat/public-pool-sources.v1.json \
  --blind-manifest "$EVIDENCE_ROOT/multiformat-public-pool/public-pool.json" \
  --routing evaluate/multiformat/reference-routing.v1.json \
  --font-bundle "$EVIDENCE_ROOT/multiformat-font-bundle/font-bundle.json" \
  --soffice /opt/homebrew/bin/soffice \
  --pdfinfo /opt/homebrew/bin/pdfinfo \
  --workers 4 \
  --output-dir "$EVIDENCE_ROOT/multiformat-native-units"
```

No skipped source is acceptable. If a frozen blind source cannot convert,
record its exact ID/error and stop this task without publication. Do not modify
the frozen pool inside this branch. Such a failure reopens the separate source
collection task, which must produce a newly frozen and reviewed pool identity
before READY assembly restarts. Never invent a count, replace a source
silently, or lower a quota.

- [ ] **Step 3: Independently validate native inventory**

Run the matching `validate` subcommand with every trusted input and
`--inventory-root "$EVIDENCE_ROOT/multiformat-native-units"`. Require 525 records,
1,050 observations, and two matching positive counts per source.

- [ ] **Step 4: Assemble the seven READY manifests**

Run Steps 4-6 in one shell session. Regenerate the canonical plan and run:

```bash
PLAN_PARENT="$(mktemp -d /tmp/multiformat-ready-plan-XXXXXX)"
PLAN_PARENT_ID="$(stat -f '%d:%i' "$PLAN_PARENT")"
PLAN="$PLAN_PARENT/plan.json"
trap 'test "$(stat -f "%d:%i" "$PLAN_PARENT")" = "$PLAN_PARENT_ID" && \
  rm -rf -- "$PLAN_PARENT"' EXIT
uv run --python 3.11 python -m evaluate.build_multiformat_conformance_plan \
  --contract evaluate/multiformat/contract.v1.json \
  --output "$PLAN"
test "$(shasum -a 256 "$PLAN" | awk '{print $1}')" = \
  609762e81c90f4d2185f7078fad699aa1ea65c76b2aa2c48680b7b001e6df94a
uv run --python 3.11 python -m evaluate.assemble_multiformat_ready_corpora \
  --contract evaluate/multiformat/contract.v1.json \
  --plan "$PLAN" \
  --pptx-conformance \
  /tmp/pptx2html-pptx-conformance-20260821-b/generation-manifest.json \
  --docx-conformance \
  /tmp/pptx2html-docx-conformance-20260821-d/generation-manifest.json \
  --xlsx-conformance \
  /tmp/pptx2html-xlsx-conformance-20260821-f/generation-manifest.json \
  --pdf-conformance \
  /tmp/pptx2html-pdf-conformance-20260821-k/generation-manifest.json \
  --legacy-conformance \
  /tmp/pptx2html-legacy-pairs-20260824-b/generation-manifest.json \
  --public-config evaluate/multiformat/public-pool-sources.v1.json \
  --blind-manifest "$EVIDENCE_ROOT/multiformat-public-pool/public-pool.json" \
  --legacy-binary-config evaluate/multiformat/legacy-binary-sources.v1.json \
  --legacy-binary-manifest \
  "$EVIDENCE_ROOT/multiformat-legacy-binary-pool/legacy-binary-pool.json" \
  --security-manifest \
  "$EVIDENCE_ROOT/multiformat-security-sources/security-sources.json" \
  --routing evaluate/multiformat/reference-routing.v1.json \
  --font-bundle "$EVIDENCE_ROOT/multiformat-font-bundle/font-bundle.json" \
  --soffice /opt/homebrew/bin/soffice \
  --pdfinfo /opt/homebrew/bin/pdfinfo \
  --native-inventory-root "$EVIDENCE_ROOT/multiformat-native-units" \
  --output-dir "$EVIDENCE_ROOT/multiformat-ready-corpora"
```

- [ ] **Step 5: Independently validate every READY manifest**

Run:

```bash
for format in pptx docx doc xlsx xls ppt pdf; do
  uv run --python 3.11 python -m evaluate.multiformat_corpus \
    --contract evaluate/multiformat/contract.v1.json \
    --manifest \
    "$EVIDENCE_ROOT/multiformat-ready-corpora/corpora/$format/manifest.json"
done
```

Then run `validate_multiformat_ready_corpora` with the same trusted inputs.
Require seven READY results and root `VALIDATED`.

- [ ] **Step 6: Prove deterministic assembly**

Assemble the same frozen inputs and same captured inventory into a fresh `/tmp`
destination. Compare the complete relative path sets and every file byte.
Repeat the exact Step 4 assembler command with only `--output-dir` changed:

```bash
(
SECOND_PARENT="$(mktemp -d /tmp/multiformat-ready-compare-XXXXXX)"
SECOND="$SECOND_PARENT/corpora"
trap 'rm -rf -- "$SECOND_PARENT"' EXIT
# Repeat Step 4 with:
#   --output-dir "$SECOND"
rtk proxy diff -qr -- \
  "$EVIDENCE_ROOT/multiformat-ready-corpora" "$SECOND"
)
```

Require `diff -qr` exit `0`; it compares complete relative path sets, file
types, and every file byte. Reuse the still-present, digest-verified `PLAN`
from Step 4. Remove only `SECOND_PARENT`, whose identity came from this
`mktemp` call. The existing EXIT trap removes the owned plan directory after
both assemblies finish.

- [ ] **Step 7: Commit only documentation if evidence changes it**

Do not commit generated binaries. If README digest/count evidence is updated,
commit it separately as:

```bash
git commit -m "docs: record validated READY corpus evidence" \
  -m "Ultraworked with [omo](https://github.com/code-yeongyu/oh-my-openagent)" \
  -m "Co-authored-by: sisyphus-dev-ai <sisyphus-dev-ai@users.noreply.github.com>"
```

### Task 10: Final review, cherry-pick, and main verification

- [ ] **Step 1: Run feature-branch quality gates**

Run once each:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest discover -s evaluate/tests -p 'test_multiformat*.py' -v
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build --workspace
```

Also run focused suites, all changed-file Ruff/format/no-excuse/LOC, LSP on
every changed Python file, the real native inventory validator, all seven
corpus validators, root validator, and the second-assembly byte comparison.

- [ ] **Step 2: Run five independent review lanes**

Require PASS from:

1. goal/contract;
2. hands-on QA;
3. code quality/types/LOC;
4. security/filesystem/process isolation;
5. context/status/claim boundaries.

Fix every blocker test-first and rerun affected gates.

- [ ] **Step 3: Cherry-pick verified commits into main**

Before integration, record and verify commit identities:

```bash
BASE=b812348
COMMIT_TMP="$(mktemp -d /tmp/multiformat-ready-commits-XXXXXX)"
COMMIT_TMP_ID="$(stat -f '%d:%i' "$COMMIT_TMP")"
COMMIT_LIST="$COMMIT_TMP/commits"
trap 'test "$(stat -f "%d:%i" "$COMMIT_TMP")" = "$COMMIT_TMP_ID" && \
  rm -rf -- "$COMMIT_TMP"' EXIT
git fsck --full --strict
(umask 077; git rev-list --reverse "$BASE..HEAD" > "$COMMIT_LIST")
test -f "$COMMIT_LIST" && test ! -L "$COMMIT_LIST"
test "$(stat -f '%l:%u' "$COMMIT_LIST")" = "1:$(id -u)"
while IFS= read -r commit; do
  test "$(stat -f '%d:%i' "$COMMIT_TMP")" = "$COMMIT_TMP_ID"
  git cat-file -e "$commit^{commit}"
  git show --check --format=fuller --name-status "$commit"
done < "$COMMIT_LIST"
git status --short
```

Review each commit's path set against its task boundary and require no tracked
path below `artifacts/`. Audit `main`, preserve `.omo/senpi-task/`, and
cherry-pick exactly the recorded commits in order. Run `git fsck --full
--strict` again after cherry-pick. Do not push.

- [ ] **Step 4: Repeat main verification**

Rerun focused/full multiformat, static/LSP, all real artifact validators,
deterministic assembly comparison, and Rust workspace gates on `main`.

- [ ] **Step 5: Close the task and clean worktrees**

Mark `seven per-format READY manifests 조립 검증` done only after every real
artifact and main gate passes. Remove the integrated feature/review worktrees.
Leave aggregate signed admission, runtime security outcomes, metrics, and
`96% under the documented general conversion evaluation contract` reports open.

### Chunk 4 Review Gate

- [ ] Review execution evidence, commits, artifact identities, and status
  language.
- [ ] Integrate only after all five lanes pass.

