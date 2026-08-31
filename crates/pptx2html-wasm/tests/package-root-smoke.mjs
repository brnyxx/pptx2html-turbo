import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

const packageDir = process.argv[3]
  ? path.resolve(process.argv[3])
  : fileURLToPath(new URL('../pkg', import.meta.url));
const fixturePath = fileURLToPath(
  new URL('../../pptx2html-cli/tests/fixtures/two-slides.pptx', import.meta.url),
);
const tempDir = await mkdtemp(path.join(tmpdir(), 'pptx2html-wasm-root-smoke-'));
const expectedPackageName =
  process.argv[2] ?? '@briank-dev/pptx-to-html';
const [scopeName, packageName] = expectedPackageName.split('/');
assert.equal(scopeName, '@briank-dev');
assert.ok(packageName);
const scopeDir = path.join(tempDir, 'node_modules', scopeName);
const packageLink = path.join(scopeDir, packageName);

await mkdir(scopeDir, { recursive: true });
await symlink(packageDir, packageLink, 'dir');

const script = `
  import assert from 'node:assert/strict';
  import { readFile } from 'node:fs/promises';
  import init, {
    convert,
    convert_with_options_metadata,
    get_info,
    pptxToHtml,
  } from ${JSON.stringify(expectedPackageName)};

  const wasmBytes = await readFile(
    new URL(
      './node_modules/${expectedPackageName}/pptx2html_wasm_bg.wasm',
      import.meta.url,
    ),
  );

  assert.equal(typeof init, 'function');
  assert.equal(typeof convert, 'function');
  assert.equal(typeof convert_with_options_metadata, 'function');
  assert.equal(typeof get_info, 'function');
  assert.equal(typeof pptxToHtml, 'function');

  const fixture = new Uint8Array(
    await readFile(${JSON.stringify(fixturePath)}),
  );
  const [html, concurrentHtml] = await Promise.all([
    pptxToHtml(new Blob([fixture]), wasmBytes),
    pptxToHtml(fixture),
  ]);
  assert.match(html, /^<!DOCTYPE html>/);
  assert.equal(concurrentHtml, html);
  assert.equal(await pptxToHtml(fixture.buffer), html);
  await assert.rejects(
    () => pptxToHtml('not PPTX bytes'),
    /Blob, ArrayBuffer, or Uint8Array/,
  );
  class OversizedBlob extends Blob {
    get size() {
      return 64 * 1024 * 1024 + 1;
    }

    arrayBuffer() {
      throw new Error('oversized Blob must not be read');
    }
  }
  await assert.rejects(
    () => pptxToHtml(new OversizedBlob()),
    /64 MiB/,
  );

  const invalidData = new Uint8Array([0, 1, 2, 3]);
  assert.throws(() => convert(invalidData), /invalid|zip|PPTX|archive/i);
  assert.throws(() => get_info(invalidData), /invalid|zip|PPTX|archive/i);
`;

await execFileAsync('node', ['--input-type=module', '--eval', script], {
  cwd: tempDir,
});

const retryScript = `
  import assert from 'node:assert/strict';
  import { readFile } from 'node:fs/promises';
  import { pptxToHtml } from ${JSON.stringify(expectedPackageName)};

  const wasmBytes = await readFile(
    new URL(
      './node_modules/${expectedPackageName}/pptx2html_wasm_bg.wasm',
      import.meta.url,
    ),
  );
  const fixture = new Uint8Array(
    await readFile(${JSON.stringify(fixturePath)}),
  );

  const [firstFailure, sharedFailure] = await Promise.allSettled([
    pptxToHtml(
      fixture,
      Promise.reject(new Error('transient init failure')),
    ),
    pptxToHtml(fixture, wasmBytes),
  ]);
  assert.equal(firstFailure.status, 'rejected');
  assert.match(firstFailure.reason.message, /transient init failure/);
  assert.equal(sharedFailure.status, 'rejected');
  assert.match(sharedFailure.reason.message, /transient init failure/);
  assert.match(
    await pptxToHtml(fixture, wasmBytes),
    /^<!DOCTYPE html>/,
  );
`;

await execFileAsync('node', ['--input-type=module', '--eval', retryScript], {
  cwd: tempDir,
});
