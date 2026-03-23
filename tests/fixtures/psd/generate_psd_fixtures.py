#!/usr/bin/env python3

from __future__ import annotations

import argparse
from io import BytesIO
from pathlib import Path
from typing import Callable

import numpy as np
from psd_tools.psd.descriptor import DescriptorBlock
from psd_tools.psd.tagged_blocks import SmartObjectLayerData, TypeToolObjectSetting
from pytoshop import enums
from pytoshop import layers as low_level_layers
from pytoshop import tagged_block
from pytoshop.user.nested_layers import Group, Image, nested_layers_to_psd


ROOT = Path(__file__).resolve().parent


def serialize_tagged_block_payload(block: object) -> bytes:
	buffer = BytesIO()
	block.write(buffer)
	return buffer.getvalue()


TYPE_LAYER_TAG_PAYLOAD = serialize_tagged_block_payload(
	TypeToolObjectSetting(
		text_version=50,
		text_data=DescriptorBlock(),
		warp=DescriptorBlock(),
	)
)

SMART_OBJECT_TAG_PAYLOAD = serialize_tagged_block_payload(
	SmartObjectLayerData(data=DescriptorBlock())
)


def parse_args() -> argparse.Namespace:
	parser = argparse.ArgumentParser(description="Generate repo-owned PSD fixtures for PhotoTux sidecar validation.")
	parser.add_argument(
		"--output-dir",
		type=Path,
		default=ROOT,
		help="Directory that will receive the generated PSD fixtures.",
	)
	return parser.parse_args()


def rgb_channels(rows: list[list[list[int]]]) -> dict[int, np.ndarray]:
	array = np.asarray(rows, dtype=np.uint8)
	if array.ndim != 3 or array.shape[2] != 3:
		raise ValueError("expected rows shaped as height x width x 3 RGB pixels")
	height, width, _ = array.shape
	return {
		0: array[:, :, 0],
		1: array[:, :, 1],
		2: array[:, :, 2],
		-1: np.full((height, width), 255, dtype=np.uint8),
	}


def cmyk_channels(rows: list[list[list[int]]]) -> dict[int, np.ndarray]:
	array = np.asarray(rows, dtype=np.uint8)
	if array.ndim != 3 or array.shape[2] != 4:
		raise ValueError("expected rows shaped as height x width x 4 CMYK pixels")
	height, width, _ = array.shape
	return {
		0: array[:, :, 0],
		1: array[:, :, 1],
		2: array[:, :, 2],
		3: array[:, :, 3],
		-1: np.full((height, width), 255, dtype=np.uint8),
	}


def raster_layer(
	name: str,
	*,
	left: int,
	top: int,
	rows: list[list[list[int]]],
	visible: bool = True,
	opacity: int = 255,
	blend_mode: enums.BlendMode = enums.BlendMode.normal,
) -> Image:
	return Image(
		name=name,
		visible=visible,
		opacity=opacity,
		blend_mode=blend_mode,
		left=left,
		top=top,
		channels=rgb_channels(rows),
	)


def cmyk_raster_layer(
	name: str,
	*,
	left: int,
	top: int,
	rows: list[list[list[int]]],
	visible: bool = True,
	opacity: int = 255,
	blend_mode: enums.BlendMode = enums.BlendMode.normal,
) -> Image:
	return Image(
		name=name,
		visible=visible,
		opacity=opacity,
		blend_mode=blend_mode,
		left=left,
		top=top,
		channels=cmyk_channels(rows),
		color_mode=enums.ColorMode.cmyk,
	)


def write_psd(
	path: Path,
	size: tuple[int, int],
	layers: list[Image | Group],
	*,
	color_mode: enums.ColorMode = enums.ColorMode.rgb,
	patcher: Callable[[object], None] | None = None,
) -> None:
	psd = nested_layers_to_psd(
		layers,
		color_mode=color_mode,
		size=size,
		compression=enums.Compression.raw,
	)
	if patcher is not None:
		patcher(psd)
	with path.open("wb") as handle:
		psd.write(handle)


def layer_record_name(record: low_level_layers.LayerRecord) -> str:
	unicode_name = getattr(record.blocks_map.get(b"luni"), "name", "")
	if unicode_name:
		return unicode_name
	return record.name


def find_layer_record(psd: object, layer_name: str) -> low_level_layers.LayerRecord:
	for record in psd.layer_and_mask_info.layer_info.layer_records:
		if layer_record_name(record) == layer_name:
			return record
	raise ValueError(f"missing expected layer record {layer_name!r}")


def patch_type_layer(psd: object, layer_name: str) -> None:
	record = find_layer_record(psd, layer_name)
	record.blocks.append(
		tagged_block.GenericTaggedBlock(code=b"TySh", data=TYPE_LAYER_TAG_PAYLOAD)
	)


def patch_smart_object_layer(psd: object, layer_name: str) -> None:
	record = find_layer_record(psd, layer_name)
	record.blocks.append(
		tagged_block.GenericTaggedBlock(code=b"SoLd", data=SMART_OBJECT_TAG_PAYLOAD)
	)


def patch_clipping_layer(psd: object, layer_name: str) -> None:
	record = find_layer_record(psd, layer_name)
	record.clipping = True


def patch_masked_layer(psd: object, layer_name: str) -> None:
	record = find_layer_record(psd, layer_name)
	record.mask = low_level_layers.LayerMask(
		top=record.top,
		left=record.left,
		bottom=record.bottom,
		right=record.right,
	)


def build_supported_simple_layers() -> list[Image]:
	return [
		raster_layer(
			"Background",
			left=0,
			top=0,
			rows=[
				[[20, 40, 80], [20, 40, 80], [20, 40, 80]],
				[[20, 40, 80], [20, 40, 80], [20, 40, 80]],
				[[20, 40, 80], [20, 40, 80], [20, 40, 80]],
			],
		),
		raster_layer(
			"Screen Accent",
			left=2,
			top=1,
			opacity=128,
			blend_mode=enums.BlendMode.screen,
			rows=[
				[[240, 180, 100], [240, 180, 100]],
				[[240, 180, 100], [240, 180, 100]],
			],
		),
	]


def build_supported_visibility_opacity() -> list[Image]:
	return [
		raster_layer(
			"Base Fill",
			left=0,
			top=0,
			rows=[
				[[30, 60, 90], [30, 60, 90], [30, 60, 90]],
				[[30, 60, 90], [30, 60, 90], [30, 60, 90]],
				[[30, 60, 90], [30, 60, 90], [30, 60, 90]],
			],
		),
		raster_layer(
			"Hidden Accent",
			left=1,
			top=1,
			visible=False,
			rows=[
				[[200, 80, 40], [200, 80, 40]],
				[[200, 80, 40], [200, 80, 40]],
			],
		),
		raster_layer(
			"Soft Overlay",
			left=2,
			top=0,
			opacity=96,
			rows=[
				[[180, 210, 120], [180, 210, 120]],
				[[180, 210, 120], [180, 210, 120]],
			],
		),
	]


def build_supported_blend_subset() -> list[Image]:
	blend_specs = [
		("Normal Swatch", enums.BlendMode.normal, [60, 60, 60]),
		("Multiply Swatch", enums.BlendMode.multiply, [120, 80, 40]),
		("Screen Swatch", enums.BlendMode.screen, [230, 220, 160]),
		("Overlay Swatch", enums.BlendMode.overlay, [180, 110, 90]),
		("Darken Swatch", enums.BlendMode.darken, [70, 130, 160]),
		("Lighten Swatch", enums.BlendMode.lighten, [200, 180, 230]),
	]
	layers: list[Image] = []
	for index, (name, blend_mode, color) in enumerate(blend_specs):
		layers.append(
			raster_layer(
				name,
				left=index,
				top=0,
				blend_mode=blend_mode,
				rows=[[[color[0], color[1], color[2]]]],
			)
		)
	return layers


def build_flattened_fallback_group() -> list[Group]:
	return [
		Group(
			name="Logo Group",
			layers=[
				raster_layer(
					"Logo Fill",
					left=1,
					top=1,
					rows=[
						[[40, 80, 120], [40, 80, 120], [40, 80, 120], [40, 80, 120]],
						[[40, 80, 120], [80, 140, 200], [80, 140, 200], [40, 80, 120]],
						[[40, 80, 120], [80, 140, 200], [80, 140, 200], [40, 80, 120]],
						[[40, 80, 120], [40, 80, 120], [40, 80, 120], [40, 80, 120]],
					],
				),
			],
		),
	]


def build_unsupported_cmyk_fallback() -> list[Image]:
	return [
		cmyk_raster_layer(
			"CMYK Proof",
			left=0,
			top=0,
			rows=[
				[[0, 196, 196, 0], [0, 196, 196, 0], [0, 196, 196, 0]],
				[[0, 196, 196, 0], [0, 128, 128, 32], [0, 196, 196, 0]],
				[[0, 196, 196, 0], [0, 196, 196, 0], [0, 196, 196, 0]],
			],
		),
	]


def build_unsupported_text_fallback() -> list[Image]:
	return [
		raster_layer(
			"Background",
			left=0,
			top=0,
			rows=[
				[[18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62]],
				[[18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62]],
				[[18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62]],
				[[18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62]],
				[[18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62]],
				[[18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62], [18, 34, 62]],
			],
		),
		raster_layer(
			"Title Layer",
			left=1,
			top=1,
			rows=[
				[[220, 180, 100], [220, 180, 100], [220, 180, 100], [220, 180, 100]],
				[[220, 180, 100], [250, 220, 160], [250, 220, 160], [220, 180, 100]],
			],
		),
	]


def build_unsupported_smart_object_fallback() -> list[Image]:
	return [
		raster_layer(
			"Background",
			left=0,
			top=0,
			rows=[
				[[24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76]],
				[[24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76]],
				[[24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76]],
				[[24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76]],
				[[24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76]],
				[[24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76], [24, 44, 76]],
			],
		),
		raster_layer(
			"Placed Badge",
			left=2,
			top=1,
			rows=[
				[[120, 210, 190], [120, 210, 190]],
				[[120, 210, 190], [180, 250, 230]],
			],
		),
	]


def build_unsupported_clipping_fallback() -> list[Image]:
	return [
		raster_layer(
			"Base Fill",
			left=0,
			top=0,
			rows=[
				[[42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82]],
				[[42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82]],
				[[42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82]],
				[[42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82]],
				[[42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82]],
				[[42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82], [42, 54, 82]],
			],
		),
		raster_layer(
			"Clipped Accent",
			left=1,
			top=1,
			rows=[
				[[220, 92, 72], [220, 92, 72], [220, 92, 72]],
				[[220, 92, 72], [255, 170, 140], [220, 92, 72]],
				[[220, 92, 72], [220, 92, 72], [220, 92, 72]],
			],
		),
	]


def build_unsupported_mask_fallback() -> list[Image]:
	return [
		raster_layer(
			"Background",
			left=0,
			top=0,
			rows=[
				[[36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74]],
				[[36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74]],
				[[36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74]],
				[[36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74]],
				[[36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74]],
				[[36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74], [36, 48, 74]],
			],
		),
		raster_layer(
			"Masked Accent",
			left=1,
			top=1,
			rows=[
				[[110, 190, 120], [110, 190, 120], [110, 190, 120]],
				[[110, 190, 120], [180, 240, 190], [110, 190, 120]],
				[[110, 190, 120], [110, 190, 120], [110, 190, 120]],
			],
		),
	]


def main() -> int:
	args = parse_args()
	output_dir = args.output_dir.resolve()
	output_dir.mkdir(parents=True, exist_ok=True)

	write_psd(output_dir / "supported-simple-layers.psd", (6, 6), build_supported_simple_layers())
	write_psd(output_dir / "supported-visibility-opacity.psd", (6, 4), build_supported_visibility_opacity())
	write_psd(output_dir / "supported-blend-subset.psd", (6, 1), build_supported_blend_subset())
	write_psd(output_dir / "flattened-fallback-group.psd", (6, 6), build_flattened_fallback_group())
	write_psd(
		output_dir / "unsupported-text-fallback.psd",
		(6, 6),
		build_unsupported_text_fallback(),
		patcher=lambda psd: patch_type_layer(psd, "Title Layer"),
	)
	write_psd(
		output_dir / "unsupported-smart-object-fallback.psd",
		(6, 6),
		build_unsupported_smart_object_fallback(),
		patcher=lambda psd: patch_smart_object_layer(psd, "Placed Badge"),
	)
	write_psd(
		output_dir / "unsupported-clipping-fallback.psd",
		(6, 6),
		build_unsupported_clipping_fallback(),
		patcher=lambda psd: patch_clipping_layer(psd, "Clipped Accent"),
	)
	write_psd(
		output_dir / "unsupported-mask-fallback.psd",
		(6, 6),
		build_unsupported_mask_fallback(),
		patcher=lambda psd: patch_masked_layer(psd, "Masked Accent"),
	)
	write_psd(
		output_dir / "unsupported-cmyk-fallback.psd",
		(3, 3),
		build_unsupported_cmyk_fallback(),
		color_mode=enums.ColorMode.cmyk,
	)
	return 0


if __name__ == "__main__":
	raise SystemExit(main())