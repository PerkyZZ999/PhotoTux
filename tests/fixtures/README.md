# PhotoTux Canonical Test Fixtures

PhotoTux currently maintains its canonical regression scenes programmatically inside the Rust test suites.

This directory exists to document the fixture set that T20 validates and to provide a stable home for future checked-in `.ptx` projects or reference exports.

Current canonical fixture categories:

- representative layered compositing scene
- repeated save and reopen scene
- PNG export parity scene
- large sparse document stress scene
- autosave and crash recovery scene
- masked compositing scene
- grouped hierarchy scene
- lasso-aware transform parity scene
- guide-snapping interaction scene
- representative destructive-filter scene
- PSD import interoperability scene set

Current implementations:

- `crates/file_io/src/lib.rs` builds representative compositing and large sparse persistence scenes for save/load and export validation
- `crates/app_core/src/lib.rs` builds a representative controller-owned scene for viewport-versus-export parity and save/undo workflow checks
- `crates/file_io/src/lib.rs` also builds masked and grouped persistence scenes used for roundtrip and flattened-output regressions
- `crates/app_core/src/lib.rs` now includes dedicated upgraded-workflow fixtures for lasso-aware transform parity and guide-snapping interaction coverage
- `docs/tests/post-mvp-editing-workflow-checklist.md` records the current manual validation checklist for masks, groups, lasso, transform, guides, and snapping behavior
- `docs/tests/post-mvp-painting-checklist.md` records the current manual validation checklist for pressure-aware painting, live preview, and repeated medium-canvas stroke validation
- `crates/app_core/src/lib.rs` now also uses its representative controller-owned scene for destructive-filter workflow coverage
- `tests/fixtures/psd/` now contains repo-owned generated PSD interoperability fixtures plus a regeneration script and fixture notes for the current sidecar-backed import subset

Fixture goals:

- keep export output aligned with the visible flattened composite
- catch persistence regressions before they ship
- exercise representative layered scenes instead of one-off ad hoc samples
- provide a clear place to add checked-in `.ptx` and reference image assets later if programmatic fixtures stop being sufficient
- keep post-MVP editing workflow regressions anchored to named representative scenes instead of drifting back toward one-off controller setup code

Current upgraded-workflow coverage:

- masked scene roundtrip and flattened-output coverage in `file_io`
- grouped hierarchy roundtrip and flattened-output coverage in `file_io`
- lasso-aware transform parity coverage in `app_core`
- guide-snapping move and transform integration coverage in `app_core`
- medium-canvas repeated pressure-stroke coverage in `app_core`
- representative-scene destructive filter and stale-result discard coverage in `app_core`
- sidecar-backed PSD fixture validation in `tools/psd_import_sidecar/test_fixture_sidecar.py`