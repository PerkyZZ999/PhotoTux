# Product Requirements Document (PRD)

## Project Name
**Project Codename:** PhotoTux
**Type:** Linux-first professional raster graphics editor for design and compositing
**Primary Platform:** Linux desktop
**Primary Desktop Target:** CachyOS on KDE Plasma with Wayland
**Owner:** Solo developer
**Status:** Draft v2

---

## 1. Product Overview

PhotoTux is a Linux-first, high-performance raster graphics editor focused on layered design work, compositing, digital painting, texture editing, mockups, and general-purpose pixel editing.

The goal is not full Photoshop feature parity. The goal is a Photoshop-like editing experience for Linux with strong canvas performance, familiar layered workflows, and a cleaner technical foundation.

This project is intended for personal use first. That means the roadmap should optimize for usefulness, correctness, and development speed rather than broad market coverage.

---

## 2. Product Positioning

PhotoTux is a design-oriented raster editor, not a photography workflow application.

The product is intended for:
- digital painting
- UI and web mockups
- layered image compositing
- texture editing
- thumbnails, banners, and general graphics work
- pixel-level editing of imported images

The product is not intended to prioritize:
- RAW development
- camera pipeline features
- lens correction workflows
- Lightroom-style library management
- print and prepress pipelines in the early product

The product should still interoperate with existing design workflows where practical, including limited PSD compatibility.

---

## 3. Problem Statement

Linux still lacks a polished, modern, native-first raster editor that combines:
- responsive canvas interaction
- reliable layered documents
- strong brush feel
- familiar editing concepts
- GPU-accelerated rendering
- Linux-specific platform attention

Existing tools often involve tradeoffs between performance, polish, predictability, extensibility, or Linux-native behavior.

PhotoTux aims to close that gap with a focused editor built around design and compositing workflows instead of trying to cover every photography or publishing use case.

---

## 4. Vision

Build a Linux-native editor that becomes the personal daily driver for design-oriented raster work.

The editor should feel:
- fast
- predictable
- visually clean
- keyboard-friendly
- trustworthy under long sessions

It should feel professional before it feels feature-complete.

---

## 5. Product Objectives

### Primary Objectives
- Deliver a smooth Linux desktop editing experience with low interaction latency.
- Support layered raster documents suitable for real design and compositing tasks.
- Build a reliable save/load/history core before expanding feature breadth.
- Establish a reusable document engine and rendering pipeline.
- Keep the initial scope narrow enough for a solo developer to ship usable milestones.

### Secondary Objectives
- Keep the UI minimalist and efficient.
- Leave room for future scripting, plugins, and more advanced layer types.
- Support later cross-platform expansion if it becomes worthwhile.
- Preserve architectural flexibility for future non-destructive features.

---

## 6. Product Principles

- **Performance before breadth:** Canvas interaction quality matters more than feature count.
- **Correctness before cleverness:** Save/load/history must be trustworthy.
- **Design-focused scope:** Prioritize compositing and design workflows over photography workflows.
- **Progressive complexity:** Start with destructive or simpler implementations where they reduce delivery risk.
- **Predictable behavior:** Tools, transforms, and history should behave consistently.
- **Separation of concerns:** Document state, rendering, UI, and file persistence must remain cleanly separated.

---

## 7. Target User

### Primary User
- Solo Linux power user
- Comfortable with professional creative tools
- Prefers keyboard shortcuts, panels, precise selection, layers, and direct manipulation

### User Characteristics
- Uses Linux as the main OS
- Values speed and precision over beginner simplification
- Wants Photoshop-like workflow familiarity without requiring exact parity
- Cares more about compositing and graphics work than camera-photo processing

---

## 8. Core Use Cases

### Core Use Cases
1. Create a new canvas for a design or artwork.
2. Import one or more raster images into a layered document.
3. Paint and erase on raster layers with low latency.
4. Reorder layers and control visibility and opacity.
5. Select and move parts of a layer.
6. Apply common transforms for layout and compositing.
7. Save layered work and reopen it without visual regression.
8. Export flattened output for sharing or publishing.

### Secondary Use Cases
1. Create simple poster or thumbnail layouts.
2. Do texture cleanup and game-asset editing.
3. Create rough paint-overs and concept mockups.
4. Perform small image edits without needing a photography pipeline.

---

## 9. Explicit Non-Goals

The following are out of scope for the first major releases:
- Camera RAW import or development workflows
- Lightroom-style catalog or asset management
- lens correction, denoise, HDR merge, panorama merge
- full CMYK and print publishing workflows
- PSD compatibility with high fidelity across all Photoshop features
- collaborative editing
- cloud sync
- web version
- full vector illustration tooling
- animation timeline
- video editing
- AI generation in early releases

These constraints are intentional. They keep the project aligned with the actual personal use case.

---

## 10. Release Strategy

PhotoTux will be delivered in three scope layers instead of one oversized MVP.

### Prototype
The prototype exists to prove that the editor feels good.

Included:
- single raster document
- one editable raster layer
- checkerboard canvas
- zoom and pan
- brush and eraser
- undo and redo for painting
- native save and reopen for a minimal document
- PNG export

Success condition:
- painting and navigation feel fast enough to justify continued development

### MVP
The MVP exists to complete real layered design tasks.

Included:
- multi-layer raster document
- create, rename, duplicate, delete, reorder layers
- visibility toggle
- opacity control
- initial blend mode set
- move tool
- rectangular selection
- simple transform workflow
- import PNG, JPEG, WebP
- export PNG, JPEG, WebP
- native layered project format
- autosave and crash recovery
- keyboard shortcuts
- fixed professional layout with core panels

Explicitly excluded from MVP:
- masks
- layer groups
- freeform lasso
- text layers
- adjustment layers
- dockable layout system
- advanced tablet support beyond a basic extension point
- PSD import and export

### Post-MVP
The first expansion phase adds higher-leverage workflow features.

Candidates:
- masks
- layer groups
- lasso selection
- better transform tools
- guides and snapping
- text layers
- destructive filters
- tablet pressure support
- limited PSD import
- limited PSD export

---

## 11. Functional Requirements

### Document
- The application must support layered raster documents in the MVP.
- The application must reopen saved projects without visual corruption.
- The application must support canvases from small UI assets to large artwork.

### Layers
- Users must be able to create, duplicate, delete, rename, and reorder raster layers.
- Users must be able to control layer visibility and opacity.
- Users must be able to apply a defined initial blend mode set.
- Layer groups are deferred until after MVP.

### Canvas Interaction
- The canvas must support smooth zooming and panning.
- The canvas must render transparency clearly.
- The viewport must remain responsive under repeated interaction.

### Painting
- Users must be able to paint and erase on raster layers.
- Brush size, hardness, opacity, flow, and spacing must be adjustable.
- Brush input latency must feel immediate during normal use.
- Pressure support is a later milestone, but the architecture must not block it.

### Selection and Transform
- MVP must support rectangular selection.
- Users must be able to clear, invert, and move selections.
- MVP transform support may begin with translate and scale before more advanced transforms.

### History
- Undo and redo must work across document and editing operations.
- Brush strokes should be grouped meaningfully.
- Save operations and autosaves must not corrupt undo state.

### File Handling
- Early import must support PNG, JPEG, and WebP.
- Export must match the visible flattened composite.
- The native project format must preserve layered state, metadata, and version information.
- PSD interoperability is required after MVP, but only as a defined supported subset.
- PSD import should be prioritized before PSD export.
- Unsupported PSD features must fail clearly rather than importing silently with incorrect results.

---

## 12. User Experience Requirements

### Initial Layout
The first usable UI should include:
- left toolbar
- top tool/options strip
- center canvas
- right properties and layer panels
- bottom status information

### UX Goals
- clean default layout
- low visual noise
- strong keyboard-first interaction
- minimal latency when using the canvas
- clear focus on the document rather than decorative chrome

### MVP UI Constraint
The MVP should use a fixed but polished panel layout rather than a full dockable layout system.

---

## 13. Quality Requirements

### Performance Targets
- Brush latency should stay low enough to feel direct.
- Zoom and pan should remain smooth on large but reasonable documents.
- Layer composition should remain usable as document complexity grows.

### Reliability Targets
- No data corruption in normal editing flows.
- Autosave and recovery must arrive no later than MVP.
- Reopening a saved project should preserve the visual result accurately.

### Platform Targets
- The product must behave correctly on KDE Plasma with Wayland.
- High-DPI and fractional scaling behavior must be acceptable.
- File dialogs, clipboard behavior, and pointer interaction must feel native enough for daily use.

---

## 14. Success Criteria

### Prototype Success
- One can paint on a canvas, undo, save, reopen, and export without obvious corruption.
- The canvas feels responsive enough to justify the stack.

### MVP Success
- A real layered design or compositing task can be completed from import to export.
- Save/load/history remain trustworthy across repeated editing.
- The application is personally preferable to existing Linux alternatives for at least some recurring tasks.

### Technical Success
- Stable native project format
- reliable undo and redo
- GPU-backed canvas composition
- clear separation between UI, document state, and rendering cache

---

## 15. Roadmap

### Milestone 0 - Feasibility Prototype
- project scaffolding
- window creation
- canvas viewport
- zoom and pan
- brush and eraser on a single raster surface
- minimal save and export path

### Milestone 1 - Document Core
- layered document model
- layer operations
- native project format
- history foundation
- import pipeline

### Milestone 2 - Usable MVP
- rectangular selection
- move tool
- simple transform workflow
- blend modes
- autosave and recovery
- polished fixed layout and shortcuts

### Milestone 3 - Workflow Upgrade
- masks
- groups
- lasso selection
- guides and snapping
- destructive filters
- tablet support improvements
- limited PSD import

### Milestone 4 - Expansion
- text layers
- adjustment layers
- plugin or scripting hooks
- better file compatibility
- limited PSD export

---

## 16. Major Risks

- Brush feel may be harder than raw rendering performance.
- Undo and redo for raster editing can become memory-heavy quickly.
- Color and alpha rules can cause correctness problems if left vague.
- Wayland-specific input and scaling behavior may expose edge cases early.
- Large-document handling may require tiling and cache rules sooner than expected.
- Scope expansion toward Photoshop parity can derail delivery.
- PSD support can become a time sink if the supported subset is not defined aggressively.

---

## 17. Key Decisions to Lock Early

- initial pixel format and working color rules
- alpha handling rules
- native project format structure
- tile size for runtime raster storage
- history model for raster edits
- MVP blend mode list
- fixed layout first versus dockable layout later
- PSD compatibility policy and supported feature subset

---

## 18. Final Product Statement

PhotoTux is a Linux-first raster editor for design, painting, compositing, and general graphics work. It is intentionally not a photography or Camera RAW application. The product will prioritize canvas feel, layered editing reliability, Linux-native usability, and practical PSD interoperability over broad feature parity.
