# pptx2html-wasm

Scope: browser bindings and npm packaging only. Rendering fidelity and document semantics remain owned by `pptx2html-core`.

## OVERVIEW
Thin `wasm-bindgen` adapter over `pptx2html-core`, shipped primarily as `@briank-dev/pptx-to-html` and compatibly as `@briank-dev/pptx2html-turbo`. Rust bindings live in `src/lib.rs`; the lazy-init npm facade lives in `npm/index.js` and `npm/index.d.ts`.
Two API generations live side by side and both are public contract:
- v0.5 legacy: `convert`, `convert_slides`, `get_slide_count`, `get_info` (JSON string).
- v0.6 enhanced: `convert_with_options`, `convert_with_metadata`, `convert_with_options_metadata`, `get_presentation_info`, plus the `PresentationInfo` and `ConversionResult` structs.
- npm facade: `pptxToHtml(input, moduleOrPath?)` lazily initializes WASM and accepts `Blob`, `ArrayBuffer`, or `Uint8Array`.
Legacy names stay. Removing or renaming one breaks published npm consumers and the smoke tests that import them by name.

## INDEXING QUIRK
`convert_slides(data, slides)` takes **0-based** indices and converts them with `to_one_based_index` before calling core. Every other slide-filtering entry point takes **1-based** indices and passes them through untouched. Core itself is 1-based.
Two unit tests pin this on purpose: `convert_slides_uses_zero_based_indices`, `convert_with_options_keeps_one_based_indices`. Don't "fix" the inconsistency; it's the compatibility surface.
Empty `slide_indices` means all slides (`optional_slide_indices` returns `None`), not zero slides.

## BINDING RULES
- Getters are `#[wasm_bindgen(getter, js_name = "...")]` with camelCase JS names over snake_case Rust fields (`slide_count` → `slideCount`). Add a field, add the getter and the JS name in the same edit.
- Struct fields stay private; JS reads them through getters that clone. No `pub` fields.
- Every fallible export returns `Result<T, JsError>` built by `to_js_error`, so JS sees a thrown `Error` carrying the core error's `Display` text. Never `unwrap()` in an export, never swallow into a default value.
- `unresolvedElements` is a JSON **string**, not an object. `serialize_unresolved` hand-writes it and `escape_json_string` handles quotes, backslashes, `\n\r\t\b\f`, and `<0x20` control chars as `\uXXXX`. Adding a variant to `UnresolvedType` requires a new match arm here or it won't compile, which is the intended tripwire.
- No `serde`/`serde-wasm-bindgen` dependency. Keep the dependency list at `pptx2html-core` + `wasm-bindgen`; WASM binary size is a shipping constraint.

## WHERE TO LOOK
| Task | Location |
|---|---|
| All exports and helpers | `src/lib.rs` |
| Lazy-init npm facade | `npm/index.js`, `npm/index.d.ts` |
| JS-visible docs, indexing note, scale semantics | `README.md` (copied into the npm tarball verbatim) |
| Browser usage example | `demo/index.html` (uses `convert_with_options_metadata` + `get_info`) |
| npm metadata rewrite + publish gate | `scripts/prepare_wasm_release_package.sh` |
| Version agreement across manifests | `scripts/read_release_version.sh` |
| CI wasm job | `.github/workflows/ci.yml`, `publish-npm.yml`, `deploy-demo.yml` |

## PACKAGING AND VERSIONS
`wasm-pack` generates `pkg/package.json`; the release script then copies the npm facade and overwrites name, version, description, keywords, author, homepage, repository, bugs, entrypoints, and `exports` from env vars. Don't hand-edit `pkg/`, it's a build artifact.
`read_release_version.sh` requires core, cli, py, wasm `Cargo.toml`, `pyproject.toml`, and every `vX.Y.Z` string in the Pages demo to agree, and a release tag to match them. Bump this crate's version alone and release fails before publish.
Package contract asserted in `tests/check-package-contract.mjs`: parameterized name, version, facade entrypoints, homepage, bugs URL, tarball allowlist, and presence of `README.md`, `LICENSE`, the facade, generated `.js`/`.d.ts`, and `_bg.wasm`.

## VALIDATION
Node and browser are separate risks. Node smoke tests catch broken exports and error mapping; they don't prove browser behavior.
- `tests/node-smoke.mjs` initializes with explicit bytes (`init({ module_or_path: wasmBytes })`), checks each export is a function, and asserts garbage input throws.
- `tests/package-root-smoke.mjs` symlinks a package directory into a temp `node_modules/@briank-dev/`, imports either package name by bare specifier, and converts a real two-slide fixture through `pptxToHtml`.
- Browser paths (default `init()` fetch, demo page, image embedding, real rendering) are only exercised by loading `demo/index.html` over HTTP after a `--target web` build. `file://` won't work.

## COMMANDS
```bash
cargo test -p pptx2html-wasm                                  # native unit tests, no wasm toolchain needed
cargo clippy -p pptx2html-wasm -- -D warnings                 # lint on host; wasm32 target fails on the zip dev-dep
wasm-pack build crates/pptx2html-wasm --target web --release  # writes pkg/
node crates/pptx2html-wasm/tests/node-smoke.mjs
node crates/pptx2html-wasm/tests/demo-contract.mjs "$(bash scripts/read_release_version.sh)"
node crates/pptx2html-wasm/tests/release-version-contract.mjs
node crates/pptx2html-wasm/tests/package-root-smoke.mjs @briank-dev/pptx-to-html
node crates/pptx2html-wasm/tests/check-package-contract.mjs crates/pptx2html-wasm/pkg "$(bash scripts/read_release_version.sh)" @briank-dev/pptx-to-html
bash scripts/read_release_version.sh v2.0.1                   # version + tag agreement
python3 -m http.server -d _site 8000                          # after workflow-equivalent _site assembly
```

## ANTI-PATTERNS
- Don't add parsing, inheritance, or rendering logic here. If a JS caller needs new behavior, it lands in core and this crate forwards it.
- Don't change an exported function's name, arity, or argument order for cleanliness. That's a breaking npm release, not a refactor.
- Don't return `Result<_, String>` or `JsValue` errors; `JsError` is the single error mapping.
- Don't hand-roll JSON outside `serialize_unresolved`/`escape_json_string`, and don't skip the escaping helper for "safe" strings.
- Don't add an export without also adding it to `README.md`'s API list and to a smoke test import.
- Don't rely on a smoke test alone for rendering changes; core tests own fidelity, these only prove the boundary.
