# PhotoTux MVP Tasks List

## Purpose

This is the end-to-end implementation list for reaching MVP.

Each task should produce a meaningful project outcome, not just isolated code movement.

## Status Legend

- `todo`: not started
- `doing`: in progress
- `done`: completed
- `blocked`: cannot proceed until a dependency is resolved

## MVP Tasks

### T01 - Scaffold the Rust workspace

- Status: `todo`
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

### T02 - Host a `wgpu` canvas inside the GTK4 shell

- Status: `todo`
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

### T03 - Implement viewport navigation basics

- Status: `todo`
- Outcome: the document can be viewed comfortably
- Includes:
  - checkerboard and pasteboard rendering
  - zoom behavior
  - pan behavior
  - viewport state model
- Depends on: T02
- Done when:
  - zoom and pan feel stable and predictable

### T04 - Create the tile-backed raster document core

- Status: `todo`
- Outcome: a source-of-truth document model suitable for editing and persistence
- Includes:
  - document, canvas, raster-layer, and metadata structures
  - tile storage
  - dirty-region tracking
  - stable IDs
- Depends on: T01
- Done when:
  - documents and layers can exist without any UI dependency

### T05 - Implement single-layer paint and erase

- Status: `todo`
- Outcome: direct editing on a raster surface works with acceptable latency
- Includes:
  - brush dab generation
  - eraser behavior
  - dirty-tile updates
  - basic brush properties: size, hardness, opacity, flow, spacing
- Depends on: T03, T04
- Done when:
  - painting and erasing work on a single document layer with low-latency feedback

### T06 - Add undo and redo for raster edits

- Status: `todo`
- Outcome: paint operations are reversible and grouped correctly
- Includes:
  - history entry model
  - tile snapshot or delta storage
  - brush stroke commit grouping
- Depends on: T05
- Done when:
  - a stroke undoes and redoes as one action without corruption

### T07 - Implement the native `.ptx` file format

- Status: `todo`
- Outcome: PhotoTux projects can be saved and reopened reliably
- Includes:
  - versioned manifest
  - per-layer payload structure
  - atomic-save behavior
  - load validation and error handling
- Depends on: T04, T06
- Done when:
  - a saved project reopens with matching structure and image content

### T08 - Implement PNG export and baseline image import

- Status: `todo`
- Outcome: external images can enter and leave the editor
- Includes:
  - flattened export path
  - PNG import
  - basic normalization into the internal raster model
- Depends on: T04, T07
- Done when:
  - imported images can be edited and exported correctly

### T09 - Expand from one layer to layered documents

- Status: `todo`
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

- Status: `todo`
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

### T11 - Implement the Layers, Properties, Color, and History panels

- Status: `todo`
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

- Status: `todo`
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

- Status: `todo`
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

- Status: `todo`
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

- Status: `todo`
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

- Status: `todo`
- Outcome: the promised MVP interchange set is complete
- Includes:
  - JPEG import and export
  - WebP import and export
  - user-facing error behavior for malformed files
- Depends on: T08
- Done when:
  - the documented import/export set works end to end

### T17 - Add keyboard shortcuts and command routing

- Status: `todo`
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

- Status: `todo`
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

- Status: `todo`
- Outcome: long-running work stops blocking the shell
- Includes:
  - background execution for save/load, autosave, import/export, and thumbnails
  - result delivery back into the session layer
  - prioritization for user-visible tasks
- Depends on: T07, T08, T16, T18
- Done when:
  - common file operations do not freeze the editor shell

### T20 - Validate and stabilize the MVP workflow

- Status: `todo`
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