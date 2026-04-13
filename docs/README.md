# PhotoTux Documentation Index

This directory is the development source of truth for PhotoTux.

The current codebase is a Linux-first raster editor with a GTK4 shell, a custom `wgpu` viewport, a Rust-owned document engine, and implemented post-MVP extensions including masks, groups, lasso selection, guides and snapping, pressure-aware painting, destructive filters, text layers, and a limited PSD import path.

## Reading Order

Read these documents in this order when planning or implementing major work:

1. `prd.md`
2. `technical-specifications.md`
3. `research/research.md`
4. `architecture-overview.md`
5. `development-workflow.md`
6. `testing-strategy.md`
7. `roadmap.md`
8. `production-readiness-checklist.md`
9. `psd-compatibility.md`

Design-specific references:

- `design-ui/design-system.md`
- `design-ui/ui-layout-spec.md`
- `design-ui/PhotoshopUI.md`

Testing and automation references:

- `kwin-mcp-testing-guide.md`
- `tests/kwin-mcp-test-checklist.md`
- `tests/post-mvp-editing-workflow-checklist.md`
- `tests/post-mvp-painting-checklist.md`

## Document Roles

### Product and Architecture

- `prd.md`: Product scope, goals, user value, MVP boundaries, and current post-MVP reality.
- `technical-specifications.md`: Locked technical direction, crate ownership, and engineering constraints.
- `research/research.md`: Architecture rationale, accepted principles, and rejected alternatives.
- `architecture-overview.md`: Practical map of how the major subsystems fit together today.
- `psd-compatibility.md`: User-facing PSD import compatibility, fallback behavior, configuration requirements, and non-goals.

### Delivery and Execution

- `development-workflow.md`: Day-to-day engineering workflow, milestone discipline, and definition of done.
- `testing-strategy.md`: What to test, when to test it, and how to avoid regressions in a raster editor.
- `roadmap.md`: Milestone-level delivery history plus the current next-focus areas.
- `production-readiness-checklist.md`: Remaining app-quality work needed before a production-ready release, excluding packaging and release-distribution tasks.

### Reference and Validation

- `research/README.md`: Purpose of the research directory.
- `tests/kwin-mcp-test-checklist.md`: GUI test checklist for the current shell and workflow behavior.
- `tests/post-mvp-editing-workflow-checklist.md`: Manual validation checklist for masks, groups, lasso, transform, guides, and snapping.
- `tests/post-mvp-painting-checklist.md`: Manual validation checklist for pressure-aware painting, previews, filters, and repeated-stroke behavior.

## Change Rules

- Update `prd.md` when product scope or target outcomes change.
- Update `technical-specifications.md` when a technical direction is locked or reversed.
- Update `research/research.md` when a new option is investigated and accepted or rejected.
- Update `roadmap.md` when milestone ordering or current priorities change.
- Update `production-readiness-checklist.md` whenever production-hardening scope or status changes materially.
- Update `testing-strategy.md` whenever the canonical automated or manual validation set changes.

## Documentation Principles

- Keep documents aligned with the actual repository state.
- Do not let aspirational features silently become implied scope.
- Prefer explicit tradeoffs over vague statements.
- Separate MVP boundaries, current implemented features, and still-pending work.
- Preserve the distinction between product decisions, technical decisions, and reference inspiration.
