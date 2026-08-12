import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';
import path from 'node:path';

const packageDir = process.argv[2] ?? 'crates/pptx2html-wasm/pkg';
const expectedVersion = process.argv[3];

if (!expectedVersion) {
  throw new Error('expected version argument is required');
}

const packageJsonPath = path.join(packageDir, 'package.json');
const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));

assert.equal(packageJson.name, '@briank-dev/pptx2html-turbo');
assert.equal(packageJson.version, expectedVersion);
assert.equal(packageJson.exports['.'].import, './pptx2html_wasm.js');
assert.equal(packageJson.exports['.'].types, './pptx2html_wasm.d.ts');
assert.equal(packageJson.homepage, 'https://github.com/kim62210/pptx2html-turbo');
assert.equal(packageJson.bugs.url, 'https://github.com/kim62210/pptx2html-turbo/issues');

const declarations = await readFile(path.join(packageDir, 'pptx2html_wasm.d.ts'), 'utf8');
assert.match(declarations, /readonly diagnosticsJson: string/);
assert.match(declarations, /readonly diagnostics: string/);
assert.match(declarations, /readonly unresolvedElements: string/);

for (const fileName of [
  'README.md',
  'LICENSE',
  'pptx2html_wasm.js',
  'pptx2html_wasm.d.ts',
  'pptx2html_wasm_bg.wasm',
]) {
  await access(path.join(packageDir, fileName));
}
