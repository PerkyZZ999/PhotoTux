# Text And Design Tools Tasks

## Purpose

This task list turns the text/design plan into an implementation sequence for adding text layers and related design-oriented workflows.

## Principles

- do not force text into the raster-layer model if that weakens architecture clarity
- keep editable text state distinct from transient shell editing state
- preserve viewport/export parity as a release-blocking rule

## Task List

### TEXT01 - Define text-layer document model

- [x] Status: completed
- Outcome: text is represented as a first-class document concept
- Includes:
  - decide text-layer structure
  - define editable content and style fields
  - decide transform ownership model for text layers
- Depends on: none
- Done when:
  - text-layer state is represented headlessly in `doc_model`

### TEXT02 - Define text rendering and rasterization boundaries

- [x] Status: completed
- Outcome: viewport, export, and save/load semantics are explicit before rendering work expands
- Includes:
  - decide how editable text is rasterized for viewport and export
  - decide where font/layout work lives
  - define parity requirements for rendered text
- Depends on: TEXT01
- Done when:
  - text rendering boundaries are documented and technically clear

### TEXT03 - Add text-layer persistence

- [x] Status: completed
- Outcome: text layers can survive save/load as editable content
- Includes:
  - `.ptx` save/load support
  - versioned manifest behavior for text layers
  - save/load regression tests
- Depends on: TEXT01
- Done when:
  - text layers roundtrip without collapsing into raster content unintentionally

### TEXT04 - Add text tool and placement workflow

- [x] Status: completed
- Outcome: users can create text layers from the shell
- Includes:
  - text tool selection and command routing
  - placement and commit/cancel flow
  - selection and active-layer integration
- Depends on: TEXT01, TEXT02
- Done when:
  - a user can create and place a text layer predictably
- Notes:
  - `ui_shell` now exposes a dedicated text tool, layer-row text selection, and placement-driven text-session requests.
  - `app_core` now owns the text placement session and commit/cancel boundary instead of shell-local transient text state.

### TEXT05 - Add text editing state and shell controls

- [x] Status: completed
- Outcome: text layers are editable rather than one-shot inserted objects
- Includes:
  - content editing workflow
  - font, size, color, and alignment controls
  - properties-panel integration
- Depends on: TEXT04
- Done when:
  - text editing is usable for real design tasks
- Notes:
  - shell-driven text editing now routes through a modal GTK dialog plus Properties-panel summary rows.
  - initial controls cover content, font family subset, font size, line height, letter spacing, fill RGBA, and alignment.

### TEXT06 - Add viewport rendering for text layers

- [x] Status: completed
- Outcome: editable text appears correctly in the canvas
- Includes:
  - text rendering integration with `render_wgpu` or its boundary
  - overlay/selection behavior for text layers
  - transform preview support where relevant
- Depends on: TEXT02, TEXT05
- Done when:
  - text layers render correctly in the viewport during normal editing
- Notes:
  - text preview now runs through the shared document flatten boundary, so committed text and in-progress text sessions both render into the viewport without GTK text ownership.
  - move-tool text repositioning uses document-owned text transforms and participates in the same visible canvas path as committed layers.

### TEXT07 - Add export support for text layers

- [x] Status: completed
- Outcome: text is part of trusted visual output, not just the viewport
- Includes:
  - flatten/export handling for text layers
  - export parity coverage with representative text scenes
- Depends on: TEXT06
- Done when:
  - export matches visible text rendering reliably
- Notes:
  - `file_io::flatten_document_rgba` now composites text layers directly, so PNG/JPEG/WebP export and viewport rendering share the same text rasterization path.
  - export parity is covered with a representative mixed raster-plus-text design scene in `app_core` tests.

### TEXT08 - Add undo/redo coverage for text workflows

- [x] Status: completed
- Outcome: text editing behaves like a first-class editor workflow
- Includes:
  - insertion/edit/style/transform undo behavior
  - grouping rules for text edits where needed
- Depends on: TEXT05, TEXT06
- Done when:
  - text workflows are trusted under history operations
- Notes:
  - text add, edit, delete, and move operations now serialize through controller-owned `TextLayerRecord` history entries.
  - representative controller tests now cover add/edit/move undo and redo behavior.

### TEXT09 - Add representative text/design fixtures

- [x] Status: completed
- Outcome: text behavior remains stable as the feature grows
- Includes:
  - simple single-text-layer scene
  - multi-layer design scene with text plus raster layers
  - export-versus-viewport parity scene
- Depends on: TEXT07, TEXT08
- Done when:
  - text support has canonical regression scenes
- Notes:
  - `app_core` tests now include a simple single-text-layer fixture, a multi-layer design fixture, and an export-versus-viewport parity fixture.
  - those scenes are now part of the normal Rust test suite instead of being ad hoc manual setups.

### TEXT10 - Define limits and non-goals for the initial text release

- [x] Status: completed
- Outcome: the project stays honest about what its text system can and cannot do
- Includes:
  - supported typography subset
  - unsupported layout features
  - docs for user expectations and future expansion boundaries
- Depends on: TEXT09
- Done when:
  - text-layer support is documented precisely enough to avoid scope drift
- Notes:
  - the initial release is documented as a bitmap-font, single-style text subset with no shaping engine, system font loading, rich text, IME, kerning, or path text.
  - the technical spec and testing strategy now both call out the text subset and its parity fixtures explicitly.

## Suggested Execution Order

1. TEXT01
2. TEXT02
3. TEXT03
4. TEXT04
5. TEXT05
6. TEXT06
7. TEXT07
8. TEXT08
9. TEXT09
10. TEXT10

## Notes

- If text rendering architecture becomes too invasive, narrow the initial text feature to a simpler editable subset rather than coupling it tightly to raster layers.
- The initial shipped subset intentionally chose that narrower editable path: deterministic bitmap text through a shared flatten boundary rather than a fully general typography stack.
