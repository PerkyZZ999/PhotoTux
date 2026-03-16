# PhotoTux KWin MCP Test Checklist

## Purpose

This document defines the GUI test checklist for the current PhotoTux application using `kwin-mcp` and `kwin-mcp-cli`.

It is intentionally scoped to the currently implemented MVP feature set. It does not assume post-MVP features such as masks, groups, lasso selection, PSD interoperability, or text layers.

The goal is to make sure we test the real app surface that exists today through KWin-based desktop automation on KDE Plasma Wayland.

## How To Use This Checklist

Use this document in three phases:

1. human manual preflight before automation work
2. initial GUI smoke coverage with `kwin-mcp`
3. deeper repeatable scenario coverage once the automation path is stable

This checklist is organized from highest-value risk reduction to more detailed validation.

## Current App Surface Covered By This Checklist

This checklist is based on the currently completed MVP feature set, including:

- app launch and fixed shell layout
- live `wgpu` canvas host
- viewport pan and zoom
- paint and erase
- layered documents
- layers, properties, color, and history panels
- move tool
- rectangular selection with clear and invert
- simple transform workflow
- initial blend modes
- PNG, JPEG, and WebP import/export
- keyboard shortcuts and command routing
- autosave and crash recovery
- lightweight background job handling

## Preflight Notes

Before using this checklist with automation:

- confirm the app launches manually
- confirm the shell looks sane visually
- confirm no obvious startup crash or render corruption exists
- confirm `kwin-mcp` itself is healthy on the machine

That manual preflight is useful because it prevents automation debugging from being confused with a broken app build.

## Test Execution Order

Use this order:

1. smoke and startup
2. shell structure and panel presence
3. document lifecycle
4. core tool workflows
5. layer and history workflows
6. persistence and recovery
7. shortcut coverage
8. import/export coverage
9. responsiveness and Linux-specific validation

## Checklist Legend

- `[ ]` not yet covered
- `[-]` partially covered or flaky
- `[x]` covered and stable

## Current Test Run Notes

### 2026-03-15 - Initial KWin MCP Pass

Completed so far:

- clean isolated KWin launch of the built PhotoTux binary
- window detection, focus, screenshot capture, and accessibility-tree inspection
- shell-region verification for the title area, menu bar, tool rail, central document region, right-side panel area, and status bar
- keyboard-driven brush, erase, marquee, undo, redo, clear-selection, invert-selection, save, zoom-out, zoom-in, and fit-to-view validation
- mouse-driven layer add and duplicate validation from the Layers panel
- transform-tool selection validation
- deterministic test-shortcut validation for layer visibility, blend-mode changes, opacity changes, layer reorder, delete, and structural undo/redo

Environment notes:

- launching with `cargo run -p app_core` inside `kwin-mcp` while `isolate_home=true` fails because the isolated home does not inherit the host `rustup` default toolchain configuration
- for GUI automation, prefer launching the already built binary instead of invoking Cargo inside the isolated session
- launching from the repository root can pick up an existing `.ptx.autosave` file and produce a recovered startup state, which contaminates clean-start smoke tests
- for clean startup tests, launch from a dedicated temporary working directory instead of the repository root
- `kwin-mcp` keyboard injection is currently more reliable than `focus_window` for PhotoTux in this session; direct focus calls found the app but did not always activate it
- several deeper panel-only workflows remain slower to automate because the current KWin tool surface in this session does not provide a direct semantic “invoke this accessibility action” primitive, so some checks still depend on careful coordinate targeting
- save, autosave, startup recovery, project reopen, and raster import/export routes are now wired through the shell surface; KWin automation has not yet been re-run against the new dialog-based file workflows

Issues found and addressed during this run:

- fixed the duplicate native/custom title bar issue by disabling GTK window decorations on the main shell window
- fixed zoom status reporting so the tab strip and status bar now show the live viewport zoom instead of a hard-coded `100%`
- fixed zoom-in shortcut handling so `Ctrl++` works through GTK key variants instead of depending only on character conversion
- added a test-only deterministic shortcut set for panel and transform actions, gated behind `PHOTOTUX_ENABLE_TEST_SHORTCUTS=1`
- replaced the first `Ctrl+Alt+...` automation shortcut attempt because it collided with KDE global shortcuts such as terminal launch
- fixed transform commit shortcut handling by moving the shell key controller to GTK capture phase so `Enter` is not swallowed by focused widgets
- wired the File menu to real controller-backed project open, raster import, and PNG/JPEG/WebP export commands using native GTK file dialogs and background jobs

## Test Shortcut Set

When running PhotoTux for KWin automation, enable the deterministic test shortcut set with:

```bash
PHOTOTUX_ENABLE_TEST_SHORTCUTS=1 /path/to/app_core
```

Current test-only shortcuts:

- `Ctrl+Shift+F1`: add layer
- `Ctrl+Shift+F2`: duplicate active layer
- `Ctrl+Shift+F3`: delete active layer
- `Ctrl+Shift+F4`: toggle active layer visibility
- `Ctrl+Shift+F5`: previous blend mode
- `Ctrl+Shift+F6`: next blend mode
- `Ctrl+Shift+F7`: decrease active-layer opacity
- `Ctrl+Shift+F8`: increase active-layer opacity
- `Ctrl+Shift+F9`: move active layer up
- `Ctrl+Shift+F10`: move active layer down
- `Ctrl+Shift+F11`: begin transform
- `Ctrl+Shift+PageUp`: scale transform up
- `Ctrl+Shift+PageDown`: scale transform down
- `Ctrl+Shift+1` through `Ctrl+Shift+9`: select layer by visible panel index
- `Ctrl+Shift+X`: swap foreground/background colors
- `Ctrl+Shift+R`: reset foreground/background colors

## 1. Smoke And Startup

### S01 - App Launches Cleanly

- [x] Launch PhotoTux in an isolated KWin session.
- [x] Verify the main window appears.
- [x] Verify the window title is present and stable.
- [x] Verify the app does not immediately crash or hang.
- [x] Verify the window can be focused by `kwin-mcp`.

### S02 - Accessibility And Screenshot Baseline

- [x] Confirm the main window is visible in the KWin accessibility tree.
- [x] Confirm screenshots can be captured for the running PhotoTux window.
- [x] Confirm the shell surface is readable enough for later test assertions.
- [x] Confirm panel and tool labels are exposed well enough for automation where expected.

### S03 - Session Stability

- [-] Confirm PhotoTux can remain open for several minutes without spontaneous crash.
- [-] Confirm screenshots and accessibility reads still work after idle time.
- [x] Confirm the app can be closed cleanly at the end of the session.

## 2. Shell Structure And Panel Presence

### UI01 - Main Shell Regions Exist

- [x] Verify the menu bar exists.
- [x] Verify the top tool/options area exists.
- [x] Verify the left tool rail exists.
- [x] Verify the central canvas/document region exists.
- [x] Verify the right-side panel area exists.
- [x] Verify the status bar exists.

### UI02 - Core Panels Exist

- [x] Verify the Layers panel is visible.
- [x] Verify the Properties panel is visible.
- [x] Verify the Color panel is visible.
- [x] Verify the History panel is visible.

### UI03 - Tabs And Document Shell Surface

- [x] Verify the document tab strip exists.
- [x] Verify at least one document tab or placeholder document state is visible.
- [x] Verify tab labels remain readable and stable after interactions.

## 3. Document Lifecycle

### DOC01 - New Session Baseline

- [x] Verify the initial session starts in a usable state.
- [x] Verify the canvas is not blank chrome only.
- [x] Verify the active document can receive tool interactions.

### DOC02 - Save Flow

- [x] Trigger save through the shell route.
- [x] Verify save completes without freezing the shell.
- [x] Verify the status surface reflects save success or failure.
- [x] Verify no crash occurs after save.

### DOC03 - Reopen Flow

- [ ] Reopen a previously saved `.ptx` document.
- [ ] Verify structure and visible content are preserved.
- [ ] Verify the reopened document remains editable.

### DOC04 - Dirty-State Expectations

- [x] Verify an edit changes the document into a dirty state where the shell exposes that concept.
- [x] Verify save clears the dirty state behavior where expected.
- [ ] Verify document state does not get silently lost across normal workflows.

## 4. Canvas And Viewport Navigation

### VIEW01 - Canvas Visibility

- [x] Verify the canvas surface renders actual document content.
- [x] Verify the canvas updates after edits.
- [x] Verify the canvas does not show stale content after tool actions.

### VIEW02 - Zoom

- [x] Verify zoom in works.
- [x] Verify zoom out works.
- [x] Verify reset or fit behavior works if exposed through shortcuts or commands.
- [x] Verify zoom updates are reflected visually on the canvas.

### VIEW03 - Pan

- [ ] Verify hand-tool or pan interaction works.
- [ ] Verify panning changes the visible viewport.
- [ ] Verify panning does not corrupt document content.

### VIEW04 - Responsiveness During Navigation

- [ ] Verify repeated zoom operations remain responsive.
- [ ] Verify pan drag remains visually smooth enough for normal use.
- [ ] Verify navigation does not visibly stall the shell.

## 5. Tools: Selection, Paint, Erase, Move, Transform

### TOOL01 - Tool Selection

- [x] Verify the Brush tool can be selected.
- [x] Verify the Eraser tool can be selected.
- [x] Verify the Move tool can be selected.
- [x] Verify the Rectangular Marquee tool can be selected.
- [x] Verify the Transform tool can be selected.
- [x] Verify the Hand tool can be selected if exposed.

### TOOL02 - Brush Workflow

- [x] Paint a stroke on the canvas.
- [x] Verify visible raster change occurs.
- [x] Verify the stroke appears as a single logical action in History.
- [x] Verify repeated strokes continue to update the live canvas.

### TOOL03 - Eraser Workflow

- [x] Erase previously painted content.
- [x] Verify visible raster change occurs.
- [x] Verify erase integrates correctly with History.

### TOOL04 - Rectangular Selection Workflow

- [x] Create a marquee selection.
- [x] Verify the overlay is visible.
- [x] Verify selection bounds look correct.
- [x] Verify clear selection works.
- [x] Verify invert selection works.

### TOOL05 - Selection-Constrained Painting

- [ ] Create a selection.
- [ ] Paint inside the selection.
- [ ] Verify paint respects the selection bounds.
- [ ] Invert the selection.
- [ ] Verify painting now affects the complementary area instead.

### TOOL06 - Move Tool Workflow

- [x] Move the active layer or content using the Move tool.
- [x] Verify live movement preview is visible.
- [x] Verify final committed position matches the preview.
- [x] Verify the move creates one meaningful history action.

### TOOL07 - Transform Workflow

- [x] Start a transform session.
- [x] Adjust translation through the transform workflow.
- [x] Adjust scale through the transform workflow.
- [x] Verify preview is visible before commit.
- [x] Commit the transform.
- [x] Verify committed output matches preview closely enough for MVP expectations.
- [x] Verify cancel works without mutating the document.

## 6. Layers, Properties, Color, And History Panels

### PANEL01 - Layers Panel Basics

- [x] Verify the active layer is visible in the Layers panel.
- [x] Verify selecting a layer changes the active layer.
- [x] Verify add layer works.
- [x] Verify duplicate layer works.
- [x] Verify delete layer works.
- [x] Verify reorder layer works.
- [x] Verify visibility toggle works.

### PANEL02 - Layer State Controls

- [x] Verify opacity can be changed from the Properties panel.
- [ ] Verify the canvas reflects opacity changes.
- [x] Verify blend-mode changes are possible through the shell.
- [ ] Verify blend-mode changes affect visible compositing.

### PANEL03 - Blend Mode Coverage

- [x] Verify Normal works.
- [x] Verify Multiply works.
- [ ] Verify Screen works.
- [ ] Verify Overlay works.
- [ ] Verify Darken works.
- [ ] Verify Lighten works.

### PANEL04 - Color Panel Basics

- [ ] Verify foreground color is visible and editable.
- [ ] Verify background color is visible and editable.
- [x] Verify swap foreground/background works.
- [x] Verify reset/default color behavior works if exposed.

### PANEL05 - History Panel Basics

- [x] Verify edits appear in the History panel.
- [ ] Verify undo from the History panel works.
- [ ] Verify redo from the History panel works.
- [ ] Verify the panel reflects the current history position correctly.

## 7. Undo And Redo Coverage

### HIST01 - Raster Edit Undo/Redo

- [x] Undo a brush stroke.
- [x] Verify the previous image state is restored.
- [x] Redo the brush stroke.
- [x] Verify the stroke returns exactly enough for visible parity.

### HIST02 - Structural Undo/Redo

- [x] Undo a layer add/delete/duplicate operation where supported through the shell.
- [x] Verify document structure is restored correctly.
- [x] Redo the operation.

### HIST03 - Selection Undo/Redo

- [ ] Undo selection creation.
- [ ] Verify the overlay disappears or reverts.
- [ ] Redo selection creation.

### HIST04 - Move And Transform Undo/Redo

- [ ] Undo a move action.
- [ ] Redo a move action.
- [ ] Undo a transform commit.
- [ ] Redo a transform commit.

### HIST05 - Save Does Not Pollute History

- [x] Perform an edit.
- [x] Save the document.
- [x] Undo once.
- [x] Verify the undo targets the last real edit, not the save action.

## 8. Import, Export, And Persistence

### IO01 - PNG Import And Export

- [ ] Import a PNG file.
- [ ] Verify visible content appears correctly.
- [ ] Export to PNG.
- [ ] Verify the exported file is created.

### IO02 - JPEG Import And Export

- [ ] Import a JPEG file.
- [ ] Verify visible content appears correctly.
- [ ] Export to JPEG.
- [ ] Verify the exported file is created.

### IO03 - WebP Import And Export

- [ ] Import a WebP file.
- [ ] Verify visible content appears correctly.
- [ ] Export to WebP.
- [ ] Verify the exported file is created.

### IO04 - Native `.ptx` Save And Reload

- [x] Save a layered document to `.ptx`.
- [ ] Close the document or app.
- [ ] Reopen the saved `.ptx` file.
- [ ] Verify layers, opacity, blend modes, and offsets still look correct.

### IO05 - Export Versus Viewport Sanity

- [ ] Create a representative layered scene.
- [ ] Export it.
- [ ] Compare the export with what the viewport showed immediately before export.
- [ ] Verify there is no obvious mismatch in composition.

### IO06 - Error Surfaces

- [ ] Attempt at least one malformed or unsupported import input where practical.
- [ ] Verify the app surfaces an error without crashing.
- [ ] Verify the shell remains responsive after the error.

## 9. Shortcuts And Command Routing

### KEY01 - Tool Shortcuts

- [x] Verify `V` selects the Move tool.
- [x] Verify `M` selects the marquee tool.
- [x] Verify `T` selects the Transform tool.
- [x] Verify `B` selects the Brush tool.
- [x] Verify `E` selects the Eraser tool.
- [x] Verify `H` selects the Hand tool.
- [x] Verify `Z` selects the Zoom tool if exposed in the shell behavior.

### KEY02 - History And Selection Shortcuts

- [x] Verify `Ctrl+Z` undo works.
- [x] Verify `Ctrl+Shift+Z` redo works.
- [x] Verify `Ctrl+Y` redo works.
- [x] Verify `Ctrl+D` clears selection.
- [x] Verify `Ctrl+I` inverts selection.

### KEY03 - Save And Viewport Shortcuts

- [x] Verify `Ctrl+S` triggers save.
- [x] Verify `Ctrl++` zooms in.
- [x] Verify `Ctrl+-` zooms out.
- [x] Verify `Ctrl+0` resets or fits zoom as intended.

### KEY04 - Transform Session Shortcuts

- [x] Verify `Enter` commits transform when a transform session is active.
- [x] Verify `Escape` cancels transform when a transform session is active.

## 10. Autosave And Crash Recovery

### REC01 - Autosave Baseline

- [x] Make a document edit.
- [x] Wait long enough for autosave to be eligible.
- [x] Verify the app remains responsive while autosave occurs.
- [x] Verify the shell surfaces status appropriately if visible.

### REC02 - Recovery File Behavior

- [x] Confirm a recovery artifact is produced after edits where expected.
- [x] Confirm a successful manual save clears stale recovery state where expected.

### REC03 - Startup Recovery Flow

- [x] Create a recoverable edited state.
- [x] Simulate abnormal termination if safe to do so.
- [x] Restart the app.
- [x] Verify recovery is detected.
- [x] Verify the recovered document opens correctly.

## 11. Status Surfaces And Background Job Behavior

### JOB01 - Status Feedback

- [ ] Verify status surfaces change during save.
- [x] Verify status surfaces change during autosave.
- [ ] Verify status surfaces change during import or export.

### JOB02 - Shell Responsiveness During Jobs

- [ ] Verify save does not freeze the shell.
- [ ] Verify export does not freeze the shell.
- [ ] Verify import does not freeze the shell.
- [ ] Verify autosave does not visibly block the UI.

## 12. Linux And Wayland-Specific Validation

### LNX01 - Focus And Windowing

- [ ] Verify the main window can regain focus cleanly after dialogs or menus.
- [ ] Verify keyboard shortcuts still work after focus changes.

### LNX02 - Wayland Visual Sanity

- [ ] Verify no obvious coordinate drift appears during selection or move interactions.
- [ ] Verify canvas interactions do not show severe offset mismatch under the normal scale factor.

### LNX03 - Fractional-Scaling Validation

- [ ] Run the app under a non-integer scale factor if available.
- [ ] Verify shell layout still looks stable.
- [ ] Verify canvas interaction coordinates still appear correct.

## 13. Representative End-To-End Scenarios

### E2E01 - Small Paint Workflow

- [ ] Launch app.
- [ ] Paint.
- [ ] Erase.
- [ ] Undo.
- [ ] Redo.
- [ ] Save to `.ptx`.
- [ ] Reopen.
- [ ] Export to PNG.

### E2E02 - Layered Compositing Workflow

- [ ] Create or import content.
- [ ] Add multiple layers.
- [ ] Reorder layers.
- [ ] Adjust opacity.
- [ ] Change blend mode.
- [ ] Move active layer.
- [ ] Export.

### E2E03 - Selection And Transform Workflow

- [ ] Create a rectangular selection.
- [ ] Paint within selection.
- [ ] Invert selection.
- [ ] Transform the active layer.
- [ ] Commit transform.
- [ ] Undo and redo the transform.

### E2E04 - Recovery Workflow

- [ ] Create unsaved work.
- [ ] Allow autosave to occur.
- [ ] Simulate abnormal exit.
- [ ] Restart.
- [ ] Recover document.
- [ ] Verify content and shell state are sane.

## 14. Priority Tiers

### Tier 1: Must Cover First

- [ ] S01
- [ ] S02
- [ ] UI01
- [ ] TOOL02
- [ ] TOOL03
- [ ] HIST01
- [ ] IO04
- [ ] IO05
- [ ] KEY02
- [ ] REC01

### Tier 2: Expand Once Smoke Is Stable

- [ ] TOOL04
- [ ] TOOL05
- [ ] TOOL06
- [ ] TOOL07
- [ ] PANEL01
- [ ] PANEL02
- [ ] PANEL05
- [ ] JOB01
- [ ] JOB02

### Tier 3: Full MVP Confidence Pass

- [ ] IO01
- [ ] IO02
- [ ] IO03
- [ ] REC03
- [ ] LNX02
- [ ] LNX03
- [ ] E2E01
- [ ] E2E02
- [ ] E2E03
- [ ] E2E04

## Notes For `kwin-mcp` Use

When running this checklist through `kwin-mcp`, prefer these validation styles:

- accessibility-tree checks for shell and widget presence
- screenshot checks for canvas, overlays, and visual sanity
- keyboard shortcuts for command routing coverage
- direct pointer interactions for canvas and button-driven workflows

Be careful with these areas:

- pure keyboard input can be app-specific or layout-sensitive
- some GTK or native popup/menu behavior may be easier to verify visually than semantically
- canvas assertions should usually combine screenshot comparison with workflow-level expectations rather than relying only on AT-SPI

## Definition Of Done For GUI Coverage

The current app has meaningful `kwin-mcp` GUI coverage when:

1. Tier 1 is stable.
2. core edit, undo/redo, save/reopen, and export workflows pass.
3. autosave and recovery are exercised.
4. no major viewport-versus-export mismatch is observed.
5. the shell remains responsive during normal file operations.