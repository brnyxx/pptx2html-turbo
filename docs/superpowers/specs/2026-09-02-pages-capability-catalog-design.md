# GitHub Pages Capability Catalog Design

## Context

The v2.1.0 Pages deployment is current, but the landing page exposes only seven PPTX
capability highlights. The complete 56-feature support contract is reachable only through a
GitHub Markdown link. This does not satisfy the requirement that visitors can inspect every
tracked capability on the deployed Pages site itself.

## Goal

Publish a dedicated `/capabilities/` page on the existing GitHub Pages site. The landing page
keeps its concise seven-row overview and links prominently to the catalog. The catalog renders
all 56 entries from `evaluate/completeness_manifest.json` as static HTML, with current and
target semantic, visual, and behavioral dispositions presented without exactness inflation.

## Non-goals

- Do not replace the detailed element inventory in `SUPPORTED_FEATURES.md`.
- Do not change capability status, promotion evidence, or the completeness manifest schema.
- Do not add a client-side framework, package, runtime data fetch, or new dependency.
- Do not redesign the existing landing page or converter.
- Do not add search, filtering, sorting controls, or collapsed-by-default capability content.

## Source of Truth

`evaluate/completeness_manifest.json` remains the sole machine-readable source of current and
target status. The generated catalog must not carry a second hand-maintained capability list.
The repository-root `SUPPORTED_FEATURES.md` remains the detailed ECMA-376 element inventory,
and `docs/architecture/CAPABILITY_MATRIX.md` remains the explanatory architecture contract.

The generator consumes these manifest fields for every feature:

- `id` and `family`
- `ooxml.qualified_name`
- `official_source` and `source_status`
- `current` and `target` dispositions for `semantic`, `visual`, and `behavioral`
- `fallback_policy.kind` and `fallback_policy.diagnostic_code`

No prose will claim independent official verification when `source_status` is `unavailable`.
Those entries receive a visible `Cross-validation required` label.

## Architecture

### 1. Landing-page link

The existing seven-row overview remains unchanged. Its scope statement continues to say that
the rows are highlights. A new `#capabilityCatalogLink` uses the same-site relative URL
`./capabilities/` and the explicit copy `Browse all 56 capabilities`. The existing
`#fullCapabilityLink` remains an external link to the detailed element inventory, preserving the
public-document contract while clearly separating the two destinations.

The catalog canonical URL is exactly
`https://brnyxx.github.io/pptx2html-turbo/capabilities/`. Its back link is `../`, which resolves
to the Pages landing page. The detailed inventory and manifest-source links remain external and
use these exact URLs:

- `https://github.com/brnyxx/pptx2html-turbo/blob/main/SUPPORTED_FEATURES.md`
- `https://github.com/brnyxx/pptx2html-turbo/blob/main/evaluate/completeness_manifest.json`
- `https://github.com/brnyxx/pptx2html-turbo/blob/main/docs/architecture/CAPABILITY_MATRIX.md`

The template contains exactly one canonical element with this literal contract:

```html
<link rel="canonical" href="https://brnyxx.github.io/pptx2html-turbo/capabilities/">
```

### 2. Static catalog template

A catalog template at `crates/pptx2html-wasm/demo/capabilities.template.html` follows the tokens
and responsive rules in `DESIGN.md`. It contains the shared product header, a concise scope and
exactness warning, summary counts, the exact external navigation links above,
generated-content markers, and the existing footer vocabulary.

The template contract contains each token exactly once:

- Scalar tokens: `@@CATALOG_FEATURE_COUNT@@`, `@@CATALOG_DIMENSION_COUNT@@`,
  `@@CATALOG_EXACT_COUNT@@`, and `@@CATALOG_SOURCE_SHA256@@`.
- HTML block markers: `<!-- CATALOG_TIER_SUMMARY -->` and
  `<!-- CATALOG_FAMILY_SECTIONS -->`.

The generator first verifies that every token occurs exactly once. Before interpolation it scans
every single- or double-quoted HTML `id` attribute and rejects any value beginning with
`family-` or `capability-`; the matching rule is equivalent to
`\bid\s*=\s*(['"])(?:family-|capability-)[^'"]*\1`. It then replaces scalar tokens in the order
listed above, replaces the tier-summary marker, replaces the family-sections marker, and fails
if any `@@CATALOG_` token or catalog block marker remains. Generated manifest values are escaped
before they enter either HTML block.

The catalog uses only existing visual roles. Two component variants are added to `DESIGN.md`:

- Capability family: a section with an anchor, exact family identifier, and feature count.
- Capability record: a semantic `article` with a heading, source status, OOXML name, a
  three-row current-versus-target disposition table, and fallback policy.

All 56 records are present in the initial HTML. Nothing is collapsed or dependent on
JavaScript, so browser find, assistive technology, crawlers, and failed-script scenarios retain
the complete contract.

### 3. Deterministic generator

`scripts/render_demo_capabilities.mjs` reads paths supplied through explicit `--manifest`,
`--template`, and `--output` arguments. It validates the fields it renders, rejects duplicate
feature IDs, requires all three dimensions in both `current` and `target`, and HTML-escapes
every manifest value before interpolation. Invalid or incomplete input exits non-zero and
produces no publishable page.

Both `family` and `id` must match `^[a-z0-9]+(?:-[a-z0-9]+)*$`; empty values are rejected.
Families are sorted by their exact `family` value in ascending Unicode code-point order.
Features inside each family are sorted by their exact `id` value using the same rule. Family
anchors are `family-` followed by the already validated lowercase-hyphen family value; feature
anchors are `capability-` followed by the already validated lowercase-hyphen feature ID.
Duplicate feature IDs fail generation. Repeated family values are grouped into one section, and
the different `family-` and `capability-` prefixes prevent cross-kind anchor collisions.
Summary counts are computed from the current dispositions at generation time, including the
explicit count of exact, approximate, fallback, and unparsed dimensions. The output contains no
generation timestamp, so identical inputs produce byte-identical HTML.
`@@CATALOG_SOURCE_SHA256@@` is the lowercase SHA-256 digest of the exact manifest file bytes,
computed with the documented `createHash` interface from the
[official Node.js crypto documentation](https://nodejs.org/api/crypto.html#cryptocreatehashalgorithm-options).

The output uses a staged replace: generation first writes a complete sibling temporary file
named from the output path and process ID, awaits that write, then renames it to the requested
output. A generation, write, or rename failure removes only that temporary file and preserves
any pre-existing output. No partial generated HTML is written directly to the final path. The
output parent must already exist. This sequencing follows the
documented promise-based `writeFile` and `rename` contract in the
[official Node.js file-system documentation](https://nodejs.org/api/fs.html).

### 4. Pages assembly

The existing deploy job remains a single job. During contract validation it generates a
temporary catalog and validates it. During site assembly it writes
`_site/capabilities/index.html`. The workflow gains no additional job, matrix, cache, or
dependency installation.

The implementation modifies exactly these existing integration files:

- `.github/workflows/deploy-demo.yml`
- `crates/pptx2html-wasm/demo/index.html`
- `crates/pptx2html-wasm/tests/demo-contract.mjs`
- `DESIGN.md`

It adds exactly these source and test files:

- `crates/pptx2html-wasm/demo/capabilities.template.html`
- `scripts/render_demo_capabilities.mjs`
- `crates/pptx2html-wasm/tests/capabilities-contract.mjs`
- `scripts/qa_demo_capabilities.py`

The existing `_site` assembly inputs remain the landing-page HTML and four WASM/npm package
files. The only new output is `_site/capabilities/index.html`; the manifest itself is not copied
to Pages because the catalog links to its exact GitHub source URL.

The workflow-equivalent commands are:

```bash
mkdir -p "$RUNNER_TEMP/pptx2html-capabilities"
node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output "$RUNNER_TEMP/pptx2html-capabilities/index.html"
node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  "$RUNNER_TEMP/pptx2html-capabilities/index.html" \
  evaluate/completeness_manifest.json

mkdir -p _site/capabilities
node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output _site/capabilities/index.html
```

The workflow path filter includes the manifest and generator so a capability contract change
cannot leave Pages stale. Existing WASM build and deployment steps remain unchanged.

The complete workflow contract gate is:

```bash
VERSION=$(bash scripts/read_release_version.sh)
node crates/pptx2html-wasm/tests/demo-contract.mjs "$VERSION"
node crates/pptx2html-wasm/tests/capabilities-contract.mjs \
  "$RUNNER_TEMP/pptx2html-capabilities/index.html" \
  evaluate/completeness_manifest.json
node crates/pptx2html-wasm/tests/release-version-contract.mjs
python3 evaluate/check_exactness_contract.py --repo-root . \
  --output-json "$RUNNER_TEMP/pptx2html-capabilities/exactness-contract.json"
```

The exactness command exercises `evaluate/public_document_contract.py`; the existing external
`#fullCapabilityLink` keeps that contract valid while the new same-site catalog link is checked
by `demo-contract.mjs`.

The end-to-end browser-QA assembly reproduces the complete Pages payload in one known scratch
directory. The `wasm-pack` target and release options match its
[official build command contract](https://rustwasm.github.io/docs/wasm-pack/commands/build.html)
and the existing deployment workflow:

```bash
SITE_DIR=/tmp/pptx2html-capability-site-20260902
rm -rf "$SITE_DIR"
mkdir -p "$SITE_DIR/pkg" "$SITE_DIR/capabilities"
wasm-pack build crates/pptx2html-wasm --target web --release
cp crates/pptx2html-wasm/demo/index.html "$SITE_DIR/index.html"
cp crates/pptx2html-wasm/pkg/pptx2html_wasm.js "$SITE_DIR/pkg/"
cp crates/pptx2html-wasm/pkg/pptx2html_wasm_bg.wasm "$SITE_DIR/pkg/"
cp crates/pptx2html-wasm/pkg/pptx2html_wasm.d.ts "$SITE_DIR/pkg/"
cp crates/pptx2html-wasm/npm/index.js "$SITE_DIR/pkg/"
node scripts/render_demo_capabilities.mjs \
  --manifest evaluate/completeness_manifest.json \
  --template crates/pptx2html-wasm/demo/capabilities.template.html \
  --output "$SITE_DIR/capabilities/index.html"
```

The scratch path is agent-owned and removed after QA. No repository file is deleted during
assembly.

## Page Content and Responsive Behavior

The catalog follows the existing dark editorial workbench design:

- The hero states `56 tracked PPTX capabilities` and `0 exact dimensions` using generated
  values rather than hard-coded counts.
- A summary strip shows the four current tier totals across 168 dimension dispositions.
- Family sections appear in manifest-derived order and expose anchor links.
- Each record names the feature ID and OOXML qualified name before status details.
- Current status is visually primary; target status is secondary but always visible.
- Verified official sources are linked. Sources marked `unavailable` are visibly labelled and
  never presented as independent confirmation.
- The fallback kind and diagnostic code remain visible so unsupported behavior is not mistaken
  for silent support.

At 1280px, disposition rows may use a two-column current/target layout. At 768px and 375px,
each record stacks without changing document order. The page must have no horizontal overflow,
overlap, or text truncation at the three project QA widths.

## Error Handling and Security

- Missing template, unreadable JSON, invalid JSON, missing required fields, duplicate IDs, or
  invalid disposition shapes cause a non-zero generator exit.
- The generator accepts only the manifest-declared dimensions `semantic`, `visual`, and
  `behavioral`; tiers `exact`, `approximate`, `fallback`, and `unparsed`; stages `parsed`,
  `resolved`, `rendered`, `fidelity-tested`, and `not-applicable`; and source statuses
  `verified` and `unavailable`.
- Manifest strings are rendered as text through one HTML-escaping boundary.
- Official-source URLs must use HTTPS, contain no username or password, and have the exact host
  `learn.microsoft.com` or `ecma-international.org`; unsafe, unsupported, or malformed URLs fail
  generation rather than becoming clickable content.
- The output path is explicit. The generator does not delete directories or discover arbitrary
  files.
- The catalog contains no file upload, conversion runtime, service worker, or client-side data
  fetch.

## Testing

Implementation follows a failing-test-first sequence.

1. Extend `demo-contract.mjs` to require the same-site catalog URL and explicit 56-capability
   link copy. The expected count is derived from the manifest rather than pinned in the test.
   Confirm it fails against the current landing page.
2. Add a catalog contract test that generates into a temporary directory and requires:
   - exactly the manifest feature count,
   - every manifest feature ID exactly once,
   - all current and target dimensions,
   - generated tier totals equal to manifest totals,
   - the current exact count computed from the manifest, which is zero for v2.1.0,
   - correct cross-validation labels,
   - the exact canonical Pages URL and exact GitHub links to the detailed inventory and manifest
     source,
   - no runtime fetch requirement.
3. Confirm the new test fails before the generator and template exist.
4. In the same test file, exercise the generator boundary with temporary fixtures:
   - every required scalar token and block marker must occur exactly once,
   - duplicate or missing tokens fail,
   - an unknown leftover `@@CATALOG_*@@` token fails,
   - pre-existing `family-*` or `capability-*` template IDs fail in both quote forms,
   - duplicate IDs, missing dimensions, invalid enums, unsafe official-source URLs, and malformed
     JSON fail,
   - a pre-existing final output remains byte-identical after validation failure,
   - a missing output parent fails without creating a final or sibling temporary file,
   - two generations from identical inputs are byte-identical,
   - `@@CATALOG_SOURCE_SHA256@@` equals the independently computed SHA-256 of the manifest file
     bytes,
   - successful output contains no scalar token or catalog block marker.
5. Implement the minimum generator, template, landing link, design-system extension, and
   workflow assembly required to make these tests pass.
6. Run the existing release-version contract and the exactness/public-document contract checks
   to ensure the catalog does not create a competing status source.

The generated DOM contract is stable and test-facing:

- The head contains exactly one `<link rel="canonical">` whose `href` is the literal catalog URL.
- `main#capabilityCatalog` has `data-feature-count` equal to `features.length`,
  `data-current-dimension-count` equal to `features.length * dimensions.length`,
  `data-exact-dimensions` equal to the manifest-derived current `exact` count, and
  `data-source-sha256` equal to the raw manifest-file SHA-256.
- For each manifest tier, exactly one
  `li[data-tier="<tier>"][data-tier-count="<manifest-derived current count>"]` exists.
- For each unique family, exactly one
  `section#family-<family>[data-capability-family="<family>"][data-feature-count="<family count>"]`
  exists. The `family-*` ID belongs to the section itself, not a child anchor.
- For each feature, exactly one
  `article#capability-<id>[data-capability-id="<id>"][data-capability-family="<family>"][data-source-status="<source_status>"]`
  exists as a descendant of its corresponding family section. The `capability-*` ID belongs to
  the article itself.
- Each article contains exactly one `tr[data-dimension="<dimension>"]` for each manifest
  dimension. Each row contains exactly one
  `td[data-disposition="current"][data-tier="<current tier>"][data-stage="<current stage>"]`
  and one equivalent `target` cell whose values equal that feature's manifest dispositions.
- Each article contains exactly one
  `dl[data-fallback-kind="<fallback kind>"][data-diagnostic-code="<diagnostic code>"]` whose
  attributes equal its manifest fallback policy.
- Each article contains one `a[data-official-source]` whose `href` equals `official_source`.
  Articles with `source_status="unavailable"` additionally contain exactly one visible
  `[data-cross-validation-required]` label; verified articles contain none.

The contract test reads these attributes rather than matching presentation copy. It also
asserts that the generated file contains no `fetch(` call and that every capability record is
present before script execution.

## Manual QA and Acceptance

Assemble the same `_site` layout used by the workflow, then serve it with the
[officially documented Python `http.server` directory option](https://docs.python.org/3/library/http.server.html#command-line-interface):

```bash
python3 -m http.server 4173 --bind 127.0.0.1 \
  --directory /tmp/pptx2html-capability-site-20260902
```

The committed manual-QA harness uses the already installed Python Playwright tooling and real
Google Chrome; it is not added to CI and adds no project dependency. A missing Playwright module
or Chrome executable is a hard, clearly logged failure. Run it with these exact inputs and output
root while the server is active:

```bash
python3 scripts/qa_demo_capabilities.py \
  --base-url http://127.0.0.1:4173/ \
  --manifest evaluate/completeness_manifest.json \
  --catalog-html /tmp/pptx2html-capability-site-20260902/capabilities/index.html \
  --evidence-dir .omo/evidence/pages-capability-catalog-20260902 \
  --git-sha "$(git rev-parse HEAD)"
```

`scripts/qa_demo_capabilities.py` performs the same procedure in real Chrome at 375px, 768px,
and 1280px. It refuses to capture when tracked files differ from HEAD, so final evidence is bound
to a committed implementation SHA:

1. Navigate to `http://127.0.0.1:4173/`, scroll `#coverage` into view, record the HTTP response,
   link state, browser errors, and layout width, then capture `landing-{width}.png`.
2. Follow `#capabilityCatalogLink`, record the catalog DOM contract and canonical URL, then
   capture `catalog-top-{width}.png`.
3. Scroll `#capability-presentation` into view and capture `catalog-records-{width}.png` as the
   stable representative family/record view.

Store the nine screenshots and `browser-qa.json` under
`.omo/evidence/pages-capability-catalog-20260902/`. The report uses this exact JSON shape; numeric
values shown here are the expected v2.1.0 values but the harness derives them from the manifest:

```json
{
  "schemaVersion": 1,
  "capturedAt": "ISO-8601 UTC string",
  "source": {
    "gitSha": "40 lowercase hexadecimal characters",
    "manifestSha256": "64 lowercase hexadecimal characters",
    "catalogHtmlSha256": "64 lowercase hexadecimal characters"
  },
  "viewports": [
    {
      "width": 375,
      "height": 900,
      "screenshots": {
        "landing": {"path": "landing-375.png", "sha256": "64 lowercase hexadecimal characters"},
        "catalogTop": {"path": "catalog-top-375.png", "sha256": "64 lowercase hexadecimal characters"},
        "catalogRecords": {"path": "catalog-records-375.png", "sha256": "64 lowercase hexadecimal characters"}
      },
      "landing": {
        "status": 200,
        "resolvedCatalogHref": "http://127.0.0.1:4173/capabilities/",
        "linkVisible": true,
        "scopeVisible": true,
        "scrollWidth": 375,
        "clientWidth": 375
      },
      "catalog": {
        "status": 200,
        "canonical": "https://brnyxx.github.io/pptx2html-turbo/capabilities/",
        "sourceSha256": "same value as source.manifestSha256",
        "featureCount": 56,
        "familyCount": 19,
        "uniqueFeatureCount": 56,
        "currentDispositionCount": 168,
        "targetDispositionCount": 168,
        "exactCurrentCount": 0,
        "tierCounts": {"exact": 0, "approximate": 54, "fallback": 114, "unparsed": 0},
        "unavailableSourceCount": 2,
        "warningsBeforeFirstRecord": true,
        "scrollWidth": 375,
        "clientWidth": 375
      },
      "errors": {
        "console": [],
        "page": [],
        "failedRequests": [],
        "non2xxResponses": []
      }
    }
  ]
}
```

The `viewports` array contains exactly 375, 768, and 1280 in ascending order, all at height 900;
no extra or missing viewport or screenshot entry is permitted. Screenshot paths are evidence-root
relative basenames and must equal exactly `landing-{width}.png`, `catalog-top-{width}.png`, and
`catalog-records-{width}.png`. `capturedAt` must match
`^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$`; each reviewer rejects evidence older than 30
minutes or timestamped in the future. Every item in the four error arrays is a string; a
non-string item invalidates the report. `gitSha` must equal the worktree HEAD,
`manifestSha256` and `catalogHtmlSha256` must equal fresh hashes of the assembled inputs, and
every screenshot hash must match its named file. The HTTP-server process and agent-owned scratch
site are removed after capture; the committed harness remains in the repository.
The harness and integrity reviewer reject unknown or missing keys at the top level and within
`source`, each viewport, `screenshots`, `landing`, `catalog`, and `errors`.

The QA orchestrator dispatches two fresh read-only agents with `fork_turns="none"`:

- `lazycodex-gate-reviewer` receives the worktree path, manifest path, assembled catalog path,
  `browser-qa.json`, and all nine screenshot paths. It independently recomputes the worktree
  HEAD, manifest SHA-256, catalog SHA-256, report SHA-256, and every screenshot SHA-256. It
  returns PASS only when all JSON counts equal the manifest, both pages return HTTP 200, the
  resolved link and canonical URL match this spec, every error array is empty, freshness passes,
  `scrollWidth` equals `clientWidth` at all three widths, and the exact report/screenshots are
  present with no missing schema field.
- `lazycodex-clone-fidelity-reviewer` receives the same report and screenshot paths. It
  independently recomputes the report and screenshot hashes, then returns PASS only when all
  nine images show readable, non-overlapping content; the landing scope and catalog link are
  discoverable; the catalog warnings precede the first record; and current/target records stack
  without clipping.

The orchestrator stores each returned review verbatim as `integrity-review.md` and
`visual-fidelity-review.md` in the evidence root. Both files must begin with `VERDICT: PASS`, name
their exact role, and include `REPORT_SHA256: <sha256>` matching the same fresh
`browser-qa.json` bytes. Reviewer dispatch is an external QA orchestration step, not a repository
script or dependency.

Acceptance requires:

- The landing-page link navigates to `/capabilities/` on the same Pages site.
- The catalog reports 56 unique records, 168 current dispositions, and 168 target dispositions
  from the manifest.
- All family sections and all records are reachable through ordinary scrolling and browser
  find.
- Exactness and fallback warnings are visible before the first capability record.
- Source-verification labels, current/target values, OOXML names, and fallback diagnostics are
  readable at every QA width.
- There is no horizontal overflow, overlap, clipping, console error, failed request, or bad HTTP
  response.
- The independent integrity and visual-fidelity reviewers both pass the same fresh viewport
  evidence and bind their reports to `browser-qa.json`.

## Delivery Boundary

The implementation is complete locally only after tests, workflow-equivalent assembly, and
browser QA pass. Commit and integration may proceed on `feature/pages-capability-catalog`.
Publishing to `origin/main` remains a separate push action and requires the user to approve the
target branch immediately before push.
