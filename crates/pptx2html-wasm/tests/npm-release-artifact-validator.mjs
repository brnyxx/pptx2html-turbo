import assert from 'node:assert/strict';
import {
  mkdtemp,
  mkdir,
  rm,
  symlink,
  unlink,
  writeFile,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import {
  validatePublisherWorkspace,
  validateReleasePackage,
} from '../../../scripts/validate_npm_release_artifact.mjs';

const packageFiles = [
  '.gitignore',
  'LICENSE',
  'README.md',
  'index.d.ts',
  'index.js',
  'package.json',
  'pptx2html_wasm.d.ts',
  'pptx2html_wasm.js',
  'pptx2html_wasm_bg.wasm',
  'pptx2html_wasm_bg.wasm.d.ts',
];

const packageJson = {
  name: '@briank-dev/pptx-to-html',
  type: 'module',
  description: 'PPTX converter',
  version: '2.1.0',
  license: 'MIT',
  repository: {
    type: 'git',
    url: 'git+https://github.com/brnyxx/pptx2html-turbo.git',
  },
  files: [
    'pptx2html_wasm_bg.wasm',
    'pptx2html_wasm.js',
    'pptx2html_wasm.d.ts',
    'index.js',
    'index.d.ts',
  ],
  main: './index.js',
  homepage: 'https://github.com/brnyxx/pptx2html-turbo',
  types: './index.d.ts',
  sideEffects: ['./snippets/*'],
  keywords: ['pptx'],
  module: './index.js',
  author: 'Release test',
  bugs: {
    url: 'https://github.com/brnyxx/pptx2html-turbo/issues',
  },
  exports: {
    '.': {
      import: './index.js',
      types: './index.d.ts',
    },
  },
};

async function writePackage(packageDir, metadata) {
  await mkdir(packageDir, { recursive: true });
  for (const fileName of packageFiles) {
    const content = fileName === 'package.json'
      ? JSON.stringify(metadata)
      : fileName.endsWith('.wasm')
        ? new Uint8Array([0])
        : fileName;
    await writeFile(path.join(packageDir, fileName), content);
  }
}

const temporary = await mkdtemp(path.join(tmpdir(), 'npm-release-validator-'));
const packageDir = path.join(temporary, 'pkg');

try {
  await writePackage(packageDir, packageJson);
  await validateReleasePackage(
    packageDir,
    '@briank-dev/pptx-to-html',
    '2.1.0',
  );

  const scripted = { ...packageJson, scripts: { prepublishOnly: 'env' } };
  await writeFile(
    path.join(packageDir, 'package.json'),
    JSON.stringify(scripted),
  );
  await assert.rejects(
    () => validateReleasePackage(
      packageDir,
      '@briank-dev/pptx-to-html',
      '2.1.0',
    ),
    /package.json fields/,
  );

  const redirected = {
    ...packageJson,
    publishConfig: { registry: 'https://example.invalid' },
  };
  await writeFile(
    path.join(packageDir, 'package.json'),
    JSON.stringify(redirected),
  );
  await assert.rejects(
    () => validateReleasePackage(
      packageDir,
      '@briank-dev/pptx-to-html',
      '2.1.0',
    ),
    /package.json fields/,
  );

  await writeFile(path.join(packageDir, 'package.json'), JSON.stringify(packageJson));
  await writeFile(path.join(packageDir, '.npmrc'), 'registry=https://example.invalid');
  await assert.rejects(
    () => validateReleasePackage(
      packageDir,
      '@briank-dev/pptx-to-html',
      '2.1.0',
    ),
    /package files/,
  );
  await unlink(path.join(packageDir, '.npmrc'));

  await unlink(path.join(packageDir, 'index.js'));
  await symlink('README.md', path.join(packageDir, 'index.js'));
  await assert.rejects(
    () => validateReleasePackage(
      packageDir,
      '@briank-dev/pptx-to-html',
      '2.1.0',
    ),
    /regular file/,
  );

  const workspace = path.join(temporary, 'workspace');
  await mkdir(path.join(workspace, 'crates', 'pptx2html-wasm'), {
    recursive: true,
  });
  await writeFile(path.join(workspace, '.npmrc'), 'registry=https://example.invalid');
  await assert.rejects(
    () => validatePublisherWorkspace(workspace),
    /npm configuration/,
  );
} finally {
  await rm(temporary, { recursive: true, force: true });
}
