# PhotoTux PSD Compatibility

## Purpose

This document describes the current PSD support that PhotoTux intentionally claims.

The native `.ptx` format remains the authoritative layered working format for PhotoTux.

## Current Direction

PhotoTux currently supports PSD import through a repo-managed `psd-tools` sidecar.

The PSD importer normalizes supported content into PhotoTux's native document model and reports unsupported structure explicitly.

PhotoTux does not currently promise PSD export.

## Supported PSD Import Subset

| Area | Current Support |
| --- | --- |
| Color mode | RGB |
| Bit depth | 8-bit |
| Layer structure | Top-level raster layers |
| Canvas size | Preserved |
| Layer names | Preserved |
| Layer order | Preserved |
| Layer visibility | Preserved |
| Layer opacity | Preserved |
| Layer offsets | Preserved |
| Blend modes | `Normal`, `Multiply`, `Screen`, `Overlay`, `Darken`, `Lighten` |

## Unsupported PSD Features

These features are currently outside the truthful editable PSD subset:

- text layers
- smart objects
- adjustment layers
- groups and deeper hierarchy
- clipping masks
- raster or vector masks
- layer effects
- unsupported blend modes
- non-RGB or print-oriented modes such as CMYK
- unsupported bit depths

## Import Result Rules

PhotoTux follows these rules when importing PSDs:

1. If the PSD stays inside the supported subset, PhotoTux imports it as editable native layers.
2. If the PSD exceeds the supported subset but exposes a truthful flattened composite, PhotoTux imports that composite as a single raster layer and surfaces diagnostics.
3. If the PSD exceeds the supported subset and no truthful fallback is available, the import fails clearly.

## Import Diagnostics

When a PSD falls back or loses fidelity, PhotoTux reports what happened instead of pretending the structure was preserved.

Current diagnostic categories include:

- unsupported color mode
- unsupported bit depth
- unsupported layer kind
- unsupported layer features
- unsupported blend mode
- flattened fallback used

## Shell Workflow Expectations

From the shell, PSD files are imported through the normal image import flow.

If the PSD imports cleanly inside the supported subset, the resulting document behaves like a native PhotoTux document.

If the PSD falls back to a flattened composite, the shell status and import report explain that the PSD was not preserved as editable layered structure.

## After Import

After importing a PSD, save layered work back to `.ptx` if you want PhotoTux-native editing fidelity.

PNG, JPEG, and WebP remain the current export formats for rendered output.

## PSD Export

PSD export is intentionally deferred.

Reasoning:

- the import-first interoperability track is now trustworthy enough to document, but the repository does not yet have a writer path with the same confidence level
- `.ptx` already covers truthful layered persistence for PhotoTux itself
- shipping PSD export without a stronger writer story would create compatibility claims the project cannot currently defend

If PSD export work is reopened later, it should begin from a fresh writer-library evaluation and a narrow documented subset rather than by implying parity with the import path.
