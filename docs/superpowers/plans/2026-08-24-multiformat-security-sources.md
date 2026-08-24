# Deterministic Multi-Format Security Sources Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development
> if subagents are available, otherwise use superpowers:executing-plans.
> Preserve RED/GREEN evidence for every behavioral slice.

**Goal:** Materialize and independently validate a deterministic external
snapshot containing ten contract-defined security sources for each of PPTX,
DOCX, DOC, XLSX, XLS, PPT, and PDF.

**Architecture:** Promote the existing deterministic test writers into
production evaluator modules, then add a contract-driven generator, an
independent exact-set/semantic validator, and two thin CLIs. Generate binary
evidence under `artifacts/`; commit only code, tests, and documentation.

**Tech Stack:** Python 3.11 standard library, strict JSON helpers, OOXML ZIP,
CFBF, and PDF fixture writers, `unittest`, Ruff, basedpyright LSP, Rust
workspace verification.

**Design:** `docs/superpowers/specs/2026-08-24-multiformat-security-sources-design.md`

---

## Task 1: Promote deterministic source writers

**Files:**

- Move: `evaluate/tests/multiformat_source_fixture.py` →
  `evaluate/multiformat_source_fixture.py`
- Move: `evaluate/tests/multiformat_security_ooxml_fixture.py` →
  `evaluate/multiformat_security_source_ooxml.py`
- Move: `evaluate/tests/multiformat_security_cfb_fixture.py` →
  `evaluate/multiformat_security_source_cfb.py`
- Move: `evaluate/tests/multiformat_security_pdf_fixture.py` →
  `evaluate/multiformat_security_source_pdf.py`
- Move: `evaluate/tests/multiformat_security_source_fixture.py` →
  `evaluate/multiformat_security_source.py`
- Modify: every test importing those five old module paths
- Modify: production imports among the moved modules

### Step 1: Capture the refactor baseline

Run:

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest discover -s evaluate/tests \
  -p 'test_multiformat_security*.py' -v
```

Expected: `34` tests pass.

### Step 2: Move one writer family at a time

Use `apply_patch` move hunks. Update imports after each move. Production modules
must not import `evaluate.tests`. Keep private helpers available to adversarial
tests that intentionally exercise malformed structures.

### Step 3: Add exclusive destination ownership to the dispatcher

In `multiformat_security_source.py`:

- accept `DocumentFormat`, not a raw format string;
- validate family identifiers;
- create the destination with exclusive/no-follow flags;
- record device/inode before delegating;
- remove a partial file only when `lstat` proves ownership;
- map unsupported format/family and I/O failures to one typed writer error.

Low-level writers may truncate the exclusively created placeholder in the
trusted local threat model; they must not replace its inode.

### Step 4: Verify no behavior drift

Run the 34-test command again, Ruff, format, LSP, no-excuse, and:

```bash
grep -R "evaluate.tests" evaluate/multiformat_*source*.py
```

Expected: tests pass; grep has no production-module hits.

### Step 5: Commit

```bash
git add evaluate
git commit -m "refactor: promote deterministic security source writers"
```

---

## Task 2: Drive snapshot generation from the contract

**Files:**

- Modify: `evaluate/multiformat_strict_json.py`
- Create: `evaluate/multiformat_security_snapshot.py`
- Create: `evaluate/multiformat_security_publish.py`
- Create: `evaluate/tests/test_generate_multiformat_security_sources.py`

### Step 1: Write failing generation tests

Cover:

- exact seven formats and ten contract families per format;
- exact IDs and `sources/{format}/{family}.{format}` paths;
- manifest status `GENERATED` and contract SHA-256;
- canonical sorted JSON plus one LF;
- two clean trees with identical relative paths and bytes;
- all 70 IDs, paths, and hashes globally unique;
- existing destination and lock contention fail without mutation;
- injected writer failure leaves destination absent;
- changed contract digest before publication fails;
- ownership mismatch refuses cleanup.

Use event/callback injection for mutation tests. Do not sleep or poll.

### Step 2: Run RED

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest \
  evaluate.tests.test_generate_multiformat_security_sources -v
```

Expected: import or missing-behavior failures.

### Step 3: Add strict byte parsing

Expose a bounded `parse_strict_object_bytes` helper from
`multiformat_strict_json.py`; make `read_strict_object` delegate to it. Preserve
duplicate-key, non-finite-number, UTF-8, object-root, and size failures.

### Step 4: Implement cooperative publication

In `multiformat_security_publish.py`:

- deterministic sibling lock name;
- exclusive/no-follow lock acquisition;
- no stale-lock stealing;
- same-parent `mkdtemp`;
- staging and lock inode ownership;
- destination checks before publication;
- typed rename failures;
- cleanup only for matching owned identities;
- lock release in `finally`.

Concurrent hostile local mutation remains outside this tool's documented
boundary.

### Step 5: Implement generation

In `multiformat_security_snapshot.py`:

- capture and strictly parse contract bytes;
- validate schema, required formats, count `10`, family identifiers, and
  `SecurityOutcome`;
- produce canonical ordered source records;
- invoke production writer;
- hash all outputs and reject duplicates;
- write canonical manifest;
- call the independent validator through an injected callable boundary;
- re-hash contract before publication;
- return a typed summary.

Keep orchestration below 250 nonblank/non-comment lines; split only when
required by the no-excuse gate.

### Step 6: Run GREEN

Run the focused generator tests once. Then run security semantic tests, Ruff,
format, no-excuse, and LSP.

### Step 7: Commit

```bash
git add evaluate/multiformat_strict_json.py \
  evaluate/multiformat_security_snapshot.py \
  evaluate/multiformat_security_publish.py \
  evaluate/tests/test_generate_multiformat_security_sources.py
git commit -m "feat: generate deterministic security snapshots"
```

---

## Task 3: Implement independent snapshot validation

**Files:**

- Create: `evaluate/multiformat_security_snapshot_validation.py`
- Create: `evaluate/tests/test_validate_multiformat_security_sources.py`

### Step 1: Write adversarial validator tests

Start from a generated positive snapshot and mutate one property per test:

- duplicate or unknown JSON keys;
- noncanonical raw manifest bytes;
- wrong schema/status/contract digest;
- missing/extra format, source, field, or filesystem entry;
- wrong ID/path/extension/family/outcome/hash/count;
- duplicate ID/path/hash;
- traversal, symlink, hard link, FIFO/special file, case alias;
- malformed safe-convert container;
- empty, multiple, or wrong semantic family.

Assert typed failure reasons, not prose wording.

### Step 2: Run RED

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest \
  evaluate.tests.test_validate_multiformat_security_sources -v
```

### Step 3: Implement contract-derived expected records

The validator must:

- require manifest basename `security-sources.json`;
- set root to `manifest.parent`;
- parse captured contract and manifest bytes independently;
- require canonical manifest byte equality;
- validate exact keys at every object level;
- derive all expected records from the contract, not writer enums;
- enumerate the exact 71-file tree;
- reject links/special files and require source/manifest link count one;
- perform one shared size/hash/uniqueness pass;
- invoke `validate_security_fixture` for every source;
- require normal format validity only for `safe-convert`.

Return counts, file count, status, and manifest digest.

### Step 4: Run GREEN and related tests

Run generator + validator tests, corpus tests, all security semantic tests, and
the evaluator manifest boundary test.

### Step 5: Commit

```bash
git add evaluate/multiformat_security_snapshot_validation.py \
  evaluate/tests/test_validate_multiformat_security_sources.py
git commit -m "feat: validate deterministic security snapshots"
```

---

## Task 4: Expose CLIs and bind the evaluator boundary

**Files:**

- Create: `evaluate/generate_multiformat_security_sources.py`
- Create: `evaluate/validate_multiformat_security_sources.py`
- Create: `evaluate/tests/test_multiformat_security_sources_cli.py`
- Modify: `evaluate/multiformat_evaluator_files.py`
- Modify: `evaluate/tests/test_multiformat_evaluator_manifest.py`

### Step 1: Write failing CLI and boundary tests

Assert:

- required generator arguments `--contract`, `--output-dir`;
- required validator arguments `--contract`, `--manifest`;
- success exit `0` and exact compact JSON stdout schema;
- domain failure exit `1` and exact error object shape on stderr;
- argparse usage exit `2`;
- no progress output;
- every promoted/new source and test path is evaluator-bound;
- removed test-writer paths are absent.

### Step 2: Run RED

Run the two new CLI tests and the evaluator manifest boundary test.

### Step 3: Implement thin adapters

Keep parsing/output in the CLIs and all behavior in snapshot modules. Serialize
success/error values with ASCII, compact separators, sorted keys, and one LF.

### Step 4: Run GREEN

Run CLI, generator, validator, corpus, and evaluator-boundary suites.

### Step 5: Commit

```bash
git add evaluate/generate_multiformat_security_sources.py \
  evaluate/validate_multiformat_security_sources.py \
  evaluate/tests/test_multiformat_security_sources_cli.py \
  evaluate/multiformat_evaluator_files.py \
  evaluate/tests/test_multiformat_evaluator_manifest.py
git commit -m "feat: expose deterministic security snapshot tools"
```

---

## Task 5: Materialize real evidence and document the workflow

**Files:**

- Modify: `evaluate/README.md`
- External: `artifacts/multiformat-security-sources/`
- Temporary: `/tmp/pptx2html-multiformat-security-sources-*`

### Step 1: Run the real generator

```bash
uv run --python 3.11 python -m \
  evaluate.generate_multiformat_security_sources \
  --contract evaluate/multiformat/contract.v1.json \
  --output-dir artifacts/multiformat-security-sources
```

Require exactly 71 files and retain the canonical success JSON.

### Step 2: Independently validate

```bash
uv run --python 3.11 python -m \
  evaluate.validate_multiformat_security_sources \
  --contract evaluate/multiformat/contract.v1.json \
  --manifest \
  artifacts/multiformat-security-sources/security-sources.json
```

Record counts and manifest SHA-256. Do not call the snapshot READY.

### Step 3: Prove clean-run byte determinism

Generate a second snapshot under `/tmp`. Compare sorted relative file sets,
SHA-256, and full bytes for all 71 files. Remove only the owned temporary tree.

### Step 4: Document status and commands

Add the two commands, external artifact policy, manifest semantics, and explicit
`GENERATED`/later-READY boundary to `evaluate/README.md`.

### Step 5: Run verification

```bash
uv run --python 3.11 --with-requirements evaluate/requirements-test.txt \
  python -m unittest \
  evaluate.tests.test_generate_multiformat_security_sources \
  evaluate.tests.test_validate_multiformat_security_sources \
  evaluate.tests.test_multiformat_security_sources_cli \
  evaluate.tests.test_multiformat_corpus \
  evaluate.tests.test_multiformat_evaluator_manifest -v

uv run --python 3.11 --with ruff ruff check <changed-python-files>
uv run --python 3.11 --with ruff ruff format --check <changed-python-files>
uv run --python 3.11 \
  /opt/homebrew/lib/node_modules/omo-ai/plugin/skills/programming/scripts/python/check-no-excuse-rules.py \
  <changed-python-files>
python -m unittest discover -s evaluate/tests -p 'test_multiformat*.py' -v
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build --workspace
```

Run LSP diagnostics on every changed Python file. The known unrelated Office
scaffold assertion remains a baseline issue unless changed by this task.

### Step 6: Commit documentation

```bash
git add evaluate/README.md
git commit -m "docs: document deterministic security snapshots"
```

---

## Task 6: Review and integrate

1. Create a detached review worktree at feature HEAD.
2. Run independent goal, QA, code-quality, security, and context reviews.
3. Fix every blocking finding test-first.
4. Cherry-pick commits into `main` in creation order.
5. Re-run focused, full multiformat, artifact, and Rust workspace gates on
   `main`.
6. Preserve `.omo/senpi-task/`, do not push, remove only owned worktrees.

