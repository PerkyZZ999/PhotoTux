# Post-MVP Plan: PSD File-Format Expansion

## Purpose

Add practical Photoshop interoperability without weakening the rule that `.ptx` remains the authoritative native project format.

## Why This Is A Natural Next Step

The current MVP can already create, save, reopen, import, export, and recover layered raster work. The next major external constraint for adoption is interoperability.

The product and technical docs already define the intended direction:

- PSD import is a post-MVP priority
- PSD import should come before PSD export
- support must be a documented subset
- unsupported features must fail clearly rather than import silently with incorrect output

## Goal

Enable users to bring common layered PSD files into PhotoTux safely, inspect what was preserved, and continue working in `.ptx`.

## Scope

### Phase 1 Scope

- PSD import only
- raster-layer subset only
- support for:
  - canvas size
  - layer order
  - visibility
  - opacity
  - initial blend subset
  - flattened composite fallback when needed

### Deferred Beyond Phase 1

- PSD export
- text fidelity guarantees
- smart objects
- adjustment layers
- advanced masks and clipping parity
- CMYK and print-specific semantics

## Recommended Delivery Order

1. PSD import subset
2. PSD diagnostics and unsupported-feature reporting
3. fixture-driven compatibility pass
4. optional PSD export subset later

## Parser And Importer Strategy

- use a `psd-tools` sidecar importer for the first serious PSD pass instead of owning a Rust-native PSD parser foundation immediately
- keep the sidecar behind the `file_io` boundary so the rest of the application still works only with native PhotoTux document structures
- have the sidecar emit a versioned intermediate manifest plus extracted raster assets rather than letting UI or document crates understand Photoshop-native binary structures directly
- treat the Adobe PSD specification as the standing validation and expansion reference for blend keys, tagged blocks, masks, compression details, and future parser upgrades
- do not depend on an arbitrary system Python installation; the importer runtime should be controlled, bundled, or otherwise managed by PhotoTux

## Work Breakdown

### Phase 1: Import Adapter Foundation

Deliverables:

- bundleable or repo-managed PSD sidecar entrypoint
- versioned intermediate manifest for PSD scene description
- PSD adapter layer in `file_io`
- normalized import into existing `doc_model` structures
- mapping for the supported initial blend subset
- flatten fallback behavior when exact fidelity is not available but import can still remain truthful

Key design rules:

- PSD support is an adapter concern, not a new source-of-truth format
- the parser runtime must be replaceable without changing `doc_model`, `ui_shell`, or the `.ptx` format
- imported content must be normalized into the existing raster document model
- if fidelity is unknown, import must report a warning or fail clearly

Exit criteria:

- a supported PSD subset imports into a trusted `.ptx` editing session without silent corruption

### Phase 2: Diagnostics And Reporting

Deliverables:

- unsupported-feature reporting model
- user-facing import summary or diagnostics surface
- tests for partial-import and unsupported-feature paths

Key design rules:

- no false claims of Photoshop parity
- diagnostics must be explicit about what was preserved, flattened, ignored, or rejected

Exit criteria:

- users can tell whether a PSD imported faithfully enough to continue working

### Phase 3: Compatibility Fixture Set

Deliverables:

- a small PSD fixture collection representing the supported subset
- regression tests for supported and unsupported feature cases
- export-versus-viewport parity checks after import normalization

Key design rules:

- use fixed representative PSD scenes rather than ad hoc one-off samples
- do not widen support faster than fixtures can prove it

Exit criteria:

- PSD import behavior is stable enough to evolve intentionally rather than opportunistically

### Phase 4: Optional PSD Export Subset

Deliverables:

- documented subset for PSD export
- explicit unsupported-feature boundaries
- export diagnostics for lossy semantic conversions

Key design rules:

- PSD export should never suggest stronger compatibility than actually exists
- exporting to PSD must not become a second authoritative project path inside the app

Exit criteria:

- users can hand off simple layered raster documents to PSD consumers with clearly documented limits

## Main Risks

- PSD complexity can sprawl quickly if the subset is not held tightly
- partial import can create user trust problems if diagnostics are weak
- blend, mask, and clipping semantics can diverge subtly from Photoshop behavior
- sidecar packaging and invocation can become fragile if the importer runtime is treated as an external system prerequisite instead of a managed project dependency

## Validation Requirements

- supported PSD subset fixtures
- unsupported-feature diagnostics fixtures
- blend-mode mapping tests
- import normalization tests into the native document model
- export-versus-viewport checks after import

## Success Condition

PhotoTux can ingest a useful PSD subset honestly and safely, increasing real-world interoperability without compromising native-format clarity.