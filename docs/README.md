# PhotoTux Documentation Index

This directory is the development source of truth for PhotoTux.

The project is being developed as a Linux-first raster editor with a GTK4 shell, a custom `wgpu` canvas, and a Rust-owned document engine.

## Reading Order

Read these documents in this order when planning or implementing major work:

1. `prd.md`
2. `technical-specifications.md`
3. `research/research.md`
4. `architecture-overview.md`
5. `development-workflow.md`
6. `testing-strategy.md`
7. `roadmap.md`
8. `pre-mvp/tasks-list.md`

Design-specific references:

- `design-ui/design-system.md`
- `design-ui/ui-layout-spec.md`
- `design-ui/PhotoshopUI.md`

Testing and automation references:

- `kwin-mcp-testing-guide.md`
- `kwin-mcp-test-checklist.md`

Post-MVP planning references:

- `post-mvp/README.md`
- `post-mvp/plans/README.md`
- `post-mvp/tasks/README.md`
- `post-mvp/plans/editing-workflow-upgrade-plan.md`
- `post-mvp/tasks/editing-workflow-upgrade-tasks.md`
- `post-mvp/plans/psd-file-format-expansion-plan.md`
- `post-mvp/tasks/psd-file-format-expansion-tasks.md`
- `post-mvp/plans/shell-usability-and-native-workflows-plan.md`
- `post-mvp/tasks/shell-usability-and-native-workflows-tasks.md`
- `post-mvp/plans/release-and-distribution-plan.md`
- `post-mvp/tasks/release-and-distribution-tasks.md`
- `post-mvp/plans/painting-and-input-expressiveness-plan.md`
- `post-mvp/tasks/painting-and-input-expressiveness-tasks.md`
- `post-mvp/plans/text-and-design-tools-plan.md`
- `post-mvp/tasks/text-and-design-tools-tasks.md`

## Document Roles

### Product and Architecture

- `prd.md`: Product scope, goals, user value, and explicit non-goals.
- `technical-specifications.md`: Locked technical direction and engineering constraints.
- `research/research.md`: Architecture rationale, accepted principles, and rejected alternatives.
- `architecture-overview.md`: A practical development map of how the major subsystems fit together.

### Delivery and Execution

- `development-workflow.md`: Day-to-day engineering workflow, milestone discipline, and definition of done.
- `testing-strategy.md`: What to test, when to test it, and how to avoid regressions in a raster editor.
- `kwin-mcp-testing-guide.md`: Host setup, VS Code MCP configuration, and staged adoption plan for KWin-based desktop automation.
- `kwin-mcp-test-checklist.md`: GUI test checklist for the current PhotoTux MVP feature set using KWin-based automation.
- `roadmap.md`: Milestone-level delivery plan from feasibility to MVP and beyond.
- `pre-mvp/tasks-list.md`: End-to-end implementation tasks required to reach MVP.

### Structured Subdirectories

- `research/README.md`: Explains the purpose of the research directory.
- `pre-mvp/README.md`: Explains the purpose of the MVP task-planning directory.
- `post-mvp/plans/README.md`: Explains the purpose of the post-MVP plan directory.
- `post-mvp/tasks/README.md`: Explains the purpose of the post-MVP task directory.

### Post-MVP Planning

- `post-mvp/README.md`: Index and suggested ordering for the next natural tracks after MVP.
- `post-mvp/plans/editing-workflow-upgrade-plan.md`: Masks, groups, lasso, transform upgrades, guides, and snapping.
- `post-mvp/tasks/editing-workflow-upgrade-tasks.md`: End-to-end implementation sequence for workflow upgrades.
- `post-mvp/plans/psd-file-format-expansion-plan.md`: PSD import-first interoperability planning.
- `post-mvp/tasks/psd-file-format-expansion-tasks.md`: End-to-end implementation sequence for PSD interoperability.
- `post-mvp/plans/shell-usability-and-native-workflows-plan.md`: Workflow polish in menus, dialogs, tabs, and shell command surfaces.
- `post-mvp/tasks/shell-usability-and-native-workflows-tasks.md`: End-to-end implementation sequence for shell and native workflow polish.
- `post-mvp/plans/release-and-distribution-plan.md`: CI, release validation, and Linux packaging planning.
- `post-mvp/tasks/release-and-distribution-tasks.md`: End-to-end implementation sequence for release and distribution hardening.
- `post-mvp/plans/painting-and-input-expressiveness-plan.md`: Pressure support, brush dynamics, and painting-oriented depth.
- `post-mvp/tasks/painting-and-input-expressiveness-tasks.md`: End-to-end implementation sequence for painting and input expansion.
- `post-mvp/plans/text-and-design-tools-plan.md`: Text layers and design-tool expansion planning.
- `post-mvp/tasks/text-and-design-tools-tasks.md`: End-to-end implementation sequence for text and design tools.

## Change Rules

- Update `prd.md` when product scope or target outcomes change.
- Update `technical-specifications.md` when a technical direction is locked or reversed.
- Update `research/research.md` when a new option is investigated and accepted or rejected.
- Update `roadmap.md` when milestone ordering changes.
- Update `pre-mvp/tasks-list.md` whenever task scope, sequencing, or completion status changes.

## Documentation Principles

- Keep documents aligned with the actual implementation plan.
- Do not let aspirational features silently become implied scope.
- Prefer explicit tradeoffs over vague statements.
- Separate MVP commitments from later-phase ideas.
- Preserve the distinction between product decisions, technical decisions, and reference inspiration.