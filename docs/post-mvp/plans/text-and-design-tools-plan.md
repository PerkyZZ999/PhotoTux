# Post-MVP Plan: Text And Design Tools

## Purpose

Expand PhotoTux from a capable raster compositor into a stronger poster, UI, and design asset tool by introducing text and layout-oriented capabilities.

## Why This Is A Natural Next Step

Text layers were explicitly excluded from MVP because they are structurally heavier than the initial raster workflow. Once the core editor is stable, text becomes one of the most useful upgrades for design-oriented tasks.

This track is valuable, but it is not the safest first post-MVP step. It should begin only after the team is comfortable with the current shell and document-model maturity.

## Goal

Add text and design-oriented tooling without collapsing the clarity of the current raster-first architecture.

## Scope

### In Scope

- text layers
- font, size, alignment, and color controls
- transform and placement support for text objects
- rasterization policy for export and compatibility

### Explicitly Out Of Scope For The First Pass

- full vector design suite
- advanced typography engine parity with dedicated design tools
- rich text layout systems
- paragraph-composition complexity beyond a practical initial subset

## Recommended Delivery Order

1. text-layer document model
2. shell controls and editing flow
3. render path support
4. persistence and export rules
5. interoperability considerations later

## Work Breakdown

### Phase 1: Text Layer Foundation

Deliverables:

- text-layer representation in the document model
- minimal editable text content and style data
- layer-panel integration for text layers

Key design rules:

- do not force text into the raster-layer type if it creates architectural confusion
- decide early whether text remains editable in `.ptx` and when it rasterizes

Exit criteria:

- the document model can represent text layers distinctly and save them safely

### Phase 2: Editing And Shell Workflow

Deliverables:

- text tool
- placement workflow
- text properties in the shell
- commit, cancel, and selection behavior for text editing

Key design rules:

- preserve keyboard-first interaction where practical
- avoid mixing transient editor state with persistent document state

Exit criteria:

- users can create, edit, place, and style text layers predictably

### Phase 3: Rendering And Export

Deliverables:

- viewport rendering of text layers
- export behavior that matches the visible result
- save/load and undo/redo support

Key design rules:

- text rendering must not create viewport/export mismatches
- export behavior should be explicit if text is rasterized at the boundary

Exit criteria:

- text layers render and export consistently enough for real design assets

## Main Risks

- text introduces new editing-state complexity beyond current raster tools
- font handling, shaping, and layout can expand rapidly in scope
- text export and future PSD interoperability can become tightly coupled if not bounded well

## Validation Requirements

- save/load tests for editable text layers
- undo/redo tests for text edits and transforms
- export-versus-viewport parity checks for text scenes
- manual validation for keyboard editing flow

## Success Condition

PhotoTux becomes more useful for posters, UI mock assets, and design composition without pretending to be a full publishing or vector-layout tool.