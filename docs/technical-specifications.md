# Technical Specifications

## Project Name
**Project Codename:** PhotoTux
**Stack:** Rust + GTK4 + wgpu
**Primary Target:** Linux desktop
**Primary Environment:** KDE Plasma on Wayland
**Architecture Style:** Modular Rust workspace
**Status:** Draft v2

---

## 1. Technical Goals

- Build a high-performance raster graphics editor optimized for Linux design workflows.
- Use GPU acceleration for viewport composition and presentation.
- Keep the document model independent from UI and GPU state.
- Deliver a strong editing core before advanced features.
- Keep the architecture compatible with later masks, text, filters, plugins, and scripting.

---

## 2. Product Scope Assumptions

Technical choices in this document assume the following:
- this is not a Camera RAW or photo-development application
- this is not a print-first or CMYK-first product
- early document editing is raster-only
- initial priority is design, compositing, painting, and general graphics work
- exact Photoshop parity is not a requirement
- limited PSD interoperability is required, but only for a clearly defined subset

These constraints are deliberate and should influence every early implementation choice.

---

## 3. Chosen Stack

### Core Language
- Rust

### UI Layer
- GTK4 via `gtk4-rs` for application chrome, panels, toolbars, dialogs, menus, and platform integration
- `relm4` is optional for shell state organization, not a locked architectural requirement

### Canvas and Rendering
- wgpu for custom canvas rendering, layer composition, overlays, and viewport presentation
- the live viewport path must target GPU-backed presentation even while some edit operations remain CPU-authored in early milestones
- raw Vulkan is deferred unless a later measured bottleneck proves `wgpu` insufficient

### Shading Language
- WGSL

### Image Processing
- CPU-side raster editing for early tools and simple operations
- GPU-side composition and viewport display
- selected GPU acceleration later where it materially improves user-facing latency
- the `image` crate is primarily for import, export, and utility operations rather than the core hot editing path

### Serialization
- serde for manifests, preferences, and metadata
- versioned container format for native project files

### Background Work
- use a lightweight job system for save/load, autosave, import/export, thumbnails, heavy transforms, and other long-running work
- keep the UI thread focused on widgets, command dispatch, and presentation state
- avoid introducing tokio unless the workload clearly justifies it

### Logging and Errors
- tracing
- thiserror
- anyhow at application boundaries only

### Testing
- cargo test
- image snapshot and image-diff testing
- integration tests for save/load/history correctness

---

## 4. Hard Architectural Decisions

The following decisions are locked for the first implementation phase.

### Source of Truth
- The document model is the source of truth.
- GPU resources are caches and presentation artifacts only.
- UI state must not own document state.

### Rendering Boundary
- GTK4 owns the app shell and platform integration.
- The canvas is a custom rendering surface driven by wgpu.
- Canvas rendering and panel UI must remain loosely coupled.
- GTK must not become the primary canvas rasterization path.
- direct low-level surface interop is a feasibility tool, not a baseline requirement for the first implementation.

### Layout Boundary
- Start with a fixed professional layout.
- Full dockable panels are deferred until the editing core is stable.

### Layer Boundary
- MVP started with raster layers only.
- The current codebase now includes raster layers, embedded per-layer masks, hierarchical groups, and document-owned text layers.
- Adjustment layers remain deferred.

### Scope Boundary
- Do not implement Camera RAW, photographic development tools, or library-management features.

---

## 5. Workspace Structure

```text
phototux/
├─ Cargo.toml
├─ crates/
│  ├─ app_core/
│  ├─ ui_shell/
│  ├─ doc_model/
│  ├─ render_wgpu/
│  ├─ tool_system/
│  ├─ history_engine/
│  ├─ file_io/
│  ├─ image_ops/
│  ├─ color_math/
│  └─ common/
├─ assets/
│  ├─ icons/
│  ├─ themes/
│  └─ shaders/
├─ docs/
└─ tests/
```

### Crate Responsibilities

#### `app_core`
- application lifecycle
- command dispatch
- current document session
- startup and shutdown
- preferences and autosave coordination

#### `ui_shell`
- GTK4 widgets and shell composition
- fixed panel layout
- command routing to application services
- menus, dialogs, status surfaces

#### `doc_model`
- document data model
- raster layers
- embedded masks and layer hierarchy
- document-owned text layers
- selection state
- guides
- transforms
- metadata
- stable IDs

#### `render_wgpu`
- GPU device and surface setup
- texture cache management
- compositing pipelines
- overlay rendering
- viewport presentation

#### `tool_system`
- input-to-tool dispatch
- brush tool
- eraser tool
- move tool
- rectangular and lasso selection tools
- pressure-aware brush sampling
- text placement and text-move interaction
- transform preview logic

#### `history_engine`
- undo and redo stacks
- command recording
- operation grouping
- tile snapshot bookkeeping

#### `file_io`
- importers
- export pipeline
- native project format read and write
- autosave and recovery files
- limited sidecar-backed PSD import and diagnostics

#### `image_ops`
- CPU raster operations
- tile copy and blend helpers
- selection raster operations
- initial destructive filters

#### `color_math`
- color conversion helpers
- blend mode math
- sRGB and linear conversions
- alpha utilities

#### `common`
- geometry types
- IDs
- shared errors
- common traits and constants

---

## 6. Rendering and Color Rules

These rules must be implemented consistently from the first render path.

### Storage Format for MVP
- Store raster layer pixels as 8-bit RGBA.
- Use sRGB-tagged content for imported and authored raster data.
- Internally represent runtime composition inputs in a way that supports conversion to linear space for blending where required.

### Working Assumptions
- Imported standard images are assumed to be sRGB unless metadata support is added later.
- The MVP does not support high-bit-depth editing or full color management.
- CMYK is out of scope.

### Alpha Rules
- The renderer uses premultiplied alpha for composition.
- File import and export must convert correctly at the boundaries.
- Alpha behavior must be documented and tested to avoid fringe artifacts.

### Blend Rules
- Define a small initial blend mode set only.
- Initial set: Normal, Multiply, Screen, Overlay, Darken, Lighten.
- Blend mode behavior must match documented math and snapshot tests.

### Resampling Rules
- Navigation can use quality/performance-biased sampling.
- committed transforms must use a deterministic resampling strategy
- nearest-neighbor mode can be a later feature, not an MVP requirement

### Rationale
- This is enough to build a trustworthy design-oriented editor without taking on the complexity of full color management too early.

---

## 7. Runtime Raster Model

### Internal Surface Strategy
- Use tiled raster storage for runtime editing.
- Initial tile size: 256 x 256 pixels.
- Track dirty tiles explicitly for painting, erase, and region updates.

### Why Tiles
- They cap memory churn during local edits.
- They make partial redraw practical.
- They improve raster-edit undo granularity.
- They allow GPU uploads to stay scoped to touched regions instead of rebuilding the full viewport texture.

### Early Constraint
- The save format does not need to mirror the runtime tile structure in v1.
- Runtime efficiency and file simplicity should be optimized independently.

---

## 8. High-Level Architecture

The application is divided into the following major layers:

1. **UI Layer**
   - GTK4 application shell
   - toolbars, panels, dialogs, menus
   - status surfaces and input routing

2. **Application Layer**
   - command dispatch
   - document lifecycle
   - tool activation
   - autosave and background work coordination

3. **Document Engine**
   - layered raster model
   - selection state
   - transform state
   - metadata and serialization-facing structures

4. **Render Engine**
   - tile upload scheduling
   - visible-layer preparation
   - GPU composition
   - overlay rendering
   - viewport presentation

5. **Tool Engine**
   - brush and eraser stroke handling
   - move and selection interactions
   - transform preview logic

6. **Persistence Layer**
   - project save and load
   - autosave
   - recovery handling
   - import and export adapters

7. **Job System**
   - background execution for long-running work
   - task prioritization for user-visible operations
   - safe result delivery back to the shell and session state

---

## 9. Rendering Model

### Core Strategy
Use a tile-aware composition pipeline with dirty-tile invalidation and an offscreen viewport composition target.

### Render Stages
1. Resolve visible document state.
2. Determine dirty tiles or dirty regions.
3. Upload changed tile content to GPU caches.
4. Composite visible layers into a viewport target.
5. Draw overlays such as selection outlines, transform bounds, and brush previews.
6. Present the final image.

### Requirements
- avoid full-document redraw when only a small area changes
- keep document-space rendering separate from overlay rendering
- support smooth zoom and pan
- keep GPU uploads bounded during brush work

### Non-Requirement for Early Phases
- Do not introduce compute-heavy GPU filters until the core viewport path is stable.
- Do not add Skia as a second rendering system unless a concrete later use case justifies the extra integration cost.

---

## 10. Document Model

### Core Entities
- `Document`
- `Canvas`
- `RasterLayer`
- `SelectionMask`
- `ViewportState`
- `HistoryEntry`
- `DocumentMetadata`

### Initial Layer Properties
- stable UUID
- name
- visibility
- opacity
- blend mode
- lock state later if needed
- bounds
- tile storage reference
- thumbnail reference

### Deferred Entities
- `TextLayer`
- `AdjustmentLayer`

### Post-MVP Document Extensions
- `Mask` is now modeled as an optional embedded alpha-mask payload on each raster layer.
- `LayerGroup` now exists as a document-owned hierarchy node for post-MVP structural workflows, with recursive `.ptx` hierarchy persistence and inherited visibility and opacity propagation during grouped flattening.
- selection state now supports both rectangular and freeform polygon-backed shapes in the document model, with shared bounds queries, hit testing, and invert semantics.
- lasso interaction now flows through generalized selection history records, and the viewport overlay path supports both rectangular and polyline selection visualization.
- selected-area move and transform behavior now operate on the active selection instead of always applying to the full layer, so freeform selection affects direct manipulation workflows consistently with brush and mask edits.
- transform workflows now support independent X/Y scale and quarter-turn rotation in addition to translation, while preserving the existing preview-versus-commit boundary and undoable layer-state history model.
- guide state now lives in the document model as persisted horizontal and vertical guides with visibility state, and the shell renders them through viewport overlays instead of baking them into raster content or export output.
- move and transform workflows now support guide snapping through controller-owned snapping state, with a persistent snap toggle and a temporary Shift bypass for direct-manipulation interactions.
- the paint interaction path now carries normalized stylus pressure from `ui_shell` through `app_core` into `tool_system` sample interpolation, while preserving mouse fallback behavior as pressure `1.0`.
- initial pressure support now includes controller-owned pressure-to-size and pressure-to-opacity toggles, with pressure applied at the per-dab brush-evaluation seam instead of in shell state or raster tile ownership.
- brush dynamics now use controller-owned radius, hardness, spacing, and flow state, with validated ranges in `tool_system`, smoother soft-edge hardness falloff in `image_ops`, and shell-exposed parameter controls through the existing Properties panel.
- brush hover preview now stays shell-local in `ui_shell`, where GTK motion and stylus updates drive overlay-only cursor feedback for radius, hardness, spacing, and pressure-sized previews without mutating document state or canvas revision on hover.
- built-in brush presets now live in the controller layer as named parameter bundles surfaced through the shell snapshot; because they are shipped defaults rather than user-authored assets, they currently do not participate in document persistence.
- destructive filters now route through `app_core` as worker-backed commands with revision-guarded completion, snapshot-based undo/redo, and an intentionally narrow initial scope of active-layer pixel operations only.
- text layers now exist as document-owned headless entities with editable content, single-style formatting, blend/visibility metadata, and a document-owned placement transform instead of shell-owned transient text state.
- `.ptx` project format version 2 stores editable text layers inline in the manifest while keeping raster payload blobs reserved for raster layers and masks.
- initial text rendering now uses a shared bitmap-font rasterization path in `file_io`, so viewport rendering, committed preview, and raster export all flow through the same compositing boundary instead of GTK-owned text widgets.
- the initial text release supports one style run per text layer with `Bitmap Sans`-style bitmap glyph rendering, fill RGBA, font size, line height, letter spacing, left/center/right alignment, blend mode, opacity, visibility, and document-owned placement transforms.
- explicit non-goals for the initial text release are system font loading, OpenType shaping, kerning pairs, rich text spans, text on path, paragraph layout beyond simple line splitting, IME composition, and other advanced typography features that would require a broader font/layout engine.

### PSD Mapping Principle
PSD interoperability must map into the PhotoTux document model without changing the internal architecture to mirror Photoshop-specific internals.

### Initial PSD Parser Strategy
- the initial PSD import path should use a dedicated `psd-tools` sidecar importer rather than a Rust-native parser foundation
- the sidecar must emit a versioned intermediate manifest plus extracted raster assets, with normalization, subset enforcement, and fallback policy remaining in `file_io` so `.ptx` stays the only authoritative project format
- the Adobe PSD specification is a standing reference for validating file structure, blend keys, tagged blocks, masks, and future expansion decisions, but it is not the sole implementation strategy
- the importer boundary must remain swappable and must not require `doc_model` or `ui_shell` to understand Photoshop-native binary structures
- PhotoTux should control the importer runtime rather than assume a user-managed system Python environment

### Initial PSD Sidecar Manifest Contract
The first PSD sidecar contract should be a JSON manifest plus sibling raster assets written into a temporary import workspace.

Example shape:

```json
{
   "manifest_version": 1,
   "source_kind": "psd",
   "source_color_mode": "rgb",
   "source_depth_bits": 8,
   "canvas": {
      "width_px": 1920,
      "height_px": 1080
   },
   "composite": {
      "available": true,
      "asset_relpath": "composite.png"
   },
   "diagnostics": [
      {
         "severity": "warning",
         "code": "unsupported_text_layer",
         "message": "Text layer will require flattened fallback.",
         "source_index": 4
      }
   ],
   "layers": [
      {
         "source_index": 0,
         "kind": "raster",
         "name": "Background",
         "visible": true,
         "opacity_0_255": 255,
         "blend_key": "norm",
         "offset_px": { "x": 0, "y": 0 },
         "bounds_px": { "left": 0, "top": 0, "width": 1920, "height": 1080 },
         "raster_asset_relpath": "layers/000-background.png",
         "unsupported_features": []
      }
   ]
}
```

Contract rules:

- asset paths are relative to the manifest directory so the temp workspace can be moved or cleaned as one unit
- the sidecar reports source facts and extracted assets; `file_io` owns final subset enforcement, blend mapping into PhotoTux enums, fallback selection, and hard-failure decisions
- every layer entry must include a structural `kind` so unsupported entries such as text, smart object, adjustment, group, or clipped layers can be diagnosed explicitly instead of disappearing silently
- `composite.available = true` is required before `file_io` may choose a warned flattened fallback import
- unknown manifest versions must fail clearly rather than being parsed speculatively
- the sidecar must never write `.ptx` directly or bypass the native import normalization path

### Rule
The document model must be testable without initializing the UI or GPU.

---

## 11. History System

### Chosen Direction
Use a hybrid history model.

### Rules
- structural operations use command-style undo entries
- raster edits use tile snapshots or tile deltas
- brush strokes are grouped into a single committed action
- interactive transforms preview live but commit once

### Why Hybrid
- pure command undo is too complex for raster edits
- full-layer snapshots are too memory-heavy
- tile-level history gives a workable middle ground for a solo MVP

### Memory Rules
- enforce an undo memory budget
- evict oldest history safely when necessary
- surface this behavior clearly in debug logs

---

## 12. Brush Engine

### Initial Features
- size
- hardness
- opacity
- flow
- spacing
- round tip
- normalized pressure-to-size and pressure-to-opacity mapping
- live hover preview for brush and eraser cursors
- built-in brush presets for fast return to common setups
- minimal destructive filters: invert colors and desaturate

### Stroke Pipeline
1. collect pointer samples
2. smooth or interpolate the stroke path
3. generate dabs
4. rasterize affected tiles on CPU
5. mark dirty tiles
6. commit the stroke as one history action

### Performance Requirements
- visible stroke feedback must feel immediate
- large brushes should degrade gracefully rather than stall
- UI thread work during strokes must stay minimal

### Interaction Rule
- direct-manipulation paths such as brush preview, pan, zoom, selection drag, and transform preview must favor low-latency incremental updates over heavyweight background scheduling

### Deferred Features
- tilt
- textured brushes
- scatter and dynamics
- advanced pressure curves and preset libraries
- broader filter families, selection-aware filter previews, and non-destructive adjustment stacks

---

## 13. Selection and Transform System

### MVP Selection Scope
- rectangular marquee
- select all
- clear selection
- invert selection

### Representation
- store selection as a raster mask aligned with document space
- tile-aware storage is preferred for consistency with editing surfaces

### Transform Scope
- start with translate and scale
- add rotation only when the preview and commit path are correct
- complex warp and perspective are later features

### Principle
Fewer transform modes implemented correctly are better than a wide but unstable transform feature set.

---

## 14. Native File Format

### Chosen Direction
Use a versioned container format with a manifest and per-layer blobs.

### Suggested Extension
`.ptx`

### Container Layout
- `manifest.json`
- `layers/<id>.png` for MVP layer raster payloads
- `thumb/preview.png`
- `meta/app.json`

### Manifest Requirements
- format version
- canvas size
- document metadata
- layer ordering
- per-layer visibility, opacity, blend mode, and names
- references to payload files

### Save Strategy
- write to a temporary file
- fsync where practical
- rename atomically on success
- preserve the previous file if save fails

### Recovery Strategy
- autosave uses separate recovery files
- recovery files must never overwrite the primary document silently

### Rationale
- zip-like containers are easier to inspect, debug, and migrate than a fully custom binary format this early
- runtime tile storage can remain an internal detail

### Interoperability Rule
PSD support must remain an import and export adapter concern. The native `.ptx` format remains the only authoritative project format.

---

## 15. Import and Export

### Early Import
- PNG
- JPEG
- WebP

### Early Export
- PNG
- JPEG
- WebP

### Rules
- imported pixel data is normalized into the document's raster representation
- export must match the visible flattened composite
- mismatches between viewport and export are release-blocking bugs

### Deferred Support
- TIFF
- layered interchange formats

### Planned PSD Support
- PSD import is a post-MVP priority.
- PSD export is a later expansion item.
- Support is limited to a documented subset of Photoshop features.
- Unsupported features must surface warnings or partial-import diagnostics.

### Expected First PSD Subset
- RGB 8-bit raster layers imported into PhotoTux's native document model
- top-level layer order, names, visibility, opacity, and offsets
- one-to-one blend mapping for `Normal`, `Multiply`, `Screen`, `Overlay`, `Darken`, and `Lighten`
- canvas size preservation
- flattened composite fallback when unsupported PSD structure is present but a truthful composite image is available

### Explicitly Unsupported in Early PSD Support
- smart objects
- adjustment layers
- text fidelity guarantees
- layer effects fidelity guarantees
- advanced masks and clipping behavior with exact parity
- CMYK and print-oriented semantics
- silent coercion of unsupported blend modes into misleading PhotoTux equivalents
- silent preservation claims for unsupported hierarchy or structure

### Early PSD Import Result Rules
- if the PSD stays inside the documented first subset, PhotoTux should import it as editable native layers
- if the PSD exceeds that subset but exposes a valid flattened composite that is more truthful than partial structural import, PhotoTux may import the composite with explicit diagnostics
- if neither faithful layered import nor truthful composite fallback is possible, the import must fail clearly

---

## 16. UI Specification

### Main Regions
- left toolbar
- top options strip
- center canvas viewport
- right layer and properties panels
- bottom status bar

### Core Panels for MVP
- Layers
- Properties
- Color
- History

### UX Rules
- keyboard-first interaction
- contextual tool options
- canvas remains the visual focus
- no dockable framework in MVP
- minimal chrome and restrained iconography
- native Linux behavior is preferred over deep custom shell chrome in early milestones

---

## 17. Input and Platform System

### Inputs
- mouse
- keyboard
- trackpad gestures where practical
- stylus later

### Command Model
Map raw input into:
- viewport commands
- tool commands
- document commands
- UI commands

### Linux and Wayland Requirements
- reliable pointer capture during drag operations
- correct coordinate handling under fractional scaling
- acceptable behavior across multi-monitor setups
- native-enough file dialog behavior
- correct clipboard integration for image data later
- stable GPU initialization and fallback handling

### Deferred Platform Features
- advanced tablet integration
- drag-and-drop import polish
- IME and text editing concerns until text layers exist

---

## 18. Concurrency and Responsiveness Model

### Execution Classes
1. UI thread for GTK widgets, menus, shortcuts, shell state, and command dispatch.
2. Render path for viewport presentation, overlays, and immediate interaction feedback.
3. Worker jobs for file IO, serialization, autosave, imports, exports, thumbnails, heavy resampling, and later filters.

### Rules
- the UI thread must never block on long-running file or image operations
- interactive viewport feedback must remain incremental and cancelable where practical
- worker results must be applied on the document/session boundary, never by mutating UI-owned state directly
- prioritise user-visible tasks over background maintenance work

### Rationale
- a responsive editor depends on clear separation between shell work, rendering, and heavyweight processing
- this model preserves responsiveness without forcing the entire application into an async runtime

---

## 19. Performance Strategy

### Core Strategies
- dirty-tile invalidation
- partial GPU uploads
- cached thumbnails
- bounded undo memory
- background autosave

### Initial Profiling Targets
- brush latency under typical brush sizes
- smooth zoom and pan under normal document loads
- layer composition cost as layers increase
- save and load latency
- memory cost of long edit sessions

### Important Constraint
The UI thread must never block on expensive file IO or long-running image operations.

---

## 20. Error Handling and Recovery

### Requirements
- actionable import and export errors
- autosave after meaningful state changes or time intervals
- crash recovery detection on startup
- atomic primary save behavior where practical
- useful development logs for failures in rendering, save, load, and recovery

### Logging Rules
- structured logs in development
- concise user-facing errors in production UI
- diagnostics should include document size, layer count, and operation type where useful

---

## 21. Testing Strategy

### Unit Tests
- geometry
- blend math
- color conversion helpers
- document operations
- selection mask logic
- history bookkeeping

### Integration Tests
- native save and load roundtrip
- export correctness against reference scenes
- layer reorder correctness
- undo and redo consistency
- autosave and recovery behavior
- PSD import scene compatibility tests once PSD support begins
- PSD diagnostic coverage for unsupported features once PSD support begins

### Visual Regression Tests
- blend modes
- alpha edges
- selection overlays
- transformed layer output
- PSD compatibility fixtures once PSD support begins

### Stress Tests
- large canvases
- many layers
- repeated brush strokes
- long undo chains
- repeated autosaves

---

## 22. Packaging and Distribution

### Primary Targets
- native Linux development build
- Arch-oriented packaging later
- AppImage later
- Flatpak later

### Build Requirements
- reproducible release builds where practical
- CI for fmt, clippy, test, and release build validation
- release profiles tuned for performance and binary size tradeoffs

---

## 23. Technical Roadmap

### Tech Milestone A - Feasibility
- workspace created
- GTK4 shell renders
- custom canvas surface works
- zoom and pan behave correctly
- single-layer paint path works

### Tech Milestone B - Document Core
- raster document model
- layer operations
- native project manifest and payload writing
- undo and redo foundation

### Tech Milestone C - Viewport and Composition
- GPU composition path
- dirty-tile uploads
- overlay rendering
- image import path

### Tech Milestone D - Usable MVP
- multi-layer workflows
- rectangular selection
- move and simple transform
- export validation
- autosave and recovery

### Tech Milestone E - Workflow Upgrade
- masks
- groups
- lasso selection
- more blend modes
- tablet extensions
- limited PSD import

### Tech Milestone F - Interoperability Expansion
- broader PSD import coverage
- limited PSD export
- compatibility diagnostics and warnings

---

## 24. Definition of Done for MVP

The MVP is complete when:
- a user can create and save a layered document
- raster layers can be added, reordered, hidden, and edited
- painting and erasing feel responsive
- rectangular selection and simple transforms work reliably
- undo and redo are trustworthy
- autosave and crash recovery function correctly
- export matches the visible composite
- a real design or compositing task can be completed without major instability
