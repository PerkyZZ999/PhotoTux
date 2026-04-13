# PhotoTux Roadmap

## Purpose

This roadmap turns the PRD and technical specification into a delivery sequence.

It is milestone-oriented rather than calendar-oriented.

Current status:

- Milestones 0 through 5 are now implemented at the task-list level
- MVP stabilization coverage now includes representative compositing scenes, repeated persistence checks, large sparse document validation, autosave and recovery regression coverage, and viewport-versus-export parity checks
- the current codebase also includes major post-MVP expansions: masks, layer groups, lasso selection, guides and snapping, pressure-aware painting, destructive filters, text layers, and limited PSD import
- next planning should focus on production hardening, startup smoothness, and performance trust rather than reopening already-completed post-MVP feature tracks

## Guiding Rules

- complete reliability layers before feature breadth
- prove responsiveness before deep workflow expansion
- keep shell complexity behind editor-core maturity
- do not move PSD work ahead of MVP editing quality

## Milestone 0: Project Foundation

Goal:

- establish a clean workspace, documentation set, and implementation boundaries

Deliverables:

- Rust workspace structure created
- core crates scaffolded
- build, lint, and test baseline established
- docs aligned with chosen architecture

Exit criteria:

- the repository is ready for feature implementation without structural ambiguity

## Milestone 1: Feasibility Prototype

Goal:

- prove that the GTK4 + `wgpu` stack feels viable for the canvas experience

Deliverables:

- application window and shell frame
- canvas surface hosted inside the shell
- viewport zoom and pan
- checkerboard background
- single editable raster layer
- brush and eraser prototype
- undo and redo for painting
- minimal save, reopen, and PNG export

Exit criteria:

- painting and navigation feel responsive enough to justify continued investment

## Milestone 2: Document Core

Goal:

- make layered document behavior reliable and testable

Deliverables:

- tile-backed raster document model
- multi-layer support
- layer create, rename, duplicate, delete, reorder
- visibility and opacity
- native `.ptx` project format
- history foundation for structural and raster edits
- import pipeline for PNG, JPEG, WebP

Exit criteria:

- layered state can be edited, saved, reopened, and trusted

## Milestone 3: Viewport and Shell Integration

Goal:

- connect the editing core to a usable fixed professional shell

Deliverables:

- left tool rail
- top tool options bar
- document tabs shell
- right-side core panels: Layers, Properties, Color, History
- status bar
- GPU composition for visible layers
- overlay rendering for selection and transform previews

Exit criteria:

- the editor looks and behaves like a usable graphics tool rather than a prototype surface

## Milestone 4: MVP Editing Workflow

Goal:

- complete an end-to-end layered editing workflow

Deliverables:

- move tool
- rectangular marquee selection
- selection clear and invert
- simple transform workflow with translate and scale
- initial blend-mode set
- keyboard shortcuts
- autosave and recovery
- export validation against visible composite

Exit criteria:

- a real design or compositing task can be completed from import to export without major instability

## Milestone 5: MVP Stabilization

Goal:

- remove major workflow risks and harden the editor for real use

Deliverables:

- regression fixes in history and persistence
- stress validation for large documents and long sessions
- performance cleanup for common brush and viewport paths
- documentation refresh for the shipped MVP boundary

Exit criteria:

- the product is trustworthy for repeated personal use on target Linux systems

## Completed Post-MVP Expansions

The current repository already includes these major post-MVP feature tracks:

- masks
- layer groups
- lasso selection
- transform upgrades
- guides and snapping
- pressure-aware painting controls
- destructive filters
- text layers
- limited PSD import

## Recommended Next Focus

The next major investments should center on production readiness:

- close remaining data-loss and workflow-safety gaps
- tighten startup behavior and startup latency
- improve runtime performance, fluidity, and frame-to-frame responsiveness
- harden install-safe resource loading, recovery paths, and corruption handling
- continue shell usability polish without expanding scope into docking or cross-platform work

## Current Anti-Goals

Do not allow these to displace the current production-hardening focus:

- docking system work
- elaborate workspace persistence
- raw Vulkan migration
- cross-platform expansion
- advanced PSD parity
- non-destructive adjustment-layer systems
