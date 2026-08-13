# pptx2html-core/tests

Scope: integration targets only. Root and crate `AGENTS.md` rules apply; not repeated here.

## TWO BUILDERS
`MinimalPptx` (`fixtures/mod.rs`): content behavior. `new(body)` wraps a `spTree` fragment; every other part has a default. Swap one part, assert on it. `with_raw_slide` when the slide document itself is under test. Layout parts appear only after `with_layout`/`with_slide_layout_rel`, and `[Content_Types].xml` follows automatically. No validation, no error type: it builds whatever you hand it, including malformed XML. That's the point for parser hostility tests.

`PackageBuilder` (`fixtures/package.rs`): package wiring. `SlideXml::from_body(...).build()` in, `validate()` or `build()` out, `FixtureError` on the way back with a stable `code()`. Assert `code()` and `target()`, never the `Display` text. `FeaturePart::notes/comments/chart/media/extra` register content types; `Relationship::internal` targets get resolved and checked against real entries, `Relationship::external` don't. Part paths must start with `ppt/`; reserved paths are rejected.

Determinism is `PackageBuilder`'s contract, not `MinimalPptx`'s: `BTreeMap` entry order, `CompressionMethod::Stored`, `DateTime::default()`. Same input, byte-identical archive. Keep it that way. `write_to_temp(namespace)` when a test needs a real path; the `TempDir` dies with the returned value, so bind it, don't drop it.

Both live behind `mod fixtures;` compiled per target. A fixture edit recompiles ~19 binaries. Batch fixture changes.

## TARGET SHAPES
- Seam: `parser_seam_test.rs` (archive → model), `renderer_seam_test.rs` (hand-built `Presentation` → HTML). Never cross the seam inside one test.
- Regression: `coverage_regression_test.rs`, `renderer_regression_test.rs`, `edge_case_test.rs`. Exercise branches that real decks hit rarely. Add here when you fix a bug; add a named case, don't grow an existing one.
- Domain: `hierarchy_test.rs`, `table_style_test.rs`, `pattern_fill_test.rs`, `picture_bullet_test.rs`, `action_semantics_test.rs`, `geometry_adjustment_*`. Big ones split into `<target>/<aspect>.rs` and pull the pieces in with `#[path = ...] mod`. Follow that split instead of appending to a 3000-line file.
- Contract: `fixture_contract_test.rs` (builders themselves), `public_api_test.rs` (module paths plus flat re-exports), `diagnostic_contract_test.rs` (diagnostic + `UnresolvedElement` shape).

## RULES
- No sleeps, no wall-clock waits, no retries. Conversion is synchronous; a test that needs timing is testing the wrong thing.
- `Command::new("python3")` appears in geometry and diagnostic contract tests to check manifests and JSON. Fine. Feed stdin with a length prefix like `diagnostic_contract_test.rs` does instead of trusting shell quoting.
- Bounded metadata: assert the exact count first (`unresolved_elements.len()`), then the specific field. `raw_reference` must round-trip byte-exact, and the escaped payload must not leak the raw string back into HTML. One `</script>` in the document, always.
- Name tests after behavior and condition: `typed_actions_are_preserved_by_trigger_when_package_is_parsed`. Prefix locals `given_`/`expected_` when a test has more than one archive in flight.
- Test-only `expect` with a sentence-shaped message is normal here. Library `unwrap` is still banned.
- Geometry manifests (`fixtures/*.tsv`) carry `sha256` headers. If the checker complains, regenerate the manifest, don't loosen the assertion.

## ANTI-PATTERNS
- Building ZIP bytes by hand. Extend a builder instead; the two exceptions that rewrite archive entries (`replace_package_entry`) exist to corrupt a package after a valid build, not to skip one.
- Full-HTML snapshots, or `assert!(html.contains(...))` on a string so short it matches by accident. Pin the attribute, path data, or declaration.
- Asserting on log output or diagnostic prose. Codes and structured fields only.
- Skipping `validate()` and blaming `build()` when the failure is dangling wiring.
- Duplicating a case across `coverage_regression_test.rs` and a domain target. Pick the narrower one.

## COMMANDS
```bash
cargo test -p pptx2html-core                                # all targets
cargo test -p pptx2html-core --test fixture_contract_test   # after touching fixtures/
cargo test -p pptx2html-core --test public_api_test         # after model re-export changes
cargo test -p pptx2html-core --test diagnostic_contract_test
cargo test -p pptx2html-core --test parser_seam_test --test renderer_seam_test
cargo test -p pptx2html-core --test coverage_regression_test -- --nocapture
cargo test -p pptx2html-core geometry -- --list             # enumerate geometry cases
```
