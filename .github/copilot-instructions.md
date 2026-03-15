# GitHub Copilot Instructions

## Priority Guidelines

When generating code for this repository:

1. Read the documentation in `docs/README.md` first.
2. Treat the documentation set as the primary source of truth until the codebase establishes stronger local patterns.
3. Preserve the documented architecture boundaries without exception.
4. Prioritize correctness, responsiveness, and maintainability over feature breadth.
5. Do not silently expand MVP scope.

## Primary Source Documents

Consult these files before making architectural or feature decisions:

- `docs/prd.md`
- `docs/technical-specifications.md`
- `docs/research/research.md`
- `docs/architecture-overview.md`
- `docs/development-workflow.md`
- `docs/testing-strategy.md`
- `docs/roadmap.md`
- `docs/pre-mvp/tasks-list.md`

Consult these UI references when working on shell or layout code:

- `docs/design-ui/design-system.md`
- `docs/design-ui/ui-layout-spec.md`
- `docs/design-ui/PhotoshopUI.md`

## Technology and Version Guidance

- Use Rust stable as installed in the repository environment.
- Use Rust edition 2024 unless existing crate configuration in the repository requires otherwise.
- The project is Linux-first and desktop-native.
- The shell technology is GTK4 via `gtk4-rs`.
- The canvas renderer is `wgpu`.
- WGSL is the shader language.
- The `image` crate is for import, export, and utility work, not the core runtime raster model.
- Do not introduce raw Vulkan, Skia, Halide, or GEGL into MVP work unless the repository documents a deliberate change.

## Architecture Boundaries

Follow the documented crate ownership model:

- `app_core`: application lifecycle, session orchestration, command routing, jobs
- `ui_shell`: GTK4 shell, panels, menus, status surfaces, shell composition
- `doc_model`: source-of-truth document model and tile-backed raster structures
- `render_wgpu`: GPU composition, overlays, viewport presentation
- `tool_system`: tool interactions and editing intent
- `history_engine`: undo and redo bookkeeping
- `file_io`: persistence, import/export, autosave, recovery
- `image_ops`: CPU raster operations
- `color_math`: color and blend utilities
- `common`: shared types, constants, IDs, traits, and errors

Hard rules:

- The document model is the source of truth.
- GPU resources are caches and presentation artifacts only.
- UI state must not own document state.
- GTK4 must not become the primary canvas rasterization path.
- Keep document logic testable without GTK4 or `wgpu` initialization.

## Scope Rules

MVP includes:

- layered raster documents
- painting and erasing
- layer management
- rectangular selection
- move tool
- simple transform workflow
- native `.ptx` save and load
- PNG, JPEG, and WebP import/export
- autosave and recovery
- fixed professional shell layout

Explicitly out of MVP unless the docs change:

- docking systems
- masks
- layer groups
- text layers
- adjustment layers
- PSD import/export
- raw photo workflows
- cross-platform expansion work

## Responsiveness Rules

The project has three execution classes:

1. UI thread for GTK widgets, menus, shortcuts, and shell state.
2. Render path for viewport presentation and direct-manipulation feedback.
3. Worker jobs for file IO, autosave, import/export, thumbnails, heavy resampling, and later filters.

Always preserve these rules:

- Do not block the UI thread on long-running file or image operations.
- Treat brush preview, pan, zoom, selection drag, and transform preview as low-latency direct-manipulation paths.
- Route worker-job results back through the application layer, not directly into widget-owned state.

## Code Generation Standards

- Prefer small, composable modules with clear ownership.
- Write self-documenting code with descriptive names.
- Keep functions focused and avoid hidden side effects.
- Add comments only where behavior is non-obvious.
- Avoid introducing new patterns unless the repository already needs them.

## Testing Standards

Follow the testing strategy in `docs/testing-strategy.md`.

Minimum expectations:

- add unit tests for pure logic when practical
- add integration coverage for persistence and workflow boundaries when practical
- protect save/load, undo/redo, alpha/blend behavior, and selection logic from regressions
- treat viewport-versus-export mismatch as a serious defect

## Workflow Rules

- Implement from model and behavior outward, then wire the shell.
- Update `docs/pre-mvp/tasks-list.md` as work progresses.
- Update `docs/roadmap.md` if milestone ordering changes.
- Update `docs/technical-specifications.md` if architectural decisions change.
- Do not treat aspirational reference material as committed scope.

## Project-Specific Guidance

- This repository is currently docs-first. Prefer consistency with the documented plan over external defaults.
- Until stronger code patterns emerge, use the documentation to choose naming, boundaries, and priorities.
- Keep Milestone 0 focused on workspace scaffolding, buildability, and clean subsystem boundaries.
- Keep Milestone 1 focused on proving the stack and interaction model rather than polishing advanced workflows.