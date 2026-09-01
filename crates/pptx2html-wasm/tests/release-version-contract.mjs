import assert from 'node:assert/strict';
import { cp, mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url));
const tempRoot = await mkdtemp(join(tmpdir(), 'pptx2html-release-version-'));
const copiedFiles = [
  'scripts/read_release_version.sh',
  'crates/document2html-core/Cargo.toml',
  'crates/document2html-native/Cargo.toml',
  'crates/document2html-py/Cargo.toml',
  'crates/document2html-wasm/Cargo.toml',
  'crates/pptx2html-core/Cargo.toml',
  'crates/pptx2html-cli/Cargo.toml',
  'crates/pptx2html-py/Cargo.toml',
  'crates/pptx2html-wasm/Cargo.toml',
  'crates/pptx2html-py/pyproject.toml',
  'crates/pptx2html-wasm/demo/index.html',
];

try {
  for (const relativePath of copiedFiles) {
    const destination = join(tempRoot, relativePath);
    await mkdir(dirname(destination), { recursive: true });
    await cp(join(repoRoot, relativePath), destination);
  }

  // Derive the tag from the manifest so a version bump cannot break this
  // contract: hardcoding a version made every release fail the demo check
  // at the earlier manifest check instead.
  const manifest = await readFile(
    join(tempRoot, 'crates/pptx2html-wasm/Cargo.toml'),
    'utf8',
  );
  const manifestVersion = manifest.match(/^version = "(.+)"$/m)?.[1];
  assert.ok(manifestVersion, 'manifest version must be readable');
  const releaseTag = `v${manifestVersion}`;

  const invalidTagResult = spawnSync(
    'bash',
    [join(tempRoot, 'scripts/read_release_version.sh'), `${releaseTag};echo injected`],
    { encoding: 'utf8' },
  );
  assert.notEqual(
    invalidTagResult.status,
    0,
    'invalid release tag syntax must fail validation',
  );
  assert.match(invalidTagResult.stderr, /must match vMAJOR\.MINOR\.PATCH/);

  const universalManifestPath = join(
    tempRoot,
    'crates/document2html-core/Cargo.toml',
  );
  const universalManifest = await readFile(universalManifestPath, 'utf8');
  await writeFile(
    universalManifestPath,
    universalManifest.replace(
      `version = "${manifestVersion}"`,
      'version = "9.9.9"',
    ),
  );
  const universalDriftResult = spawnSync(
    'bash',
    [join(tempRoot, 'scripts/read_release_version.sh'), releaseTag],
    { encoding: 'utf8' },
  );
  assert.notEqual(
    universalDriftResult.status,
    0,
    'universal package version drift must fail release validation',
  );
  assert.match(universalDriftResult.stderr, /version mismatch/i);
  await writeFile(universalManifestPath, universalManifest);

  const demoPath = join(tempRoot, 'crates/pptx2html-wasm/demo/index.html');
  const demo = await readFile(demoPath, 'utf8');
  await writeFile(demoPath, demo.replaceAll(releaseTag, 'v9.9.9'));

  const result = spawnSync(
    'bash',
    [join(tempRoot, 'scripts/read_release_version.sh'), releaseTag],
    { encoding: 'utf8' },
  );

  assert.notEqual(result.status, 0, 'demo version drift must fail release validation');
  assert.match(result.stderr, /demo version/i);
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}
