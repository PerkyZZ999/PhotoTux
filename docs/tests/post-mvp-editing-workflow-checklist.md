# PhotoTux Post-MVP Editing Workflow Checklist

## Purpose

This checklist covers the currently implemented post-MVP editing workflow upgrades:

- masks
- groups
- lasso selection
- expanded transform behavior
- guides
- snapping

It is intended for manual validation on the Linux desktop shell after automated tests pass.

## How To Use This Checklist

Run this checklist after the affected automated suites pass.

Prioritize:

1. save/load and export trust
2. undo/redo trust
3. direct-manipulation feel

Record any mismatch between visible canvas output and saved/exported output as release-blocking.

## Workflow Checks

### FIX01 - Masked Compositing

- [ ] Open a document with a masked layer.
- [ ] Toggle mask visibility or mask-enabled state if available.
- [ ] Paint and erase while editing the mask.
- [ ] Save, reopen, and confirm the masked result matches the pre-save canvas.
- [ ] Export PNG and confirm the flattened output matches the visible composite.

### FIX02 - Grouped Layer Structure

- [ ] Open a document with grouped layers.
- [ ] Toggle group visibility.
- [ ] Move a layer into and back out of a group if that workflow is exposed.
- [ ] Save, reopen, and confirm hierarchy plus flattened output are preserved.

### FIX03 - Lasso Selection Editing

- [ ] Create a lasso selection around only part of a painted area.
- [ ] Confirm paint and erase affect only the selected region.
- [ ] Confirm move affects only the selected region.
- [ ] Clear and invert the lasso selection and confirm clipping behavior updates correctly.

### FIX04 - Transform Expansion

- [ ] Start a transform session on a layer or selected region.
- [ ] Apply non-uniform X/Y scaling.
- [ ] Apply quarter-turn rotation.
- [ ] Drag the transform preview and confirm preview bounds remain plausible.
- [ ] Commit the transform and confirm the committed result matches the previewed intent.
- [ ] Undo and redo the transform commit.

### FIX05 - Guides

- [ ] Add both a horizontal and a vertical guide.
- [ ] Toggle guide visibility off and back on.
- [ ] Save, reopen, and confirm guide visibility plus positions persist.
- [ ] Confirm guides remain overlays only and do not appear in export output.

### FIX06 - Snapping

- [ ] With snapping enabled, drag a layer near a guide and confirm it snaps predictably.
- [ ] Start a transform drag near a guide and confirm translation snaps predictably.
- [ ] Disable snapping and confirm the same drag no longer snaps.
- [ ] Hold Shift during a drag and confirm temporary bypass works without changing the persistent snapping toggle.
- [ ] Confirm snapping does not feel sticky when the pointer is clearly outside the snap threshold.

## Regression Notes

- If guide snapping feels unpredictable, record which edge snapped and whether Shift bypass corrected it.
- If transform preview and commit differ, capture both the preview state and the committed result.
- If save/load preserves document state but export diverges from the canvas, treat that as a parity defect rather than a UI-only defect.