# PhotoTux Documentation Index

This directory is the development source of truth for PhotoTux.

The project is being developed as a Linux-first raster editor with a GTK4 shell, a custom `wgpu` canvas, and a Rust-owned document engine.

## Reading Order

Read these documents in this order when planning or implementing major work:

1. `prd.md`
2. `technical-specifications.md`
3. `research.md`
4. `architecture-overview.md`
5. `development-workflow.md`
6. `testing-strategy.md`
7. `roadmap.md`
8. `tasks-list.md`

Design-specific references:

- `design-ui/design-system.md`
- `design-ui/ui-layout-spec.md`
- `design-ui/PhotoshopUI.md`

## Document Roles

### Product and Architecture

- `prd.md`: Product scope, goals, user value, and explicit non-goals.
- `technical-specifications.md`: Locked technical direction and engineering constraints.
- `research.md`: Architecture rationale, accepted principles, and rejected alternatives.
- `architecture-overview.md`: A practical development map of how the major subsystems fit together.

### Delivery and Execution

- `development-workflow.md`: Day-to-day engineering workflow, milestone discipline, and definition of done.
- `testing-strategy.md`: What to test, when to test it, and how to avoid regressions in a raster editor.
- `roadmap.md`: Milestone-level delivery plan from feasibility to MVP and beyond.
- `tasks-list.md`: End-to-end implementation tasks required to reach MVP.

## Change Rules

- Update `prd.md` when product scope or target outcomes change.
- Update `technical-specifications.md` when a technical direction is locked or reversed.
- Update `research.md` when a new option is investigated and accepted or rejected.
- Update `roadmap.md` when milestone ordering changes.
- Update `tasks-list.md` whenever task scope, sequencing, or completion status changes.

## Documentation Principles

- Keep documents aligned with the actual implementation plan.
- Do not let aspirational features silently become implied scope.
- Prefer explicit tradeoffs over vague statements.
- Separate MVP commitments from later-phase ideas.
- Preserve the distinction between product decisions, technical decisions, and reference inspiration.