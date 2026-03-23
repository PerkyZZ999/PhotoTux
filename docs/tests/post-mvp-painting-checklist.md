# PhotoTux Post-MVP Painting Checklist

## Purpose

This checklist covers the currently implemented post-MVP painting and input upgrades:

- stylus pressure plumbing
- pressure-to-size and pressure-to-opacity mapping
- richer brush radius, hardness, spacing, and flow controls
- live brush and eraser hover preview
- paint-heavy medium-canvas regression checks
- minimal destructive filters

It is intended for manual validation on the Linux desktop shell after the affected automated suites pass.

## How To Use This Checklist

Run this checklist after the affected automated suites pass.

Prioritize:

1. stroke feel and preview trust
2. undo/redo trust
3. medium-canvas responsiveness

Record any mismatch between visible stroke feedback and committed/exported output as a regression.

## Current Run Notes

### 2026-03-23 - Linux KWin Mouse Validation Pass

Validated in a fresh `kwin-mcp` isolated session using the built `app_core` binary launched from a dedicated temporary working directory with `PHOTOTUX_ENABLE_TEST_SHORTCUTS=1`:

- mouse-driven brush painting committed visible raster changes successfully
- eraser strokes removed previously painted pixels as expected
- `Ctrl+Z` and `Ctrl+Shift+Z` restored and replayed the erase action correctly
- the shell remained responsive throughout the paint, erase, and history pass, and the app log showed no new runtime errors

Still open after this pass:

- stylus-pressure behavior could not be validated because no pressure-capable tablet device is available in this environment
- this pass did not yet cover brush-parameter tweaking, hover-preview parity under changing settings, or save/reopen parity for the painted result
- the startup document still presented an active rectangular selection overlay, so this run validated paint and erase behavior inside the active selection bounds rather than a no-selection baseline
- the broader post-MVP sequence now treats the missing stylus pass as explicitly deferred by hardware availability rather than as a blocker for continuing into the next plan

## Workflow Checks

### PNT01 - Mouse Brush Regression

- [ ] Select the brush tool with pressure toggles disabled.
- [ ] Paint several short strokes and confirm the hover preview matches the visible stroke width.
- [ ] Increase and decrease radius, hardness, spacing, and flow from the Properties panel.
- [ ] Confirm the preview updates immediately before the next stroke is committed.
- [ ] Undo and redo the last few strokes and confirm the canvas returns to the exact expected state.

### PNT02 - Pressure-Sensitive Brush Preview

- [ ] Use a stylus device that reports pressure.
- [ ] Enable pressure-to-size and confirm the hover preview radius changes as pressure changes before paint is committed.
- [ ] Enable pressure-to-opacity and confirm lighter pressure visibly reduces committed stroke density.
- [ ] Confirm mouse input still behaves like full-pressure input when the same toggles are enabled.

### PNT03 - Medium-Canvas Repeated Stroke Pass

- [ ] Open or create a medium canvas around 1024x768.
- [ ] Paint repeated strokes across several areas of the canvas with pressure enabled.
- [ ] Confirm preview feedback stays responsive while the stroke is in progress.
- [ ] Confirm the shell remains responsive while repeated strokes are committed.
- [ ] Save, reopen, and confirm the painted result matches the visible pre-save canvas.

### PNT04 - Eraser Preview And Commit Parity

- [ ] Switch to the eraser tool.
- [ ] Confirm the eraser hover preview tracks the active radius and hardness settings.
- [ ] Erase several painted regions and confirm the committed result matches the previewed cursor intent.
- [ ] Undo and redo the eraser strokes and confirm parity is preserved.

### PNT05 - Destructive Filter Workflow

- [ ] With layer-pixel editing active, run `Invert Colors` from the Filter menu and confirm the visible layer result changes immediately after the worker completes.
- [ ] Undo and redo the filter and confirm the exact pre-filter and post-filter states return.
- [ ] Run `Desaturate` and confirm the affected layer loses color without alpha corruption.
- [ ] Save, reopen, and confirm the filtered result matches the pre-save canvas.
- [ ] Switch to mask editing and confirm destructive filters are rejected instead of applying ambiguously.

## Regression Notes

- If stylus pressure affects committed paint but not the hover preview, treat that as a preview-parity defect.
- If repeated strokes feel delayed only after several commits, record approximate canvas size, active brush settings, and whether pressure was enabled.
- If save/load preserves paint data but the reopened canvas differs from the in-session result, treat that as a persistence parity defect rather than a shell-only issue.
- If a filter result arrives after other edits and still changes the document, treat that as a revision-guard defect.