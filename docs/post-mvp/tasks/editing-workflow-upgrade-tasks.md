# Editing Workflow Upgrade Tasks

## Purpose

This task list turns the editing workflow upgrade plan into an implementation sequence.

It covers masks, groups, lasso selection, transform upgrades, and guides/snapping in a dependency-aware order.

## Principles

- preserve the current document-first architecture
- keep viewport and export parity release-blocking
- add structural document features before shell polish for those features
- keep each feature undoable, saveable, and testable before expanding to the next one

## Task List

### EW01 - Define mask ownership model in the document layer

- [ ] Status: not started
- Outcome: masks have a stable data model before editing behavior is added
- Includes:
  - decide whether masks are embedded per layer or referenced as a parallel raster payload
  - define visibility, enable/disable, and edit-target state boundaries
  - define save/load representation in `.ptx`
- Depends on: none
- Done when:
  - mask structure is documented and represented in `doc_model` without shell-owned state

### EW02 - Add mask persistence and flatten support

- [ ] Status: not started
- Outcome: masks affect real output rather than shell-only previews
- Includes:
  - mask-aware flattening in `file_io`
  - mask roundtrip coverage in save/load
  - export-versus-viewport parity coverage for masked scenes
- Depends on: EW01
- Done when:
  - masked documents save, reopen, and export without visual drift

### EW03 - Add mask editing workflows

- [ ] Status: not started
- Outcome: users can create and edit layer masks using the existing painting model
- Includes:
  - add/delete/enable/disable mask commands
  - target-mask editing state routed through `app_core`
  - paint and erase application into masks through `tool_system`
  - undo/redo for mask edits
- Depends on: EW02
- Done when:
  - a user can non-destructively hide and reveal content with a layer mask

### EW04 - Add shell support for masks

- [ ] Status: not started
- Outcome: masks are usable in the layer workflow rather than only via internal APIs
- Includes:
  - mask affordances in the layers/properties panels
  - edit-target visibility in the shell
  - status messaging for mask editing mode
- Depends on: EW03
- Done when:
  - the shell clearly exposes whether the user is editing a layer or its mask

### EW05 - Define group node document structure

- [ ] Status: not started
- Outcome: group support has a stable persistence and ordering model
- Includes:
  - group node representation in `doc_model`
  - hierarchy rules and parent-child constraints
  - visibility and opacity propagation semantics
- Depends on: none
- Done when:
  - group structure exists headlessly and can be reasoned about independently of the shell

### EW06 - Add group flatten, save, and load support

- [ ] Status: not started
- Outcome: groups are trusted document structure rather than temporary UI nesting
- Includes:
  - group-aware flatten evaluation
  - save/load roundtrip coverage for nested groups
  - group export parity tests
- Depends on: EW05
- Done when:
  - grouped documents preserve order, visibility, and output through persistence

### EW07 - Add group editing commands and shell presentation

- [ ] Status: not started
- Outcome: users can organize complex documents structurally
- Includes:
  - create/ungroup/move into group/move out of group operations
  - nested layers panel presentation
  - undo/redo for group structure changes
- Depends on: EW06
- Done when:
  - real grouped editing workflows are possible without order corruption

### EW08 - Define freeform selection representation

- [ ] Status: not started
- Outcome: lasso support rests on a headless selection model rather than shell geometry only
- Includes:
  - polygon or path-based selection representation
  - point-in-selection and bounds queries
  - invert and clear semantics for non-rectangular selections
- Depends on: none
- Done when:
  - freeform selection state is testable in `doc_model`

### EW09 - Add lasso interaction and edit clipping

- [ ] Status: not started
- Outcome: freeform selection affects real paint and edit operations
- Includes:
  - lasso tool behavior in `tool_system`
  - overlay rendering in `render_wgpu`
  - paint/erase/move/transform respect for lasso selection
  - undo/redo coverage
- Depends on: EW08
- Done when:
  - lasso selection is usable for real editing, not just visibly drawn

### EW10 - Upgrade transform behavior beyond translate and uniform scale

- [ ] Status: not started
- Outcome: transform workflows cover more practical editing cases
- Includes:
  - rotate support
  - non-uniform scale support
  - improved preview bounds and commit behavior
  - transform regression coverage
- Depends on: none
- Done when:
  - transform workflows materially exceed the MVP subset without regressing parity

### EW11 - Add guides and guide rendering

- [ ] Status: not started
- Outcome: alignment assistance exists as real editor state
- Includes:
  - guide representation
  - add/remove/show/hide guide commands
  - overlay rendering for guides
- Depends on: none
- Done when:
  - guides can be created, toggled, and visualized predictably

### EW12 - Add snapping for move and transform workflows

- [ ] Status: not started
- Outcome: alignment-sensitive workflows become faster and more precise
- Includes:
  - snapping to guides
  - snapping toggles and temporary bypass behavior
  - move/transform integration tests
- Depends on: EW11, EW10
- Done when:
  - snapping materially improves placement workflows without feeling unpredictable

### EW13 - Build representative regression fixtures for upgraded workflows

- [ ] Status: not started
- Outcome: masks, groups, lasso, transform, and guides stay stable over time
- Includes:
  - masked compositing fixture
  - grouped layered scene fixture
  - lasso selection fixture
  - transform parity fixture
  - snapping/manual validation checklist
- Depends on: EW04, EW07, EW09, EW10, EW12
- Done when:
  - the upgraded workflow set has stable regression coverage and manual validation notes

## Suggested Execution Order

1. EW01
2. EW02
3. EW03
4. EW04
5. EW05
6. EW06
7. EW07
8. EW08
9. EW09
10. EW10
11. EW11
12. EW12
13. EW13

## Notes

- If masks or groups reveal flatten/export mismatches, pause feature expansion and fix parity first.
- If lasso selection creates performance issues in edit clipping, tighten the representation before adding more selection tools.