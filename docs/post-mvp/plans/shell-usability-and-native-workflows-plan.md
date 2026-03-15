# Post-MVP Plan: Shell Usability And Native Workflows

## Purpose

Reduce day-to-day workflow friction in the GTK shell and native Linux integration now that the core MVP editing loop is stable.

## Why This Is A Natural Next Step

At MVP completion, the editing engine is credible. The next set of quality gains comes from the shell: file workflows, menus, status surfaces, native dialogs, and interaction polish.

This track improves daily usability without changing the document model fundamentally.

## Goal

Make the editor feel more complete, faster to operate, and more native on Linux systems without introducing a full docking framework.

## Scope

### In Scope

- open/save/export dialog flows
- recovery prompt polish
- menu routing for common document commands
- stronger shortcut discoverability and command consistency
- better status and error surfaces
- document-tab behavior improvements
- workflow polish around selection, transform, and file operations

### Explicitly Out Of Scope

- full dockable layout system
- deep custom window chrome
- non-Linux-first shell abstractions

## Recommended Delivery Order

1. file and export dialog flows
2. menu and command routing polish
3. status, notifications, and recovery UX
4. document-tab and multi-document workflow improvements
5. broader shell ergonomics cleanup

## Work Breakdown

### Phase 1: File Workflow Completeness

Deliverables:

- open document flow
- save-as flow
- export-format selection flow
- clear distinction between project save and flattened export

Key design rules:

- file commands route through the application layer, not widget-owned state
- long-running file operations stay on the worker/job path

Exit criteria:

- the shell no longer depends on implicit current-directory saves for normal use

### Phase 2: Menu And Command Surface

Deliverables:

- real File, Edit, Layer, Select, and View command routing
- menu-item enable/disable behavior from controller state
- consistent shortcut mapping across menus and key handling

Key design rules:

- menus reflect command availability, not static placeholders
- command logic stays centralized in `app_core`

Exit criteria:

- users can reach major commands from both keyboard and menus without duplication drift

### Phase 3: Status And Recovery UX

Deliverables:

- clearer status text for save, export, autosave, and recovery
- concise error presentation for failed file operations
- recovery prompt or recovery banner flow that is more intentional than passive status text alone

Key design rules:

- keep production-facing messages concise
- preserve richer diagnostics in tracing output for development

Exit criteria:

- recovery and file-state behavior is visible and understandable during everyday use

### Phase 4: Multi-Document And Tab Polish

Deliverables:

- cleaner tab labeling and dirty-state display
- open-document switching behavior
- close handling with unsaved-work checks

Key design rules:

- document/session logic still belongs in `app_core`
- shell remains a consumer of session state, not the owner of document truth

Exit criteria:

- multi-document use feels intentional instead of purely structural

## Main Risks

- shell polish can sprawl into UI-framework work if command boundaries are not respected
- dialog integration can accidentally reintroduce UI-thread blocking
- menu routing can drift from shortcut behavior if implemented twice

## Validation Requirements

- controller tests for command routing where practical
- integration coverage for file-command behavior
- manual checks for Wayland, dialogs, recovery flow, and keyboard ergonomics

## Success Condition

PhotoTux feels significantly more complete in everyday use without changing its fixed-layout MVP shell model.