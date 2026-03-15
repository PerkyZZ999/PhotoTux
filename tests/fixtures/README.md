# PhotoTux Canonical Test Fixtures

PhotoTux currently maintains its canonical regression scenes programmatically inside the Rust test suites.

This directory exists to document the fixture set that T20 validates and to provide a stable home for future checked-in `.ptx` projects or reference exports.

Current canonical fixture categories:

- representative layered compositing scene
- repeated save and reopen scene
- PNG export parity scene
- large sparse document stress scene
- autosave and crash recovery scene

Current implementations:

- `crates/file_io/src/lib.rs` builds representative compositing and large sparse persistence scenes for save/load and export validation
- `crates/app_core/src/lib.rs` builds a representative controller-owned scene for viewport-versus-export parity and save/undo workflow checks

Fixture goals:

- keep export output aligned with the visible flattened composite
- catch persistence regressions before they ship
- exercise representative layered scenes instead of one-off ad hoc samples
- provide a clear place to add checked-in `.ptx` and reference image assets later if programmatic fixtures stop being sufficient