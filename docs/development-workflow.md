# PhotoTux Development Workflow

## Purpose

This document defines how development work should be planned, executed, and verified.

PhotoTux is a solo-developed desktop application with performance-sensitive behavior, so the workflow prioritizes scope discipline, correctness, and incremental validation.

## Development Priorities

When there is a tradeoff, use this priority order:

1. correctness and data safety
2. responsiveness and interaction quality
3. architectural clarity
4. MVP completion speed
5. convenience features and polish

## Planning Rules

Before implementing a feature:

1. confirm it is inside MVP or explicitly post-MVP
2. identify the owning crate and subsystem boundary
3. define the document-model changes required
4. define renderer invalidation and history implications
5. define test coverage before coding the feature

Do not start implementation from the shell outward.
Start from the document model and behavior rules, then wire the shell.

## Implementation Order

Use this general order for major features:

1. data structures and domain model
2. pure logic and tests
3. persistence rules if needed
4. renderer integration
5. tool interaction logic
6. shell wiring and UX polish

This order keeps core behavior testable and reduces UI-driven architectural drift.

## Branch and Commit Discipline

Recommended branch pattern:

- `main` for the active integration branch if working solo
- feature branches for larger efforts

Recommended commit pattern:

- one problem per commit
- separate refactors from feature behavior when practical
- do not mix scope expansion with bug fixes unless the two are inseparable

## Definition of Ready

A task is ready when:

- its scope is explicit
- it belongs to a milestone
- owning crates are known
- acceptance criteria are written
- obvious dependencies are identified

If those conditions are missing, the task should be refined before implementation.

## Definition of Done

A development task is done when:

- the feature behaves as specified
- relevant automated tests exist or an explicit testing limitation is documented
- persistence and history implications are handled
- error behavior is acceptable
- performance is acceptable for the task's expected usage
- documentation is updated if the task changes scope or architecture

## Daily Development Loop

Recommended loop:

1. choose one active task from `roadmap.md`, `production-readiness-checklist.md`, or the relevant focused checklist under `docs/tests/`
2. restate acceptance criteria
3. implement the model and logic first
4. add or update tests
5. wire renderer and shell behavior
6. manually validate the interaction
7. update task status and any impacted documentation

## Milestone Discipline

Each milestone should have a clear exit condition.

### Feasibility Milestone

Goal:

- prove that the stack feels viable

Exit condition:

- viewport, paint, undo, save, reopen, and export all work with acceptable responsiveness

### Document Core Milestone

Goal:

- make layered documents reliable

Exit condition:

- layers, history, and native save/load behave predictably

### MVP Milestone

Goal:

- complete an actual design or compositing workflow end to end

Exit condition:

- a user can import, edit, save, reopen, and export a layered document without major instability

## Risk Management Rules

If work exposes one of these risk areas, stop and tighten the design before proceeding:

- save/load integrity
- history corruption
- alpha and blend correctness
- scaling and coordinate mismatch under Wayland
- renderer invalidation blowing up into full redraws
- UI thread blocking during heavy operations

## Performance Validation Rules

Measure or at least manually validate these whenever relevant code changes:

- brush latency
- pan and zoom smoothness
- GPU upload behavior on local edits
- save latency
- autosave behavior during editing
- memory growth across long sessions

## Documentation Maintenance

Update documentation whenever work changes:

- scope
- milestone ordering
- architecture boundaries
- testing expectations
- project structure

At minimum, review these after each milestone-sized change:

- `technical-specifications.md`
- `architecture-overview.md`
- `roadmap.md`
- `production-readiness-checklist.md`

## What Not To Do Early

- do not build a docking framework before the fixed shell is stable
- do not add new feature families because the shell can visually support them
- do not optimize with raw Vulkan before measuring a real `wgpu` limitation
- do not introduce broad async complexity unless the lightweight job model becomes insufficient
- do not let PSD compatibility work displace core editing reliability
