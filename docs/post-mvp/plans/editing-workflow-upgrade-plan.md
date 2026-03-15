# Post-MVP Plan: Editing Workflow Upgrades

## Purpose

Extend the MVP into a stronger layered editing tool by adding the highest-leverage workflow features that improve real compositing and design tasks without changing the core architecture.

## Why This Is A Natural Next Step

The MVP already supports layered raster editing, transforms, selection, save/load, import/export, and recovery. The next biggest user-value gap is not basic correctness anymore. It is workflow depth.

The main missing pieces are the ones already called out in the product docs as post-MVP candidates:

- masks
- layer groups
- lasso selection
- better transforms
- guides and snapping

These improve real projects immediately while staying aligned with the current document-first architecture.

## Goal

Make PhotoTux capable of more serious non-destructive and semi-structured editing workflows without introducing text, PSD, or a new shell framework first.

## Scope

### In Scope

- raster layer masks
- layer groups with visibility and basic hierarchy behavior
- freeform lasso selection or polygonal lasso as the first non-rectangular selection tool
- stronger transform controls beyond simple translate and scale
- guides and snapping for composition and alignment

### Explicitly Out Of Scope

- text layers
- adjustment layers
- smart objects
- dockable shell systems
- PSD import and export
- full vector editing

## Recommended Delivery Order

1. layer masks
2. layer groups
3. lasso selection
4. transform workflow upgrade
5. guides and snapping

This order keeps the work aligned with the existing document model and gives user value at each step.

## Work Breakdown

### Phase 1: Layer Masks

Deliverables:

- per-layer mask ownership in `doc_model`
- mask-aware flatten/composition behavior in `file_io`
- mask editing intent in `tool_system`
- shell controls for add, delete, enable, disable, and target mask editing
- undo and redo support for mask edits

Key design rules:

- mask state belongs to the document model, not the shell
- mask composition must match viewport and export output
- mask editing should re-use existing paint infrastructure where practical

Exit criteria:

- a user can add a mask, paint or erase on it, save, reopen, undo, redo, and export without visible mismatch

### Phase 2: Layer Groups

Deliverables:

- document support for group nodes
- visibility and opacity propagation rules
- basic nesting in the layer panel
- save/load support in `.ptx`
- flattened export behavior for grouped content

Key design rules:

- groups are document-owned structural nodes, not shell-only presentation rows
- flattening and export must preserve group evaluation order exactly
- do not add blending semantics for groups unless intentionally designed and tested

Exit criteria:

- grouped layers can be organized, toggled, saved, reopened, and exported without order corruption

### Phase 3: Lasso Selection

Deliverables:

- freeform selection representation in `doc_model`
- hit testing and fill rules for selection queries
- lasso interaction in `tool_system`
- overlay rendering in `render_wgpu`
- save/load behavior only if selection persistence is considered necessary

Key design rules:

- selection logic remains headless and testable
- paint, erase, move, and transform must all respect the new selection path consistently

Exit criteria:

- lasso selection behaves correctly for invert, clear, edit clipping, and undo/redo

### Phase 4: Transform Upgrade

Deliverables:

- rotate and non-uniform scale support
- clearer preview and commit flow
- better bounds and handle behavior
- stronger keyboard integration for transform operations

Key design rules:

- preview remains a preview path, committed document state remains authoritative
- export must match committed state, not transient preview state

Exit criteria:

- transforms are visibly more capable without destabilizing existing move and scale workflows

### Phase 5: Guides And Snapping

Deliverables:

- document or session-level guide representation
- overlay rendering for guides
- snapping hooks for move and transform workflows
- visibility toggle and clear commands

Key design rules:

- guide state should not accidentally become baked into exports
- snapping must be predictable and easy to disable

Exit criteria:

- alignment-sensitive workflows become materially easier without adding shell complexity

## Main Risks

- masks create new correctness risk for flatten, save/load, undo/redo, and export parity
- groups can introduce ordering and hierarchy bugs in persistence
- lasso selection can create expensive or inconsistent selection queries if represented poorly
- transform upgrades can regress the current preview-versus-commit boundary

## Validation Requirements

- canonical fixture for masked compositing
- save/load roundtrip coverage for groups and masks
- export-versus-viewport parity coverage for masks and groups
- undo/redo regression coverage for lasso and transform changes
- manual validation for snapping ergonomics and transform feel

## Success Condition

PhotoTux becomes materially more capable for real layered composition work while preserving the MVP architecture boundaries and trust level.