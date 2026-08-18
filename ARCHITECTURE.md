# Architecture

## Pipeline Overview

```
PPTX file (ZIP archive)
    │
    ▼
┌─────────────────────────────────────────────────┐
│  parser/                                        │
│  SAX streaming XML parser (quick-xml)           │
│                                                 │
│  presentation.xml → slide size, rel IDs, dts    │
│  theme1.xml       → ColorScheme, FontScheme     │
│  slideMaster1.xml → ClrMap, TxStyles, shapes    │
│  slideLayout1.xml → placeholder shapes          │
│  slide1.xml       → shapes, text, fills         │
│  *.rels           → relationship target lookup  │
└───────────────────┬─────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────┐
│  model/                                         │
│  Rust structs mirroring PresentationML schema   │
│                                                 │
│  Presentation                                   │
│    ├── themes: Vec<Theme>                       │
│    ├── masters: Vec<SlideMaster>                │
│    ├── layouts: Vec<SlideLayout>                │
│    └── slides: Vec<Slide>                       │
│         └── shapes: Vec<Shape>                  │
│              ├── ShapeType (rect, picture, …)   │
│              ├── Fill / Border / TextBody       │
│              └── Color (kind + modifiers)       │
└───────────────────┬─────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────┐
│  resolver/                                      │
│  Property inheritance cascade                   │
│                                                 │
│  slide shape                                    │
│    → layout placeholder                         │
│    → master placeholder                         │
│    → txStyles (title/body/other)                │
│    → defaultTextStyle                           │
│    → hardcoded default                          │
│                                                 │
│  Also resolves:                                 │
│  • ClrMap overrides per slide/layout            │
│  • Style refs (fillRef, lnRef, fontRef)         │
│  • Background inheritance                       │
└───────────────────┬─────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────┐
│  renderer/                                      │
│  HTML/CSS generation                            │
│                                                 │
│  • Self-contained HTML with inline styles       │
│  • Absolute positioning (EMU → CSS px)          │
│  • SVG <path> for 187 classified presets        │
│  • Base64 image embedding (optional)            │
│  • Color resolution via Theme + ClrMap          │
└─────────────────────────────────────────────────┘
```

## Module Responsibilities

| Module | Purpose |
|--------|---------|
| `parser/mod.rs` | Entry point: ZIP extraction, orchestrates parsing of all XML parts |
| `parser/slide_parser.rs` | SAX parser for `slide*.xml` — shapes, text, fills, tables, groups |
| `parser/master_parser.rs` | SAX parser for `slideMaster*.xml` — ClrMap, TxStyles, master shapes |
| `parser/layout_parser.rs` | SAX parser for `slideLayout*.xml` — placeholder shapes |
| `parser/theme_parser.rs` | SAX parser for `theme*.xml` — ColorScheme, FontScheme, FmtScheme |
| `parser/relationships.rs` | Parses `.rels` files into `HashMap<rId, target>` |
| `parser/xml_utils.rs` | Shared XML helpers: `local_name()`, `attr_str()` |
| `model/` | Data model structs (no logic, just data) |
| `model/color.rs` | Color resolution: RGB/Theme/System/Preset → ResolvedColor with modifiers |
| `model/presentation.rs` | Top-level `Presentation`, `Theme`, `ColorScheme`, `ClrMap` |
| `model/slide.rs` | `Slide`, `Shape`, `ShapeType`, `TextBody`, `TextParagraph` |
| `model/hierarchy.rs` | `SlideMaster`, `SlideLayout`, `TxStyles`, `ListStyle` |
| `model/style.rs` | `Fill`, `Border`, `TextStyle`, `FontStyle` |
| `model/geometry.rs` | `Emu`, `Position`, `Size` with unit conversions |
| `resolver/inheritance.rs` | Property cascade: position, fill, border, background, ClrMap |
| `resolver/placeholder.rs` | Placeholder matching by type and index |
| `resolver/style_ref.rs` | Resolves `fillRef`, `lnRef`, `fontRef` against FmtScheme |
| `renderer/mod.rs` | HTML/CSS string generation from resolved model |
| `renderer/geometry.rs` and `renderer/geometry/` | SVG path generation for 187 classified presets plus shared path helpers |
| `error.rs` | `PptxError` enum and `PptxResult` type alias |
| `lib.rs` | Public file/byte conversion APIs, option and metadata variants, `get_info`, and `get_info_from_bytes` |

## Color Resolution Chain

```
Color { kind: Theme("accent1"), modifiers: [LumMod(75000)] }
    │
    ├── 1. ClrMap lookup:  accent1 → accent1 (identity mapping)
    ├── 2. Theme lookup:   accent1 → "4472C4" (hex from ColorScheme)
    ├── 3. Parse hex:      "4472C4" → RGB(68, 114, 196)
    ├── 4. Apply LumMod:   HSL luminance × 0.75
    └── 5. Result:         ResolvedColor { r, g, b, a: 255 }
```

## Slide Hierarchy Inheritance

```
Slide → SlideLayout → SlideMaster → Theme
  │         │              │           │
  │         │              ├── ClrMap   ├── ColorScheme (12 colors)
  │         │              ├── TxStyles ├── FontScheme (major/minor)
  │         ├── ClrMapOvr  ├── Shapes   └── FmtScheme (fill/ln/bgFill lists)
  ├── ClrMapOvr            └── Background
  ├── Shapes (with placeholder refs)
  └── Background
```

Properties are resolved bottom-up: if a slide shape has a fill, use it;
otherwise check the matching layout placeholder, then the master placeholder,
then style refs, then defaults.

## Adding Support for New PPTX Features

1. **Parse**: Add element detection in the appropriate `parser/*.rs` SAX loop
2. **Model**: Add fields to the corresponding struct in `model/`
3. **Resolve**: If the property participates in inheritance, add cascade logic in `resolver/`
4. **Render**: Emit HTML/CSS in `renderer/mod.rs`
5. **Test**: Add test in `tests/integration_test.rs` using `MinimalPptx` builder
