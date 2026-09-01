# pptx2html-turbo Demo Design System

This document codifies the existing single-file GitHub Pages demo in
`crates/pptx2html-wasm/demo/index.html`. It is an extraction of the shipped visual system,
not a redesign. Copy-nearby changes must preserve these tokens, components, and states.

## 1. Atmosphere & Identity

The demo feels like a dark editorial workbench: quiet, exact, and visibly mechanical rather
than glossy or playful. Its signature is the animated drawing bed in the hero, where a deck is
unpacked, resolved, and rendered across a measured machine surface. Warm ember accents mark
actions and progress; amber and celadon communicate inspection and success.

## 2. Color

The product is dark-only. The Light column is intentionally not shipped.

### Palette

| Role | Token | Light | Dark | Usage |
|---|---|---:|---:|---|
| Surface/primary | `--ink` | not shipped | `#0E1211` | Page background |
| Surface/raised | `--ink-raised` | not shipped | `#131917` | Controls and bars |
| Surface/panel | `--ink-panel` | not shipped | `#19201D` | Hovered drop platen |
| Surface/well | `--ink-well` | not shipped | `#0A0D0C` | Output and input wells |
| Border/default | `--edge` | not shipped | `#242D29` | Dividers and frames |
| Border/strong | `--edge-strong` | not shipped | `#35403A` | Scrollbars and status rails |
| Border/control | `--edge-control` | not shipped | `#5E6D65` | Interactive outlines |
| Text/primary | `--paper` | not shipped | `#ECF1EE` | Headings and primary copy |
| Text/secondary | `--paper-soft` | not shipped | `#9FB3A9` | Body copy and hints |
| Text/tertiary | `--paper-faint` | not shipped | `#75897F` | Labels and dormant states |
| Accent/primary | `--ember` | not shipped | `#F2703C` | Primary action and progress |
| Accent/hover | `--ember-lift` | not shipped | `#FF8552` | Primary action hover |
| Accent/deep | `--ember-deep` | not shipped | `#C4501F` | Selection background |
| Text/on-accent | `--on-ember` | not shipped | `#180E08` | Copy over ember |
| Status/inspection | `--amber` | not shipped | `#F0C24E` | Focus, active stage, diagnostics |
| Status/success | `--celadon` | not shipped | `#8FD9AE` | Completed stage and success |
| Status/error | `--flag` | not shipped | `#F2705C` | Conversion errors |
| Illustration/line | `--plate-line` | not shipped | `#4E5C54` | Machine-bed drawing |
| Surface/wash | `--wash` | not shipped | `rgba(255, 255, 255, 0.028)` | Quiet interactive hover |
| Surface/topbar | `--topbar-veil` | not shipped | `rgba(12, 16, 15, 0.93)` | Sticky navigation veil |

### Rules

- Ember is reserved for actions, progress, the hero emphasis, and the pipeline's active mark.
- Amber means focus, inspection, or an active diagnostic state; celadon means completion.
- Depth comes from tonal surface changes and one-pixel borders. Do not add drop shadows.
- Extend this table before adding a reusable color role. One-off illustration colors remain
  accepted debt until a separately approved component extraction.

## 3. Typography

### Scale

| Level | Size | Weight | Line height | Tracking | Usage |
|---|---|---:|---:|---:|---|
| Display | `clamp(2.7rem, 7.2vw, 4.4rem)` | 400 | 1.02 | `-0.015em` | Hero statement |
| Section | `clamp(1.9rem, 3.4vw, 2.6rem)` | 400 | 1.08 | `-0.01em` | Section headings |
| Control title | 20px | 600 | inherited | `-0.005em` | File-drop title |
| Lead | 19px | 400 | 1.62 | normal | Hero deck copy |
| Body | 17px | 400 | 1.6 | normal | Page default |
| Body/small | 15px | 400-600 | 1.5-1.6 | normal | Cards, buttons, status |
| Label | 12-13px | 400-500 | 1.4-1.5 | `0.04em-0.08em` | Metadata and process labels |

### Font Stack

- Display: `Newsreader`, then Iowan/Palatino/Georgia serif fallbacks.
- Text: `Archivo`, then Segoe UI/Liberation Sans/sans-serif fallbacks.
- Mono: `IBM Plex Mono`, then platform monospace fallbacks.
- Three families are intentional: editorial display, readable interface text, and machine/data
  notation each carry a distinct role.

### Rules

- Body text remains at least 15px, except compact machine labels at 11-14px.
- Large headings use `clamp()` and a bounded measure; mobile overrides must prevent four-line
  fragments and horizontal overflow.
- Numeric output uses tabular numerals through the mono stack.

## 4. Spacing & Layout

### Base Unit

Spacing intent follows a 4px base even though the extracted single-file demo includes optical
exceptions.

| Token | Value | Usage |
|---|---:|---|
| `space-1` | 4px | Tight alignment |
| `space-2` | 8px | Icon-label and rail gaps |
| `space-3` | 12px | Compact controls |
| `space-4` | 16px | Standard inset |
| `space-5` | 20px | Control panels |
| `space-6` | 24px | Cards and row rhythm |
| `space-8` | 32px | Section-local separation |
| `space-10` | 40px | Dense section boundary |
| `space-12` | 48px | Major grouping |
| `space-16` | 64px | Wide section rhythm |
| `space-24` | 96px | Maximum section rhythm |

### Grid

- Content maximum: 1180px.
- Fluid page gutter: `clamp(20px, 5vw, 72px)`.
- Reading measure: 38rem for prose and 60-64 characters for technical explanations.
- Breakpoints: 640px for compact mobile, 760px for narrow navigation, and 900px for the
  editorial rail-to-stack transition.
- Layout primitives: full-width band, centered band inner, flex cluster, auto-fit facts grid,
  editorial two-column rail, and single-column mobile stack.

### Rules

- Keep intrinsic layout mechanics such as `clamp()`, `minmax()`, percentages, and viewport
  units in the component CSS rather than inventing tokens for them.
- Optical values already present in the extracted page (3px radii, 9/13/18/22/30px insets)
  may be copied only inside the same component family. New reusable spacing intent uses the
  table above.

## 5. Components

The demo page itself is the state showcase. Its CSS selectors and JavaScript state classes
exercise the required variants over the 375px, 768px, and 1280px QA viewports.

### Project Header and Release Chip

- **Structure**: sticky `header`, wordmark, version chip, and project-link `nav`.
- **Variants**: wordmark, internal anchor, external npm/GitHub link, release chip.
- **States**: default, hover, keyboard focus, and initial settle animation.
- **Accessibility**: semantic navigation, visible `:focus-visible`, expanded hit areas.
- **Motion**: chip settle uses the emphasis easing and is removed for reduced motion.
- **Layout**: wrapping cluster; navigation moves below at narrow widths.

### Action Button

- **Structure**: inline-flex anchor or button with optional SVG glyph.
- **Variants**: primary ember, ghost outline, compact `mini`, completed compact action.
- **States**: default, hover, focus, copied/completed; no disabled variant is currently used.
- **Accessibility**: semantic control, visible focus, at least 44px touch height on mobile.
- **Motion**: only transform, color, background, and border transitions.

### Converter Platen

- **Structure**: file-input label, progress edge, outline, icon, title, subtitle, and privacy hint.
- **Variants**: empty and loaded.
- **States**: default, hover, focus-within, drag-hot, busy, loaded, and landed.
- **Accessibility**: native file input, associated label, focus outline, textual status outside
  the decorative outline.
- **Motion**: border march and icon transform signal a real hover/drag state; reduced motion
  disables them.
- **Layout**: centered stack that compacts into a loaded inline cluster.

### Process Rail and Status

- **Structure**: ordered rail plus polite live status region.
- **Variants**: read, inspect, render, and paint stages.
- **States**: dormant, active, done; status info, success, and error.
- **Accessibility**: decorative rail is hidden from assistive technology; status uses
  `role="status"` and `aria-live="polite"`.
- **Motion**: color and transform communicate stage progress only.

### Zoom Controls

- **Structure**: labelled range, numeric input, and explanatory hint inside a grouped panel.
- **States**: hidden before conversion, visible after conversion, hover, and focus.
- **Accessibility**: labelled group, explicit input labels, shared `aria-describedby`, keyboard
  range and number controls.
- **Layout**: flexible range plus bounded numeric column; compact mobile column remains within
  the viewport.

### Output Stage

- **Structure**: stage bar, idle state, actions, scroll body, and sandboxed iframe.
- **States**: idle, rendered, downloadable, and error status in the adjacent live region.
- **Accessibility**: iframe title is `Converted slide output`; converted active HTML runs with
  `allow-scripts` and without `allow-same-origin`.
- **Layout**: contained scroll owner; the generated slide scales as a whole without reflow.

### Editorial Band, Specification Table, and Code Pane

- **Structure**: section heading, optional note, content rail/table/pane grid.
- **States**: reveal, hover row/step, copy action, and copied confirmation.
- **Accessibility**: semantic headings, table headers/caption, copy button labels, and natural
  document reading order when the layout stacks.
- **Layout**: two-column editorial rail above 900px; one readable column below it. The table
  becomes labelled blocks below 640px.

## 6. Motion & Interaction

### Timing

| Type | Duration | Easing | Usage |
|---|---:|---|---|
| Micro | 160-220ms | `--ease` / `--ease-out` | Hover, focus, slider, borders |
| Standard | 280-460ms | `--ease-out` | Progress and metadata arrival |
| Emphasis | 560-760ms | `--ease-out` | Hero, release chip, save action |
| Signature | 14s cycle | component curves | Hero machine-bed narrative |
| Scroll reveal | observer-driven | `--ease-out` | Section and pipeline entry |

### Rules

- Motion must explain state, progress, or the hero's single pipeline narrative.
- Runtime transitions animate transform, opacity, color, background, or border; do not add
  layout-property animations.
- Scroll reveals use `IntersectionObserver`. Pointer tracking may alter the platen edge light
  but must not move layout.
- `prefers-reduced-motion: reduce` removes non-essential animation and makes revealed content
  immediately visible.

## 7. Depth & Surface

### Strategy

The strategy is mixed tonal-shift plus borders. `--ink`, `--ink-raised`, `--ink-panel`, and
`--ink-well` establish layers; one-pixel edge tokens make control and table boundaries exact.
There are no component drop shadows. The sticky topbar alone uses a translucent veil and
backdrop blur so content can pass underneath without losing navigation contrast.

## 8. Accessibility Constraints & Accepted Debt

### Constraints

- Target WCAG 2.2 AA for body contrast, keyboard operation, focus visibility, and reduced
  motion.
- Every interactive control remains keyboard reachable and visibly focused.
- Primary content must have no horizontal overflow at 375px, 768px, or 1280px.
- Converted output stays in an opaque-origin sandbox and input larger than 64 MiB is rejected
  before allocation.
- Capability copy must label summaries as highlights, disclose approximate/fallback limits,
  distinguish browser and native format scope, and link to the generated 56-feature ledger.
  Copy-only corrections reuse existing typography, spacing, and link primitives.

### Accepted Debt

| Item | Location | Why accepted | Owner / Exit |
|---|---|---|---|
| Single-file component layer | `demo/index.html` | Preserving the already reviewed deployment surface avoids a release-time refactor | Extract only with an explicitly approved demo architecture change |
| Optical raw values and one-off state colors | `demo/index.html` | Existing illustration and state details predate this extraction and are visually validated | Consolidate only during an approved visual-system refactor |
| English-only visible demo copy | `demo/index.html` | No localization infrastructure exists; the bilingual repository READMEs carry the Korean guidance | Add locale resources and a language contract in a separately scoped change |
| Remote Google Fonts | `demo/index.html` | Current visual identity depends on Newsreader, Archivo, and IBM Plex Mono | Self-host only after font licensing, payload, and fallback QA are reviewed |
