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
- [-] T05 - Implement single-layer paint and erase
- [-] T06 - Add undo and redo for raster edits
- [-] T07 - Implement the native `.ptx` file format
- [-] T10 - Build the fixed professional shell layout

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

- [-] Status: in progress
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

### T06 - Add undo and redo for raster edits

- [-] Status: in progress
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

### T07 - Implement the native `.ptx` file format

- [-] Status: in progress
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

### T08 - Implement PNG export and baseline image import

- [ ] Status: not started
- Outcome: external images can enter and leave the editor
- Includes:
  - flattened export path
  - PNG import
  - basic normalization into the internal raster model
- Depends on: T04, T07
- Done when:
  - imported images can be edited and exported correctly

### T09 - Expand from one layer to layered documents

- [ ] Status: not started
- Outcome: the editor supports real compositing structure
- Includes:
  - create, rename, duplicate, delete, reorder layers
  - visibility toggle
  - opacity control
  - layer composite ordering
- Depends on: T04, T07, T08
- Done when:
  - multi-layer projects can be edited, saved, reopened, and exported

### T10 - Build the fixed professional shell layout

- [-] Status: in progress
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
- the shell already has a header area, menu placeholder, tool options bar, left tool rail, document region, right dock, and status bar
- the document region now hosts a live renderer-backed canvas instead of a placeholder

### T11 - Implement the Layers, Properties, Color, and History panels

- [ ] Status: not started
- Outcome: the main editing panels are operational rather than decorative
- Includes:
  - layer list interactions
  - property display for current selection or layer
  - color controls for foreground and background where needed
  - visible history list
- Depends on: T09, T10
- Done when:
  - core editing state can be inspected and controlled from the shell

### T12 - Add move-tool workflow

- [ ] Status: not started
- Outcome: raster content can be repositioned intentionally
- Includes:
  - move interaction model
  - document updates
  - history integration
  - viewport preview behavior
- Depends on: T09, T10
- Done when:
  - selected content or layer movement behaves predictably and is undoable

### T13 - Add rectangular selection workflow

- [ ] Status: not started
- Outcome: selection becomes part of the editing model
- Includes:
  - marquee selection creation
  - clear selection
  - invert selection
  - overlay rendering
- Depends on: T09, T10
- Done when:
  - selection state is visible, stable, and usable by editing tools

### T14 - Add simple transform workflow

- [ ] Status: not started
- Outcome: users can scale and translate content reliably
- Includes:
  - transform preview
  - commit path
  - deterministic resampling
  - history integration
- Depends on: T12, T13
- Done when:
  - preview and committed output match expected results

### T15 - Add initial blend modes

- [ ] Status: not started
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

### T16 - Complete import and export support for JPEG and WebP

- [ ] Status: not started
- Outcome: the promised MVP interchange set is complete
- Includes:
  - JPEG import and export
  - WebP import and export
  - user-facing error behavior for malformed files
- Depends on: T08
- Done when:
  - the documented import/export set works end to end

### T17 - Add keyboard shortcuts and command routing

- [ ] Status: not started
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

### T18 - Add autosave and crash recovery

- [ ] Status: not started
- Outcome: editing sessions are resilient to crashes or forced shutdowns
- Includes:
  - autosave triggers
  - recovery-file management
  - startup recovery prompt or flow
  - protection of the primary document file
- Depends on: T07, T09
- Done when:
  - recovery works without corrupting the main document or history behavior

### T19 - Harden responsiveness with a lightweight job system

- [ ] Status: not started
- Outcome: long-running work stops blocking the shell
- Includes:
  - background execution for save/load, autosave, import/export, and thumbnails
  - result delivery back into the session layer
  - prioritization for user-visible tasks
- Depends on: T07, T08, T16, T18
- Done when:
  - common file operations do not freeze the editor shell

### T20 - Validate and stabilize the MVP workflow

- [ ] Status: not started
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