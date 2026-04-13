# PhotoTux Testing Strategy

## Purpose

PhotoTux is a stateful graphics application. The main failure modes are not only crashes, but also subtle visual regressions, history corruption, save/load mismatch, and interaction stutter.

This testing strategy exists to catch those problems early.

## Testing Goals

The project must verify:

1. document integrity
2. save/load correctness
3. undo/redo correctness
4. image-output correctness
5. acceptable responsiveness for core workflows

## Test Pyramid

### 1. Unit Tests

Use unit tests for pure logic and deterministic rules.

Target areas:

- geometry math
- blend math
- alpha conversion rules
- tile indexing and dirty-region logic
- selection-mask logic
- history bookkeeping
- project manifest parsing and validation
- template-backed shell metadata and required widget-ID coverage

Unit tests should dominate the codebase because they are cheap and fast.

### 2. Integration Tests

Use integration tests for subsystem boundaries.

Target areas:

- native project save and load roundtrips
- document import normalization
- export matching expected composite output
- undo and redo across structural and raster edits
- autosave and recovery behavior
- renderer-facing dirty-tile invalidation contracts where testable

### 3. Visual Regression Tests

Use image-based regression tests for rendering-sensitive behavior.

Target areas:

- blend modes
- alpha edges
- transform output
- selection overlays
- checkerboard and pasteboard framing where useful
- export fixtures for representative scenes

Use tolerances conservatively and document them.
Do not hide real visual errors behind loose thresholds.

### 4. Manual Validation

Some problems are best caught through interactive testing.

Target areas:

- brush feel
- pan and zoom smoothness
- shortcut ergonomics
- shell responsiveness under autosave
- high-DPI and fractional-scaling behavior
- multi-monitor behavior on Wayland
- template-backed dialog and panel layout, focus, and sensitivity behavior under a real GTK session

## Representative Test Scenes

Maintain a small set of canonical test documents over time.

Recommended fixture categories:

- small single-layer paint scene
- medium layered compositing scene
- transparency edge scene
- blend-mode scene
- transform stress scene
- save/reopen fidelity scene

Current implemented fixture set:

- representative layered compositing scene in `file_io` tests
- repeated save/reopen scene in `file_io` tests
- PNG export parity scene in `file_io` tests
- large sparse document stress scene in `file_io` tests
- controller viewport/export parity scene in `app_core` tests
- autosave and recovery scenes in `app_core` tests
- masked compositing and grouped-hierarchy scenes in `file_io` tests
- lasso-aware transform parity fixture in `app_core` tests
- guide-snapping interaction fixtures in `app_core` tests
- single-text-layer editing scene in `app_core` tests
- mixed raster-plus-text design scene in `app_core` tests
- text export-versus-viewport parity scene in `app_core` tests

Current manual validation companion docs:

- MVP shell checklist in `docs/tests/kwin-mcp-test-checklist.md`
- post-MVP editing workflow checklist in `docs/tests/post-mvp-editing-workflow-checklist.md`
- post-MVP painting checklist in `docs/tests/post-mvp-painting-checklist.md`

These should be used repeatedly for regressions instead of inventing new ad hoc files every time.

Current GTK template-migration validation expectations:

- default automated coverage for template-backed shell surfaces lives in `crates/ui_shell/src/ui_templates.rs` and should verify embedded `.ui` metadata, required object IDs, CSS classes, and stable widget names where practical
- GTK builder-runtime assertions that instantiate widgets are not reliable in the default Cargo harness because GTK requires the real process main thread; keep those checks manual or explicitly ignored until a dedicated harness exists
- when migrating a new dialog or panel shell, manually confirm label hierarchy, tooltips, CSS class application, focus behavior, sensitivity, and overall GTK-native layout on a real desktop session

## Feature-Level Testing Requirements

### Painting

Must verify:

- tile updates are localized
- strokes commit as one history action
- undo restores the exact previous result
- eraser behavior follows alpha rules
- brush preview reflects active size and softness settings without requiring document mutation for hover-only updates
- repeated medium-canvas strokes preserve viewport-versus-flattened parity under pressure-enabled painting

### Filters

Must verify:

- destructive filters run through the worker/job path rather than blocking the UI thread
- undo restores the exact pre-filter layer state
- redo reapplies the same filtered result deterministically
- save, reopen, and export preserve the filtered result without viewport mismatch
- stale worker results do not overwrite newer document edits

### Layers

Must verify:

- reorder correctness
- visibility toggles
- opacity changes
- blend-mode application
- save and reopen fidelity

### Selection and Transform

Must verify:

- selection bounds correctness
- selection invert and clear
- transform preview versus committed result
- export result matching committed state

### Persistence

Must verify:

- atomic save behavior where implemented
- recovery-file isolation from the primary document
- versioned manifest compatibility behavior
- clear errors on malformed files

## Performance Test Expectations

Not every change needs formal benchmarking, but performance-sensitive areas must be checked deliberately.

Minimum checks:

- repeated brush strokes on a medium canvas
- pan and zoom on a medium layered document
- autosave during a normal editing session
- large import and export behavior

## Regression Policy

Any bug in these categories should produce a new test when practical:

- corrupted save/load
- undo mismatch
- visible alpha or blend artifacts
- broken selection math
- renderer invalidation causing incorrect output

## Pre-Milestone Validation Checklist

Before closing a milestone, confirm:

1. core automated tests pass
2. key visual fixtures still match expected output
3. representative manual workflows were exercised
4. no known data-loss bug remains open
5. no release-blocking mismatch exists between viewport and export output

T20 validation summary:

- representative compositing scenes now roundtrip through `.ptx` save/load without flattened-output drift
- repeated save/load cycles now preserve representative scene output
- PNG export is now checked against the same representative scene used for flattened composite validation
- large sparse layered documents now have automated save/load and export consistency coverage
- app-core controller tests now assert that the viewport-facing canvas raster matches the shared flattened export path when no preview-only workflow is active
- manual save no longer pollutes undo history, so the first undo after save still reaches the last real edit

## MVP Release-Blocking Bugs

These should block MVP completion:

- save/load corruption
- undo/redo corruption
- export not matching visible composite
- major canvas interaction stalls in common workflows
- incorrect coordinate behavior under normal scaling conditions
- crash recovery failure that risks data loss
