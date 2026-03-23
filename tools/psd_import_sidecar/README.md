# PhotoTux PSD Import Sidecar

This directory contains the repo-managed `psd-tools` sidecar entrypoint used by the current PhotoTux PSD import path.

## Purpose

- open a PSD file with `psd-tools`
- export a versioned JSON manifest that matches PhotoTux's current `file_io` contract
- export per-layer PNG assets for supported raster layers
- export a flattened composite PNG for truthful fallback import when layered fidelity is outside the current subset

The sidecar does not write `.ptx` files directly. It only emits a manifest plus sibling raster assets into the import workspace created by PhotoTux.

## Setup

From the repository root:

```bash
python3 -m venv tools/psd_import_sidecar/.venv
tools/psd_import_sidecar/.venv/bin/pip install -r tools/psd_import_sidecar/requirements.txt
```

Then point PhotoTux at the Python runtime and sidecar entrypoint:

```bash
export PHOTOTUX_PSD_IMPORT_SIDECAR="$PWD/tools/psd_import_sidecar/.venv/bin/python3"
export PHOTOTUX_PSD_IMPORT_SIDECAR_ARGS="$PWD/tools/psd_import_sidecar/phototux_psd_sidecar.py"
```

If you prefer a single executable path, `tools/psd_import_sidecar/run_sidecar.sh` is also included as a convenience launcher. In that case, point `PHOTOTUX_PSD_IMPORT_SIDECAR` at the shell script instead of `python3`.

## Fixture Validation

The repository now includes a small PSD fixture corpus under `tests/fixtures/psd/` for exercising the current sidecar contract against real PSD files owned by PhotoTux.

After installing this directory's runtime dependencies, run:

```bash
python3 tools/psd_import_sidecar/test_fixture_sidecar.py
```

That validation runs the real sidecar against the checked-in PSD fixtures and asserts the emitted manifest shape for supported layered cases plus unsupported text, smart-object, clipping, mask, group, and CMYK composite-fallback cases.

## Current Scope

- emits the manifest fields already consumed by `file_io`
- reports PSD source color mode and depth from `psd-tools`
- exports top-level layer records in background-to-foreground order
- marks unsupported structure through `kind` and `unsupported_features`
- exports a flattened composite when `psd-tools` can produce one

Current limitations intentionally match PhotoTux's first PSD subset work:

- only top-level raster layers are intended for faithful import
- group, text, smart object, adjustment, mask, clipping, and effect structures are diagnosed rather than preserved as editable PhotoTux constructs
- runtime packaging is still repo-managed, not bundled into a release artifact yet