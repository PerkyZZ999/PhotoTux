# PhotoTux Architecture Overview

## Purpose

This document explains how the major parts of PhotoTux fit together during development.
It is a practical architecture map for implementation work.

The canonical technical rules still live in `technical-specifications.md`.

## Top-Level Architecture

PhotoTux is split into four primary execution concerns:

1. Shell and platform integration
2. Document and editing state
3. GPU viewport rendering
4. Background jobs and persistence

This separation exists to preserve responsiveness and reduce coupling.

## Core System Boundaries

### 1. Shell Layer

Owned by GTK4.

Responsibilities:

- application window and header bar
- menus, dialogs, shortcuts, and status surfaces
- fixed workspace layout
- panel widgets and toolbars
- raw input events before they are translated into tool or viewport commands

Non-responsibilities:

- document ownership
- raster editing logic
- persistent file-format logic
- authoritative viewport state for the renderer

## 2. Application Layer

Owned by `app_core`.

Responsibilities:

- application lifecycle
- command dispatch
- session management
- orchestration between shell, document model, tools, renderer, and file IO
- autosave scheduling and recovery coordination
- background job submission and result handling

This layer is the integration hub.

## 3. Document Engine

Owned primarily by `doc_model`, with support from `history_engine`, `image_ops`, and `color_math`.

Responsibilities:

- layered raster document model
- stable IDs and metadata
- tile-backed raster storage
- selection state
- transform state
- edit operations and history bookkeeping

Hard rule:

- the document model is the source of truth

The document engine must remain testable without GTK4 or `wgpu` initialization.

## 4. Render Engine

Owned by `render_wgpu`.

Responsibilities:

- GPU device and surface setup
- texture upload scheduling
- layer composition
- viewport presentation
- overlays such as selections, guides, transform handles, and brush previews

Hard rules:

- GPU resources are caches, not the source of truth
- the renderer must be driven from document and viewport state, not widget state
- small edits should only trigger bounded GPU work

## 5. Tool Engine

Owned by `tool_system`.

Responsibilities:

- translate input events into editing intent
- run brush, eraser, move, marquee, and transform interactions
- support low-latency preview behavior during direct manipulation

The tool engine should own interaction logic, not document persistence or panel UI behavior.

## 6. Persistence and Recovery

Owned by `file_io`.

Responsibilities:

- native `.ptx` save and load
- import and export for PNG, JPEG, and WebP
- autosave files and recovery discovery
- versioned manifest and payload handling
- future PSD adapters

Hard rule:

- `.ptx` remains the authoritative project format

## Execution Model

PhotoTux uses three classes of work:

### UI Thread

Handles:

- GTK widgets
- shortcuts and menus
- command routing
- shell state updates

Must not block on:

- file IO
- large imports or exports
- expensive raster processing
- heavyweight transforms

### Render Path

Handles:

- viewport redraws
- overlays
- zoom and pan updates
- direct-manipulation feedback

This path must remain incremental and bounded.

### Worker Jobs

Handles:

- save and load
- autosave
- import and export
- thumbnail generation
- heavy resampling
- future destructive filters

Worker-job results are applied through the application layer, never directly into widget-owned state.

## Data Flow

Typical edit flow:

1. GTK4 receives input.
2. `app_core` routes the event to the active tool or viewport handler.
3. `tool_system` generates editing intent.
4. `doc_model` and `image_ops` update document state.
5. `history_engine` records the committed operation.
6. `render_wgpu` receives dirty-region or dirty-tile information.
7. The viewport is recomposited and presented.

Typical save flow:

1. User triggers save.
2. `app_core` snapshots the relevant document state for a save job.
3. `file_io` writes the project in the background.
4. Success or failure is returned to `app_core`.
5. The shell shows status without blocking the editing session.

## Crate Map

- `app_core`: application lifecycle and integration hub
- `ui_shell`: GTK4 shell, panels, and menus
- `doc_model`: document state and tile-backed raster structures
- `render_wgpu`: GPU composition and viewport presentation
- `tool_system`: tool interactions and editing intent
- `history_engine`: undo and redo bookkeeping
- `file_io`: project persistence, import, export, autosave, recovery
- `image_ops`: CPU raster operations
- `color_math`: color and blend utilities
- `common`: shared geometry, IDs, traits, constants, and errors

## Early Architecture Constraints

- fixed layout before any docking system
- raster layers only in MVP
- limited blend mode set in MVP
- no text layers in MVP
- no PSD import or export in MVP
- no raw Vulkan or second rendering stack unless a proven bottleneck forces it

## Architecture Checklist For New Work

Before adding a feature, verify:

1. Which crate owns the feature?
2. Does it mutate the source-of-truth document model?
3. Does it require history integration?
4. Does it affect renderer invalidation or GPU upload policy?
5. Can any part of it block the UI thread?
6. Is it MVP scope or later scope?

If any of those answers are unclear, the architecture is not ready for implementation yet.