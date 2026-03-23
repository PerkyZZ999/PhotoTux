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

- [x] Status: completed
- Outcome: PSD work begins from a known technical path rather than speculative adapter code
- Includes:
  - evaluate library or parser strategy
  - confirm maintenance risk and subset feasibility
  - document accepted and rejected parser options
- Depends on: none
- Done when:
  - a PSD parsing strategy is selected and recorded

Progress notes:
- the selected primary parser strategy is a `psd-tools` sidecar importer that runs outside the Rust workspace and feeds `file_io` a versioned intermediate manifest plus extracted raster assets for normalization into `doc_model`.
- `psd-tools` is the best current fit for the documented import-first adapter boundary because it gives mature layered PSD inspection without forcing PhotoTux to own a full binary PSD parser in Rust before the supported subset is proven.
- the Adobe PSD specification remains a standing reference for file-structure validation, blend-key interpretation, and future unsupported-feature handling, but it is not treated as a reason to maintain a low-level Rust parser from scratch in the first implementation pass.
- `rawpsd` 0.2.2 is no longer the primary foundation; the maintenance and fidelity risk are too high for the intended interoperability scope, even though its low-level visibility was initially attractive.
- direct Krita or `libkritapsd` integration remains a later reference path rather than the current dependency strategy, because the fidelity upside does not yet justify pulling a large C++/Qt/KDE surface into the Rust-first import path.
- GraphicsMagick or ImageMagick style flatten-first importers were rejected as the primary path because they are useful for preview generation but too lossy for reconstructive layered import.

### PSD02 - Define the supported first PSD subset

- [x] Status: completed
- Outcome: PSD scope is explicit and testable before implementation expands
- Includes:
  - supported raster-layer feature matrix
  - supported blend subset mapping
  - fallback and failure rules
  - unsupported feature diagnostics policy
- Depends on: PSD01
- Done when:
  - the first PSD subset is documented precisely enough to write fixtures against it

Progress notes:
- the first faithful-import subset is intentionally tight: RGB 8-bit layered PSD documents whose usable content can be normalized into top-level raster layers with preserved canvas size, layer order, layer names, visibility, opacity, offsets, and a limited blend subset.
- the initial one-to-one blend subset is `Normal`, `Multiply`, `Screen`, `Overlay`, `Darken`, and `Lighten`, because those already exist in PhotoTux and can be tested without introducing silent semantic remapping.
- unsupported structural or semantic features are not silently approximated: text layers, smart objects, adjustment layers, clipping masks, advanced mask semantics, unsupported blend modes, and non-RGB or print-oriented modes must either fall back to a clearly warned flattened composite import or fail outright if a truthful fallback is not available.
- the fallback rule for `PSD03` and `PSD04` is now explicit: if the source PSD exceeds the supported subset but exposes a valid flattened composite that is more honest than partial structural import, PhotoTux may import that composite as a single raster result with diagnostics instead of pretending unsupported structure survived.
- hard-failure rules are also explicit for the first pass: malformed PSD data, unreadable channel payloads, unsupported color modes without a truthful composite fallback, or any case where the import path cannot explain what was preserved versus dropped should stop the import instead of degrading silently.

### PSD03 - Build PSD import normalization and sidecar boundary

- [x] Status: completed
- Outcome: PSD content can enter the existing document model safely
- Includes:
  - adapter code in `file_io`
  - sidecar invocation contract and manifest versioning
  - canvas size normalization
  - layer order, visibility, and opacity mapping
  - supported blend mapping
  - extracted raster asset ingestion
- Depends on: PSD02
- Done when:
  - supported PSD files import into a native `Document` from a versioned intermediate manifest without ad hoc shell logic
  - the PSD importer boundary stays swappable without changing `doc_model` or `ui_shell`

Progress notes:
- PSD03 will consume a versioned intermediate manifest emitted by the `psd-tools` sidecar instead of parsing PSD binaries directly inside Rust application code.
- the manifest must at minimum describe document size, color-mode/depth summary, flattened-composite availability, import diagnostics, and ordered layer entries.
- each layer entry must carry an import index, structural kind, display name, visibility, opacity, offsets, source blend key, unsupported-feature markers, and any raster asset reference needed for the extracted pixel payload.
- unsupported structure must also be surfaced in the manifest so `file_io` can decide between faithful layer import, warned flattened fallback, or hard failure according to `PSD02` rules.
- the importer runtime must be controlled by PhotoTux rather than assumed from an arbitrary system Python installation, so packaging and invocation are part of the boundary design rather than an afterthought.
- `file_io` now has the first Rust-side PSD03 foundation in place: versioned `PsdImportManifest` and `PsdImportResult` types, manifest loading, supported-subset validation, layered normalization into `Document`, and flattened composite fallback import.
- `file_io` now also has a real sidecar runtime seam: `PsdImportSidecar`, temporary import-workspace creation, subprocess invocation, manifest handoff, and automatic cleanup on both success and failure.
- focused `cargo test -p file_io` coverage now exercises supported layered import, unsupported-structure fallback to composite, no-fallback failure for unsupported color modes, manifest-version rejection, sidecar-driven import success, and sidecar-failure cleanup behavior.
- `app_core` now routes PSD imports through the existing worker-job path instead of rejecting `.psd` files up front, and it passes the configured `PsdImportSidecar` into background import jobs so PSD loading stays off the UI path.
- `ui_shell` now exposes PSD selection through the normal import dialog by including `*.psd` in the supported import filter, so the new PSD path is reachable through the existing shell workflow when a sidecar is configured.
- the repository now includes a repo-managed `psd-tools` sidecar entrypoint under `tools/psd_import_sidecar`, and the current implementation satisfies the PSD03 done criteria even though release bundling remains a separate future packaging concern.

### PSD04 - Add flattened composite fallback rules

- [x] Status: completed
- Outcome: partially compatible PSDs degrade honestly when necessary
- Includes:
  - flattened fallback behavior where exact structure cannot be preserved
  - explicit marking of fallback imports in diagnostics
- Depends on: PSD03
- Done when:
  - fallback behavior is intentional and tested rather than accidental

Progress notes:
- `file_io` now makes the layered-versus-flattened decision intentionally: supported manifests normalize into editable native layers, unsupported manifests with a truthful composite fall back to a single flattened import, and unsupported manifests without a truthful fallback fail clearly.
- fallback imports are no longer accidental side effects of missing parser coverage; they are driven by the documented subset checks in `collect_psd_layered_import_limitations` plus the explicit `flattened_fallback_used` diagnostic.
- the fallback path is covered both by manifest-shaped tests and by real repo PSD fixtures for grouped structure, CMYK sources, text layers, smart objects, clipping, and masks.

### PSD05 - Add import diagnostics model

- [x] Status: completed
- Outcome: users can tell what was preserved, dropped, or flattened
- Includes:
  - diagnostics structure in the application boundary
  - warnings for unsupported features
  - hard failure rules for unsafe or misleading imports
- Depends on: PSD02
- Done when:
  - PSD import results are transparent rather than silently approximate

Progress notes:
- `file_io` now owns a structured `PsdImportDiagnostic` model with severity, code, message, and optional source-layer index, and that model survives both faithful layered imports and flattened fallback imports.
- `app_core` now projects PSD fallback and warning diagnostics into a shell-facing import report instead of collapsing everything into plain status text.
- `ui_shell` now presents that report once after import completion, so unsupported PSD behavior is explicit at the application boundary rather than being buried inside lower-level logs or tests.

### PSD06 - Expose PSD import workflow in the shell

- [x] Status: completed
- Outcome: PSD import is reachable through normal file workflows
- Includes:
  - open/import command routing
  - UI presentation for import warnings or partial-import notices
  - worker/job integration for PSD import path
- Depends on: PSD03, PSD05
- Done when:
  - a user can import a supported PSD from the shell and understand the result

Progress notes:
- the existing shell import flow now accepts `*.psd` selections, and `app_core` routes those imports through the same background job path used for other file operations instead of blocking on the UI thread.
- current user-facing PSD feedback is intentionally narrow: missing-sidecar configuration fails clearly, and successful flattened-fallback imports append a short fallback notice to the status message.
- `app_core` now keeps a structured PSD import report in the shell snapshot when a PSD import falls back or surfaces non-info diagnostics, and `ui_shell` presents that report once through a native info dialog after the background import completes.
- repo-fixture controller coverage now exercises that PSD06 shell-path contract against the real sidecar and checked-in PSDs: a supported layered PSD import updates controller state through background jobs without surfacing a warning report, while the CMYK fallback fixture surfaces the expected flattened-import report and history entry at the application boundary.
- richer diagnostics presentation can still evolve later, but the current workflow already satisfies the PSD06 done criteria: supported PSDs are reachable from the shell and fallback imports are understandable from the shell surface.

### PSD07 - Build the supported PSD fixture set

- [x] Status: completed
- Outcome: import support is anchored to representative cases
- Includes:
  - simple raster-layer PSD fixture
  - layered visibility/opacity fixture
  - supported blend subset fixture
  - flattened fallback fixture
- Depends on: PSD02
- Done when:
  - PSD support has a stable regression fixture set

Progress notes:
- the repository now has a first repo-owned PSD fixture corpus under `tests/fixtures/psd/` generated from source code in the repository rather than copied from third-party PSD samples.
- the initial checked-in fixtures cover the PSD07 target shapes directly: simple raster layers, visibility/opacity metadata, the current supported blend subset, and a grouped-structure fallback case.
- `tools/psd_import_sidecar/test_fixture_sidecar.py` now runs the real `psd-tools` sidecar against those fixtures and asserts the emitted manifest contract, so the sidecar behavior is no longer validated only through shell-script doubles.
- `file_io` now also has fixture-backed end-to-end regression coverage for these repo PSDs when `python3` plus `psd-tools` are available in the test environment, so the real fixtures exercise the Rust importer seam instead of stopping at manifest-shape validation.
- the supported fixture set is now stable and validated through the real sidecar plus real `file_io` import coverage, so PSD07 is complete and later unsupported-feature additions live under PSD08 rather than keeping the supported fixture task open.

### PSD08 - Add unsupported-feature diagnostics fixtures

- [x] Status: completed
- Outcome: unsupported PSD features stay explicit as support expands
- Includes:
  - masks/clipping fixture if unsupported
  - text fixture if unsupported
  - smart object fixture if unsupported
  - CMYK or print-oriented fixture if unsupported
- Depends on: PSD05, PSD07
- Done when:
  - unsupported PSD behavior is covered by tests and messaging expectations

Progress notes:
- PSD08 has started with a real repo-owned CMYK fallback fixture under `tests/fixtures/psd/unsupported-cmyk-fallback.psd`, generated from repository source code rather than copied from a third-party PSD sample.
- the real sidecar fixture suite now asserts that this source is reported as `cmyk` with a truthful composite available, which anchors unsupported color-mode diagnostics to an actual PSD instead of only manifest-shaped doubles.
- `file_io` now also exercises that fixture through the real sidecar/importer seam and expects the current documented behavior: layered import is rejected for the unsupported color mode, the flattened composite fallback is used, and the resulting diagnostics include `unsupported_color_mode` plus `flattened_fallback_used`.
- the repo-owned fixture generator now also emits real unsupported text, smart-object, clipping, and mask PSDs by patching low-level PSD records and tagged blocks after raster fixture construction, so the unsupported-feature corpus is no longer limited to CMYK alone.
- the real sidecar suite now asserts those unsupported structures directly, and `file_io` now verifies that each one falls back truthfully and reports the expected unsupported-layer diagnostics instead of relying on manifest-only doubles.

### PSD09 - Validate imported-scene viewport/export parity

- [x] Status: completed
- Outcome: imported scenes render and export consistently after normalization
- Includes:
  - imported scene flatten parity checks
  - export-versus-viewport validation on imported documents
- Depends on: PSD03, PSD07
- Done when:
  - imported supported PSD scenes behave like native documents after import

Progress notes:
- PSD09 has started with a real fixture-backed export parity regression in `file_io`: the supported layered PSD fixture is imported through the real sidecar, exported back to PNG, and the exported pixels are checked against `flatten_document_rgba` for the imported native document.
- this does not yet close the full viewport-versus-export question at the shell/render boundary, but it does anchor the imported-scene parity story to a real PSD fixture instead of only native-document test doubles.
- `app_core` now also has a real-fixture export roundtrip regression: the supported layered PSD fixture is imported through the real sidecar at the controller boundary, exported to PNG from the imported document, and re-imported to confirm the flattened pixels still match the original imported scene.
- `app_core` now also verifies that `canvas_raster()` for a real imported PSD scene matches `flatten_document_rgba(&controller.document)`, which closes the remaining viewport-raster parity gap for the current shell path.

### PSD10 - Document PSD import support and limitations

- [x] Status: completed
- Outcome: compatibility claims remain accurate and supportable
- Includes:
  - supported subset table
  - unsupported-feature list
  - fallback behavior explanation
  - user-facing docs for import expectations
- Depends on: PSD06, PSD08, PSD09
- Done when:
  - the project makes no ambiguous claims about PSD compatibility

Progress notes:
- `docs/psd-compatibility.md` now records the supported PSD import subset, unsupported structures, fallback rules, shell workflow expectations, and the current PSD export non-goal in one user-facing reference.
- the repository now has an explicit document to point at when discussing PSD support, instead of leaving the compatibility claims implied by tests or buried only in the post-MVP task plan.

### PSD11 - Define PSD export subset and non-goals

- [x] Status: completed
- Outcome: export scope is bounded before code is written
- Includes:
  - decide whether export is worth shipping in the near term
  - define first export subset if yes
  - document what export will not preserve
- Depends on: PSD10
- Done when:
  - PSD export is either intentionally deferred or clearly scoped

Progress notes:
- PSD export is now intentionally deferred rather than implicitly unresolved.
- `docs/psd-compatibility.md` states the current non-goal directly: PhotoTux supports PSD import, but `.ptx` remains the authoritative layered working format and PSD export is not currently promised.
- this closes PSD11 by decision, not by omission: the project now has an explicit near-term no-go for PSD export until a writer path is trustworthy enough to defend publicly.

### PSD12 - Implement PSD export subset

- [x] Status: completed
- Outcome: PSD export implementation is intentionally closed for the current track because PSD11 explicitly deferred PSD export rather than scoping it for near-term delivery
- Includes:
  - export adapter in `file_io`
  - diagnostics for lossy conversions
  - fixture-based export validation
- Depends on: PSD11
- Done when:
  - the task is either reopened under a future writer-path decision or explicitly closed by the PSD11 defer decision

Progress notes:
- no PSD export code was added under this track.
- this task is closed intentionally because PSD11 chose the explicit non-goal path instead of opening a writer implementation effort.
- if PSD export work is reopened later, it should start from a fresh writer evaluation and a newly scoped subset rather than by pretending this track already committed to export parity.

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