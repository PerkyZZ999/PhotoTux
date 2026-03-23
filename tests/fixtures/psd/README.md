# PhotoTux PSD Fixtures

This directory contains repo-owned PSD fixtures used to validate the current `psd-tools` sidecar import contract.

The fixtures are generated from source code in this repository rather than copied from third-party PSD samples.

## Current Fixtures

- `supported-simple-layers.psd`
  - two top-level raster layers
  - preserved offsets, opacity, and supported `Screen` blend mapping
- `supported-visibility-opacity.psd`
  - top-level raster layers covering hidden-layer and reduced-opacity metadata
- `supported-blend-subset.psd`
  - top-level raster layers covering the current supported blend subset
  - `Normal`, `Multiply`, `Screen`, `Overlay`, `Darken`, `Lighten`
- `flattened-fallback-group.psd`
  - top-level unsupported group structure that should drive composite-backed fallback behavior rather than faithful layered import
- `unsupported-text-fallback.psd`
  - top-level type layer represented through a real PSD tagged-block path
  - used to assert unsupported text-layer fallback diagnostics against the real sidecar contract
- `unsupported-smart-object-fallback.psd`
  - top-level smart object layer represented through a real PSD tagged-block path
  - used to assert unsupported smart-object fallback diagnostics against the real sidecar contract
- `unsupported-clipping-fallback.psd`
  - raster layer stack with real clipping metadata
  - used to assert unsupported clipping diagnostics plus truthful composite-backed fallback behavior
- `unsupported-mask-fallback.psd`
  - raster layer stack with a non-placeholder layer mask
  - used to assert unsupported mask diagnostics plus truthful composite-backed fallback behavior
- `unsupported-cmyk-fallback.psd`
  - print-oriented CMYK source that should stay explicit as unsupported layered import
  - used to assert real unsupported-color-mode diagnostics plus truthful composite-backed fallback behavior

## Regeneration

Fixture generation is intentionally separate from normal PhotoTux runtime dependencies.

One practical local path is:

```bash
python3 -m venv /tmp/phototux-psd-fixtures
/tmp/phototux-psd-fixtures/bin/pip install numpy psd-tools pytoshop six
/tmp/phototux-psd-fixtures/bin/python tests/fixtures/psd/generate_psd_fixtures.py
```

## Validation

After installing the sidecar runtime dependencies described in `tools/psd_import_sidecar/README.md`, run:

```bash
python3 tools/psd_import_sidecar/test_fixture_sidecar.py
```

That script runs the real sidecar against each fixture and asserts the emitted manifest contract for the current supported subset plus unsupported text, smart-object, clipping, mask, group, and CMYK fallback behavior.

If `python3` plus `psd-tools` are available in the test environment, `cargo test -p file_io` now also exercises these repo fixtures through `import_psd_from_path_with_sidecar`, so the real PSD files cover the Rust importer boundary as well as the sidecar manifest contract.