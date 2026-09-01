import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const demoPath = fileURLToPath(new URL('../demo/index.html', import.meta.url));
const html = await readFile(demoPath, 'utf8');
const manifestPath = fileURLToPath(new URL('../Cargo.toml', import.meta.url));
const manifestVersion = (await readFile(manifestPath, 'utf8')).match(
  /^version = "(.+)"$/m,
)?.[1];
assert.ok(manifestVersion, 'manifest version must be readable');
const expectedVersion = process.argv[2] ?? manifestVersion;

function tagWithId(id) {
  const match = html.match(new RegExp(`<[^>]+\\bid="${id}"[^>]*>`));
  assert.ok(match, `missing element #${id}`);
  return match[0];
}

function attribute(tag, name) {
  const match = tag.match(new RegExp(`\\b${name}="([^"]*)"`));
  assert.ok(match, `missing ${name} on ${tag}`);
  return match[1];
}

const frameTag = tagWithId('output');
const sandbox = new Set(attribute(frameTag, 'sandbox').split(/\s+/).filter(Boolean));
assert.ok(sandbox.has('allow-scripts'), 'output frame must execute renderer-owned runtimes');
assert.ok(!sandbox.has('allow-same-origin'), 'output frame must keep an opaque origin');
assert.equal(attribute(frameTag, 'title'), 'Converted slide output');

const statusTag = tagWithId('status');
assert.equal(attribute(statusTag, 'role'), 'status');
assert.equal(attribute(statusTag, 'aria-live'), 'polite');
assert.doesNotMatch(html, /\.status\s*\{[^}]*display:\s*none/s);

const controlsTag = tagWithId('controls');
assert.equal(attribute(controlsTag, 'role'), 'group');
assert.equal(attribute(controlsTag, 'aria-labelledby'), 'scaleLabel');
assert.equal(attribute(tagWithId('scaleLabel'), 'for'), 'scaleRange');

const rangeTag = tagWithId('scaleRange');
assert.equal(attribute(rangeTag, 'min'), '0.1');
assert.equal(attribute(rangeTag, 'step'), '0.01');
assert.equal(attribute(rangeTag, 'aria-label'), 'Slide zoom slider');
assert.equal(attribute(rangeTag, 'aria-describedby'), 'scaleHint');

const numberTag = tagWithId('scaleNumber');
assert.equal(attribute(numberTag, 'min'), '0.1');
assert.equal(attribute(numberTag, 'step'), '0.01');
assert.equal(attribute(numberTag, 'aria-label'), 'Slide zoom value');
assert.equal(attribute(numberTag, 'aria-describedby'), 'scaleHint');

assert.match(html, /\bconvert_with_options_metadata\b/);
assert.match(html, /JSON\.parse\(result\.diagnostics\)/);
assert.match(html, /currentDiagnostics\.length/);
assert.match(html, /result\.free\(\)/);
assert.match(html, /meta\.replaceChildren\(/);
assert.doesNotMatch(html, /meta\.innerHTML\s*=/);
assert.match(html, /fitScaleToContainer\(\)/);
assert.match(html, /syncScaleInputs\(fitScaleToContainer\(\)\)/);
assert.match(html, /const RENDERED_DOCUMENT_HORIZONTAL_PADDING = 40;/);
assert.match(
  html,
  /availableWidth - RENDERED_DOCUMENT_HORIZONTAL_PADDING/,
);
assert.match(html, /controls\.style\.display = 'none';\s+currentBuffer = null;\s+currentInfo = null;/);
assert.match(html, /const MAX_PPTX_BYTES = 64 \* 1024 \* 1024;/);
const sizeGuardIndex = html.indexOf('if (file.size > MAX_PPTX_BYTES)');
const fileReadIndex = html.indexOf('await file.arrayBuffer()');
assert.ok(sizeGuardIndex >= 0, 'demo must reject oversized PPTX files');
assert.ok(fileReadIndex >= 0, 'demo must read accepted files');
assert.ok(sizeGuardIndex < fileReadIndex, 'size guard must run before file allocation');
const canonicalUrl = 'https://brnyxx.github.io/pptx2html-turbo/';
assert.ok(
  html.includes(`<link rel="canonical" href="${canonicalUrl}">`),
  'demo must identify its canonical Pages URL',
);

const latestReleaseUrl = 'https://github.com/brnyxx/pptx2html-turbo/releases/latest';
assert.equal(
  html.split(latestReleaseUrl).length - 1,
  2,
  'header and footer must link to the latest published release',
);

const versions = new Set(
  [...html.matchAll(/\bv(\d+\.\d+\.\d+)\b/g)].map((match) => match[1]),
);
assert.deepEqual([...versions], [expectedVersion]);
