# Painting And Input Expressiveness Tasks

## Purpose

This task list turns the painting/input plan into an implementation sequence for improving illustration-oriented workflows.

## Principles

- preserve the current low-latency paint path
- do not let richer input regress mouse workflows
- keep destructive operations off the UI thread

## Task List

### PAINT01 - Audit current brush/input extension points

- [x] Status: completed
- Outcome: richer input work starts from measured extension points rather than assumptions
- Includes:
  - identify current brush parameter boundaries
  - identify where stylus pressure can enter safely
  - document current latency-sensitive hotspots
- Depends on: none
- Done when:
  - the brush/input expansion path is technically mapped

Progress notes:
- the current brush parameter boundary is controller-owned: `app_core` creates `BrushSettings` through `current_brush_settings`, which currently fixes radius, hardness, opacity, spacing, and color before handing stroke work to `tool_system`.
- `ui_shell` currently forwards drag position only through `GestureDrag`; there is no pressure, tilt, or stylus-specific sample data entering the controller path yet, which makes the shell-to-controller seam the correct entry point for future pressure plumbing.
- `tool_system` is the current stroke-evaluation boundary: it interpolates dab positions from point samples using spacing, resolves selection clipping, and converts `BrushSettings` into `image_ops::BrushDab` instances.
- `image_ops` is already pressure-ready in shape: radius, hardness, and opacity are per-dab values, so pressure mapping should happen before raster application rather than being embedded into raster-tile ownership or shell code.
- the latency-sensitive hotspots are the shell drag-to-controller interaction path, `app_core::apply_active_layer_stroke_segment`, `tool_system::interpolate_dab_positions`, and the direct dab application/update-flattened-region loop that keeps brush edits visible without rebuilding the entire canvas every sample.

### PAINT02 - Add pressure data plumbing through the interaction path

- [x] Status: completed
- Outcome: stylus data can reach brush evaluation cleanly
- Includes:
  - shell input plumbing for pressure where supported
  - controller/tool-system propagation of pressure samples
  - mouse fallback behavior
- Depends on: PAINT01
- Done when:
  - brush evaluation can receive pressure information without changing document ownership boundaries

Progress notes:
- `ui_shell` now attaches a GTK `GestureStylus` controller alongside the existing drag controller and captures pressure through `AxisUse::Pressure` where the device reports it.
- `app_core` now exposes pressure-aware interaction entry points for canvas begin/update events while preserving the previous default path for mouse callers as pressure `1.0`.
- `tool_system` now carries explicit `BrushSample` values with pressure through stroke interpolation, so brush evaluation receives sample pressure without moving document ownership into the shell.
- mouse workflows remain stable because non-stylus callers still flow through the same brush path with normalized pressure fallback set to `1.0`.

### PAINT03 - Add initial pressure mapping controls

- [x] Status: completed
- Outcome: pressure support becomes useful instead of merely available
- Includes:
  - pressure-to-size mapping
  - pressure-to-opacity or flow mapping
  - toggle or preset support
- Depends on: PAINT02
- Done when:
  - pressure-sensitive devices materially affect brush behavior in a controlled way

Progress notes:
- `tool_system::BrushSettings` now has explicit pressure-to-size and pressure-to-opacity mapping controls, with conservative mappings that keep mouse behavior unchanged while allowing stylus pressure to modulate dab radius and opacity.
- `app_core` owns the initial mapping toggles and exposes them through shell snapshots, keeping brush-behavior policy out of GTK widget state.
- `ui_shell` now surfaces the initial pressure controls in the existing Properties panel rather than introducing a second brush settings surface.
- regression coverage now includes pressure-sample interpolation, pressure-sensitive dab mapping, and controller snapshot updates for the initial pressure-toggle path.

### PAINT04 - Improve brush dynamics and parameter range

- [x] Status: completed
- Outcome: brush behavior becomes more expressive for both mouse and stylus use
- Includes:
  - spacing improvements
  - flow behavior review
  - hardness behavior refinement
  - parameter validation and regression tests
- Depends on: PAINT01
- Done when:
  - brush behavior has broader control without becoming unstable

Progress notes:
- `app_core` now owns adjustable brush radius, hardness, spacing, and flow state instead of hard-coding all brush dynamics in one fixed settings helper.
- `tool_system::BrushSettings` now validates radius, hardness, spacing, opacity, and flow ranges before dab evaluation, which keeps richer brush controls bounded and predictable.
- brush spacing is now clamped against the active radius range, and tile-touch resolution now uses the effective dab radius rather than the previous base radius, which avoids oversized invalidation when pressure shrinks a dab.
- `image_ops` now applies a smoother soft-edge falloff for partial-hardness brushes and includes explicit flow in per-dab alpha evaluation, which makes soft brushes and lower-flow strokes behave more naturally without changing ownership boundaries.
- `ui_shell` now surfaces the current brush radius, hardness, spacing, and flow in the existing Properties panel with direct adjustment controls, and regression coverage now includes settings validation, flow behavior, and controller snapshot updates for the brush parameter path.

### PAINT05 - Add brush preset structure if needed

- [x] Status: completed
- Outcome: richer brush behavior stays usable in practice
- Includes:
  - preset model
  - preset switching hooks in shell/controller
  - persistence rules if presets become user data
- Depends on: PAINT03, PAINT04
- Done when:
  - users can return to useful brush setups quickly

Progress notes:
- `app_core` now owns a lightweight built-in preset model with named brush setups for balanced round work, soft shading, and ink-style tapering, so the brush system can jump between useful parameter bundles instead of forcing repeated manual tweaking.
- `ui_shell` now exposes preset switching hooks in the existing Properties panel and surfaces the active preset name through `ShellSnapshot` alongside the underlying brush controls.
- manual parameter or pressure-toggle edits now clear the active preset back to `Custom`, which keeps preset state honest instead of pretending a tweaked setup still matches a built-in bundle.
- preset persistence is intentionally deferred: current presets are built-in controller-owned defaults, not user-authored data, so they do not yet affect `.ptx` persistence rules or workspace preferences.

### PAINT06 - Strengthen direct-manipulation preview for richer brushes

- [x] Status: completed
- Outcome: preview quality keeps pace with more expressive brush behavior
- Includes:
  - preview-path refinement
  - brush cursor/preview improvements where appropriate
  - latency regression checks
- Depends on: PAINT03, PAINT04
- Done when:
  - richer brushes still feel immediate during normal use

Progress notes:
- `ui_shell` now tracks canvas hover locally through GTK motion events, so brush and eraser preview updates stay in the shell-side direct-manipulation path instead of mutating document state or forcing controller-owned hover bookkeeping.
- brush and eraser tools now render a live overlay preview from the current pointer position using the existing viewport overlay path system: an outer radius ring, a hardness ring, a center crosshair, and a spacing marker.
- the hover preview respects the current brush radius, hardness, spacing, and pressure-to-size toggle, so stylus pressure changes are visible before pixels commit.
- focused `ui_shell` regression coverage now checks preview-radius pressure mapping, preview-path construction, and status-surface reporting for richer brush settings.

### PAINT07 - Add regression and profiling coverage for paint-heavy paths

- [x] Status: completed
- Outcome: richer painting features do not silently degrade responsiveness
- Includes:
  - repeated stroke stress checks
  - medium-canvas painting validation
  - pressure-enabled manual validation notes
- Depends on: PAINT06
- Done when:
  - the richer brush path has explicit performance guardrails

Progress notes:
- `app_core` now includes a dedicated medium-canvas repeated-stroke regression that drives twelve pressure-enabled brush strokes end to end, checks viewport-versus-flattened parity, and verifies undo/redo roundtrips across the full stress pass.
- the same regression asserts that paint-heavy interaction remains localized to a bounded tile set instead of degenerating into full-canvas churn for a medium document.
- `docs/tests/post-mvp-painting-checklist.md` now records the manual validation pass for mouse regression, pressure preview parity, repeated medium-canvas strokes, and eraser preview/commit parity.
- the painting/input track now has both focused automated coverage and a named manual checklist instead of relying on ad hoc brush testing.

### PAINT08 - Define destructive filter command model

- [x] Status: completed
- Outcome: later filter work can plug into the existing architecture cleanly
- Includes:
  - filter command routing
  - history integration rules
  - worker/job execution rules
- Depends on: none
- Done when:
  - destructive raster operations have a clean architectural slot

Progress notes:
- `app_core` now owns an explicit destructive-filter command path instead of leaving Filter menu work as UI-only placeholders, so filter actions route through the same controller layer that already owns history, autosave, and file operations.
- destructive filters now run through the existing worker-job system with revision-guarded result application: if the document changes before a queued filter finishes, the result is discarded instead of silently overwriting newer edits.
- history integration now uses full `LayerStateSnapshot` before/after records for destructive filters, which keeps undo and redo trustworthy without inventing a second persistence path for raster operations.
- the current command model is intentionally narrow: filters apply to active layer pixels only for now, and mask-target or in-transform states are rejected explicitly instead of being handled inconsistently.

### PAINT09 - Implement a minimal destructive filter set

- [x] Status: completed
- Outcome: the first non-paint raster operations become available safely
- Includes:
  - choose a narrow first filter set
  - CPU-side implementation in `image_ops` or appropriate boundary
  - undo/redo and save/load validation
- Depends on: PAINT08
- Done when:
  - at least one destructive filter workflow is end-to-end and trustworthy

Progress notes:
- the first shipped destructive filter set is deliberately narrow: `Invert Colors` and `Desaturate`, both CPU-side in `image_ops` and both scoped to active layer pixels.
- `ui_shell` now exposes those filters through the Filter menu, while `app_core` handles queueing, completion status, and history recording.
- focused regression coverage now verifies end-to-end background execution, viewport-versus-flattened parity after filter application, undo/redo correctness, and safe discard when the document changes before a filter result is applied.
- the post-MVP painting checklist now includes manual filter validation so save/reopen and commit parity are checked outside unit coverage as well.

### PAINT10 - Validate stylus and paint ergonomics on target Linux systems

- [x] Status: completed
- Outcome: the expressive-painting track is grounded in real device behavior
- Includes:
  - stylus manual validation
  - mouse regression validation
  - latency and responsiveness notes
- Depends on: PAINT03, PAINT06, PAINT09
- Done when:
  - painting improvements are validated beyond unit correctness

Progress notes:
- a fresh `kwin-mcp` isolated session now starts successfully in this environment again, which allowed a live Linux mouse-regression pass against the built `app_core` binary instead of stopping at planning.
- the 2026-03-23 pass validated brush paint, eraser paint removal, and undo/redo routing end to end through real pointer and keyboard input while the shell stayed responsive and the app log remained clean.
- the same pass confirmed that launching the built binary from a dedicated temporary working directory avoids the earlier Cargo-in-isolated-home problem and keeps the run focused on runtime behavior instead of toolchain setup.
- stylus-specific validation remains hardware-dependent, but the current continuation explicitly skips that requirement because no pressure-reporting tablet is available in this environment.
- `PAINT10` is therefore closed on the validated Linux mouse path: painting, erasing, undo/redo, and shell responsiveness were confirmed, while any future stylus-specific follow-up can be treated as additional validation rather than a blocker for the rest of post-MVP sequencing.

## Suggested Execution Order

1. PAINT01
2. PAINT02
3. PAINT03
4. PAINT04
5. PAINT06
6. PAINT07
7. PAINT05
8. PAINT08
9. PAINT09
10. PAINT10

## Notes

- If pressure input is inconsistent across devices, ship a narrow pressure subset before broad brush dynamics.
- If richer brush behavior hurts latency, prioritize path optimization over adding more brush parameters.