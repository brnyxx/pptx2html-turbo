import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import init, {
  convert,
  convert_slides,
  convert_with_metadata,
  convert_with_options,
  get_info,
  get_presentation_info,
  get_slide_count,
} from '../pkg/pptx2html_wasm.js';

const wasmBytes = await readFile(new URL('../pkg/pptx2html_wasm_bg.wasm', import.meta.url));
await init({ module_or_path: wasmBytes });

assert.equal(typeof convert, 'function');
assert.equal(typeof convert_slides, 'function');
assert.equal(typeof convert_with_options, 'function');
assert.equal(typeof convert_with_metadata, 'function');
assert.equal(typeof get_info, 'function');
assert.equal(typeof get_presentation_info, 'function');
assert.equal(typeof get_slide_count, 'function');

const invalidData = new Uint8Array([0, 1, 2, 3]);

assert.throws(() => convert(invalidData), /invalid|zip|PPTX|archive/i);
assert.throws(() => get_info(invalidData), /invalid|zip|PPTX|archive/i);

const fixturePath = process.argv[2];
if (fixturePath) {
  const fixture = new Uint8Array(await readFile(fixturePath));
  const result = convert_with_metadata(fixture);
  const marker = '<script type="application/json" id="pptx2html-diagnostics">';
  const embedded = result.html.split(marker, 2)[1].split('</script>', 1)[0];
  assert.equal(result.diagnosticsJson, embedded);
  assert.equal(result.diagnostics, embedded);
  assert.ok(Array.isArray(JSON.parse(result.diagnosticsJson)));
  assert.ok(!result.diagnosticsJson.includes('</script>'));
  assert.equal(result.unresolvedElements.startsWith('['), true);
  process.stdout.write(result.diagnosticsJson);
}
