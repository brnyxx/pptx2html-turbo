# GitHub Pages Capability Catalog Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish all 56 manifest-backed PPTX capabilities as a static, same-site `/capabilities/` catalog while preserving the concise seven-row landing-page overview.

**Architecture:** A dependency-free Node.js generator validates `evaluate/completeness_manifest.json`, escapes repository-controlled data, and injects deterministic static HTML into a catalog template. Contract tests bind the generated DOM, landing link, and Pages workflow to the manifest; a committed Python Playwright harness then verifies the assembled site in real Chrome at 375px, 768px, and 1280px.

**Tech Stack:** Node.js ESM and built-in modules, static HTML/CSS, GitHub Actions Pages, Rust/WASM built by `wasm-pack`, Python 3.14 with Playwright for manual QA.

**Design spec:** `docs/superpowers/specs/2026-09-02-pages-capability-catalog-design.md`

**Implementation worktree:** `/Users/adminstrator/Desktop/hyungjoo-drb/personal/pptx2html-turbo/.worktrees/pages-capability-catalog`

---

## File Responsibility Map

| File | Responsibility |
|---|---|
| `scripts/render_demo_capabilities.mjs` | Validate manifest/template input, render deterministic escaped catalog HTML, and stage-replace one explicit output path. |
| `crates/pptx2html-wasm/demo/capabilities.template.html` | Own the static catalog document, design-system styling, canonical URL, generated markers, and non-generated explanatory copy. |
| `crates/pptx2html-wasm/tests/capabilities-contract.mjs` | Exercise generator failure boundaries and validate the generated DOM and Pages workflow against the manifest. |
| `crates/pptx2html-wasm/demo/index.html` | Keep seven highlights and expose the same-site catalog plus the existing detailed-inventory link. |
| `crates/pptx2html-wasm/tests/demo-contract.mjs` | Bind landing-page catalog href and visible count copy to the manifest. |
| `DESIGN.md` | Add only the capability-family and capability-record variants to the existing visual contract. |
| `.github/workflows/deploy-demo.yml` | Generate, validate, assemble, and deploy `_site/capabilities/index.html` in the existing Pages job. |
| `scripts/qa_demo_capabilities.py` | Capture hash-bound Chrome DOM/network/layout evidence and nine viewport screenshots; it is manual QA tooling, not a CI dependency. |
| `evaluate/tests/test_qa_demo_capabilities.py` | Lock QA preflight, cleanup, closed-schema, and public query-propagation boundaries. |

## Chunk 1: Manifest-to-HTML Contract and Generator

### Task 1: Define the failing catalog contract

**Files:**
- Create: `crates/pptx2html-wasm/tests/capabilities-contract.mjs`
- Reference: `evaluate/completeness_manifest.json`
- Reference: `docs/superpowers/specs/2026-09-02-pages-capability-catalog-design.md`

- [ ] **Step 1: Add manifest-derived contract helpers**

Create the test as a direct Node ESM program. Use only built-in modules:

```js
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { renderCapabilityCatalog, writeCapabilityCatalog } from '../../../scripts/render_demo_capabilities.mjs';

const [generatedPathArg, manifestPathArg] = process.argv.slice(2);
assert.ok(generatedPathArg, 'usage: capabilities-contract.mjs <generated-html> <manifest>');
assert.ok(manifestPathArg, 'usage: capabilities-contract.mjs <generated-html> <manifest>');

const generatedPath = path.resolve(generatedPathArg);
const manifestPath = path.resolve(manifestPathArg);
const manifestBytes = await readFile(manifestPath);
const manifest = JSON.parse(manifestBytes.toString('utf8'));
const html = await readFile(generatedPath, 'utf8');
const sourceSha256 = createHash('sha256').update(manifestBytes).digest('hex');
```

Add focused helpers that extract a tag by exact `id`, read quoted attributes, count exact
`data-*` matches, and compute current tier totals. Do not add an HTML parser dependency and do
not reproduce generator validation logic inside the test.

- [ ] **Step 2: Assert the stable generated DOM**

Assert every equation from the spec:

```js
assert.equal(manifest.features.length, 56);
assert.equal(manifest.dimensions.length, 3);

const catalogTag = tagWithId(html, 'capabilityCatalog');
assert.equal(attribute(catalogTag, 'data-feature-count'), String(manifest.features.length));
assert.equal(
  attribute(catalogTag, 'data-current-dimension-count'),
  String(manifest.features.length * manifest.dimensions.length),
);
assert.equal(
  attribute(catalogTag, 'data-exact-dimensions'),
  String(currentTierTotals(manifest).exact),
);
assert.equal(attribute(catalogTag, 'data-source-sha256'), sourceSha256);
assert.equal(countMatches(html, /<link\b[^>]*\brel=["']canonical["'][^>]*>/g), 1);
assert.equal(countMatches(html, /<link rel="canonical" href="https:\/\/brnyxx\.github\.io\/pptx2html-turbo\/capabilities\/">/g), 1);
assert.equal(countMatches(html, /href="\.\.\/"/g), 1, 'catalog must link back to the Pages landing page');
assert.doesNotMatch(html, /fetch\s*\(/);
assert.doesNotMatch(html, /<script\b/i);
assert.doesNotMatch(html, /@@CATALOG_|<!-- CATALOG_(?:TIER_SUMMARY|FAMILY_SECTIONS) -->/);
```

Assert the exact GitHub `href` values for `SUPPORTED_FEATURES.md`,
`evaluate/completeness_manifest.json`, and `docs/architecture/CAPABILITY_MATRIX.md`. For each
manifest family, feature, dimension, disposition, fallback policy, source status, and declared
OOXML binding, assert exactly one matching owner element and exact attribute values. Extract each
family section's bounded HTML and assert every corresponding feature article is inside that
section rather than merely present elsewhere in the document. Assert each
`a[data-official-source]` has the exact manifest `official_source` value.
Assert each article has one `dl[data-ooxml-binding]`; render optional
`[data-ooxml-qualified-name]` and `[data-ooxml-relationship-type]` owners when those non-empty
strings are present, or one `[data-ooxml-not-declared]` owner with exact text
`Not declared in manifest` when neither is present. Assert unavailable sources have one
`data-cross-validation-required` label whose exact visible text is `Cross-validation required`,
and verified sources have none.

- [ ] **Step 3: Add generator-boundary fixtures**

Inside a `mkdtemp()` directory, call the exported functions and assert:

- missing or duplicate scalar/block tokens reject;
- leftover `@@CATALOG_UNKNOWN@@` rejects;
- single- and double-quoted template IDs beginning `family-` or `capability-` reject;
- duplicate feature IDs, invalid identifier regex, missing dimensions, invalid enum values,
  malformed JSON, and unsafe official-source URLs reject;
- `current` or `target` objects with any missing or additional dimension key reject;
- root `dimensions`, `tiers`, and `stages` values that differ in content or order from the exact
  declared arrays reject;
- non-object `ooxml` values and empty `qualified_name` or `relationship_type` strings reject;
- official-source URLs with a username, password, unsupported host, non-HTTPS scheme, or
  non-default explicit port reject;
- an otherwise valid official-source URL with explicit HTTPS default port `:443` generates
  successfully, locking the documented WHATWG normalization boundary;
- a valid manifest fixture containing `&`, `<`, `>`, `"`, and `'` in rendered text and quoted
  attribute fields proves all five characters cross the single escaping boundary correctly;
- invalid input leaves an existing output byte-identical;
- a missing output parent creates neither final nor sibling temporary output;
- two identical generations are byte-identical;
- output SHA text equals the independent raw-file SHA;
- successful output contains every feature and no unresolved token.
- a fixture with intentionally shuffled family and feature input proves output families and IDs
  use direct Unicode code-point order.

Also invoke the generator CLI through `node:child_process` for missing, duplicate, and unknown
options. Assert each subprocess exits non-zero with the documented usage boundary so the exported
function fixtures cannot mask a broken command-line parser.

Wrap temporary creation in `try/finally` and run `await rm(root, { recursive: true, force: true })`
in `finally`.

- [ ] **Step 4: Run the test and verify RED**

Run:

```bash
rtk node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  /tmp/pptx2html-capability-contract-20260902/index.html \
  evaluate/completeness_manifest.json
```

Expected: non-zero exit with `ERR_MODULE_NOT_FOUND` for
`scripts/render_demo_capabilities.mjs`. This is the required RED caused by the missing feature,
not a syntax or fixture error.

### Task 2: Implement deterministic static catalog generation

**Files:**
- Create: `scripts/render_demo_capabilities.mjs`
- Create: `crates/pptx2html-wasm/demo/capabilities.template.html`
- Test: `crates/pptx2html-wasm/tests/capabilities-contract.mjs`

- [ ] **Step 1: Implement the generator's public boundary**

Export exactly these functions:

```js
export function renderCapabilityCatalog({ manifestBytes, templateText }) {
  // Return complete HTML or throw before any filesystem write.
}

export async function writeCapabilityCatalog({ manifestPath, templatePath, outputPath }) {
  // Read inputs, render in memory, write a sibling temp file, then rename it.
}
```

The CLI accepts exactly `--manifest`, `--template`, and `--output`; reject missing, duplicate, or
unknown options with a usage error. Match the repository's direct-invocation guard using
`pathToFileURL(path.resolve(process.argv[1])).href`.

- [ ] **Step 2: Implement strict validation before rendering**

Use these constants and boundaries:

```js
const DIMENSIONS = ['semantic', 'visual', 'behavioral'];
const TIERS = ['exact', 'approximate', 'fallback', 'unparsed'];
const STAGES = ['parsed', 'resolved', 'rendered', 'fidelity-tested', 'not-applicable'];
const SOURCE_STATUSES = ['verified', 'unavailable'];
const OFFICIAL_SOURCE_HOSTS = new Set(['learn.microsoft.com', 'ecma-international.org']);
const IDENTIFIER = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
```

Require one JSON object, one features array, unique feature IDs, valid family/ID strings, and root
`dimensions`, `tiers`, and `stages` arrays exactly equal to the declared constants in both content
and order. Require valid current and target dimension objects, string fallback fields, and HTTPS
official URLs with empty username/password, an empty normalized `URL.port`, and an allowed exact
hostname. This rejects non-default explicit ports while allowing the WHATWG normalization of an
explicit HTTPS default port. Require `current` and `target` keys to equal the three declared
dimensions exactly, with no missing or additional key. Require every `ooxml` value to be an object;
when present, `qualified_name` and `relationship_type` must be non-empty strings. Permit either,
both, or neither key. Fail before interpolation on any mismatch.

- [ ] **Step 3: Implement one HTML-escaping boundary and deterministic ordering**

Use one helper for both text and quoted attributes:

```js
function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}
```

Sort families and feature IDs with direct code-point comparison (`left < right ? -1 : left > right ? 1 : 0`), not locale-sensitive sorting. Compute raw manifest SHA-256 and all four current tier counts from validated data.

- [ ] **Step 4: Render the exact DOM owners**

Generate:

- one tier `<li>` for each declared tier;
- one `section#family-<family>` per family;
- one `article#capability-<id>` per feature under its family;
- one three-row disposition table per article;
- one fallback `<dl>` and one official-source link per article;
- one `dl[data-ooxml-binding]` per article that shows `qualified_name`, `relationship_type`, both,
  or exact text `Not declared in manifest` when neither key is declared;
- one visible cross-validation label only for `source_status="unavailable"`.

Every `id` and `data-*` value must satisfy the equations in the spec. Do not add client-side
rendering, filtering, collapsed records, or `fetch()`.

- [ ] **Step 5: Implement template token validation and staged output replacement**

Require each scalar token and block marker exactly once, reject existing ID prefixes with the
quote-aware regex from the spec, replace in the specified order, and reject leftovers. Render to
memory first. Write `${outputPath}.tmp-${process.pid}`, await the write, then await `rename()`.
In `catch`, remove only that temp path with `force: true` and rethrow. Require the output parent
to exist; do not create or delete arbitrary directories.

- [ ] **Step 6: Build the static catalog template**

Create a complete English HTML document with:

- the exact catalog canonical element;
- one catalog back link with exact `href="../"`;
- the existing fonts, dark-only color tokens, topbar, focus, gutter, and footer vocabulary from
  `DESIGN.md`;
- a hero that uses the scalar tokens and states the approximate/fallback boundary;
- a four-tier summary list containing `<!-- CATALOG_TIER_SUMMARY -->`;
- a catalog main containing `<!-- CATALOG_FAMILY_SECTIONS -->`;
- exact external links to `SUPPORTED_FEATURES.md`, the manifest source, and
  `CAPABILITY_MATRIX.md`;
- no runtime script, animation dependency, or hidden record.

Use CSS Grid only for the wide current/target columns; at `max-width: 900px` stack the catalog
rail and make each disposition row a labelled block, then tighten spacing at `max-width: 640px`.
Both 768px and 375px must show the stacked record layout with no horizontal overflow. Reuse
existing tokens; add no new color.

- [ ] **Step 7: Generate and verify GREEN**

Run:

```bash
rtk mkdir -p /tmp/pptx2html-capability-contract-20260902
rtk node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output /tmp/pptx2html-capability-contract-20260902/index.html
rtk node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  /tmp/pptx2html-capability-contract-20260902/index.html \
  evaluate/completeness_manifest.json
```

Expected: both commands exit 0 with no warning or assertion output.

- [ ] **Step 8: Commit the generator, template, and contract together**

Run:

```bash
rtk git add scripts/render_demo_capabilities.mjs \
  crates/pptx2html-wasm/demo/capabilities.template.html \
  crates/pptx2html-wasm/tests/capabilities-contract.mjs
rtk git diff --cached --check
rtk git commit -m "feat: generate full Pages capability catalog"
```

Expected: one atomic `feat:` commit and a clean targeted contract rerun.

## Chunk 2: Landing Page and Pages Workflow Integration

### Task 3: Link the landing page to the same-site catalog

**Files:**
- Modify: `crates/pptx2html-wasm/tests/demo-contract.mjs`
- Modify: `crates/pptx2html-wasm/demo/index.html:1470-1483`
- Modify: `DESIGN.md:145-214`

- [ ] **Step 1: Write the failing landing-link contract**

After `expectedFeatureCount` is computed, assert:

```js
const catalogLink = anchorWithId('capabilityCatalogLink');
assert.equal(attribute(catalogLink.openingTag, 'href'), './capabilities/');
assert.equal(
  catalogLink.text,
  `Browse all ${expectedFeatureCount} capabilities`,
  'landing page must expose every manifest capability on the same Pages site',
);
```

Implement `anchorWithId()` as a focused exact-ID extractor that returns the matched opening tag
and that anchor's trimmed text. Do not use a page-global copy regex that could pass when the text
appears on a different element.

Keep the existing `#fullCapabilityLink` external URL assertion unchanged.

- [ ] **Step 2: Run the contract and verify RED**

Run:

```bash
rtk node crates/pptx2html-wasm/tests/demo-contract.mjs 2.1.0
```

Expected: non-zero exit with `missing element #capabilityCatalogLink`.

- [ ] **Step 3: Add both catalog and detailed-inventory links**

Change only the coverage note. It must continue to state that the seven rows are highlights,
retain zero exact dimensions, link first to:

```html
<a id="capabilityCatalogLink" href="./capabilities/">Browse all 56 capabilities</a>
```

and retain `#fullCapabilityLink` as the external detailed ECMA-376 element inventory. Do not
change the seven highlight rows or converter behavior.

- [ ] **Step 4: Extend the existing design contract**

In `DESIGN.md`, add capability family and capability record under the existing editorial/table
component family. Record:

- existing tokens only;
- current status primary, target secondary;
- all records initially visible;
- semantic section/article/table ownership;
- 1280 two-column and 768/375 stacked behavior;
- visible `Cross-validation required` treatment for unavailable sources;
- no horizontal overflow and no collapsed content.

Do not add a new design direction, palette, font, or interaction primitive.

- [ ] **Step 5: Run the landing and generator contracts**

Run:

```bash
rtk node crates/pptx2html-wasm/tests/demo-contract.mjs 2.1.0
rtk node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  /tmp/pptx2html-capability-contract-20260902/index.html \
  evaluate/completeness_manifest.json
```

Expected: both exit 0.

- [ ] **Step 6: Commit the landing and design change**

Run:

```bash
rtk git add DESIGN.md crates/pptx2html-wasm/demo/index.html \
  crates/pptx2html-wasm/tests/demo-contract.mjs
rtk git diff --cached --check
rtk git commit -m "docs: link the full Pages capability catalog"
```

### Task 4: Assemble and validate the catalog in the existing Pages job

**Files:**
- Modify: `crates/pptx2html-wasm/tests/capabilities-contract.mjs`
- Modify: `.github/workflows/deploy-demo.yml:1-70`

- [ ] **Step 1: Add failing workflow assertions**

Read `.github/workflows/deploy-demo.yml` from the repository root and assert it contains:

- path triggers for `evaluate/completeness_manifest.json` and
  `scripts/render_demo_capabilities.mjs`;
- validation generation under `$RUNNER_TEMP/pptx2html-capabilities/index.html`;
- the catalog contract command with generated page and manifest arguments;
- the exactness/public-document contract command writing
  `$RUNNER_TEMP/pptx2html-capabilities/exactness-contract.json`;
- `_site/capabilities` creation;
- final generation to `_site/capabilities/index.html`;
- exactly one `deploy-pages` job step and no new job or matrix.

- [ ] **Step 2: Verify workflow-contract RED**

Run:

```bash
rtk mkdir -p /tmp/pptx2html-capability-contract-20260902
rtk node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output /tmp/pptx2html-capability-contract-20260902/index.html
rtk node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  /tmp/pptx2html-capability-contract-20260902/index.html \
  evaluate/completeness_manifest.json
```

Expected: non-zero exit identifying the missing manifest path trigger, catalog assembly command,
or exactness/public-document gate.

- [ ] **Step 3: Update the existing workflow without adding a job**

Add these path filters:

```yaml
      - 'evaluate/completeness_manifest.json'
      - 'scripts/render_demo_capabilities.mjs'
```

In `Validate demo contracts`, create `$RUNNER_TEMP/pptx2html-capabilities`, render the catalog,
run `capabilities-contract.mjs` before the release-version contract, then run
`evaluate/check_exactness_contract.py` with `--repo-root .` and the exactness JSON output under
that same runner-temporary directory. In `Assemble site`, create `_site/pkg` and
`_site/capabilities`, retain all existing copies, then render the final catalog. Use ordinary
runner commands, not local `rtk`, inside workflow YAML.

- [ ] **Step 4: Run workflow and public-document gates**

Run:

```bash
rtk node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  /tmp/pptx2html-capability-contract-20260902/index.html \
  evaluate/completeness_manifest.json
rtk node crates/pptx2html-wasm/tests/release-version-contract.mjs
rtk python3 evaluate/check_exactness_contract.py --repo-root . \
  --output-json /tmp/pptx2html-capability-contract-20260902/exactness-contract.json
```

Expected: Node contracts exit 0; exactness JSON has `"ok": true` and the command exits 0.

- [ ] **Step 5: Commit the Pages workflow integration**

Run:

```bash
rtk git add .github/workflows/deploy-demo.yml \
  crates/pptx2html-wasm/tests/capabilities-contract.mjs
rtk git diff --cached --check
rtk git commit -m "ci: publish the Pages capability catalog"
```

## Chunk 3: Real-Browser Evidence and Release Gate

### Task 5: Add the manifest-bound Chrome QA harness

**Files:**
- Create: `scripts/qa_demo_capabilities.py`
- Create: `evaluate/tests/test_qa_demo_capabilities.py`
- Reference: `crates/pptx2html-wasm/tests/package_browser_smoke.py`
- Reference: `docs/superpowers/specs/2026-09-02-pages-capability-catalog-design.md`

- [ ] **Step 1: Write failing QA-boundary tests**

Create a standard-library `unittest` module that imports the planned QA helpers and covers three
boundaries without launching Chrome:

- Git binding accepts only the supplied 40-character lowercase SHA when it equals the mocked
  `git rev-parse HEAD`, and rejects both a mismatched SHA and mocked tracked changes;
- evidence cleanup removes exactly the nine known screenshots plus `browser-qa.json`,
  `integrity-review.md`, and `visual-fidelity-review.md`, while an unrelated sentinel file remains
  byte-identical;
- report validation accepts one complete fixture matching the spec's exact three-viewport schema,
  then rejects a missing key and an unknown key at every closed object level;
- catalog URL derivation leaves a local no-query link unchanged and reapplies an exact public
  base-URL query only after the resolved link destination has been recorded.

Use `unittest.mock` for Git subprocess results and temporary directories for filesystem cases.
Do not weaken production behavior to make fixtures easier.

- [ ] **Step 2: Run the QA-boundary test and verify RED**

Run:

```bash
rtk python3 -m unittest evaluate.tests.test_qa_demo_capabilities -v
```

Expected: non-zero exit because `scripts.qa_demo_capabilities` or its required boundary helpers do
not exist. This is the required RED, not a syntax or fixture failure.

- [ ] **Step 3: Implement strict CLI and preflight boundaries**

Use `argparse`, `hashlib`, `json`, `logging`, `subprocess`, `datetime`, and `pathlib.Path`. Accept
exactly `--base-url`, `--manifest`, `--catalog-html`, `--evidence-dir`, and `--git-sha`.

Before launching Chrome:

- resolve every local path;
- require the manifest and catalog to be regular files;
- require a 40-character lowercase hexadecimal Git SHA equal to `git rev-parse HEAD`;
- reject a dirty tracked worktree using `git status --porcelain --untracked-files=no`;
- create only the explicit evidence directory;
- compute manifest and catalog SHA-256 values;
- remove only the twelve known prior QA outputs in that evidence directory: nine screenshots,
  `browser-qa.json`, `integrity-review.md`, and `visual-fidelity-review.md`.

Use `LOGGER`, never `print()`, and catch only explicit filesystem, JSON, subprocess, and
Playwright exceptions with exception chaining.

- [ ] **Step 4: Capture exact browser and screenshot evidence**

Import Playwright inside `main()` so a missing manual-QA dependency fails at the boundary. Launch
`playwright.chromium.launch(channel="chrome", headless=True)`. For each width in
`(375, 768, 1280)` at height 900:

- attach console, page-error, request-failure, and response listeners before navigation;
- navigate landing, scroll `#coverage`, collect link/scope/layout state, and screenshot
  `landing-{width}.png`;
- click `#capabilityCatalogLink`, wait for navigation to `/capabilities/`, record the response and
  resolved URL, then collect canonical/count/hash/source/layout state and screenshot
  `catalog-top-{width}.png`; scroll `#capability-presentation`, and screenshot
  `catalog-records-{width}.png`;
- record errors as strings and close each page before the next width.

The report must match the exact closed schema in the spec. Hash each screenshot after capture,
write `browser-qa.json` with sorted keys and a trailing newline, and log only the evidence path
and aggregate counts.

If `--base-url` contains a query string, first record the unchanged link's resolved no-query URL,
then apply that same query string to the resolved catalog URL before collecting catalog state and
screenshots. This supports public cache-busting without changing the committed link contract.

- [ ] **Step 5: Verify tests, syntax, and CLI boundary GREEN**

Run:

```bash
rtk python3 -m unittest evaluate.tests.test_qa_demo_capabilities -v
rtk python3 -m py_compile scripts/qa_demo_capabilities.py
rtk python3 scripts/qa_demo_capabilities.py --help
```

Expected: tests and compile exit 0; help lists exactly the five required options.

- [ ] **Step 6: Commit the QA harness and its boundary tests**

Run:

```bash
rtk git add scripts/qa_demo_capabilities.py evaluate/tests/test_qa_demo_capabilities.py
rtk git diff --cached --check
rtk git commit -m "test: add Pages capability browser QA"
```

### Task 6: Run full local gates and capture final evidence

**Files:**
- Evidence only: `.omo/evidence/pages-capability-catalog-20260902/`
- Scratch only: `/tmp/pptx2html-capability-site-20260902/`

- [ ] **Step 1: Confirm the committed worktree and exact history**

Run:

```bash
rtk git status --short --branch
rtk git log --oneline --no-merges main..HEAD
```

Expected: clean `feature/pages-capability-catalog`; one design commit, one reviewed-plan commit,
and four focused implementation commits; no unrelated file.

- [ ] **Step 2: Run source, contract, Rust, and WASM gates**

Run each command once and retain its exit code/output:

```bash
rtk rm -rf /tmp/pptx2html-capability-contract-20260902
rtk mkdir -p /tmp/pptx2html-capability-contract-20260902
rtk cargo fmt --all -- --check
rtk cargo clippy --workspace -- -D warnings
rtk cargo test --workspace
rtk cargo build --workspace
rtk wasm-pack build crates/pptx2html-wasm --target web --release
rtk node crates/pptx2html-wasm/tests/demo-contract.mjs 2.1.0
rtk node crates/pptx2html-wasm/tests/release-version-contract.mjs
rtk node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output /tmp/pptx2html-capability-contract-20260902/index.html
rtk node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  /tmp/pptx2html-capability-contract-20260902/index.html \
  evaluate/completeness_manifest.json
rtk python3 evaluate/check_exactness_contract.py --repo-root . \
  --output-json /tmp/pptx2html-capability-contract-20260902/exactness-contract.json
```

Expected: every command exits 0; no warning is ignored. If a pre-existing unrelated workspace
failure appears, record the exact command and failure before deciding whether it blocks this
release.

- [ ] **Step 3: Assemble the exact Pages payload in agent-owned scratch space**

Run:

```bash
rtk rm -rf /tmp/pptx2html-capability-site-20260902
rtk mkdir -p /tmp/pptx2html-capability-site-20260902/pkg \
  /tmp/pptx2html-capability-site-20260902/capabilities
rtk cp crates/pptx2html-wasm/demo/index.html \
  /tmp/pptx2html-capability-site-20260902/index.html
rtk cp crates/pptx2html-wasm/pkg/pptx2html_wasm.js \
  crates/pptx2html-wasm/pkg/pptx2html_wasm_bg.wasm \
  crates/pptx2html-wasm/pkg/pptx2html_wasm.d.ts \
  crates/pptx2html-wasm/npm/index.js \
  /tmp/pptx2html-capability-site-20260902/pkg/
rtk node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output /tmp/pptx2html-capability-site-20260902/capabilities/index.html
```

Expected: root landing page, four package files, and catalog page exist; no repository file is
written by assembly.

- [ ] **Step 4: Serve and capture the real Chrome scenario**

Start one managed terminal process:

```bash
rtk python3 -m http.server 4173 --bind 127.0.0.1 \
  --directory /tmp/pptx2html-capability-site-20260902
```

Then run:

```bash
rtk python3 scripts/qa_demo_capabilities.py \
  --base-url http://127.0.0.1:4173/ \
  --manifest evaluate/completeness_manifest.json \
  --catalog-html /tmp/pptx2html-capability-site-20260902/capabilities/index.html \
  --evidence-dir .omo/evidence/pages-capability-catalog-20260902 \
  --git-sha "$(rtk git rev-parse HEAD)"
```

Expected: exit 0; `browser-qa.json` plus the exact nine screenshots; three HTTP 200 viewport
records; 56 unique features; 168 current and 168 target dispositions; no error-array entries;
no width overflow.

- [ ] **Step 5: Run the required dual evidence review**

Dispatch a fresh `lazycodex-gate-reviewer` and `lazycodex-clone-fidelity-reviewer`, both with
`fork_turns="none"`, the exact worktree, report, catalog, manifest, and nine screenshot paths.
Require independent hashes and the PASS conditions from the spec. Save their verbatim responses
as:

- `.omo/evidence/pages-capability-catalog-20260902/integrity-review.md`
- `.omo/evidence/pages-capability-catalog-20260902/visual-fidelity-review.md`

Expected: both begin `VERDICT: PASS` and carry the same `REPORT_SHA256`.

- [ ] **Step 6: Fix any QA defect through a new RED-GREEN cycle**

If either reviewer fails, add or tighten the smallest contract assertion first, observe RED,
fix the owning source, rerun all affected gates, commit with the repository prefix that matches
the defect, discard stale screenshots/reviews, and repeat Steps 2-5. Do not patch evidence or
weaken the reviewer criterion.

- [ ] **Step 7: Clean scratch artifacts and verify branch state**

Stop the HTTP server, then run:

```bash
rtk rm -rf /tmp/pptx2html-capability-site-20260902 \
  /tmp/pptx2html-capability-contract-20260902
rtk git status --short --branch
rtk git diff --stat main...HEAD
```

Expected: scratch removed; feature branch clean; only approved plan/spec and catalog delivery
files differ from `main`; `.omo/evidence` remains ignored.

### Task 7: Prepare the deployment handoff

**Files:**
- No source changes expected.

- [ ] **Step 1: Re-read the user request and delivery checklist**

Confirm: all 56 capabilities are on Pages output; seven highlights remain; exactness boundary is
visible; main link is same-site; external detailed inventory remains; CI adds no new job; tests,
build, exactness, browser QA, and both reviews pass.

- [ ] **Step 2: Integrate linearly into local `main` and revalidate**

Use the repository's preferred linear history. Cherry-pick the reviewed feature commits into local
`main` without touching the user's existing untracked `.gjc/` or `.omo/senpi-task/` paths. Run the
landing, capability, and release-version Node contracts on the integrated full SHA, then confirm
the local `main` diff matches the reviewed feature range. Never force-push.

Regenerate the contract page on integrated `main` before the capability test:

```bash
rtk rm -rf /tmp/pptx2html-capability-integrated-20260902
rtk mkdir -p /tmp/pptx2html-capability-integrated-20260902
rtk node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output /tmp/pptx2html-capability-integrated-20260902/index.html
rtk node crates/pptx2html-wasm/tests/demo-contract.mjs 2.1.0
rtk node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  /tmp/pptx2html-capability-integrated-20260902/index.html \
  evaluate/completeness_manifest.json
rtk node crates/pptx2html-wasm/tests/release-version-contract.mjs
```

- [ ] **Step 3: Report the exact integrated range and request push authorization**

Report:

```bash
rtk git log --oneline --no-merges 6b3085e6dba79fcd727359599163daac4be0a907..main
rtk git diff --stat 6b3085e6dba79fcd727359599163daac4be0a907...main
```

List `main` as the only intended push target. Do not push until the user explicitly approves
that target immediately before the action. This approval request must be the last action before
the push; do not perform another integration or source mutation between approval and push.

- [ ] **Step 4: After approval, publish only the integrated branch**

Push only `main`. Never force-push.

- [ ] **Step 5: Verify the public deployment rather than the workflow alone**

Wait for the `Deploy Demo` run tied to the pushed full SHA to succeed. Navigate a cache-busted
`https://brnyxx.github.io/pptx2html-turbo/?verify=<12-char-sha>-20260902` in real Chrome at
375px, 768px, and 1280px. Follow `#capabilityCatalogLink`, record its unmodified resolved URL,
then let the QA harness reapply the cache-busting query to the catalog verification request.
Confirm live DOM counts, canonical URL, HTTP 200, no browser errors, no overflow, and the
landing-page link.

Use the officially documented `gh run list` `--workflow`, `--commit`, and `--json` options to save
the one matching run's `databaseId`, `headSha`, `status`, `conclusion`, `url`, and `workflowName`;
then run `rtk gh run watch <run-id> --exit-status`. Reject zero/multiple runs, a mismatched
`headSha`, a final `databaseId` different from the watched run ID, any workflow name other than
`Deploy Demo`, or any conclusion other than `success`. The first list command must show exactly one
record before its `databaseId` is substituted as `<run-id>`.
Run the QA harness with the cache-busted public base URL, the integrated manifest/generated catalog,
the integrated SHA, and evidence root
`.omo/evidence/pages-capability-catalog-public-20260902/`.

Run these commands after substituting the reviewed full SHA, its first twelve characters, and the
returned workflow run ID:

```bash
rtk mkdir -p /tmp/pptx2html-capability-public-20260902/capabilities
rtk test ! -e .omo/evidence/pages-capability-catalog-public-20260902
rtk mkdir -p .omo/evidence/pages-capability-catalog-public-20260902
rtk node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output /tmp/pptx2html-capability-public-20260902/capabilities/index.html
rtk gh run list --workflow deploy-demo.yml --commit <full-sha> --limit 20 \
  --json databaseId,headSha,status,conclusion,url,workflowName
rtk gh run watch <run-id> --exit-status
rtk gh run list --workflow deploy-demo.yml --commit <full-sha> --limit 20 \
  --json databaseId,headSha,status,conclusion,url,workflowName \
  > .omo/evidence/pages-capability-catalog-public-20260902/deployment-run.json
rtk python3 scripts/qa_demo_capabilities.py \
  --base-url "https://brnyxx.github.io/pptx2html-turbo/?verify=<12-char-sha>-20260902" \
  --manifest evaluate/completeness_manifest.json \
  --catalog-html /tmp/pptx2html-capability-public-20260902/capabilities/index.html \
  --evidence-dir .omo/evidence/pages-capability-catalog-public-20260902 \
  --git-sha <full-sha>
rtk node --input-type=module - \
  .omo/evidence/pages-capability-catalog-public-20260902/browser-qa.json \
  .omo/evidence/pages-capability-catalog-public-20260902/deployment-run.json \
  .omo/evidence/pages-capability-catalog-public-20260902/public-browser-qa.json \
  <full-sha> <run-id> <12-char-sha>-20260902 <<'NODE'
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';

const [qaPath, runPath, outputPath, expectedSha, expectedRunId, cacheBuster] = process.argv.slice(2);
const browserQa = JSON.parse(await readFile(qaPath, 'utf8'));
const runs = JSON.parse(await readFile(runPath, 'utf8'));
assert.equal(runs.length, 1);
const [run] = runs;
assert.equal(run.headSha, expectedSha);
assert.equal(run.databaseId, Number(expectedRunId));
assert.equal(run.status, 'completed');
assert.equal(run.conclusion, 'success');
assert.equal(run.workflowName, 'Deploy Demo');
assert.equal(browserQa.source.gitSha, expectedSha);
assert.equal(cacheBuster, `${expectedSha.slice(0, 12)}-20260902`);
const report = {
  schemaVersion: 1,
  deployment: {
    gitSha: expectedSha,
    workflowName: run.workflowName,
    runId: run.databaseId,
    runUrl: run.url,
    conclusion: run.conclusion,
    cacheBuster,
  },
  browserQa,
};
function sortKeys(value) {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, sortKeys(value[key])]));
  }
  return value;
}
await writeFile(outputPath, `${JSON.stringify(sortKeys(report), null, 2)}\n`, { flag: 'wx' });
NODE
rtk rm .omo/evidence/pages-capability-catalog-public-20260902/browser-qa.json \
  .omo/evidence/pages-capability-catalog-public-20260902/deployment-run.json
```

Create `public-browser-qa.json` with sorted keys and a trailing newline. It has exactly
`schemaVersion`, `deployment`, and `browserQa`; `deployment` has exactly `gitSha`,
`workflowName`, `runId`, `runUrl`, `conclusion`, and `cacheBuster`; `browserQa` is the complete
closed report produced by the harness. Assert `deployment.gitSha`, the workflow `headSha`, and
`browserQa.source.gitSha` all equal the pushed full SHA, the run succeeded, and the cache buster is
`<first-12-sha>-20260902`. Retain the exact nine public screenshots beside that report before
reporting deployment complete. The verified GitHub CLI option contract is documented at
`https://cli.github.com/manual/gh_run_list` and
`https://cli.github.com/manual/gh_run_watch`.
