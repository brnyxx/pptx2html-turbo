import init, { convert } from './pptx2html_wasm.js';

export * from './pptx2html_wasm.js';
export default init;

/** @typedef {Blob | ArrayBuffer | Uint8Array} PptxInput */

/** @type {Promise<import('./pptx2html_wasm.js').InitOutput> | undefined} */
let initialization;

/**
 * @param {PptxInput} input
 * @returns {Promise<Uint8Array>}
 */
function toPptxBytes(input) {
  if (input instanceof Uint8Array) {
    return Promise.resolve(input);
  }
  if (input instanceof ArrayBuffer) {
    return Promise.resolve(new Uint8Array(input));
  }
  if (typeof Blob !== 'undefined' && input instanceof Blob) {
    return input.arrayBuffer().then((buffer) => new Uint8Array(buffer));
  }
  throw new TypeError(
    'pptxToHtml input must be a Blob, ArrayBuffer, or Uint8Array',
  );
}

/**
 * @param {import('./pptx2html_wasm.js').InitInput | Promise<import('./pptx2html_wasm.js').InitInput> | undefined} moduleOrPath
 * @returns {Promise<import('./pptx2html_wasm.js').InitOutput>}
 */
function initialize(moduleOrPath) {
  if (initialization !== undefined) {
    return initialization;
  }
  const pending = init(
    moduleOrPath === undefined ? undefined : { module_or_path: moduleOrPath },
  );
  initialization = pending.catch((error) => {
    initialization = undefined;
    throw error;
  });
  return initialization;
}

/**
 * Convert PPTX input to self-contained HTML with lazy WASM initialization.
 *
 * Browser callers can omit `moduleOrPath`. Node callers may pass WASM bytes.
 * Concurrent calls share the first initialization attempt and its outcome.
 * A call made after a failed attempt starts a new initialization attempt.
 *
 * @param {PptxInput} input
 * @param {import('./pptx2html_wasm.js').InitInput | Promise<import('./pptx2html_wasm.js').InitInput>} [moduleOrPath]
 * @returns {Promise<string>}
 */
export async function pptxToHtml(input, moduleOrPath) {
  const bytesPromise = toPptxBytes(input);
  const ready = initialize(moduleOrPath);
  const [bytes] = await Promise.all([bytesPromise, ready]);
  return convert(bytes);
}
