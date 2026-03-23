#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SIDE_CAR_PATH = REPO_ROOT / "tools" / "psd_import_sidecar" / "phototux_psd_sidecar.py"
FIXTURE_DIR = REPO_ROOT / "tests" / "fixtures" / "psd"


def run_sidecar(fixture_name: str) -> tuple[dict, Path]:
	temporary_directory = tempfile.TemporaryDirectory(prefix="phototux-psd-sidecar-")
	workspace_dir = Path(temporary_directory.name)
	manifest_path = workspace_dir / "manifest.json"
	result = subprocess.run(
		[
			sys.executable,
			str(SIDE_CAR_PATH),
			str(FIXTURE_DIR / fixture_name),
			str(workspace_dir),
			str(manifest_path),
		],
		check=True,
		capture_output=True,
		text=True,
	)
	if result.stderr:
		raise AssertionError(f"sidecar wrote unexpected stderr for {fixture_name}: {result.stderr}")
	manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
	manifest["_temporary_directory"] = temporary_directory
	return manifest, workspace_dir


class SidecarFixtureTests(unittest.TestCase):
	def layer_by_name(self, manifest: dict, layer_name: str) -> dict:
		for layer in manifest["layers"]:
			if layer["name"] == layer_name:
				return layer
		raise AssertionError(f"missing layer {layer_name!r}")

	def assert_raster_assets_exist(self, manifest: dict, workspace_dir: Path) -> None:
		for layer in manifest["layers"]:
			raster_asset_relpath = layer.get("raster_asset_relpath")
			if raster_asset_relpath is None:
				continue
			self.assertTrue((workspace_dir / raster_asset_relpath).is_file(), raster_asset_relpath)

	def test_supported_simple_layers_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("supported-simple-layers.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertEqual(manifest["source_color_mode"], "rgb")
		self.assertEqual(manifest["source_depth_bits"], 8)
		self.assertEqual(manifest["canvas"], {"width_px": 6, "height_px": 6})
		self.assertTrue(manifest["composite"]["available"])
		self.assertEqual(len(manifest["layers"]), 2)
		self.assertEqual(
			manifest["layers"],
			[
				{
					"source_index": 0,
					"kind": "raster",
					"name": "Background",
					"visible": True,
					"opacity_0_255": 255,
					"blend_key": "norm",
					"offset_px": {"x": 0, "y": 0},
					"bounds_px": {"left": 0, "top": 0, "width": 3, "height": 3},
					"raster_asset_relpath": "layers/000-background.png",
					"unsupported_features": [],
				},
				{
					"source_index": 1,
					"kind": "raster",
					"name": "Screen Accent",
					"visible": True,
					"opacity_0_255": 128,
					"blend_key": "scrn",
					"offset_px": {"x": 2, "y": 1},
					"bounds_px": {"left": 2, "top": 1, "width": 2, "height": 2},
					"raster_asset_relpath": "layers/001-screen-accent.png",
					"unsupported_features": [],
				},
			],
		)
		self.assert_raster_assets_exist(manifest, workspace_dir)

	def test_supported_visibility_opacity_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("supported-visibility-opacity.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertEqual(
			[(layer["name"], layer["visible"], layer["opacity_0_255"]) for layer in manifest["layers"]],
			[
				("Base Fill", True, 255),
				("Hidden Accent", False, 255),
				("Soft Overlay", True, 96),
			],
		)
		self.assert_raster_assets_exist(manifest, workspace_dir)

	def test_supported_blend_subset_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("supported-blend-subset.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertEqual(manifest["canvas"], {"width_px": 6, "height_px": 1})
		self.assertEqual(
			[(layer["name"], layer["blend_key"]) for layer in manifest["layers"]],
			[
				("Normal Swatch", "norm"),
				("Multiply Swatch", "mul "),
				("Screen Swatch", "scrn"),
				("Overlay Swatch", "over"),
				("Darken Swatch", "dark"),
				("Lighten Swatch", "lite"),
			],
		)
		self.assert_raster_assets_exist(manifest, workspace_dir)

	def test_flattened_fallback_group_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("flattened-fallback-group.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertTrue(manifest["composite"]["available"])
		self.assertEqual(len(manifest["layers"]), 1)
		self.assertEqual(manifest["layers"][0]["name"], "Logo Group")
		self.assertEqual(manifest["layers"][0]["kind"], "group")
		self.assertEqual(manifest["layers"][0]["raster_asset_relpath"], None)
		self.assertIn("group_hierarchy", manifest["layers"][0]["unsupported_features"])
		self.assertIn("non_raster_layer_kind", manifest["layers"][0]["unsupported_features"])
		self.assertTrue((workspace_dir / "composite.png").is_file())

	def test_unsupported_cmyk_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("unsupported-cmyk-fallback.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertEqual(manifest["source_color_mode"], "cmyk")
		self.assertEqual(manifest["source_depth_bits"], 8)
		self.assertEqual(manifest["canvas"], {"width_px": 3, "height_px": 3})
		self.assertTrue(manifest["composite"]["available"])
		self.assertEqual(len(manifest["layers"]), 1)
		self.assertEqual(manifest["layers"][0]["name"], "CMYK Proof")
		self.assertEqual(manifest["layers"][0]["kind"], "raster")
		self.assertEqual(manifest["layers"][0]["blend_key"], "norm")
		self.assertEqual(manifest["layers"][0]["unsupported_features"], [])
		self.assert_raster_assets_exist(manifest, workspace_dir)
		self.assertTrue((workspace_dir / "composite.png").is_file())

	def test_unsupported_text_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("unsupported-text-fallback.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertTrue(manifest["composite"]["available"])
		text_layer = self.layer_by_name(manifest, "Title Layer")
		self.assertEqual(text_layer["kind"], "text")
		self.assertIsNone(text_layer["raster_asset_relpath"])
		self.assertIn("non_raster_layer_kind", text_layer["unsupported_features"])
		self.assertTrue((workspace_dir / "composite.png").is_file())

	def test_unsupported_smart_object_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("unsupported-smart-object-fallback.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertTrue(manifest["composite"]["available"])
		smart_object_layer = self.layer_by_name(manifest, "Placed Badge")
		self.assertEqual(smart_object_layer["kind"], "smart_object")
		self.assertIsNone(smart_object_layer["raster_asset_relpath"])
		self.assertIn("non_raster_layer_kind", smart_object_layer["unsupported_features"])
		self.assertTrue((workspace_dir / "composite.png").is_file())

	def test_unsupported_clipping_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("unsupported-clipping-fallback.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertTrue(manifest["composite"]["available"])
		clipped_layer = self.layer_by_name(manifest, "Clipped Accent")
		self.assertEqual(clipped_layer["kind"], "raster")
		self.assertIn("clipping_mask", clipped_layer["unsupported_features"])
		self.assert_raster_assets_exist(manifest, workspace_dir)
		self.assertTrue((workspace_dir / "composite.png").is_file())

	def test_unsupported_mask_fixture(self) -> None:
		manifest, workspace_dir = run_sidecar("unsupported-mask-fallback.psd")
		self.addCleanup(manifest["_temporary_directory"].cleanup)
		manifest.pop("_temporary_directory")

		self.assertTrue(manifest["composite"]["available"])
		masked_layer = self.layer_by_name(manifest, "Masked Accent")
		self.assertEqual(masked_layer["kind"], "raster")
		self.assertIn("mask", masked_layer["unsupported_features"])
		self.assert_raster_assets_exist(manifest, workspace_dir)
		self.assertTrue((workspace_dir / "composite.png").is_file())


if __name__ == "__main__":
	unittest.main()