# pptx-to-html

Convert PPTX slides to high-fidelity HTML in the browser with a Rust/WASM core.

This package is the browser-focused WASM distribution of the `pptx2html-turbo` project.
New installations use `@briank-dev/pptx-to-html`; the
`@briank-dev/pptx2html-turbo` package remains the supported legacy name.

## Install

```bash
npm install @briank-dev/pptx-to-html
```

## Usage

The convenience API initializes WASM on first use and accepts a browser
`File`/`Blob`, `ArrayBuffer`, or `Uint8Array`.

```html
<iframe
  id="output"
  sandbox="allow-scripts"
  title="Converted slide output"
></iframe>
<script type="module">
import { pptxToHtml } from '@briank-dev/pptx-to-html';

const html = await pptxToHtml(file);
document.getElementById('output').srcdoc = html;
</script>
```

Converted output is active, untrusted HTML. Render it in a sandboxed iframe
with `allow-scripts` and without `allow-same-origin`, as shown above.

The lower-level API remains available for explicit initialization, filtering,
metadata, and compatibility:

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

## API

- `pptxToHtml(input, moduleOrPath?)` — lazily initialize WASM and convert browser or byte input; concurrent calls share the first initialization attempt and its success or failure, while a later call retries after failure
- `init()` — initialize the WASM module
- `convert(data)` — convert PPTX bytes to HTML
- `convert_with_options(data, embedImages, includeHidden, slideIndices, scale)`
- `convert_with_metadata(data)` — convert and return canonical diagnostics JSON plus unresolved-element metadata
- `convert_with_options_metadata(data, embedImages, includeHidden, slideIndices, scale)`
- Metadata results expose `diagnostics` (canonical ordered JSON), `diagnosticsJson`, and the backward-compatible `unresolvedElements` JSON
- `get_presentation_info(data)` — typed presentation metadata
- `get_info(data)` / `get_slide_count(data)` — backward-compatible helpers

### Slide Indexing

- `convert_slides(data, slides)` uses **0-based** slide indices.
- `convert_with_options(..., slideIndices)` and `convert_with_options_metadata(..., slideIndices)` use **1-based** slide indices.

### Slide Scale

- `scale` is required; pass `1.0` for the original slide size.
- Values like `2.0` enlarge the whole slide canvas uniformly without recomputing coordinates or reflowing text.
- The included browser demo displays ordered diagnostics, executes renderer-owned actions and timing in an opaque-origin frame, and initializes image-like zoom to the available width.

## Package Scope

This npm package is intended for **browser ESM/WASM usage**.

`@briank-dev/pptx2html-turbo` is the legacy package name. It remains available
with the same API and release versions during migration.

## Project

- Repository: https://github.com/brnyxx/pptx2html-turbo
- Issues: https://github.com/brnyxx/pptx2html-turbo/issues
- Demo: https://brnyxx.github.io/pptx2html-turbo/
- License: MIT
