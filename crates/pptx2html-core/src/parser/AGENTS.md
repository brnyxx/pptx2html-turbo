# PARSER LAYER

Scope: ZIP traversal plus OOXML → `model` types. No inheritance resolution, no rendering.
Root `AGENTS.md` holds workspace-wide rules; this file only adds parser specifics.

## ARCHIVE ORCHESTRATION
- `PptxParser::parse_file` reads bytes, delegates to `parse_bytes`; `parse_bytes` owns the whole package walk.
- `ZipArchive` stays local to `parse_bytes` and is passed `&mut` down to sub-parsers that need extra parts (images, embedded charts).
- `read_entry` returns `PptxError::MissingFile` for absent parts. Only `ppt/presentation.xml` is fatal.
- `EncryptedPackage`/`EncryptionInfo` entries mean password-protected input → `UnsupportedFormat`, no partial parse.
- `collect_package_diagnostics` (in `preserved_parser`) is a second, independent pass over the archive for diagnostics; keep it out of the model-building path.

## PARSING ORDER (do not reorder)
1. `[Content_Types].xml` → `ContentTypes` (needed for picture-bullet media typing)
2. `ppt/presentation.xml` → slide size, `sldId` list, `defaultTextStyle`
3. `ppt/_rels/presentation.xml.rels` → relationship records + table styles part
4. themes → `presentation.themes`
5. masters → `presentation.masters`, each linked to a theme by index
6. layouts, discovered through master `.rels`, linked by `master_idx`
7. slides in `sldId` order, linked by `layout_idx`
Later steps depend on index maps built by earlier ones (`master_path_to_idx`, `layout_path_to_idx`).

## RELATIONSHIP PATHS
- `parse_relationship_records` keeps `Id`, `Type`, `Target`, `TargetMode`; `target_map` flattens to `{rId → target}` when mode doesn't matter.
- Type matching is by last URI segment (`collect_targets_by_type(rels, "slideLayout")`), never full-URI equality.
- `collect_targets_by_type` sorts targets, so master/layout/theme indices stay deterministic across archives.
- Path helpers, pick the right one: `normalize_ppt_path` (prefix `ppt/`), `resolve_relative_path` (walks `..`), `rels_path_for` (`dir/_rels/file.ext.rels`), `canonical_part_name` (lowercase file name, used as map key so `slideLayout1.xml` and `slidelayout1.xml` collapse).
- `resolve_internal_target` is the safe resolver for untrusted targets. It rejects empty, absolute, backslash, percent-encoded, URI-scheme, dot-segment, and empty-segment targets. Use it for anything reached from package data, not just string concatenation.
- External targets (`TargetMode::External`, hyperlinks) stay verbatim, no filesystem resolution.

## SAX STATE
- `quick_xml::Reader`/`NsReader` streaming only. Never buffer a part into a DOM or regex over XML.
- State is explicit locals in the parse loop: `in_*` booleans for containers, `Option<Builder>` for the object under construction, `depth: Vec<String>` for ancestor tags.
- `depth_contains(&depth, "xfrm")` distinguishes same-named children (`off`/`ext` under `xfrm` vs `chOff`/`chExt`); push in `Start`, pop in `End`, and keep both sides in sync or the whole slide skews.
- `Empty` events must be handled beside every `Start` handler. PowerPoint emits both forms for `defRPr`, `srgbClr`, `lvlNpPr`, and friends.
- Shared text state lives in `TextSaxState` (`text_parser.rs`); `ShapeBuilder` in `slide_parser.rs`. Reuse them instead of new ad hoc flag sets.
- Builders finalize on the matching `End` event, so partial elements never reach the model.

## NAMESPACES
- Strip prefixes with `xml_utils::local_name`; attributes with `xml_utils::attr_str`. Prefixes are author-chosen, so matching `p:sp` literally is a bug.
- Where the same local name exists in two namespaces, resolve for real: `reader.resolve_element(...)` plus `is_presentationml`, and track `presentationml_depth` alongside `depth`.
- `r:id` is matched by suffix (`key.ends_with("id") && key.contains(':')`) because the relationship prefix varies.

## ERROR TOLERANCE
- Missing or malformed non-critical parts degrade: `continue` the loop, `unwrap_or_default`, `warn!` once. One broken layout must not kill the deck.
- Malformed XML inside a required part propagates as `PptxError::Xml`.
- Unknown OOXML isn't dropped silently. Emit a `ConversionDiagnostic` with family, support tier, and `DiagnosticLocation` (part name, slide index, relationship id) instead of inventing geometry or color.
- No library `unwrap()`/`expect()`; `flatten()` over attribute iterators is the accepted way to skip bad attributes.

## TESTS
- Unit tests live at the bottom of each parser file with raw XML string literals. Include the `xmlns` declarations the real format uses, including odd prefixes.
- Cross-part behavior (relationships, ordering, hierarchy links) belongs in `tests/parser_seam_test.rs` using `PackageBuilder`/`SlideXml`/`MinimalPptx`, never a checked-in binary PPTX.
- Cover both `Start` and `Empty` spellings of any new element, plus the absent-part path.

## ANTI-PATTERNS
- Building a full-part string tree, or reaching for `serde`/DOM/regex parsing.
- Comparing tag names with prefixes attached, or full relationship-type URIs.
- Concatenating relationship targets into paths without the resolver, `..` escapes the package.
- Applying layout/master/theme inheritance here. Parser records what the XML says; `resolver/` decides what it means.
- Storing `Rc`/`Arc` hierarchy links instead of `Vec` indices.
- Defaulting an unsupported feature to something plausible rather than emitting a diagnostic.
