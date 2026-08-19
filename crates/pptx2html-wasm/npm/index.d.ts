import type { InitInput } from './pptx2html_wasm.js';

export { default } from './pptx2html_wasm.js';
export * from './pptx2html_wasm.js';

export type PptxInput = Blob | ArrayBuffer | Uint8Array;

/**
 * Convert PPTX input to self-contained HTML with lazy WASM initialization.
 *
 * Browser callers can omit `moduleOrPath`. Node callers may pass WASM bytes.
 * Concurrent calls share the first initialization attempt and its outcome.
 * A call made after a failed attempt starts a new initialization attempt.
 */
export function pptxToHtml(
  input: PptxInput,
  moduleOrPath?: InitInput | Promise<InitInput>,
): Promise<string>;
