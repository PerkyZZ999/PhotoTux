# PSD File-Format Expansion Tasks

## Purpose

This task list turns the PSD expansion plan into an implementation sequence focused on PSD import first and PSD export later.

## Principles

- `.ptx` remains the authoritative native format
- PSD support is an adapter concern in `file_io`
- import comes before export
- unsupported features must fail clearly or report partial fidelity explicitly

## Task List

### PSD01 - Choose and validate the PSD parser approach

- [ ] Status: not started
- Outcome: PSD work begins from a known technical path rather than speculative adapter code
- Includes:
  - evaluate library or parser strategy
  - confirm maintenance risk and subset feasibility
  - document accepted and rejected parser options
- Depends on: none
- Done when:
  - a PSD parsing strategy is selected and recorded

### PSD02 - Define the supported first PSD subset

- [ ] Status: not started
- Outcome: PSD scope is explicit and testable before implementation expands
- Includes:
  - supported raster-layer feature matrix
  - supported blend subset mapping
  - fallback and failure rules
  - unsupported feature diagnostics policy
- Depends on: PSD01
- Done when:
  - the first PSD subset is documented precisely enough to write fixtures against it

### PSD03 - Build PSD import normalization layer

- [ ] Status: not started
- Outcome: PSD content can enter the existing document model safely
- Includes:
  - adapter code in `file_io`
  - canvas size normalization
  - layer order, visibility, and opacity mapping
  - supported blend mapping
- Depends on: PSD02
- Done when:
  - supported PSD files import into a native `Document` without ad hoc shell logic

### PSD04 - Add flattened composite fallback rules

- [ ] Status: not started
- Outcome: partially compatible PSDs degrade honestly when necessary
- Includes:
  - flattened fallback behavior where exact structure cannot be preserved
  - explicit marking of fallback imports in diagnostics
- Depends on: PSD03
- Done when:
  - fallback behavior is intentional and tested rather than accidental

### PSD05 - Add import diagnostics model

- [ ] Status: not started
- Outcome: users can tell what was preserved, dropped, or flattened
- Includes:
  - diagnostics structure in the application boundary
  - warnings for unsupported features
  - hard failure rules for unsafe or misleading imports
- Depends on: PSD02
- Done when:
  - PSD import results are transparent rather than silently approximate

### PSD06 - Expose PSD import workflow in the shell

- [ ] Status: not started
- Outcome: PSD import is reachable through normal file workflows
- Includes:
  - open/import command routing
  - UI presentation for import warnings or partial-import notices
  - worker/job integration for PSD import path
- Depends on: PSD03, PSD05
- Done when:
  - a user can import a supported PSD from the shell and understand the result

### PSD07 - Build the supported PSD fixture set

- [ ] Status: not started
- Outcome: import support is anchored to representative cases
- Includes:
  - simple raster-layer PSD fixture
  - layered visibility/opacity fixture
  - supported blend subset fixture
  - flattened fallback fixture
- Depends on: PSD02
- Done when:
  - PSD support has a stable regression fixture set

### PSD08 - Add unsupported-feature diagnostics fixtures

- [ ] Status: not started
- Outcome: unsupported PSD features stay explicit as support expands
- Includes:
  - masks/clipping fixture if unsupported
  - text fixture if unsupported
  - smart object fixture if unsupported
  - CMYK or print-oriented fixture if unsupported
- Depends on: PSD05, PSD07
- Done when:
  - unsupported PSD behavior is covered by tests and messaging expectations

### PSD09 - Validate imported-scene viewport/export parity

- [ ] Status: not started
- Outcome: imported scenes render and export consistently after normalization
- Includes:
  - imported scene flatten parity checks
  - export-versus-viewport validation on imported documents
- Depends on: PSD03, PSD07
- Done when:
  - imported supported PSD scenes behave like native documents after import

### PSD10 - Document PSD import support and limitations

- [ ] Status: not started
- Outcome: compatibility claims remain accurate and supportable
- Includes:
  - supported subset table
  - unsupported-feature list
  - fallback behavior explanation
  - user-facing docs for import expectations
- Depends on: PSD06, PSD08, PSD09
- Done when:
  - the project makes no ambiguous claims about PSD compatibility

### PSD11 - Define PSD export subset and non-goals

- [ ] Status: not started
- Outcome: export scope is bounded before code is written
- Includes:
  - decide whether export is worth shipping in the near term
  - define first export subset if yes
  - document what export will not preserve
- Depends on: PSD10
- Done when:
  - PSD export is either intentionally deferred or clearly scoped

### PSD12 - Implement PSD export subset

- [ ] Status: not started
- Outcome: simple layered raster documents can be handed off to PSD consumers
- Includes:
  - export adapter in `file_io`
  - diagnostics for lossy conversions
  - fixture-based export validation
- Depends on: PSD11
- Done when:
  - the chosen PSD export subset works and its limits are explicit

## Suggested Execution Order

1. PSD01
2. PSD02
3. PSD03
4. PSD04
5. PSD05
6. PSD07
7. PSD08
8. PSD06
9. PSD09
10. PSD10
11. PSD11
12. PSD12

## Notes

- If PSD parsing fidelity is weaker than expected, narrow the subset instead of widening diagnostics to excuse incorrect imports.
- Do not start PSD export before import support is trusted.