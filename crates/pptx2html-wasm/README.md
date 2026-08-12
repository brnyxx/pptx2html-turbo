# @briank-dev/pptx2html-turbo

Convert PPTX slides to high-fidelity HTML in the browser with a Rust/WASM core.

This package is the browser-focused WASM distribution of the `pptx2html-turbo` project.

## Install

```bash
npm install @briank-dev/pptx2html-turbo
```

## Usage

```html
<script type="module">
import init, {
  convert,
  convert_with_options,
  convert_with_metadata,
  convert_with_options_metadata,
  get_presentation_info,
} from '@briank-dev/pptx2html-turbo';

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

- `scale` defaults to `1.0`.
- Values like `2.0` enlarge the whole slide canvas uniformly without recomputing coordinates or reflowing text.
- The included browser demo exposes this as an image-like zoom control.

## Package Scope

This npm package is intended for **browser ESM/WASM usage**.

## Project

- Repository: https://github.com/kim62210/pptx2html-turbo
- Issues: https://github.com/kim62210/pptx2html-turbo/issues
- Demo: https://kim62210.github.io/pptx2html-turbo/
- License: MIT
