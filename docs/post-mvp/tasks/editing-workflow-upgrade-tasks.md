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

- [x] Status: completed
- Outcome: masks have a stable data model before editing behavior is added
- Includes:
  - decide whether masks are embedded per layer or referenced as a parallel raster payload
  - define visibility, enable/disable, and edit-target state boundaries
  - define save/load representation in `.ptx`
- Depends on: none
- Done when:
  - mask structure is documented and represented in `doc_model` without shell-owned state

Progress notes:
- masks are now modeled as optional per-layer embedded alpha-mask payloads in `doc_model`, which keeps mask ownership document-local instead of creating shell-owned parallel state
- mask state now includes explicit enable/disable behavior plus a document-owned active edit target (`LayerPixels` versus `LayerMask`) so later editing workflows can switch targets without introducing widget-owned source of truth
- `.ptx` persistence now records the active edit target and per-layer mask metadata plus alpha tile payloads, giving the format a stable roundtrip representation before mask-aware flattening is introduced in `EW02`
- regression coverage now exercises mask creation/removal, edit-target constraints, duplicate-layer mask cloning semantics, and project/manifest roundtrips for masked layers

### EW02 - Add mask persistence and flatten support

- [x] Status: completed
- Outcome: masks affect real output rather than shell-only previews
- Includes:
  - mask-aware flattening in `file_io`
  - mask roundtrip coverage in save/load
  - export-versus-viewport parity coverage for masked scenes
- Depends on: EW01
- Done when:
  - masked documents save, reopen, and export without visual drift

Progress notes:
- `file_io` flattening and incremental region recomposition now apply enabled per-layer mask alpha before blend-mode composition, so masked layers affect both full exports and app-core cached viewport refreshes through the same source of truth
- disabled masks are ignored during composition, while missing mask tiles default to fully visible content, matching the embedded-mask ownership model established in `EW01`
- save/load and manifest roundtrip coverage now includes masked scenes whose flattened output must stay stable across `.ptx` persistence
- controller-level parity coverage in `app_core` now verifies that a masked document produces identical viewport and exported composite pixels, which closes the release-blocking viewport-versus-export risk for this task

### EW03 - Add mask editing workflows

- [x] Status: completed
- Outcome: users can create and edit layer masks using the existing painting model
- Includes:
  - add/delete/enable/disable mask commands
  - target-mask editing state routed through `app_core`
  - paint and erase application into masks through `tool_system`
  - undo/redo for mask edits
- Depends on: EW02
- Done when:
  - a user can non-destructively hide and reveal content with a layer mask

Progress notes:
- `app_core` now owns explicit layer-mask commands for add, delete, enable, disable, and edit-target switching, which keeps mask workflow state document-driven instead of shell-owned while still exposing it through the shell controller surface.
- `tool_system::BrushTool` now records whether a stroke touched layer pixels or layer-mask tiles, so mask hide and reveal strokes use the same undoable stroke model as normal paint and erase instead of a one-off controller path.
- mask stroke routing respects layer offsets when computing touched tiles and dab placement, fixing brush and mask painting so moved layers still edit under the cursor rather than in pre-offset tile space.
- controller-level regression coverage now exercises mask command snapshots plus mask hide/reveal interactions with undo and redo, while `tool_system` coverage protects mask tile history roundtrips directly.

### EW04 - Add shell support for masks

- [x] Status: completed
- Outcome: masks are usable in the layer workflow rather than only via internal APIs
- Includes:
  - mask affordances in the layers/properties panels
  - edit-target visibility in the shell
  - status messaging for mask editing mode
- Depends on: EW03
- Done when:
  - the shell clearly exposes whether the user is editing a layer or its mask

Progress notes:
- the `ui_shell` snapshot now exposes active edit-target state plus per-layer mask presence, enabled state, and whether the active layer is currently in mask-editing mode.
- the Layer menu now surfaces add, delete, enable or disable, and edit-target actions for masks through controller commands instead of requiring internal APIs.
- the Properties and Layers panels now show mask state directly and include quick actions for adding a mask, toggling it on or off, deleting it, and switching between editing layer pixels versus the mask.
- shell status text now calls out mask-editing mode explicitly, including the current hide-versus-reveal behavior for brush and eraser so the active editing target stays visible during interaction.
- the shell now gives each layer row explicit `L` and `M` target chips plus a dedicated mask-state banner in Properties, making mask edit mode and disabled-mask state visible without relying only on status text.

### EW05 - Define group node document structure

- [x] Status: completed
- Outcome: group support has a stable persistence and ordering model
- Includes:
  - group node representation in `doc_model`
  - hierarchy rules and parent-child constraints
  - visibility and opacity propagation semantics
- Depends on: none
- Done when:
  - group structure exists headlessly and can be reasoned about independently of the shell

Progress notes:
- `doc_model` now owns a serializable headless layer hierarchy made of `LayerHierarchyNode` and `LayerGroup`, keeping group structure document-local instead of introducing shell-owned nesting metadata.
- the initial group model uses dedicated group IDs, stored visibility, and stored opacity so structural group state can be defined before compositing semantics are wired into flattening and persistence.
- hierarchy validation now enforces that every document layer is referenced exactly once and that group IDs are unique, which establishes clear parent-child constraints before controller and shell editing commands exist.
- regression coverage now verifies group creation order and hierarchy validation, while existing flat-layer operations continue to rebuild the plain hierarchy when no groups are present so current editing workflows stay stable ahead of `EW06` and `EW07`.

### EW06 - Add group flatten, save, and load support

- [x] Status: completed
- Outcome: groups are trusted document structure rather than temporary UI nesting
- Includes:
  - group-aware flatten evaluation
  - save/load roundtrip coverage for nested groups
  - group export parity tests
- Depends on: EW05
- Done when:
  - grouped documents preserve order, visibility, and output through persistence

Progress notes:
- `file_io` flattening and regional recomposition now walk the recursive document hierarchy instead of the flat layer list, so group visibility and group opacity propagate through child layer composition without inventing separate shell-only rules.
- `.ptx` project manifests now serialize the recursive layer hierarchy alongside layer payloads, while load restores nested groups through `doc_model::set_layer_hierarchy` so hierarchy validation remains enforced on persisted documents.
- grouped documents without hierarchy metadata still fall back to the flat layer order on load, which keeps earlier project files readable while allowing new grouped scenes to roundtrip with their full structure intact.
- regression coverage now includes grouped flatten behavior, grouped save/load hierarchy roundtrips, grouped regional recomposition parity, and grouped PNG export parity so grouped viewport and exported output stay aligned.

### EW07 - Add group editing commands and shell presentation

- [x] Status: completed
- Outcome: users can organize complex documents structurally
- Includes:
  - create/ungroup/move into group/move out of group operations
  - nested layers panel presentation
  - undo/redo for group structure changes
- Depends on: EW06
- Done when:
  - real grouped editing workflows are possible without order corruption

Progress notes:
- `doc_model` now exposes validated hierarchy mutations for wrapping a node in a group, ungrouping a group, moving a node into a group, moving a node out of its parent group, and toggling stored group visibility, which keeps structural editing headless and testable.
- `app_core` now tracks the selected structure target separately from the active paint layer, so group rows can be selected as structural targets without moving document ownership of hierarchy state into the shell.
- the controller now exposes shell-facing commands for grouping the active layer, ungrouping the selected group, moving the active layer into the selected group, and moving the active layer out of its parent group, all with dedicated undo and redo history records.
- `ui_shell` now renders nested layer and group rows with indentation, group selection, group visibility toggles, and group action chips in the Layers panel, making grouped organization workflows possible without flattening or order corruption.

### EW08 - Define freeform selection representation

- [x] Status: completed
- Outcome: lasso support rests on a headless selection model rather than shell geometry only
- Includes:
  - polygon or path-based selection representation
  - point-in-selection and bounds queries
  - invert and clear semantics for non-rectangular selections
- Depends on: none
- Done when:
  - freeform selection state is testable in `doc_model`

Progress notes:
- `doc_model` selection state now stores a document-owned `SelectionShape` enum instead of only a rectangular bounds box, which establishes a real headless foundation for lasso-style selections without moving geometry into the shell.
- the freeform branch is represented as a polygon-backed `FreeformSelection` with integer control points, keeping the initial model simple, serializable, and easy to query in pure tests.
- selection queries now flow through shared shape-aware bounds and hit-testing logic, so rectangular and freeform selections use the same invert and edit-allowance path instead of forking separate document rules.
- regression coverage now verifies polygon bounds calculation, point-in-selection behavior, and inverted edit semantics for freeform selections while preserving the existing rectangular selection behavior used by current tools.

### EW09 - Add lasso interaction and edit clipping

- [x] Status: completed
- Outcome: freeform selection affects real paint and edit operations
- Includes:
  - lasso tool behavior in `tool_system`
  - overlay rendering in `render_wgpu`
  - paint/erase/move/transform respect for lasso selection
  - undo/redo coverage
- Depends on: EW08
- Done when:
  - lasso selection is usable for real editing, not just visibly drawn

Progress notes:
- `tool_system` now has a real `LassoTool` and generalized selection history records, so freeform selections can be created, undone, and redone through the same document-owned selection model used by rectangular marquee.
- `app_core` now exposes a lasso interaction path and snapshot data for both committed and in-progress freeform selection points, which makes lasso state visible to the shell without pushing geometry ownership into GTK state.
- `render_wgpu` now supports polyline overlays in addition to rectangle overlays, so lasso previews and committed freeform selections render as paths instead of collapsing to bounding boxes.
- brush, mask, move, and transform workflows now all respect freeform selection clipping, so lasso selection affects real editing instead of only changing overlay state.
- regression coverage now includes tool-level and controller-level selected move and selected transform behavior, alongside lasso creation, path overlay rendering, and brush/mask clipping behavior.

### EW10 - Upgrade transform behavior beyond translate and uniform scale

- [x] Status: completed
- Outcome: transform workflows cover more practical editing cases
- Includes:
  - rotate support
  - non-uniform scale support
  - improved preview bounds and commit behavior
  - transform regression coverage
- Depends on: none
- Done when:
  - transform workflows materially exceed the MVP subset without regressing parity

Progress notes:
- `tool_system` transform behavior now supports independent X/Y scaling and quarter-turn rotation with the same preview-versus-commit boundary used by the existing translate workflow.
- selection-aware transforms continue to work under the expanded model, so non-uniform scale and rotation apply to either the full layer or the active selection without forking separate shell-owned state.
- `app_core` now tracks richer transform session state and exposes it through shell snapshots, while `ui_shell` surfaces the new controls in the existing Properties panel rather than introducing a second transform mode UI.
- regression coverage now includes preview bounds for non-uniform scale plus rotation, quarter-turn transform application, controller snapshot updates for transform state, and controller commit coverage for the expanded transform path.

### EW11 - Add guides and guide rendering

- [x] Status: completed
- Outcome: alignment assistance exists as real editor state
- Includes:
  - guide representation
  - add/remove/show/hide guide commands
  - overlay rendering for guides
- Depends on: none
- Done when:
  - guides can be created, toggled, and visualized predictably

Progress notes:
- `doc_model` now owns persisted horizontal and vertical guide records plus guide visibility state, keeping guides as headless document data rather than shell-only UI state.
- `app_core` projects guide state through shell snapshots and records add/remove/show-hide guide operations in undoable history, so guide changes behave like real editor state.
- `ui_shell` now exposes guide controls in the View menu and Properties panel, while the viewport overlay path renders visible guides as document-space lines through the existing renderer integration.
- `file_io` now persists guide state in project manifests, and regression coverage verifies document, controller, and save/load behavior for guides.

### EW12 - Add snapping for move and transform workflows

- [x] Status: completed
- Outcome: alignment-sensitive workflows become faster and more precise
- Includes:
  - snapping to guides
  - snapping toggles and temporary bypass behavior
  - move/transform integration tests
- Depends on: EW11, EW10
- Done when:
  - snapping materially improves placement workflows without feeling unpredictable

Progress notes:
- move and transform translation now snap to document guides in `app_core`, using controller-owned snapping state rather than GTK-owned interaction logic.
- snapping has a persistent enable/disable toggle and a temporary Shift bypass during drag, which keeps guide-assisted alignment available without making direct manipulation feel sticky.
- `ui_shell` now exposes snapping controls through the existing View menu and Properties panel, and shell status text reflects when snapping is active or temporarily bypassed.
- regression coverage now includes snapped move, snapped transform preview, disabled snapping, and temporary bypass behavior through controller-level move/transform integration tests.

### EW13 - Build representative regression fixtures for upgraded workflows

- [x] Status: completed
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

Progress notes:
- the canonical fixture documentation now explicitly covers masked compositing, grouped hierarchy, lasso-aware transform parity, and guide-snapping interaction scenes instead of leaving those upgraded workflows implied.
- `file_io` continues to carry the representative masked and grouped persistence fixtures, while `app_core` now has a dedicated lasso-aware transform parity fixture and reusable guide-snapping fixture coverage.
- a new post-MVP editing workflow checklist now documents the manual validation pass for masks, groups, lasso, transform, guides, and snapping so the upgraded workflow set has written manual validation notes rather than tribal knowledge.
- regression coverage for the upgraded workflow track now has an explicit documented home in both `tests/fixtures/README.md` and `docs/testing-strategy.md`, which closes the loop between automated fixtures and manual validation expectations.

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