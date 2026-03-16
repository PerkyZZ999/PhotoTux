# PhotoTux MVP Tasks List

## Purpose

This is the end-to-end implementation list for reaching MVP.

Each task should produce a meaningful project outcome, not just isolated code movement.

## Checklist Legend

- `[ ]` not started
- `[-]` in progress
- `[x]` completed
- `[!]` blocked

## Current Progress Snapshot

- [x] Copilot instructions created
- [x] T01 - Scaffold the Rust workspace
- [x] T02 - Host a `wgpu` canvas inside the GTK4 shell
- [x] T03 - Implement viewport navigation basics
- [x] T04 - Create the tile-backed raster document core
- [x] T05 - Implement single-layer paint and erase
- [x] T06 - Add undo and redo for raster edits
- [x] T07 - Implement the native `.ptx` file format
- [x] T08 - Implement PNG export and baseline image import
- [x] T09 - Expand from one layer to layered documents
- [x] T10 - Build the fixed professional shell layout
- [x] T11 - Implement the Layers, Properties, Color, and History panels
- [x] T12 - Add move-tool workflow
- [x] T13 - Add rectangular selection workflow
- [x] T14 - Add simple transform workflow
- [x] T15 - Add initial blend modes
- [x] T16 - Complete import and export support for JPEG and WebP
- [x] T17 - Add keyboard shortcuts and command routing
- [x] T18 - Add autosave and crash recovery
- [x] T19 - Harden responsiveness with a lightweight job system
- [x] T20 - Validate and stabilize the MVP workflow

## MVP Tasks

### T01 - Scaffold the Rust workspace

- [x] Status: completed
- Outcome: a clean multi-crate workspace matching the planned architecture
- Includes:
  - root workspace setup
  - crate creation for core subsystems
  - base dependencies and shared linting configuration
  - basic CI or local validation commands
- Depends on: none
- Done when:
  - the workspace builds
  - crate boundaries reflect the architecture docs

Progress notes:
- workspace root `Cargo.toml` created
- architecture crates scaffolded under `crates/`
- shared lint and dependency policy added
- `cargo check` and `cargo test` pass for the scaffolded workspace

### T02 - Host a `wgpu` canvas inside the GTK4 shell

- [x] Status: completed
- Outcome: a working native window with a live GPU-backed canvas region
- Includes:
  - GTK4 application shell startup
  - canvas widget or host surface integration
  - resize handling
  - scale-factor handling
  - stable initialization and shutdown paths
- Depends on: T01
- Done when:
  - the shell opens reliably
  - the canvas redraws correctly on resize and scale changes

Progress notes:
- `ui_shell` now hosts a live `wgpu`-driven canvas view inside the GTK shell
- the canvas redraw path handles resize and scale-factor changes
- startup was smoke-tested through `cargo run -p app_core`

### T03 - Implement viewport navigation basics

- [x] Status: completed
- Outcome: the document can be viewed comfortably
- Includes:
  - checkerboard and pasteboard rendering
  - zoom behavior
  - pan behavior
  - viewport state model
- Depends on: T02
- Done when:
  - zoom and pan feel stable and predictable

Progress notes:
- viewport zoom and pan state lives in `render_wgpu`
- fit-to-canvas and zoom-towards math are covered by unit tests
- the live canvas host supports pan drag and scroll zoom
- checkerboard canvas rendering and pasteboard framing now come from the `wgpu` render path

### T04 - Create the tile-backed raster document core

- [x] Status: completed
- Outcome: a source-of-truth document model suitable for editing and persistence
- Includes:
  - document, canvas, raster-layer, and metadata structures
  - tile storage
  - dirty-region tracking
  - stable IDs
- Depends on: T01
- Done when:
  - documents and layers can exist without any UI dependency

Progress notes:
- `Document`, `RasterLayer`, and blend-mode foundations are in place
- layer add, rename, reorder, visibility, opacity, and delete behavior are covered by tests
- tile size, tile-grid math, lazy tile allocation, and dirty-tile tracking are now part of the document model foundation

### T05 - Implement single-layer paint and erase

- [x] Status: completed
- Outcome: direct editing on a raster surface works with acceptable latency
- Includes:
  - brush dab generation
  - eraser behavior
  - dirty-tile updates
  - basic brush properties: size, hardness, opacity, flow, spacing
- Depends on: T03, T04
- Done when:
  - painting and erasing work on a single document layer with low-latency feedback

Progress notes:
- `image_ops` now contains a tested round-brush dab raster operation
- the current brush foundation supports radius, hardness, opacity, tile origin handling, and alpha blending
- `tool_system` now applies paint and erase strokes directly into tile-backed document layers
- stroke records now capture per-tile before and after snapshots for undoable edits
- stroke interpolation now places multiple dabs along a path using brush spacing
- `app_core` now drives live brush and eraser interactions from the shell and commits them as single undoable stroke actions
- `ui_shell` and `render_wgpu` now present the live flattened canvas raster so paint and erase are visible on the actual document surface during interaction

### T06 - Add undo and redo for raster edits

- [x] Status: completed
- Outcome: paint operations are reversible and grouped correctly
- Includes:
  - history entry model
  - tile snapshot or delta storage
  - brush stroke commit grouping
- Depends on: T05
- Done when:
  - a stroke undoes and redoes as one action without corruption

Progress notes:
- `history_engine` now has a tested undo/redo stack foundation with proper redo invalidation on new edits
- budget configuration and core stack transitions are covered by unit tests
- brush strokes now produce a single `BrushStrokeRecord` with tile snapshots before and after the edit
- `tool_system` tests now validate undo and redo of a full stroke as one action through the history stack

### T07 - Implement the native `.ptx` file format

- [x] Status: completed
- Outcome: PhotoTux projects can be saved and reopened reliably
- Includes:
  - versioned manifest
  - per-layer payload structure
  - atomic-save behavior
  - load validation and error handling
- Depends on: T04, T06
- Done when:
  - a saved project reopens with matching structure and image content

Progress notes:
- project manifest structures are in place in `file_io`
- manifest JSON roundtrip tests pass
- version and per-layer payload-path foundations are established
- `file_io` now writes and loads `.ptx` project files with embedded per-layer tile payloads
- save operations use a temporary file plus rename for atomic replacement behavior
- save/load roundtrip tests verify restored structure and tile image content

### T08 - Implement PNG export and baseline image import

- [x] Status: completed
- Outcome: external images can enter and leave the editor
- Includes:
  - flattened export path
  - PNG import
  - basic normalization into the internal raster model
- Depends on: T04, T07
- Done when:
  - imported images can be edited and exported correctly

Progress notes:
- `file_io` now flattens document layers into RGBA output for PNG export
- baseline PNG import restores raster content into the internal tile-backed document model
- PNG export/import roundtrip tests pass with pixel-content verification

### T09 - Expand from one layer to layered documents

- [x] Status: completed
- Outcome: the editor supports real compositing structure
- Includes:
  - create, rename, duplicate, delete, reorder layers
  - visibility toggle
  - opacity control
  - layer composite ordering
- Depends on: T04, T07, T08
- Done when:
  - multi-layer projects can be edited, saved, reopened, and exported

Progress notes:
- `doc_model` now supports active-layer selection and layer duplication with copied tile content
- multi-layer save/load roundtrip tests verify preserved ordering, opacity, and per-layer tile payloads
- flattening tests verify composite order, visibility toggles, and opacity handling during export

### T10 - Build the fixed professional shell layout

- [x] Status: completed
- Outcome: the application has the planned three-column editor workspace
- Includes:
  - header bar or title region
  - menu bar
  - top tool options strip
  - left tool rail
  - central document region
  - right panel stack
  - status bar
- Depends on: T02, T03
- Done when:
  - the shell structure matches the layout spec and stays usable at target sizes

Progress notes:
- GTK application startup is wired through `app_core` and `ui_shell`
- the shell now follows the documented dark pro layout direction from the design docs with denser chrome and panel framing
- the document region includes a tab strip, ruler framing, and a live renderer-backed canvas surface
- the right side is structured as grouped panel sections with a Photoshop-style fixed dock instead of generic placeholders
- the shell chrome now uses a vendored Remix Icon subset for the tool rail, dock strip, search affordance, file actions, and visibility controls, matching the icon-first direction in the UI layout spec

### T11 - Implement the Layers, Properties, Color, and History panels

- [x] Status: completed
- Outcome: the main editing panels are operational rather than decorative
- Includes:
  - layer list interactions
  - property display for current selection or layer
  - color controls for foreground and background where needed
  - visible history list
- Depends on: T09, T10
- Done when:
  - core editing state can be inspected and controlled from the shell

Progress notes:
- `app_core` now owns a live shell controller rather than leaving document-like state inside the UI layer
- `ui_shell` panels now read real document, color, and history snapshots through that controller boundary
- the Layers panel supports selection, visibility toggles, add, duplicate, delete, and reorder actions
- the Properties panel shows active-layer state and supports opacity adjustment controls
- the Color panel exposes foreground and background color state with swap and reset actions
- the History panel shows a visible action list driven from the controller history stack

### T12 - Add move-tool workflow

- [x] Status: completed
- Outcome: raster content can be repositioned intentionally
- Includes:
  - move interaction model
  - document updates
  - history integration
  - viewport preview behavior
- Depends on: T09, T10
- Done when:
  - selected content or layer movement behaves predictably and is undoable

Progress notes:
- `doc_model` now supports per-layer positional offsets as part of the document source of truth
- `file_io` save/load and PNG export now respect layer offsets during persistence and flattening
- `tool_system` now has an undoable move-layer record with unit tests for apply, undo, and redo behavior
- `ui_shell` now routes canvas drag gestures through the selected tool instead of treating every drag as viewport pan
- the move tool now previews active-layer bounds live on the canvas while dragging and commits a single history entry on release
- `app_core` now stores typed history entries for move operations instead of only text labels, and the History panel can undo and redo them from the shell
- live canvas raster presentation now makes move-tool preview visible on the document image itself, not only through bounds overlays

### T13 - Add rectangular selection workflow

- [x] Status: completed
- Outcome: selection becomes part of the editing model
- Includes:
  - marquee selection creation
  - clear selection
  - invert selection
  - overlay rendering
- Depends on: T09, T10
- Done when:
  - selection state is visible, stable, and usable by editing tools

Progress notes:
- `doc_model` now has source-of-truth rectangular selection state and selection helpers
- `tool_system` now includes a rectangular marquee tool with normalized selection records and undo/redo tests
- `app_core` and `ui_shell` now support marquee drag interactions on the canvas
- `render_wgpu` now draws visible selection overlays into the offscreen frame for live feedback
- selection state now supports clear and invert actions with undo/redo through the shell History panel and Properties controls
- paint and erase strokes in `tool_system` now respect the current rectangular selection, including inverted selection state

### T14 - Add simple transform workflow

- [x] Status: completed
- Outcome: users can scale and translate content reliably
- Includes:
  - transform preview
  - commit path
  - deterministic resampling
  - history integration
- Depends on: T12, T13
- Done when:
  - preview and committed output match expected results

Progress notes:
- `tool_system` now includes a simple transform tool that scales and translates active-layer raster content with deterministic nearest-neighbor resampling
- `doc_model` now exposes layer-state snapshots so transform commit and history can restore raster content and offsets exactly
- `app_core` now owns a transform preview session with drag-to-translate, scale controls, and single-action commit or cancel behavior
- `ui_shell` now exposes a Transform tool plus Properties controls for starting, scaling, committing, and canceling the previewed transform
- transform preview now flows through the same flattened canvas presentation path as the committed image, reducing preview-versus-commit mismatch in the current architecture

### T15 - Add initial blend modes

- [x] Status: completed
- Outcome: layered compositing becomes meaningfully useful
- Includes:
  - Normal
  - Multiply
  - Screen
  - Overlay
  - Darken
  - Lighten
- Depends on: T09
- Done when:
  - blend output is correct in viewport, save, and export paths

Progress notes:
- `color_math` now contains tested blend math for Normal, Multiply, Screen, Overlay, Darken, and Lighten compositing
- `file_io` flattening now uses the shared blend-math path, which keeps viewport-fed raster presentation and export output aligned in the current architecture
- `doc_model` now exposes active-layer blend-mode updates instead of treating blend mode as load-only metadata
- `app_core` and `ui_shell` now expose simple blend-mode cycling controls in the Properties panel so the initial blend set is usable from the shell

### T16 - Complete import and export support for JPEG and WebP

- [x] Status: completed
- Outcome: the promised MVP interchange set is complete
- Includes:
  - JPEG import and export
  - WebP import and export
  - user-facing error behavior for malformed files
- Depends on: T08
- Done when:
  - the documented import/export set works end to end

Progress notes:
- `file_io` now supports JPEG and WebP import/export alongside PNG using the shared flattened composite path
- JPEG export now flattens transparency against an opaque white background while WebP export preserves alpha through the RGBA path
- malformed JPEG and WebP imports now return contextual errors instead of failing with raw codec messages only
- regression tests now cover PNG, JPEG, and WebP roundtrips plus malformed-file error handling

### T17 - Add keyboard shortcuts and command routing

- [x] Status: completed
- Outcome: the editor becomes efficient for repeat use
- Includes:
  - core tool shortcuts
  - undo and redo
  - save
  - zoom controls
  - selection shortcuts where in scope
- Depends on: T10, T11, T12, T13
- Done when:
  - the main workflow no longer depends on menu-only access

Progress notes:
- `ui_shell` now installs a window-level keyboard controller for tool selection, undo and redo, selection actions, transform commit and cancel, and viewport zoom controls
- `app_core` now exposes a quick-save command route so `Ctrl+S` saves the current document to its known path or to the current working directory using the current document title
- zoom shortcuts now operate through the existing canvas viewport state rather than introducing duplicate zoom state in the controller layer
- the shortcut set covers the current MVP workflows: `V`, `M`, `T`, `B`, `E`, `H`, `Z`, `Ctrl+Z`, `Ctrl+Shift+Z`, `Ctrl+Y`, `Ctrl+S`, `Ctrl+D`, `Ctrl+I`, `Ctrl++`, `Ctrl+-`, `Ctrl+0`, `Enter`, and `Escape`

### T18 - Add autosave and crash recovery

- [x] Status: completed
- Outcome: editing sessions are resilient to crashes or forced shutdowns
- Includes:
  - autosave triggers
  - recovery-file management
  - startup recovery prompt or flow
  - protection of the primary document file
- Depends on: T07, T09
- Done when:
  - recovery works without corrupting the main document or history behavior

Progress notes:
- `app_core` now tracks dirty state separately for primary saves and autosaves, and triggers background autosave after an idle interval following document changes
- autosave writes a recovery file beside the primary `.ptx` target using `file_io` recovery-path helpers instead of touching the primary document file
- successful primary saves now clear stale recovery files so crash-recovery state does not linger after an explicit save
- startup now detects a recovery file for the current working document, loads it through the controller boundary, restores the document state, and records the recovery event in history
- regression tests now cover autosave recovery-file creation and startup recovery loading

### T19 - Harden responsiveness with a lightweight job system

- [x] Status: completed
- Outcome: long-running work stops blocking the shell
- Includes:
  - background execution for save/load, autosave, import/export, and thumbnails
  - result delivery back into the session layer
  - prioritization for user-visible tasks
- Depends on: T07, T08, T16, T18
- Done when:
  - common file operations do not freeze the editor shell

Progress notes:
- `app_core` now owns a lightweight prioritized worker queue built on standard threads, mutexes, condition variables, and result polling rather than an async runtime
- user-visible jobs are prioritized ahead of background maintenance jobs, which gives manual save and recovery load precedence over autosave work when both are queued
- manual save, autosave, and startup recovery load now run off the UI path and return results through the session/controller boundary before shell state is refreshed
- `ui_shell` now polls background job completions during its normal refresh cycle and surfaces controller status text in the status bar without moving document ownership into the shell
- the current worker path is ready to absorb future import/export and thumbnail jobs as those shell commands are expanded
- shell refresh now uses cached snapshots and live zoom tracking instead of rebuilding panel widgets on every timer tick, which removes the noticeable lag from tool-state updates while keeping background job polling lightweight

### T20 - Validate and stabilize the MVP workflow

- [x] Status: completed
- Outcome: MVP is trustworthy enough for real use
- Includes:
  - regression fixes
  - representative visual fixtures
  - save/load stress validation
  - large-document manual checks
  - export-versus-viewport verification
  - documentation refresh
- Depends on: T01 through T19
- Done when:
  - a real layered design or compositing workflow can be completed without major instability

Progress notes:
- representative compositing scenes now have explicit regression coverage for `.ptx` save/load, repeated roundtrips, and PNG export parity in `file_io`
- controller-level tests now assert that the viewport-facing canvas raster matches the shared flattened export path for a representative layered scene
- large sparse layered documents now have automated persistence and export consistency coverage to reduce risk around larger canvases and sparse tile distributions
- manual save no longer inserts a history entry, so undo still targets the last real edit immediately after a save
- documentation now records the canonical fixture categories and the current MVP validation coverage

## Suggested Execution Order

For fastest risk reduction, use this sequence:

1. T01
2. T02
3. T03
4. T04
5. T05
6. T06
7. T07
8. T08
9. T09
10. T10
11. T11
12. T12
13. T13
14. T14
15. T15
16. T16
17. T17
18. T18
19. T19
20. T20

## Notes

- If any task reveals a data-integrity problem, pause feature work and fix it first.
- If any task reveals a UI-thread blocking problem, treat it as a priority architectural issue.
- Do not start PSD, masks, groups, docking, or text work before T20 is complete.