# Post-MVP Planning Index

This directory captures the most useful next-step options after the current MVP.

These plans are not committed scope yet. They are decision-ready option sets that can be selected, reordered, narrowed, or combined depending on the next product goal.

## Recommended Reading Order

1. `plans/editing-workflow-upgrade-plan.md`
2. `tasks/editing-workflow-upgrade-tasks.md`
3. `plans/psd-file-format-expansion-plan.md`
4. `tasks/psd-file-format-expansion-tasks.md`
5. `plans/shell-usability-and-native-workflows-plan.md`
6. `tasks/shell-usability-and-native-workflows-tasks.md`
7. `plans/release-and-distribution-plan.md`
8. `tasks/release-and-distribution-tasks.md`
9. `plans/painting-and-input-expressiveness-plan.md`
10. `tasks/painting-and-input-expressiveness-tasks.md`
11. `plans/text-and-design-tools-plan.md`
12. `tasks/text-and-design-tools-tasks.md`

## Suggested Priority Order

If the goal is maximum practical value for design work in the shortest time, use this sequence:

1. editing workflow upgrades
2. PSD import subset
3. shell usability and native workflow polish
4. release and distribution hardening
5. painting and input expressiveness
6. text and design tools

## Plan Set

### 1. Editing Workflow Upgrades

See `plans/editing-workflow-upgrade-plan.md`.

Implementation tasks:

- `tasks/editing-workflow-upgrade-tasks.md`

Focus:

- masks
- layer groups
- lasso and selection depth
- better transforms
- guides and snapping

Why it matters:

- this is the fastest route to making the editor more capable for real layered composition work without changing the product identity

### 2. PSD and File-Format Expansion

See `plans/psd-file-format-expansion-plan.md`.

Implementation tasks:

- `tasks/psd-file-format-expansion-tasks.md`

Focus:

- limited PSD import first
- later PSD export subset
- diagnostics for unsupported features

Why it matters:

- interoperability is the clearest route to pulling real work into PhotoTux from external tools

### 3. Shell Usability and Native Workflows

See `plans/shell-usability-and-native-workflows-plan.md`.

Implementation tasks:

- `tasks/shell-usability-and-native-workflows-tasks.md`

Focus:

- better file workflows
- native dialogs and menu routing
- faster common interactions
- stronger keyboard-first shell behavior

Why it matters:

- MVP works, but a lot of daily-use friction still lives in shell and workflow polish rather than raster correctness

### 4. Release and Distribution Hardening

See `plans/release-and-distribution-plan.md`.

Implementation tasks:

- `tasks/release-and-distribution-tasks.md`

Focus:

- CI hardening
- packaging
- release validation
- repeatable builds

Why it matters:

- once feature direction is clear, shipping quality and repeatability become the next trust multiplier

### 5. Painting and Input Expressiveness

See `plans/painting-and-input-expressiveness-plan.md`.

Implementation tasks:

- `tasks/painting-and-input-expressiveness-tasks.md`

Focus:

- tablet pressure
- better brush behavior
- filter groundwork
- richer paint ergonomics

Why it matters:

- this track is useful if the next target user is more illustration-oriented than layout/compositing-oriented

### 6. Text and Design Tools

See `plans/text-and-design-tools-plan.md`.

Implementation tasks:

- `tasks/text-and-design-tools-tasks.md`

Focus:

- text layers
- typography controls
- layout-oriented design assistance

Why it matters:

- this is high-value for poster, UI, and marketing asset workflows, but it is also structurally heavier than the options above

## Decision Guidance

Choose the next track based on the dominant goal:

- Better everyday compositing workflow: start with editing workflow upgrades.
- Better interoperability with external tools: start with PSD import.
- Better daily usability on Linux: start with shell and native workflow polish.
- Better readiness for broader testing or release: start with release and distribution hardening.
- Better illustration feel: start with painting and input expressiveness.
- Better poster and UI design capability: start with text and design tools.