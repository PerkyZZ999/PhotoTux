# PhotoTux Research Summary

## Chosen Direction

PhotoTux will use a thin native shell with a custom GPU-backed canvas.

- **Language:** Rust
- **Shell/UI Toolkit:** GTK4 via `gtk4-rs`
- **Canvas Renderer:** `wgpu`
- **Shaders:** WGSL
- **Image Utilities:** `image` crate for import, export, and supporting utilities
- **Core Editing Model:** PhotoTux-owned raster and tile structures plus editor-specific logic

This is a Linux-first architecture. It prioritizes native desktop behavior, predictable performance, and a maintainable solo-developer implementation path.

## Architecture Principles Adopted

### 1. The UI Toolkit Must Not Own Canvas Rendering

GTK4 should own windows, menus, dialogs, panels, shortcuts, and platform integration.
The editing canvas should be rendered through a custom `wgpu` path.

This keeps the shell separate from the high-frequency viewport work.

### 2. Responsiveness Depends on Work Separation

The editor must separate work into three execution classes:

1. UI thread for GTK widgets, menus, shell state, and command dispatch.
2. Render path for viewport presentation and immediate interaction feedback.
3. Worker jobs for save/load, autosave, import/export, thumbnails, heavy transforms, and later filters.

This is the main architectural rule behind perceived responsiveness.

### 3. Direct-Manipulation Paths Need Immediate Feedback

Brush preview, pan, zoom, selection drag, and transform preview should not be treated like generic background jobs.
They need fast, incremental updates with bounded work per frame.

### 4. GPU for Viewport, CPU for Early Editing Core

Early PhotoTux should use CPU-authored raster edits combined with GPU composition and presentation.
GPU compute or heavier shader-based processing should be added only where it clearly improves user-facing latency.

## Technology Decisions

### GTK4

GTK4 is the chosen shell technology because it gives good Linux-native behavior for:

- windowing
- dialogs
- clipboard integration
- keyboard handling
- Wayland behavior
- future tablet and input-device support

`relm4` remains optional. It may help shell state organization, but it is not a locked project dependency.

### wgpu Instead of Raw Vulkan

`wgpu` is the current choice.

Reasons:

- it preserves GPU-backed architecture without forcing raw Vulkan complexity early
- it is sufficient for composition, overlays, zoom/pan, and custom shaders
- it keeps the project focused on editor architecture rather than low-level graphics engineering

Raw Vulkan is a later escape hatch, not the baseline implementation plan.

### image Crate Role

The `image` crate is useful, but its role should stay narrow:

- image decoding and encoding
- import/export plumbing
- utility transforms where performance is not on the critical path

It should not define the editor's core runtime raster model.

## Research Considered But Not Adopted for MVP

### Skia

Skia is strong for 2D rendering, but it would introduce a second major rendering system beside GTK4 and `wgpu`.
For PhotoTux MVP, that is more integration complexity than value.

### Halide

Halide is powerful for optimized image-processing pipelines, but it is not a good early-project fit.
It becomes interesting only after specific image-processing bottlenecks are measured.

### GEGL

GEGL is a serious graph-based image-processing framework, but adopting it early would pull PhotoTux toward a much heavier processing architecture.
That is not justified for the initial raster-editor scope.

### Raw Surface Interop as a Baseline

Direct low-level surface interop via `raw-window-handle` may be useful for feasibility spikes.
It should not be treated as a required baseline for the first implementation.

## Linux and Wayland Notes

### High-DPI and Fractional Scaling

GTK4 handles desktop scaling well at the shell level.
The canvas renderer still needs explicit logical-to-physical sizing.

The practical rule is:

`physical buffer size = logical size * scale factor`

### Plasma and Wayland Requirements

PhotoTux should validate these early:

- canvas resize correctness
- scale-factor changes
- pointer coordinates under fractional scaling
- multi-monitor behavior
- stable GPU initialization and fallback handling

## Build and Optimization Notes

Target-specific CPU optimization may help for local development and benchmarking.

Example:

- `RUSTFLAGS="-C target-cpu=native"`

This is useful for local builds and profiling, but it should not become a release portability assumption.

## MVP Stack Summary

| Layer | Technology | Purpose |
| --- | --- | --- |
| Windowing and shell | `gtk4` | Native Linux shell, dialogs, menus, panels, shortcuts |
| State orchestration | Rust application layer | Session state, commands, jobs, document lifecycle |
| Canvas | `wgpu` | GPU-backed viewport, overlays, zoom/pan, presentation |
| Shaders | WGSL | Blend, overlay, and later preview/filter logic |
| Image utilities | `image` | Import/export codecs and non-critical helpers |
| Background work | Rust threads or job system | Autosave, import/export, thumbnails, heavy operations |

## Final Position

The current chosen direction remains sound:

- GTK4 for the shell
- `wgpu` for the canvas
- Rust-owned document and raster model
- explicit responsiveness boundaries
- no Skia, Halide, or GEGL in MVP

This gives PhotoTux a strong Linux-native architecture without taking on avoidable systems complexity too early.

## Post-MVP PSD Interoperability Direction

PhotoTux should not treat a Rust-native PSD parser as the default path for the first interoperability pass.

Accepted direction:

- use `psd-tools` as a sidecar importer behind the `file_io` boundary
- emit a versioned intermediate manifest plus extracted raster assets
- keep Rust responsible for normalization into the native document model, supported-subset enforcement, fallback rules, and diagnostics
- use the Adobe PSD specification as the standing reference for validating file structure and guiding future PSD support expansion

Rejected or deferred for the current phase:

- `rawpsd` as the primary foundation, because maintenance and fidelity risk are too high for the planned subset
- ImageMagick or GraphicsMagick as the main importer path, because flatten-first behavior is too lossy for reconstructive layered import
- direct Krita or `libkritapsd` integration for now, because the dependency and build surface are too heavy for the current Rust-first implementation path even though the fidelity potential is strong