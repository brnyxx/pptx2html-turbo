import { lstat, readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const allowedPackageFiles = [
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

const allowedPackageJsonFields = [
  'author',
  'bugs',
  'description',
  'exports',
  'files',
  'homepage',
  'keywords',
  'license',
  'main',
  'module',
  'name',
  'repository',
  'sideEffects',
  'type',
  'types',
  'version',
];

const publishedFiles = [
  'index.d.ts',
  'index.js',
  'pptx2html_wasm.d.ts',
  'pptx2html_wasm.js',
  'pptx2html_wasm_bg.wasm',
];

function requireEqual(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label} mismatch`);
  }
}

export async function validateReleasePackage(
  packageDir,
  expectedName,
  expectedVersion,
) {
  const packageMetadata = await lstat(packageDir);
  if (!packageMetadata.isDirectory()) {
    throw new Error(`package root is not a directory: ${packageDir}`);
  }
  const entries = await readdir(packageDir, { withFileTypes: true });
  requireEqual(
    entries.map((entry) => entry.name).sort(),
    allowedPackageFiles,
    'package files',
  );
  for (const entry of entries) {
    const metadata = await lstat(path.join(packageDir, entry.name));
    if (!metadata.isFile()) {
      throw new Error(`package entry is not a regular file: ${entry.name}`);
    }
  }

  const packageJson = JSON.parse(
    await readFile(path.join(packageDir, 'package.json'), 'utf8'),
  );
  requireEqual(
    Object.keys(packageJson).sort(),
    allowedPackageJsonFields,
    'package.json fields',
  );
  requireEqual(packageJson.name, expectedName, 'package name');
  requireEqual(packageJson.version, expectedVersion, 'package version');
  requireEqual(packageJson.type, 'module', 'package type');
  requireEqual(packageJson.license, 'MIT', 'package license');
  requireEqual(packageJson.files.slice().sort(), publishedFiles, 'published files');
  requireEqual(packageJson.main, './index.js', 'package main');
  requireEqual(packageJson.module, './index.js', 'package module');
  requireEqual(packageJson.types, './index.d.ts', 'package types');
  requireEqual(
    packageJson.repository,
    {
      type: 'git',
      url: 'git+https://github.com/brnyxx/pptx2html-turbo.git',
    },
    'package repository',
  );
  requireEqual(
    packageJson.homepage,
    'https://github.com/brnyxx/pptx2html-turbo',
    'package homepage',
  );
  requireEqual(
    packageJson.bugs,
    { url: 'https://github.com/brnyxx/pptx2html-turbo/issues' },
    'package bugs',
  );
  requireEqual(packageJson.sideEffects, ['./snippets/*'], 'package side effects');
  requireEqual(
    packageJson.exports,
    {
      '.': {
        import: './index.js',
        types: './index.d.ts',
      },
    },
    'package exports',
  );
}

export async function validatePublisherWorkspace(workspaceRoot) {
  for (const relativePath of [
    '.npmrc',
    'crates/.npmrc',
    'crates/pptx2html-wasm/.npmrc',
  ]) {
    try {
      await lstat(path.join(workspaceRoot, relativePath));
    } catch (error) {
      if (error && typeof error === 'object' && error.code === 'ENOENT') {
        continue;
      }
      throw error;
    }
    throw new Error(`unexpected npm configuration: ${relativePath}`);
  }
}

const invokedPath = process.argv[1]
  ? pathToFileURL(path.resolve(process.argv[1])).href
  : '';
if (import.meta.url === invokedPath) {
  const [expectedVersion, primaryName, legacyName] = process.argv.slice(2);
  if (!expectedVersion || !primaryName || !legacyName) {
    throw new Error('usage: validate_npm_release_artifact.mjs <version> <primary-name> <legacy-name>');
  }
  await validatePublisherWorkspace(process.cwd());
  await validateReleasePackage(
    'crates/pptx2html-wasm/pkg',
    primaryName,
    expectedVersion,
  );
  await validateReleasePackage(
    'crates/pptx2html-wasm/pkg-legacy',
    legacyName,
    expectedVersion,
  );
}
