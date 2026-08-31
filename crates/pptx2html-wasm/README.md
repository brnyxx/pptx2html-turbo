<div align="center">

<img src="https://raw.githubusercontent.com/brnyxx/pptx2html-turbo/main/.github/assets/hero.svg" alt="pptx2html-turbo — PPTX to HTML, without a server. A pure-Rust ECMA-376 renderer compiled to WebAssembly." width="920">

[![npm](https://img.shields.io/npm/v/@briank-dev/pptx-to-html?style=flat-square&labelColor=0B0F0E&color=F2703C&label=npm)](https://www.npmjs.com/package/@briank-dev/pptx-to-html) [![Rust](https://img.shields.io/badge/rust-2024_edition-8FD9AE?style=flat-square&labelColor=0B0F0E)](https://github.com/brnyxx/pptx2html-turbo) [![WebAssembly](https://img.shields.io/badge/wasm-browser_ready-F0C24E?style=flat-square&labelColor=0B0F0E)](https://brnyxx.github.io/pptx2html-turbo/) [![License](https://img.shields.io/badge/license-MIT-ECF1EE?style=flat-square&labelColor=0B0F0E)](https://github.com/brnyxx/pptx2html-turbo/blob/main/LICENSE)

**English** · [한국어](https://github.com/brnyxx/pptx2html-turbo/blob/main/crates/pptx2html-wasm/README.ko.md)

</div>

Convert PowerPoint decks to HTML **entirely in the browser**. The renderer is a from-scratch
[ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/)
implementation written in Rust and compiled to WebAssembly — no server round trip, no upload,
no PowerPoint or LibreOffice install. The file never leaves the tab.

Anything the renderer cannot resolve exactly is reported as a coded diagnostic rather than
silently dropped or replaced with a guess.

**[Try the live demo](https://brnyxx.github.io/pptx2html-turbo/)** — drop a `.pptx` and watch it convert.

## Install

```bash
npm install @briank-dev/pptx-to-html
```

## Usage

The convenience API initializes WASM on first use and accepts a browser `File`/`Blob`,
`ArrayBuffer`, or `Uint8Array`. Inputs larger than 64 MiB are rejected before conversion.

```html
<iframe id="output" sandbox="allow-scripts" title="Converted slide output"></iframe>
<script type="module">
import { pptxToHtml } from '@briank-dev/pptx-to-html';

const html = await pptxToHtml(file);
document.getElementById('output').srcdoc = html;
</script>
```

> **Security:** converted output is active, untrusted HTML. Render it in a sandboxed iframe
> with `allow-scripts` and **without** `allow-same-origin`, as shown above.

<details>
<summary><b>Lower-level API — explicit initialization, filtering, metadata</b></summary>

```html
<script type="module">
import init, {
  convert,
  convert_with_options,
  convert_with_metadata,
  convert_with_options_metadata,
  get_presentation_info,
} from '@briank-dev/pptx-to-html';

await init();

const response = await fetch('/presentation.pptx');
const data = new Uint8Array(await response.arrayBuffer());

const html = convert(data);

const filtered = convert_with_options(
  data,
  true,
  false,
  new Uint32Array([1, 3]),
  1.0,
);

const info = get_presentation_info(data);
console.log(info.slideCount, info.widthPx, info.heightPx, info.title);

const withMetadata = convert_with_metadata(data);
console.log(JSON.parse(withMetadata.diagnostics));
console.log(withMetadata.unresolvedElements);

const filteredWithMetadata = convert_with_options_metadata(
  data,
  true,
  false,
  new Uint32Array([1, 3]),
  1.0,
);
</script>
```

</details>

## API

- `pptxToHtml(input, moduleOrPath?)` — lazily initialize WASM and convert browser or byte input; concurrent calls share the first initialization attempt and its success or failure, while a later call retries after failure
- `init()` — initialize the WASM module
- `convert(data)` — convert PPTX bytes to HTML
- `convert_with_options(data, embedImages, includeHidden, slideIndices, scale)`
- `convert_with_metadata(data)` — convert and return canonical diagnostics JSON plus unresolved-element metadata
- `convert_with_options_metadata(data, embedImages, includeHidden, slideIndices, scale)`
- `get_presentation_info(data)` — typed presentation metadata
- `get_info(data)` / `get_slide_count(data)` — backward-compatible helpers

Metadata results expose `diagnostics` (canonical ordered JSON), `diagnosticsJson`, and the
backward-compatible `unresolvedElements` JSON.

### Slide indexing

- `convert_slides(data, slides)` uses **0-based** slide indices.
- `convert_with_options(..., slideIndices)` and `convert_with_options_metadata(..., slideIndices)` use **1-based** slide indices.

### Slide scale

- `scale` is required; pass `1.0` for the original slide size.
- Values like `2.0` enlarge the whole slide canvas uniformly without recomputing coordinates or reflowing text.
- The included browser demo displays ordered diagnostics, executes renderer-owned actions and timing in an opaque-origin frame, and initializes image-like zoom to the available width.

## Scope

This npm package is the **browser ESM/WASM** distribution and converts **PPTX only**. The
parent project additionally converts DOCX, DOC, XLSX, XLS, PPT, and PDF, but those formats
require a native pipeline backed by installed LibreOffice and Poppler executables and are
**not available in browser WASM**. See
[Universal document conversion](https://github.com/brnyxx/pptx2html-turbo/blob/main/docs/UNIVERSAL_DOCUMENTS.md).

`@briank-dev/pptx2html-turbo` is the legacy package name. It remains available with the same
API and release versions during migration.

## Project

- Repository: https://github.com/brnyxx/pptx2html-turbo
- Issues: https://github.com/brnyxx/pptx2html-turbo/issues
- Demo: https://brnyxx.github.io/pptx2html-turbo/
- License: MIT
