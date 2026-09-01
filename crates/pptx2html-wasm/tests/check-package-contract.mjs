import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';
import path from 'node:path';

const packageDir = process.argv[2] ?? 'crates/pptx2html-wasm/pkg';
const expectedVersion = process.argv[3];
const expectedPackageName =
  process.argv[4] ?? '@briank-dev/pptx-to-html';

if (!expectedVersion) {
  throw new Error('expected version argument is required');
}

const packageJsonPath = path.join(packageDir, 'package.json');
const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));

assert.equal(packageJson.name, expectedPackageName);
assert.equal(packageJson.version, expectedVersion);
assert.equal(packageJson.exports['.'].import, './index.js');
assert.equal(packageJson.exports['.'].types, './index.d.ts');
assert.equal(packageJson.main, './index.js');
assert.equal(packageJson.module, './index.js');
assert.equal(packageJson.types, './index.d.ts');
assert.ok(packageJson.files.includes('index.js'));
assert.ok(packageJson.files.includes('index.d.ts'));
assert.equal(packageJson.homepage, 'https://github.com/brnyxx/pptx2html-turbo');
assert.equal(packageJson.bugs.url, 'https://github.com/brnyxx/pptx2html-turbo/issues');
assert.equal('scripts' in packageJson, false);
assert.equal('publishConfig' in packageJson, false);
assert.equal('config' in packageJson, false);

const declarations = await readFile(path.join(packageDir, 'pptx2html_wasm.d.ts'), 'utf8');
const facadeDeclarations = await readFile(path.join(packageDir, 'index.d.ts'), 'utf8');
const normalizedFacadeDeclarations = facadeDeclarations.replace(/\s+/g, ' ');
assert.match(declarations, /readonly diagnosticsJson: string/);
assert.match(declarations, /readonly diagnostics: string/);
assert.match(declarations, /readonly unresolvedElements: string/);
assert.match(
  normalizedFacadeDeclarations,
  /function pptxToHtml\(\s*input: PptxInput,\s*moduleOrPath\?: InitInput \| Promise<InitInput>,?\s*\): Promise<string>/,
);

for (const fileName of [
  'README.md',
  'LICENSE',
  'index.js',
  'index.d.ts',
  'pptx2html_wasm.js',
  'pptx2html_wasm.d.ts',
  'pptx2html_wasm_bg.wasm',
]) {
  await access(path.join(packageDir, fileName));
}
