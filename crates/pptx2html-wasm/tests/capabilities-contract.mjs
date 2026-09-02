import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  renderCapabilityCatalog,
  writeCapabilityCatalog,
} from '../../../scripts/render_demo_capabilities.mjs';

const CATALOG_URL = 'https://brnyxx.github.io/pptx2html-turbo/capabilities/';
const SUPPORTED_FEATURES_URL =
  'https://github.com/brnyxx/pptx2html-turbo/blob/main/SUPPORTED_FEATURES.md';
const MANIFEST_URL =
  'https://github.com/brnyxx/pptx2html-turbo/blob/main/evaluate/completeness_manifest.json';
const MATRIX_URL =
  'https://github.com/brnyxx/pptx2html-turbo/blob/main/docs/architecture/CAPABILITY_MATRIX.md';
const REPOSITORY_URL = 'https://github.com/brnyxx/pptx2html-turbo';
const RELEASES_URL = 'https://github.com/brnyxx/pptx2html-turbo/releases/latest';
const NPM_URL = 'https://www.npmjs.com/package/@briank-dev/pptx-to-html';
const DIMENSIONS = ['semantic', 'visual', 'behavioral'];
const TIERS = ['exact', 'approximate', 'fallback', 'unparsed'];
const STAGES = ['parsed', 'resolved', 'rendered', 'fidelity-tested', 'not-applicable'];
const GENERATOR_PATH = fileURLToPath(
  new URL('../../../scripts/render_demo_capabilities.mjs', import.meta.url),
);
const WORKFLOW_PATH = fileURLToPath(
  new URL('../../../.github/workflows/deploy-demo.yml', import.meta.url),
);
const TEMPLATE_TEXT = `<!DOCTYPE html>
<html lang="en">
<head>
<link rel="canonical" href="${CATALOG_URL}">
</head>
<body>
<a id="backLink" href="../">Back</a>
<a id="supportedFeaturesLink" href="${SUPPORTED_FEATURES_URL}">Supported</a>
<a id="manifestSourceLink" href="${MANIFEST_URL}">Manifest</a>
<a id="capabilityMatrixLink" href="${MATRIX_URL}">Matrix</a>
<main id="capabilityCatalog" data-feature-count="@@CATALOG_FEATURE_COUNT@@" data-current-dimension-count="@@CATALOG_DIMENSION_COUNT@@" data-exact-dimensions="@@CATALOG_EXACT_COUNT@@" data-source-sha256="@@CATALOG_SOURCE_SHA256@@">
<ul><!-- CATALOG_TIER_SUMMARY --></ul>
<!-- CATALOG_FAMILY_SECTIONS -->
</main>
</body>
</html>
`;

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function parseJson(bytes) {
  return JSON.parse(Buffer.from(bytes).toString('utf8'));
}

function escapeRegExp(value) {
  return value.replace(/[\\^$.*+?()[\]{}|]/g, '\\$&');
}

function countMatches(html, pattern) {
  return [...html.matchAll(pattern)].length;
}

function assertContains(text, needle, label) {
  assert.ok(text.includes(needle), label);
}

function tagById(html, tag, id) {
  const match = html.match(new RegExp(`<${tag}\\b[^>]*\\bid="${escapeRegExp(id)}"[^>]*>`));
  assert.ok(match, `missing ${tag}#${id}`);
  return match[0];
}

function tagsByName(html, tag) {
  return [...html.matchAll(new RegExp(`<${tag}\\b[^>]*>`, 'g'))].map((match) => match[0]);
}

function attribute(tag, name) {
  const match = tag.match(new RegExp(`\\b${name}="([^"]*)"`));
  assert.ok(match, `missing ${name} on ${tag}`);
  return match[1];
}

function elementBlock(html, tag, id) {
  const pattern = new RegExp(
    `<${tag}\\b[^>]*\\bid="${escapeRegExp(id)}"[^>]*>[\\s\\S]*?</${tag}>`,
  );
  const match = html.match(pattern);
  assert.ok(match, `missing block ${tag}#${id}`);
  return match[0];
}

function familyCounts(features) {
  const counts = new Map();
  for (const feature of features) {
    counts.set(feature.family, (counts.get(feature.family) ?? 0) + 1);
  }
  return counts;
}

function tierCounts(manifest) {
  const counts = Object.fromEntries(TIERS.map((tier) => [tier, 0]));
  for (const feature of manifest.features) {
    for (const dimension of manifest.dimensions) {
      counts[feature.current[dimension].tier] += 1;
    }
  }
  return counts;
}

function featureById(manifest) {
  return new Map(manifest.features.map((feature) => [feature.id, feature]));
}

function sortedCodePoint(values) {
  return [...values].sort((left, right) => {
    if (left < right) {
      return -1;
    }
    if (left > right) {
      return 1;
    }
    return 0;
  });
}

function featureFixture(overrides = {}) {
  return {
    id: 'alpha',
    family: 'zeta',
    official_source:
      'https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document',
    source_status: 'verified',
    ooxml: {
      qualified_name: 'p:alpha',
    },
    fallback_policy: {
      kind: 'preserve-with-diagnostic',
      diagnostic_code: 'PPTX_COMPLETENESS_FALLBACK',
    },
    current: {
      semantic: { tier: 'approximate', stage: 'parsed' },
      visual: { tier: 'fallback', stage: 'rendered' },
      behavioral: { tier: 'fallback', stage: 'not-applicable' },
    },
    target: {
      semantic: { tier: 'approximate', stage: 'parsed' },
      visual: { tier: 'fallback', stage: 'rendered' },
      behavioral: { tier: 'fallback', stage: 'not-applicable' },
    },
    ...overrides,
  };
}

function manifestFixture(features = [featureFixture()], overrides = {}) {
  return {
    schema_version: '2.0',
    dimensions: [...DIMENSIONS],
    tiers: [...TIERS],
    stages: [...STAGES],
    features,
    ...overrides,
  };
}

function renderFixture(manifest, templateText = TEMPLATE_TEXT) {
  const manifestBytes = Buffer.from(JSON.stringify(manifest));
  return renderCapabilityCatalog({ manifestBytes, templateText });
}

async function withTempDir(fn) {
  const dir = await mkdtemp(path.join(tmpdir(), 'pptx2html-capabilities-test-'));
  try {
    return await fn(dir);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

function assertThrowsMessage(fn, pattern, label) {
  assert.throws(fn, pattern, label);
}

async function assertRejectsMessage(fn, pattern, label) {
  await assert.rejects(fn, pattern, label);
}

function assertCatalogDom(html, manifest, manifestBytes) {
  const expectedFeatureCount = manifest.features.length;
  const expectedCurrentDimensionCount = expectedFeatureCount * manifest.dimensions.length;
  assert.ok(expectedFeatureCount > 0, 'manifest must contain at least one feature');
  assert.deepEqual(manifest.dimensions, DIMENSIONS);
  assert.equal(manifest.dimensions.length, 3);
  assert.equal(countMatches(html, /<link\b[^>]*\brel="canonical"[^>]*>/g), 1);
  assert.ok(
    html.includes(`<link rel="canonical" href="${CATALOG_URL}">`),
    'catalog canonical must be exact literal',
  );
  assert.equal(countMatches(html, /\bhref="\.\.\/"/g), 1, 'catalog must have exactly one back link');
  assert.equal(attribute(tagById(html, 'a', 'backLink'), 'href'), '../');
  assert.equal(attribute(tagById(html, 'a', 'supportedFeaturesLink'), 'href'), SUPPORTED_FEATURES_URL);
  assert.equal(attribute(tagById(html, 'a', 'manifestSourceLink'), 'href'), MANIFEST_URL);
  assert.equal(attribute(tagById(html, 'a', 'capabilityMatrixLink'), 'href'), MATRIX_URL);
  assert.match(html, />pptx2html-turbo v2\.1\.0 &middot; MIT</);
  assert.equal(attribute(tagById(html, 'a', 'footerRepositoryLink'), 'href'), REPOSITORY_URL);
  assert.equal(attribute(tagById(html, 'a', 'footerReleasesLink'), 'href'), RELEASES_URL);
  assert.equal(attribute(tagById(html, 'a', 'footerNpmLink'), 'href'), NPM_URL);
  assert.equal(countMatches(html, /<script\b/gi), 0);
  assert.doesNotMatch(html, /fetch\s*\(/);
  assert.doesNotMatch(html, /@@CATALOG_/);
  assert.doesNotMatch(html, /CATALOG_TIER_SUMMARY|CATALOG_FAMILY_SECTIONS/);

  const mainTag = tagById(html, 'main', 'capabilityCatalog');
  const expectedTierCounts = tierCounts(manifest);
  assert.equal(attribute(mainTag, 'data-feature-count'), String(expectedFeatureCount));
  assert.equal(
    attribute(mainTag, 'data-current-dimension-count'),
    String(expectedCurrentDimensionCount),
  );
  assert.equal(attribute(mainTag, 'data-exact-dimensions'), String(expectedTierCounts.exact));
  assert.equal(attribute(mainTag, 'data-source-sha256'), sha256(manifestBytes));

  for (const tier of manifest.tiers) {
    const matches = tagsByName(html, 'li').filter(
      (tag) =>
        attribute(tag, 'data-tier') === tier &&
        attribute(tag, 'data-tier-count') === String(expectedTierCounts[tier]),
    );
    assert.equal(matches.length, 1, `tier summary for ${tier} must have one owner`);
  }

  const families = familyCounts(manifest.features);
  const familyOrder = tagsByName(html, 'section')
    .filter((tag) => tag.includes('data-capability-family='))
    .map((tag) => attribute(tag, 'data-capability-family'));
  assert.deepEqual(familyOrder, sortedCodePoint(families.keys()));
  for (const [family, count] of families) {
    const sectionId = `family-${family}`;
    const sectionTag = tagById(html, 'section', sectionId);
    assert.equal(attribute(sectionTag, 'data-capability-family'), family);
    assert.equal(attribute(sectionTag, 'data-feature-count'), String(count));
    assert.equal(countMatches(html, new RegExp(`<section\\b[^>]*\\bid="${sectionId}"`, 'g')), 1);
  }

  const byId = featureById(manifest);
  const articleTags = tagsByName(html, 'article').filter((tag) =>
    tag.includes('data-capability-id='),
  );
  assert.equal(articleTags.length, expectedFeatureCount);
  for (const feature of manifest.features) {
    const articleId = `capability-${feature.id}`;
    assert.equal(countMatches(html, new RegExp(`<article\\b[^>]*\\bid="${articleId}"`, 'g')), 1);
    const articleTag = tagById(html, 'article', articleId);
    assert.equal(attribute(articleTag, 'data-capability-id'), feature.id);
    assert.equal(attribute(articleTag, 'data-capability-family'), feature.family);
    assert.equal(attribute(articleTag, 'data-source-status'), feature.source_status);

    const familyBlock = elementBlock(html, 'section', `family-${feature.family}`);
    assert.ok(familyBlock.includes(`id="${articleId}"`), `${articleId} must be inside family`);
    const article = elementBlock(html, 'article', articleId);

    for (const dimension of manifest.dimensions) {
      const rows = [...article.matchAll(new RegExp(`<tr\\b[^>]*\\bdata-dimension="${dimension}"[^>]*>[\\s\\S]*?</tr>`, 'g'))];
      assert.equal(rows.length, 1, `${feature.id} ${dimension} row count`);
      const row = rows[0][0];
      for (const dispositionName of ['current', 'target']) {
        const disposition = feature[dispositionName][dimension];
        const cellMatches = [
          ...row.matchAll(
            new RegExp(
              `<td\\b[^>]*\\bdata-disposition="${dispositionName}"[^>]*\\bdata-tier="${disposition.tier}"[^>]*\\bdata-stage="${disposition.stage}"[^>]*>`,
              'g',
            ),
          ),
        ];
        assert.equal(cellMatches.length, 1, `${feature.id} ${dimension} ${dispositionName}`);
      }
    }

    const fallbackSelector = new RegExp(
      `<dl\\b[^>]*\\bdata-fallback-kind="${escapeRegExp(feature.fallback_policy.kind)}"[^>]*\\bdata-diagnostic-code="${escapeRegExp(feature.fallback_policy.diagnostic_code)}"[^>]*>`,
      'g',
    );
    assert.equal(countMatches(article, fallbackSelector), 1, `${feature.id} fallback dl`);

    assert.equal(countMatches(article, /<dl\b[^>]*\bdata-ooxml-binding\b[^>]*>/g), 1);
    const hasQualifiedName = typeof feature.ooxml?.qualified_name === 'string';
    const hasRelationshipType = typeof feature.ooxml?.relationship_type === 'string';
    assert.equal(
      countMatches(article, /\bdata-ooxml-qualified-name="/g),
      hasQualifiedName ? 1 : 0,
      `${feature.id} qualified name`,
    );
    if (hasQualifiedName) {
      const qualifiedNameTag = article.match(/<dd\b[^>]*\bdata-ooxml-qualified-name="[^"]*"[^>]*>/)?.[0];
      assert.ok(qualifiedNameTag, `${feature.id} qualified name tag`);
      assert.equal(
        attribute(qualifiedNameTag, 'data-ooxml-qualified-name'),
        feature.ooxml.qualified_name,
      );
      assert.match(
        article,
        new RegExp(`>${escapeRegExp(feature.ooxml.qualified_name)}<`),
        `${feature.id} qualified name visible value`,
      );
    }
    assert.equal(
      countMatches(article, /\bdata-ooxml-relationship-type="/g),
      hasRelationshipType ? 1 : 0,
      `${feature.id} relationship type`,
    );
    if (hasRelationshipType) {
      const relationshipTypeTag = article.match(/<dd\b[^>]*\bdata-ooxml-relationship-type="[^"]*"[^>]*>/)?.[0];
      assert.ok(relationshipTypeTag, `${feature.id} relationship type tag`);
      assert.equal(
        attribute(relationshipTypeTag, 'data-ooxml-relationship-type'),
        feature.ooxml.relationship_type,
      );
      assert.match(
        article,
        new RegExp(`>${escapeRegExp(feature.ooxml.relationship_type)}<`),
        `${feature.id} relationship type visible value`,
      );
    }
    if (!hasQualifiedName && !hasRelationshipType) {
      assert.equal(countMatches(article, /\bdata-ooxml-not-declared\b/g), 1);
      assert.equal(countMatches(article, />Not declared in manifest</g), 1);
    } else {
      assert.equal(countMatches(article, /\bdata-ooxml-not-declared\b/g), 0);
    }

    const sourceLinks = [
      ...article.matchAll(/<a\b[^>]*\bdata-official-source\b[^>]*>/g),
    ].map((match) => match[0]);
    assert.equal(sourceLinks.length, 1, `${feature.id} official source`);
    assert.equal(attribute(sourceLinks[0], 'href'), feature.official_source);
    const crossValidationLabels = countMatches(article, /\bdata-cross-validation-required\b/g);
    assert.equal(
      crossValidationLabels,
      feature.source_status === 'unavailable' ? 1 : 0,
      `${feature.id} cross-validation label`,
    );
    if (feature.source_status === 'unavailable') {
      assert.match(article, />Cross-validation required</);
    }
    assert.ok(byId.has(feature.id), `${feature.id} is known`);
  }
}

function assertRootValidation() {
  const base = manifestFixture();
  assertThrowsMessage(
    () => renderFixture({ ...base, dimensions: ['visual', 'semantic', 'behavioral'] }),
    /dimensions/,
    'dimension order matters',
  );
  assertThrowsMessage(
    () => renderFixture({ ...base, tiers: ['approximate', 'exact', 'fallback', 'unparsed'] }),
    /tiers/,
    'tier order matters',
  );
  assertThrowsMessage(
    () => renderFixture({ ...base, stages: ['parsed'] }),
    /stages/,
    'stage content matters',
  );
}

function assertBoundaryFixtures() {
  const duplicateScalar = TEMPLATE_TEXT.replace('@@CATALOG_FEATURE_COUNT@@', '@@CATALOG_FEATURE_COUNT@@@@CATALOG_FEATURE_COUNT@@');
  assertThrowsMessage(() => renderFixture(manifestFixture(), duplicateScalar), /token/, 'duplicate scalar token');
  assertThrowsMessage(
    () => renderFixture(manifestFixture(), TEMPLATE_TEXT.replace('@@CATALOG_FEATURE_COUNT@@', '')),
    /token/,
    'missing scalar token',
  );
  assertThrowsMessage(
    () => renderFixture(manifestFixture(), TEMPLATE_TEXT.replace('<!-- CATALOG_TIER_SUMMARY -->', '<!-- CATALOG_TIER_SUMMARY --><!-- CATALOG_TIER_SUMMARY -->')),
    /marker/,
    'duplicate block token',
  );
  assertThrowsMessage(
    () => renderFixture(manifestFixture(), TEMPLATE_TEXT.replace('<!-- CATALOG_FAMILY_SECTIONS -->', '')),
    /marker/,
    'missing block token',
  );
  assertThrowsMessage(
    () => renderFixture(manifestFixture(), TEMPLATE_TEXT.replace('</main>', '@@CATALOG_UNKNOWN@@</main>')),
    /leftover/,
    'unknown leftover token',
  );
  assertThrowsMessage(
    () => renderFixture(manifestFixture(), TEMPLATE_TEXT.replace('<body>', '<body><div id="family-reserved"></div>')),
    /reserved/,
    'reserved double-quoted family ID',
  );
  assertThrowsMessage(
    () => renderFixture(manifestFixture(), TEMPLATE_TEXT.replace('<body>', "<body><div id='capability-reserved'></div>")),
    /reserved/,
    'reserved single-quoted capability ID',
  );

  assertThrowsMessage(
    () => renderFixture(manifestFixture([featureFixture(), featureFixture({ family: 'omega' })])),
    /duplicate/i,
    'duplicate feature ID',
  );
  assertThrowsMessage(
    () => renderFixture(manifestFixture([featureFixture({ id: 'BadId' })])),
    /identifier/,
    'invalid feature ID',
  );
  assertThrowsMessage(
    () => renderFixture(manifestFixture([featureFixture({ family: 'BadFamily' })])),
    /identifier/,
    'invalid family ID',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            current: {
              semantic: { tier: 'approximate', stage: 'parsed' },
              visual: { tier: 'fallback', stage: 'rendered' },
            },
          }),
        ]),
      ),
    /current/,
    'missing current dimension',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            current: {
              semantic: { tier: 'approximate', stage: 'parsed' },
              visual: { tier: 'fallback', stage: 'rendered' },
              behavioral: { tier: 'fallback', stage: 'not-applicable' },
              extra: { tier: 'fallback', stage: 'not-applicable' },
            },
          }),
        ]),
      ),
    /current/,
    'extra current dimension',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            target: {
              semantic: { tier: 'approximate', stage: 'parsed' },
              visual: { tier: 'fallback', stage: 'rendered' },
            },
          }),
        ]),
      ),
    /target/,
    'missing target dimension',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            target: {
              semantic: { tier: 'approximate', stage: 'parsed' },
              visual: { tier: 'fallback', stage: 'rendered' },
              behavioral: { tier: 'fallback', stage: 'not-applicable' },
              extra: { tier: 'fallback', stage: 'not-applicable' },
            },
          }),
        ]),
      ),
    /target/,
    'extra target dimension',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            current: {
              semantic: { tier: 'invented', stage: 'parsed' },
              visual: { tier: 'fallback', stage: 'rendered' },
              behavioral: { tier: 'fallback', stage: 'not-applicable' },
            },
          }),
        ]),
      ),
    /tier/,
    'invalid tier',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            current: {
              semantic: { tier: 'approximate', stage: 'invented' },
              visual: { tier: 'fallback', stage: 'rendered' },
              behavioral: { tier: 'fallback', stage: 'not-applicable' },
            },
          }),
        ]),
      ),
    /stage/,
    'invalid stage',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            source_status: 'community',
          }),
        ]),
      ),
    /source_status/,
    'unsupported source status',
  );
  assertThrowsMessage(
    () => renderCapabilityCatalog({ manifestBytes: Buffer.from('{'), templateText: TEMPLATE_TEXT }),
    /JSON/,
    'malformed JSON',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            ooxml: {
              qualified_name: '',
            },
          }),
        ]),
      ),
    /ooxml/,
    'empty OOXML string',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            ooxml: {
              relationship_type: '',
            },
          }),
        ]),
      ),
    /ooxml/,
    'empty OOXML relationship type',
  );
  assertThrowsMessage(
    () =>
      renderFixture(
        manifestFixture([
          featureFixture({
            ooxml: [],
          }),
        ]),
      ),
    /ooxml/,
    'non-object OOXML binding',
  );
  const relationshipOnlyHtml = renderFixture(
    manifestFixture([
      featureFixture({
        ooxml: {
          relationship_type:
            'http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide',
        },
      }),
    ]),
  );
  assert.match(relationshipOnlyHtml, /data-ooxml-relationship-type=/);
  assert.doesNotMatch(relationshipOnlyHtml, /data-ooxml-qualified-name=/);
  const undeclaredOoxmlHtml = renderFixture(
    manifestFixture([
      featureFixture({
        ooxml: {},
      }),
    ]),
  );
  assert.match(undeclaredOoxmlHtml, /data-ooxml-not-declared>Not declared in manifest</);

  for (const official_source of [
    'http://learn.microsoft.com/en-us/office/open-xml',
    'https://user@learn.microsoft.com/en-us/office/open-xml',
    'https://:secret@learn.microsoft.com/en-us/office/open-xml',
    'https://example.com/en-us/office/open-xml',
    'https://learn.microsoft.com:444/en-us/office/open-xml',
  ]) {
    assertThrowsMessage(
      () => renderFixture(manifestFixture([featureFixture({ official_source })])),
      /official_source/,
      `unsafe URL ${official_source}`,
    );
  }
  const explicitDefaultPortSource = 'https://learn.microsoft.com:443/en-us/office/open-xml';
  const explicitDefaultPortHtml = renderFixture(
    manifestFixture([
      featureFixture({
        official_source: explicitDefaultPortSource,
      }),
    ]),
  );
  assert.ok(
    explicitDefaultPortHtml.includes(`href="${explicitDefaultPortSource}"`),
    'explicit HTTPS :443 succeeds and preserves the manifest literal href',
  );

  const escapedHtml = renderFixture(
    manifestFixture([
      featureFixture({
        id: 'escape-case',
        family: 'escape-family',
        official_source: 'https://ecma-international.org/publications-and-standards/standards/ecma-376/',
        ooxml: {
          qualified_name: 'p:&<>"\'',
          relationship_type: 'rel:&<>"\'',
        },
        fallback_policy: {
          kind: 'kind-&<>"\'',
          diagnostic_code: 'code-&<>"\'',
        },
      }),
    ]),
  );
  assert.match(escapedHtml, /p:&amp;&lt;&gt;&quot;&#39;/);
  assert.match(escapedHtml, /data-ooxml-qualified-name="p:&amp;&lt;&gt;&quot;&#39;"/);
  assert.match(escapedHtml, /kind-&amp;&lt;&gt;&quot;&#39;/);
  assert.match(escapedHtml, /data-fallback-kind="kind-&amp;&lt;&gt;&quot;&#39;"/);

  const orderHtml = renderFixture(
    manifestFixture([
      featureFixture({ id: 'zulu', family: 'zulu' }),
      featureFixture({ id: 'alpha', family: 'alpha' }),
      featureFixture({ id: 'mango', family: 'alpha' }),
    ]),
  );
  assert.ok(
    orderHtml.indexOf('id="family-alpha"') < orderHtml.indexOf('id="family-zulu"'),
    'families use direct code-point ordering',
  );
  assert.ok(
    orderHtml.indexOf('id="capability-alpha"') < orderHtml.indexOf('id="capability-mango"'),
    'features use direct code-point ordering',
  );
}

async function assertWriteBoundaries() {
  await withTempDir(async (dir) => {
    const manifestPath = path.join(dir, 'manifest.json');
    const templatePath = path.join(dir, 'template.html');
    const outputPath = path.join(dir, 'index.html');
    await writeFile(manifestPath, JSON.stringify(manifestFixture()), 'utf8');
    await writeFile(templatePath, TEMPLATE_TEXT, 'utf8');
    await writeCapabilityCatalog({ manifestPath, templatePath, outputPath });
    const first = await readFile(outputPath);
    await writeCapabilityCatalog({ manifestPath, templatePath, outputPath });
    const second = await readFile(outputPath);
    assert.deepEqual(first, second, 'identical generation is byte-identical');

    await writeFile(outputPath, 'existing output', 'utf8');
    await writeFile(manifestPath, JSON.stringify(manifestFixture([featureFixture({ id: 'BadId' })])), 'utf8');
    await assertRejectsMessage(
      () => writeCapabilityCatalog({ manifestPath, templatePath, outputPath }),
      /identifier/,
      'invalid input rejects',
    );
    assert.equal(await readFile(outputPath, 'utf8'), 'existing output');

    const missingParentOutput = path.join(dir, 'missing', 'index.html');
    await writeFile(manifestPath, JSON.stringify(manifestFixture()), 'utf8');
    await assertRejectsMessage(
      () => writeCapabilityCatalog({ manifestPath, templatePath, outputPath: missingParentOutput }),
      /parent/,
      'missing parent rejects',
    );
    await assertRejectsMessage(
      () => stat(path.dirname(missingParentOutput)),
      /ENOENT/,
      'missing parent directory remains absent',
    );
    await assertRejectsMessage(
      () => readFile(missingParentOutput),
      /ENOENT/,
      'missing parent creates no output',
    );
    const nearestAncestorEntries = await readdir(dir);
    assert.deepEqual(
      nearestAncestorEntries.filter(
        (entry) => entry === 'missing' || entry.startsWith('index.html.tmp-'),
      ),
      [],
      'missing parent failure creates no sibling temp entry in nearest existing ancestor',
    );
  });
}

function assertCliBoundaries() {
  for (const args of [[], ['--manifest', 'manifest.json', '--template', 'template.html']]) {
    const result = spawnSync(process.execPath, [GENERATOR_PATH, ...args], {
      encoding: 'utf8',
    });
    assert.notEqual(result.status, 0, `CLI rejects ${args.join(' ')}`);
  }
  const duplicateOption = spawnSync(
    process.execPath,
    [GENERATOR_PATH, '--manifest', 'manifest.json', '--manifest', 'other.json', '--output', 'index.html'],
    { encoding: 'utf8' },
  );
  assert.notEqual(duplicateOption.status, 0, 'CLI rejects duplicate option');
  assert.match(duplicateOption.stderr, /duplicate option: --manifest/);
  const unknownOption = spawnSync(
    process.execPath,
    [GENERATOR_PATH, '--manifest', 'manifest.json', '--unknown', 'template.html', '--output', 'index.html'],
    { encoding: 'utf8' },
  );
  assert.notEqual(unknownOption.status, 0, 'CLI rejects unknown option');
  assert.match(unknownOption.stderr, /unknown option: --unknown/);
}

function workflowStepRunBlock(workflowText, stepName) {
  const lines = workflowText.split('\n');
  const nameLine = `      - name: ${stepName}`;
  const startIndex = lines.indexOf(nameLine);
  assert.notEqual(startIndex, -1, `missing workflow step: ${stepName}`);
  assert.equal(lines[startIndex + 1], '        run: |', `${stepName} must use a block run`);
  const blockLines = [];
  for (const line of lines.slice(startIndex + 2)) {
    if (line.startsWith('      - name: ') || line.startsWith('      - uses: ')) {
      break;
    }
    blockLines.push(line);
  }
  return blockLines.join('\n');
}

function assertCommandOrder(block, labels) {
  let previousIndex = -1;
  for (const [label, needle] of labels) {
    const index = block.indexOf(needle);
    assert.ok(index >= 0, `missing workflow command: ${label}`);
    assert.ok(index > previousIndex, `workflow command order: ${label}`);
    previousIndex = index;
  }
}

function assertWorkflowContract(workflowText) {
  assertContains(
    workflowText,
    "      - 'evaluate/completeness_manifest.json'",
    'workflow must trigger when the capability manifest changes',
  );
  assertContains(
    workflowText,
    "      - 'scripts/render_demo_capabilities.mjs'",
    'workflow must trigger when the catalog generator changes',
  );

  const jobsBlock = workflowText.match(/^jobs:\n([\s\S]*)$/m)?.[1];
  assert.ok(jobsBlock, 'workflow jobs block must exist');
  const jobNames = [...jobsBlock.matchAll(/^  ([a-zA-Z0-9_-]+):\s*$/gm)].map(
    (match) => match[1],
  );
  assert.deepEqual(jobNames, ['deploy'], 'workflow must keep exactly one deploy job');
  assert.equal(
    countMatches(workflowText, /uses:\s*actions\/deploy-pages@/g),
    1,
    'workflow must have exactly one deploy-pages action',
  );
  assert.doesNotMatch(workflowText, /^\s*matrix:/m, 'workflow must not add a matrix');

  const validationBlock = workflowStepRunBlock(workflowText, 'Validate demo contracts');
  assertContains(
    validationBlock,
    'mkdir -p "$RUNNER_TEMP/pptx2html-capabilities"',
    'validation must use a runner temp capability directory',
  );
  assertCommandOrder(validationBlock, [
    ['release version read', 'VERSION=$(bash scripts/read_release_version.sh)'],
    ['temporary catalog generation', 'node scripts/render_demo_capabilities.mjs'],
    ['demo contract', 'node crates/pptx2html-wasm/tests/demo-contract.mjs "$VERSION"'],
    ['capability contract', 'node crates/pptx2html-wasm/tests/capabilities-contract.mjs'],
    ['release version contract', 'node crates/pptx2html-wasm/tests/release-version-contract.mjs'],
    ['exactness contract', 'python3 evaluate/check_exactness_contract.py --repo-root .'],
  ]);
  for (const needle of [
    '--manifest evaluate/completeness_manifest.json',
    '--template crates/pptx2html-wasm/demo/capabilities.template.html',
    '--output "$RUNNER_TEMP/pptx2html-capabilities/index.html"',
    '"$RUNNER_TEMP/pptx2html-capabilities/index.html"',
    'evaluate/completeness_manifest.json',
    '--output-json "$RUNNER_TEMP/pptx2html-capabilities/exactness-contract.json"',
  ]) {
    assertContains(validationBlock, needle, `validation block must contain ${needle}`);
  }

  const assembleBlock = workflowStepRunBlock(workflowText, 'Assemble site');
  assertContains(
    assembleBlock,
    'mkdir -p _site/pkg _site/capabilities',
    'assemble step must create the package and capability directories',
  );
  assertCommandOrder(assembleBlock, [
    ['package directory creation', 'mkdir -p _site/pkg _site/capabilities'],
    ['landing page copy', 'cp crates/pptx2html-wasm/demo/index.html _site/'],
    ['final catalog generation', 'node scripts/render_demo_capabilities.mjs'],
  ]);
  for (const needle of [
    '--manifest evaluate/completeness_manifest.json',
    '--template crates/pptx2html-wasm/demo/capabilities.template.html',
    '--output _site/capabilities/index.html',
  ]) {
    assertContains(assembleBlock, needle, `assemble block must contain ${needle}`);
  }
}

if (process.argv.length !== 4) {
  throw new Error('usage: capabilities-contract.mjs <generated-html> <manifest>');
}

const generatedHtmlPath = process.argv[2];
const manifestPath = process.argv[3];
const [html, manifestBytes] = await Promise.all([
  readFile(generatedHtmlPath, 'utf8'),
  readFile(manifestPath),
]);
const manifest = parseJson(manifestBytes);
const workflowText = await readFile(WORKFLOW_PATH, 'utf8');

assertCatalogDom(html, manifest, manifestBytes);
assertRootValidation();
assertBoundaryFixtures();
await assertWriteBoundaries();
assertCliBoundaries();
assertWorkflowContract(workflowText);
