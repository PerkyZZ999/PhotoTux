# Post-MVP Plan: Painting And Input Expressiveness

## Purpose

Improve the feel and capability of painting workflows for users who need more expressive mark-making than the MVP brush foundation currently provides.

## Why This Is A Natural Next Step

The MVP already proves the paint path and editing architecture. The next quality jump for illustration-oriented workflows comes from richer input and brush behavior rather than basic correctness.

The product docs already leave room for:

- advanced tablet support beyond MVP
- tablet pressure support
- destructive filters as a post-MVP candidate

## Goal

Make painting feel more direct and capable while keeping the low-latency interaction rules intact.

## Scope

### In Scope

- tablet pressure support
- richer brush dynamics
- spacing and flow improvements
- stronger direct-manipulation feedback
- early destructive filter groundwork where it supports raster workflows directly

### Explicitly Out Of Scope

- non-destructive adjustment layers
- full filter ecosystem
- brush systems that require a different core raster architecture immediately

## Recommended Delivery Order

1. tablet pressure support
2. brush dynamics and parameter expansion
3. latency and preview polish
4. destructive filter foundation

## Work Breakdown

### Phase 1: Pressure Support

Deliverables:

- stylus-pressure input path where platform support is practical
- mapping of pressure to size, opacity, or flow
- fallback behavior for mouse input

Key design rules:

- pressure support must not degrade mouse behavior
- keep input handling decoupled from the document model

Exit criteria:

- pressure-sensitive devices produce stable and useful variation during brush strokes

### Phase 2: Brush Dynamics

Deliverables:

- parameter expansion for flow, spacing, and hardness behavior
- better stroke interpolation controls
- stronger brush preset structure if needed

Key design rules:

- preserve stroke grouping in history
- avoid hidden parameter interactions that make regression testing harder

Exit criteria:

- brush behavior becomes noticeably more expressive without compromising predictability

### Phase 3: Preview And Latency Polish

Deliverables:

- improved direct-manipulation preview
- paint-path profiling and optimization where needed
- regression checks for latency-sensitive brush paths

Exit criteria:

- richer brush behavior does not degrade the current responsiveness standard

### Phase 4: Destructive Filter Groundwork

Deliverables:

- command model for applying destructive raster operations
- history integration for destructive changes
- limited initial filter set if desired

Key design rules:

- keep filter execution off the UI path
- ensure save/load and undo/redo remain trustworthy after filter operations

Exit criteria:

- a small number of destructive operations can be applied safely and predictably

## Main Risks

- input-platform differences can make pressure support unstable across environments
- richer brushes can regress latency quickly if not profiled carefully
- filter work can accidentally start a much larger image-processing track than intended

## Validation Requirements

- manual validation on supported tablet hardware where available
- latency checks for common brush settings
- undo/redo coverage for expanded brush and filter operations

## Success Condition

PhotoTux becomes materially better for painting-heavy workflows without violating the responsiveness model that the MVP established.