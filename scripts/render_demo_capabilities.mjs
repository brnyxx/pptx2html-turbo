import { createHash } from 'node:crypto';
import { constants } from 'node:fs';
import { access, readFile, rename, stat, unlink, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const DIMENSIONS = ['semantic', 'visual', 'behavioral'];
const TIERS = ['exact', 'approximate', 'fallback', 'unparsed'];
const STAGES = ['parsed', 'resolved', 'rendered', 'fidelity-tested', 'not-applicable'];
const SOURCE_STATUSES = ['verified', 'unavailable'];
const OFFICIAL_HOSTS = ['learn.microsoft.com', 'ecma-international.org'];
const IDENTIFIER_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const SCALAR_TOKENS = [
  '@@CATALOG_FEATURE_COUNT@@',
  '@@CATALOG_DIMENSION_COUNT@@',
  '@@CATALOG_EXACT_COUNT@@',
  '@@CATALOG_SOURCE_SHA256@@',
];
const TIER_MARKER = '<!-- CATALOG_TIER_SUMMARY -->';
const FAMILY_MARKER = '<!-- CATALOG_FAMILY_SECTIONS -->';

function countOccurrences(text, needle) {
  let count = 0;
  let index = text.indexOf(needle);
  while (index !== -1) {
    count += 1;
    index = text.indexOf(needle, index + needle.length);
  }
  return count;
}

function fail(message) {
  throw new Error(message);
}

function assertExactArray(name, actual, expected) {
  if (!Array.isArray(actual) || actual.length !== expected.length) {
    fail(`${name} must equal ${expected.join(', ')}`);
  }
  for (let index = 0; index < expected.length; index += 1) {
    if (actual[index] !== expected[index]) {
      fail(`${name} must equal ${expected.join(', ')}`);
    }
  }
}

function assertString(value, name) {
  if (typeof value !== 'string' || value.length === 0) {
    fail(`${name} must be a non-empty string`);
  }
  return value;
}

function assertIdentifier(value, name) {
  const text = assertString(value, name);
  if (!IDENTIFIER_RE.test(text)) {
    fail(`${name} must be a lowercase hyphen identifier`);
  }
  return text;
}

function assertPlainObject(value, name) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    fail(`${name} must be an object`);
  }
  return value;
}

function assertEnum(value, allowed, name) {
  const text = assertString(value, name);
  if (!allowed.includes(text)) {
    fail(`${name} must be one of ${allowed.join(', ')}`);
  }
  return text;
}

function assertDispositionSet(value, name) {
  const object = assertPlainObject(value, name);
  const keys = Object.keys(object);
  if (keys.length !== DIMENSIONS.length || !DIMENSIONS.every((dimension) => keys.includes(dimension))) {
    fail(`${name} dimensions must exactly equal ${DIMENSIONS.join(', ')}`);
  }
  const result = {};
  for (const dimension of DIMENSIONS) {
    const disposition = assertPlainObject(object[dimension], `${name}.${dimension}`);
    result[dimension] = {
      tier: assertEnum(disposition.tier, TIERS, `${name}.${dimension}.tier`),
      stage: assertEnum(disposition.stage, STAGES, `${name}.${dimension}.stage`),
    };
  }
  return result;
}

function validateOfficialSource(value, name) {
  const source = assertString(value, name);
  let url;
  try {
    url = new URL(source);
  } catch (error) {
    throw new Error(`${name} must be a valid URL`, { cause: error });
  }
  if (
    url.protocol !== 'https:' ||
    url.username !== '' ||
    url.password !== '' ||
    !OFFICIAL_HOSTS.includes(url.hostname) ||
    url.port !== ''
  ) {
    fail(`${name} must be a safe official HTTPS URL`);
  }
  return url.href;
}

function validateOoxml(value, name) {
  const object = assertPlainObject(value, name);
  const result = {};
  if (Object.hasOwn(object, 'qualified_name')) {
    result.qualified_name = assertString(object.qualified_name, `${name}.qualified_name`);
  }
  if (Object.hasOwn(object, 'relationship_type')) {
    result.relationship_type = assertString(object.relationship_type, `${name}.relationship_type`);
  }
  return result;
}

function validateManifest(manifest) {
  const object = assertPlainObject(manifest, 'manifest');
  assertExactArray('dimensions', object.dimensions, DIMENSIONS);
  assertExactArray('tiers', object.tiers, TIERS);
  assertExactArray('stages', object.stages, STAGES);
  if (!Array.isArray(object.features) || object.features.length === 0) {
    fail('features must be a non-empty array');
  }

  const seenIds = new Set();
  const features = object.features.map((feature, index) => {
    const rawFeature = assertPlainObject(feature, `features[${index}]`);
    const id = assertIdentifier(rawFeature.id, `features[${index}].id`);
    if (seenIds.has(id)) {
      fail(`duplicate feature id: ${id}`);
    }
    seenIds.add(id);
    const family = assertIdentifier(rawFeature.family, `features[${index}].family`);
    const fallbackPolicy = assertPlainObject(
      rawFeature.fallback_policy,
      `features[${index}].fallback_policy`,
    );
    return {
      id,
      family,
      official_source: validateOfficialSource(
        rawFeature.official_source,
        `features[${index}].official_source`,
      ),
      source_status: assertEnum(
        rawFeature.source_status,
        SOURCE_STATUSES,
        `features[${index}].source_status`,
      ),
      ooxml: validateOoxml(rawFeature.ooxml, `features[${index}].ooxml`),
      fallback_policy: {
        kind: assertString(
          fallbackPolicy.kind,
          `features[${index}].fallback_policy.kind`,
        ),
        diagnostic_code: assertString(
          fallbackPolicy.diagnostic_code,
          `features[${index}].fallback_policy.diagnostic_code`,
        ),
      },
      current: assertDispositionSet(rawFeature.current, `features[${index}].current`),
      target: assertDispositionSet(rawFeature.target, `features[${index}].target`),
    };
  });

  return {
    dimensions: [...DIMENSIONS],
    tiers: [...TIERS],
    stages: [...STAGES],
    features,
  };
}

function validateTemplate(templateText) {
  for (const token of SCALAR_TOKENS) {
    if (countOccurrences(templateText, token) !== 1) {
      fail(`template token ${token} must occur exactly once`);
    }
  }
  for (const marker of [TIER_MARKER, FAMILY_MARKER]) {
    if (countOccurrences(templateText, marker) !== 1) {
      fail(`template marker ${marker} must occur exactly once`);
    }
  }
  const reservedIdPattern = /\bid\s*=\s*(['"])(?:family-|capability-)[^'"]*\1/;
  if (reservedIdPattern.test(templateText)) {
    fail('template contains reserved catalog id');
  }
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function compareCodePoint(left, right) {
  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }
  return 0;
}

function tierCounts(features) {
  const counts = Object.fromEntries(TIERS.map((tier) => [tier, 0]));
  for (const feature of features) {
    for (const dimension of DIMENSIONS) {
      counts[feature.current[dimension].tier] += 1;
    }
  }
  return counts;
}

function groupByFamily(features) {
  const groups = new Map();
  for (const feature of features) {
    const group = groups.get(feature.family) ?? [];
    group.push(feature);
    groups.set(feature.family, group);
  }
  return [...groups.entries()]
    .sort(([left], [right]) => compareCodePoint(left, right))
    .map(([family, familyFeatures]) => [
      family,
      familyFeatures.sort((left, right) => compareCodePoint(left.id, right.id)),
    ]);
}

function renderTierSummary(features) {
  const counts = tierCounts(features);
  return TIERS.map(
    (tier) => `<li data-tier="${tier}" data-tier-count="${counts[tier]}">
<span class="tier-name">${escapeHtml(tier)}</span>
<span class="tier-count mono">${counts[tier]}</span>
</li>`,
  ).join('\n');
}

function renderOoxmlBinding(feature) {
  const parts = [];
  if (Object.hasOwn(feature.ooxml, 'qualified_name')) {
    const value = escapeHtml(feature.ooxml.qualified_name);
    parts.push(`<dt>Qualified name</dt><dd data-ooxml-qualified-name="${value}">${value}</dd>`);
  }
  if (Object.hasOwn(feature.ooxml, 'relationship_type')) {
    const value = escapeHtml(feature.ooxml.relationship_type);
    parts.push(`<dt>Relationship type</dt><dd data-ooxml-relationship-type="${value}">${value}</dd>`);
  }
  if (parts.length === 0) {
    parts.push('<dt>OOXML binding</dt><dd data-ooxml-not-declared>Not declared in manifest</dd>');
  }
  return `<dl class="capability-ooxml" data-ooxml-binding>
${parts.join('\n')}
</dl>`;
}

function renderDispositionRows(feature) {
  return DIMENSIONS.map((dimension) => {
    const current = feature.current[dimension];
    const target = feature.target[dimension];
    return `<tr data-dimension="${dimension}">
<th scope="row">${escapeHtml(dimension)}</th>
<td data-disposition="current" data-tier="${current.tier}" data-stage="${current.stage}">
<span class="tier-value">${escapeHtml(current.tier)}</span>
<span class="stage-value mono">${escapeHtml(current.stage)}</span>
</td>
<td data-disposition="target" data-tier="${target.tier}" data-stage="${target.stage}">
<span class="tier-value">${escapeHtml(target.tier)}</span>
<span class="stage-value mono">${escapeHtml(target.stage)}</span>
</td>
</tr>`;
  }).join('\n');
}

function renderFeature(feature) {
  const id = escapeHtml(feature.id);
  const family = escapeHtml(feature.family);
  const sourceStatus = escapeHtml(feature.source_status);
  const sourceLink = escapeHtml(feature.official_source);
  const fallbackKind = escapeHtml(feature.fallback_policy.kind);
  const diagnosticCode = escapeHtml(feature.fallback_policy.diagnostic_code);
  const crossValidation =
    feature.source_status === 'unavailable'
      ? '<span class="source-warning" data-cross-validation-required>Cross-validation required</span>'
      : '';

  return `<article class="capability-record" id="capability-${id}" data-capability-id="${id}" data-capability-family="${family}" data-source-status="${sourceStatus}">
<div class="record-head">
<h3>${id}</h3>
<p class="record-family mono">${family}</p>
</div>
<div class="source-row">
<a data-official-source href="${sourceLink}">Official source</a>
${crossValidation}
</div>
${renderOoxmlBinding(feature)}
<table class="disposition-table">
<thead>
<tr><th scope="col">Dimension</th><th scope="col">Current</th><th scope="col">Target</th></tr>
</thead>
<tbody>
${renderDispositionRows(feature)}
</tbody>
</table>
<dl class="fallback-policy" data-fallback-kind="${fallbackKind}" data-diagnostic-code="${diagnosticCode}">
<dt>Fallback kind</dt><dd>${fallbackKind}</dd>
<dt>Diagnostic code</dt><dd class="mono">${diagnosticCode}</dd>
</dl>
</article>`;
}

function renderFamilySections(features) {
  return groupByFamily(features).map(([family, familyFeatures]) => {
    const safeFamily = escapeHtml(family);
    return `<section class="capability-family" id="family-${safeFamily}" data-capability-family="${safeFamily}" data-feature-count="${familyFeatures.length}">
<div class="family-heading">
<h2>${safeFamily}</h2>
<a href="#family-${safeFamily}" class="family-anchor">#</a>
<span class="family-count mono">${familyFeatures.length} capabilities</span>
</div>
${familyFeatures.map(renderFeature).join('\n')}
</section>`;
  }).join('\n');
}

function replaceOnce(text, needle, replacement) {
  return text.replace(needle, replacement);
}

function ensureCatalogMainAttributes(html, manifest, counts, sourceSha256) {
  const mainPattern = /<main\b[^>]*\bid="capabilityCatalog"[^>]*>/;
  const match = html.match(mainPattern);
  if (!match) {
    fail('template must contain main#capabilityCatalog');
  }
  const tag = match[0];
  const attributes = {
    'data-feature-count': String(manifest.features.length),
    'data-current-dimension-count': String(manifest.features.length * manifest.dimensions.length),
    'data-exact-dimensions': String(counts.exact),
    'data-source-sha256': sourceSha256,
  };
  let nextTag = tag;
  for (const [name, value] of Object.entries(attributes)) {
    const attributePattern = new RegExp(`\\b${name}="[^"]*"`);
    if (attributePattern.test(nextTag)) {
      nextTag = nextTag.replace(attributePattern, `${name}="${value}"`);
    } else {
      nextTag = nextTag.replace(/>$/, ` ${name}="${value}">`);
    }
  }
  return html.replace(tag, nextTag);
}

export function renderCapabilityCatalog({ manifestBytes, templateText }) {
  if (!(manifestBytes instanceof Uint8Array)) {
    fail('manifestBytes must be a Uint8Array');
  }
  if (typeof templateText !== 'string') {
    fail('templateText must be a string');
  }
  validateTemplate(templateText);

  let manifest;
  try {
    manifest = JSON.parse(Buffer.from(manifestBytes).toString('utf8'));
  } catch (error) {
    throw new Error('manifest JSON could not be parsed', { cause: error });
  }
  const validatedManifest = validateManifest(manifest);
  const counts = tierCounts(validatedManifest.features);
  const sourceSha256 = createHash('sha256').update(manifestBytes).digest('hex');
  let html = templateText;
  html = replaceOnce(html, '@@CATALOG_FEATURE_COUNT@@', String(validatedManifest.features.length));
  html = replaceOnce(
    html,
    '@@CATALOG_DIMENSION_COUNT@@',
    String(validatedManifest.features.length * validatedManifest.dimensions.length),
  );
  html = replaceOnce(html, '@@CATALOG_EXACT_COUNT@@', String(counts.exact));
  html = replaceOnce(html, '@@CATALOG_SOURCE_SHA256@@', sourceSha256);
  html = replaceOnce(html, TIER_MARKER, renderTierSummary(validatedManifest.features));
  html = replaceOnce(html, FAMILY_MARKER, renderFamilySections(validatedManifest.features));
  html = ensureCatalogMainAttributes(html, validatedManifest, counts, sourceSha256);
  if (/@@CATALOG_|CATALOG_TIER_SUMMARY|CATALOG_FAMILY_SECTIONS/.test(html)) {
    fail('template interpolation left leftover catalog marker');
  }
  return html;
}

export async function writeCapabilityCatalog({ manifestPath, templatePath, outputPath }) {
  const parent = path.dirname(outputPath);
  const parentStat = await stat(parent).catch((error) => {
    throw new Error(`output parent does not exist: ${parent}`, { cause: error });
  });
  if (!parentStat.isDirectory()) {
    fail(`output parent is not a directory: ${parent}`);
  }
  await access(parent, constants.W_OK);

  const [manifestBytes, templateText] = await Promise.all([
    readFile(manifestPath),
    readFile(templatePath, 'utf8'),
  ]);
  const html = renderCapabilityCatalog({ manifestBytes, templateText });
  const tempPath = `${outputPath}.tmp-${process.pid}`;
  try {
    await writeFile(tempPath, html, 'utf8');
    await rename(tempPath, outputPath);
  } catch (error) {
    await unlink(tempPath).catch((unlinkError) => {
      if (unlinkError.code !== 'ENOENT') {
        throw unlinkError;
      }
    });
    throw error;
  }
}

function parseCliArgs(argv) {
  const expected = ['--manifest', '--template', '--output'];
  if (argv.length !== expected.length * 2) {
    fail('usage: render_demo_capabilities.mjs --manifest <path> --template <path> --output <path>');
  }
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const option = argv[index];
    const value = argv[index + 1];
    if (!expected.includes(option)) {
      fail(`unknown option: ${option}`);
    }
    if (values.has(option)) {
      fail(`duplicate option: ${option}`);
    }
    values.set(option, value);
  }
  for (const option of expected) {
    if (!values.has(option)) {
      fail(`missing option: ${option}`);
    }
  }
  return {
    manifestPath: values.get('--manifest'),
    templatePath: values.get('--template'),
    outputPath: values.get('--output'),
  };
}

async function main() {
  const args = parseCliArgs(process.argv.slice(2));
  await writeCapabilityCatalog(args);
}

if (import.meta.url === pathToFileURL(path.resolve(process.argv[1] ?? '')).href) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
