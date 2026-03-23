#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from psd_tools import PSDImage


CURRENT_MANIFEST_VERSION = 1

COLOR_MODE_MAP = {
    "bitmap": "bitmap",
    "cmyk": "cmyk",
    "duotone": "duotone",
    "grayscale": "grayscale",
    "indexed": "indexed",
    "lab": "lab",
    "multichannel": "multichannel",
    "rgb": "rgb",
}

LAYER_KIND_MAP = {
    "adjustment": "adjustment",
    "group": "group",
    "pixel": "raster",
    "smartobject": "smart_object",
    "type": "text",
}

BLEND_MODE_NAME_TO_KEY = {
    "color": "colr",
    "color burn": "idiv",
    "color dodge": "div ",
    "darken": "dark",
    "difference": "diff",
    "dissolve": "diss",
    "divide": "fdiv",
    "exclusion": "smud",
    "hard light": "hLit",
    "lighten": "lite",
    "linear burn": "lbrn",
    "linear dodge": "lddg",
    "linear light": "lLit",
    "luminosity": "lum ",
    "multiply": "mul ",
    "normal": "norm",
    "overlay": "over",
    "pass through": "pass",
    "pass_through": "pass",
    "pin light": "pLit",
    "screen": "scrn",
    "soft light": "sLit",
    "subtract": "fsub",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export a PSD into the PhotoTux PSD import manifest contract."
    )
    parser.add_argument("source_psd", type=Path)
    parser.add_argument("workspace_dir", type=Path)
    parser.add_argument("manifest_path", type=Path)
    return parser.parse_args()


def slugify(value: str) -> str:
    cleaned = "".join(character.lower() if character.isalnum() else "-" for character in value)
    while "--" in cleaned:
        cleaned = cleaned.replace("--", "-")
    return cleaned.strip("-") or "layer"


def normalize_layer_name(value: Any, fallback: str) -> str:
    if value is None:
        return fallback
    return str(value).replace("\x00", "").strip() or fallback


def diagnostic(severity: str, code: str, message: str, source_index: int | None = None) -> dict[str, Any]:
    return {
        "severity": severity,
        "code": code,
        "message": message,
        "source_index": source_index,
    }


def normalize_color_mode(psd: PSDImage) -> str:
    raw_name = getattr(getattr(psd, "color_mode", None), "name", None)
    normalized = (raw_name or str(getattr(psd, "color_mode", "other"))).strip().lower()
    return COLOR_MODE_MAP.get(normalized, "other")


def normalize_layer_kind(layer: Any) -> str:
    kind = str(getattr(layer, "kind", "other")).strip().lower().replace(" ", "")
    return LAYER_KIND_MAP.get(kind, "other")


def blend_key_for_layer(layer: Any) -> str:
    blend_mode = getattr(layer, "blend_mode", None)
    candidates: list[str] = []
    if blend_mode is not None:
        for attribute in ("value", "name"):
            candidate = getattr(blend_mode, attribute, None)
            if isinstance(candidate, str):
                candidates.append(candidate)
        candidates.append(str(blend_mode))

    for candidate in candidates:
        normalized = candidate.strip().lower().replace("_", " ")
        if candidate and len(candidate) == 4:
            return candidate
        if normalized in BLEND_MODE_NAME_TO_KEY:
            return BLEND_MODE_NAME_TO_KEY[normalized]

    return "unknown"


def layer_bounds(layer: Any) -> tuple[int, int, int, int]:
    left, top, right, bottom = getattr(layer, "bbox", (0, 0, 0, 0))
    width = max(0, int(right) - int(left))
    height = max(0, int(bottom) - int(top))
    return int(left), int(top), width, height


def save_rgba_png(image: Any, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    image.convert("RGBA").save(path, format="PNG")


def has_supported_subset_mask_placeholder(layer: Any) -> bool:
    mask = getattr(layer, "mask", None)
    if mask is None:
        return False

    size = getattr(mask, "size", None)
    if isinstance(size, tuple) and len(size) == 2:
        try:
            return int(size[0]) == 0 and int(size[1]) == 0
        except (TypeError, ValueError):
            pass

    for width_attr, height_attr in (("width", "height"),):
        width = getattr(mask, width_attr, None)
        height = getattr(mask, height_attr, None)
        if width is not None and height is not None:
            try:
                return int(width) == 0 and int(height) == 0
            except (TypeError, ValueError):
                pass

    bbox = getattr(mask, "bbox", None)
    if isinstance(bbox, tuple) and len(bbox) == 4:
        try:
            left, top, right, bottom = (int(value) for value in bbox)
            return left == right and top == bottom
        except (TypeError, ValueError):
            pass

    return False


def layer_unsupported_features(layer: Any, normalized_kind: str) -> list[str]:
    features: list[str] = []
    if normalized_kind != "raster":
        features.append("non_raster_layer_kind")
    if getattr(layer, "clipping", False):
        features.append("clipping_mask")
    if hasattr(layer, "has_clip_layers") and layer.has_clip_layers():
        features.append("clip_layers")
    if hasattr(layer, "has_mask") and layer.has_mask() and not has_supported_subset_mask_placeholder(layer):
        features.append("mask")
    if hasattr(layer, "has_vector_mask") and layer.has_vector_mask():
        features.append("vector_mask")
    if hasattr(layer, "has_effects") and layer.has_effects():
        features.append("layer_effects")
    if normalized_kind == "group":
        features.append("group_hierarchy")

    _, _, width, height = layer_bounds(layer)
    if normalized_kind == "raster" and (width == 0 or height == 0):
        features.append("empty_bounds")

    if normalized_kind == "raster" and hasattr(layer, "has_pixels") and not layer.has_pixels():
        features.append("missing_pixels")

    return sorted(set(features))


def export_layer_record(index: int, layer: Any, workspace_dir: Path) -> dict[str, Any]:
    normalized_kind = normalize_layer_kind(layer)
    left, top, width, height = layer_bounds(layer)
    unsupported_features = layer_unsupported_features(layer, normalized_kind)
    raster_asset_relpath: str | None = None
    layer_name = normalize_layer_name(getattr(layer, "name", None), f"Layer {index}")

    if normalized_kind == "raster" and width > 0 and height > 0:
        image = layer.topil()
        if image is not None:
            raster_asset_relpath = f"layers/{index:03d}-{slugify(layer_name)}.png"
            save_rgba_png(image, workspace_dir / raster_asset_relpath)
        else:
            unsupported_features.append("pixel_export_unavailable")

    return {
        "source_index": index,
        "kind": normalized_kind,
        "name": layer_name,
        "visible": bool(getattr(layer, "visible", True)),
        "opacity_0_255": int(getattr(layer, "opacity", 255)),
        "blend_key": blend_key_for_layer(layer),
        "offset_px": {"x": left, "y": top},
        "bounds_px": {
            "left": left,
            "top": top,
            "width": width,
            "height": height,
        },
        "raster_asset_relpath": raster_asset_relpath,
        "unsupported_features": sorted(set(unsupported_features)),
    }


def export_composite(psd: PSDImage, workspace_dir: Path, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    composite_relpath = workspace_dir / "composite.png"
    try:
        composite = psd.composite()
    except ImportError as exc:
        diagnostics.append(
            diagnostic(
                "warning",
                "composite_dependencies_missing",
                f"Flattened composite export is unavailable because psd-tools compositing dependencies are missing: {exc}",
            )
        )
        return {"available": False, "asset_relpath": None}
    except Exception as exc:
        diagnostics.append(
            diagnostic(
                "warning",
                "composite_export_failed",
                f"Flattened composite export failed: {exc}",
            )
        )
        return {"available": False, "asset_relpath": None}

    if composite is None:
        diagnostics.append(
            diagnostic(
                "warning",
                "composite_missing",
                "Flattened composite export was unavailable for this PSD.",
            )
        )
        return {"available": False, "asset_relpath": None}

    save_rgba_png(composite, composite_relpath)
    return {"available": True, "asset_relpath": "composite.png"}


def build_manifest(psd: PSDImage, workspace_dir: Path) -> dict[str, Any]:
    diagnostics: list[dict[str, Any]] = [
        diagnostic("info", "source_loaded", "PSD manifest decoded successfully."),
    ]
    if int(getattr(psd, "version", 1)) != 1:
        diagnostics.append(
            diagnostic(
                "warning",
                "unsupported_psb_variant",
                "The source is a PSB document variant; PhotoTux currently treats this as outside the initial PSD subset.",
            )
        )

    layers = [
        export_layer_record(index, layer, workspace_dir)
        for index, layer in enumerate(reversed(list(psd)))
    ]
    composite = export_composite(psd, workspace_dir, diagnostics)

    return {
        "manifest_version": CURRENT_MANIFEST_VERSION,
        "source_kind": "psd",
        "source_color_mode": normalize_color_mode(psd),
        "source_depth_bits": int(getattr(psd, "depth", 0)),
        "canvas": {
            "width_px": int(getattr(psd, "width", 0)),
            "height_px": int(getattr(psd, "height", 0)),
        },
        "composite": composite,
        "diagnostics": diagnostics,
        "layers": layers,
    }


def main() -> int:
    args = parse_args()
    args.workspace_dir.mkdir(parents=True, exist_ok=True)
    args.manifest_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        psd = PSDImage.open(args.source_psd)
        manifest = build_manifest(psd, args.workspace_dir)
        args.manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    except Exception as exc:
        print(f"PhotoTux PSD sidecar failed: {exc}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())