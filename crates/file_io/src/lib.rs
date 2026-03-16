use anyhow::Context;
use color_math::{blend_rgba_over, BlendModeMath};
use common::{CanvasSize, LayerId};
use doc_model::{
	BlendMode, Document, LayerEditTarget, LayerHierarchyNode, MaskTile, RasterLayer, RasterTile,
	TileCoord,
};
use image::{ImageBuffer, ImageFormat, Rgb, Rgba};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

pub const PROJECT_FILE_EXTENSION: &str = "ptx";
pub const CURRENT_PROJECT_FORMAT_VERSION: u32 = 1;
pub const RECOVERY_FILE_SUFFIX: &str = ".autosave";

pub fn recovery_path_for_project_path(path: &Path) -> std::path::PathBuf {
	let mut file_name = OsString::from(path.as_os_str());
	file_name.push(RECOVERY_FILE_SUFFIX);
	std::path::PathBuf::from(file_name)
}

pub fn remove_file_if_exists(path: &Path) -> anyhow::Result<()> {
	match fs::remove_file(path) {
		Ok(()) => Ok(()),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(error) => Err(error.into()),
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
	pub format_version: u32,
	pub canvas_size: CanvasSize,
	pub active_edit_target: LayerEditTarget,
	pub layers: Vec<ManifestLayerRecord>,
}

impl From<&Document> for ProjectManifest {
	fn from(document: &Document) -> Self {
		let layers = document
			.layers
			.iter()
			.map(|layer| ManifestLayerRecord {
				id: layer.id,
				name: layer.name.clone(),
				visible: layer.visible,
				opacity_percent: layer.opacity_percent,
				blend_mode: layer.blend_mode,
				mask_enabled: layer.mask.as_ref().map(|mask| mask.enabled).unwrap_or(false),
				offset_x: layer.offset_x,
				offset_y: layer.offset_y,
				payload_path: format!("layers/{}.png", layer.id.0),
				mask_payload_path: layer
					.mask
					.as_ref()
					.map(|_| format!("masks/{}.alpha", layer.id.0)),
			})
			.collect();

		Self {
			format_version: CURRENT_PROJECT_FORMAT_VERSION,
			canvas_size: document.canvas_size,
			active_edit_target: document.active_edit_target,
			layers,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestLayerRecord {
	pub id: LayerId,
	pub name: String,
	pub visible: bool,
	pub opacity_percent: u8,
	pub blend_mode: BlendMode,
	pub mask_enabled: bool,
	pub offset_x: i32,
	pub offset_y: i32,
	pub payload_path: String,
	pub mask_payload_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
	pub manifest: ProjectManifest,
	pub layers: Vec<LayerPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerPayload {
	pub id: LayerId,
	pub tiles: Vec<TilePayload>,
	pub mask_tiles: Option<Vec<MaskTilePayload>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilePayload {
	pub coord: TileCoord,
	pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskTilePayload {
	pub coord: TileCoord,
	pub alpha: Vec<u8>,
}

impl From<&Document> for ProjectFile {
	fn from(document: &Document) -> Self {
		let layers = document
			.layers
			.iter()
			.map(|layer| LayerPayload {
				id: layer.id,
				tiles: layer
					.tiles
					.iter()
					.map(|(coord, tile)| TilePayload {
						coord: *coord,
						pixels: tile.pixels.clone(),
					})
					.collect(),
				mask_tiles: layer.mask.as_ref().map(|mask| {
					mask
						.tiles
						.iter()
						.map(|(coord, tile)| MaskTilePayload {
							coord: *coord,
							alpha: tile.alpha.clone(),
						})
						.collect()
				}),
			})
			.collect();

		Self {
			manifest: ProjectManifest::from(document),
			layers,
		}
	}
}

impl TryFrom<ProjectFile> for Document {
	type Error = anyhow::Error;

	fn try_from(project_file: ProjectFile) -> anyhow::Result<Self> {
		let ProjectFile { manifest, layers } = project_file;
		if manifest.format_version != CURRENT_PROJECT_FORMAT_VERSION {
			anyhow::bail!(
				"unsupported .ptx version: expected {}, got {}",
				CURRENT_PROJECT_FORMAT_VERSION,
				manifest.format_version
			);
		}

		let mut restored_layers = Vec::with_capacity(manifest.layers.len());
		for manifest_layer in manifest.layers {
			let payload = layers
				.iter()
				.find(|candidate| candidate.id == manifest_layer.id)
				.ok_or_else(|| anyhow::anyhow!("missing layer payload for {}", manifest_layer.id.0))?;

			let mut tiles = std::collections::HashMap::new();
			for tile in &payload.tiles {
				tiles.insert(tile.coord, RasterTile {
					pixels: tile.pixels.clone(),
				});
			}

			let mask = payload.mask_tiles.as_ref().map(|mask_tiles| {
				let mut tiles = std::collections::HashMap::new();
				for tile in mask_tiles {
					tiles.insert(tile.coord, MaskTile {
						alpha: tile.alpha.clone(),
					});
				}

				doc_model::RasterMask {
					enabled: manifest_layer.mask_enabled,
					tiles,
					dirty_tiles: std::collections::HashSet::new(),
				}
			});

			restored_layers.push(RasterLayer {
				id: manifest_layer.id,
				name: manifest_layer.name,
				visible: manifest_layer.visible,
				opacity_percent: manifest_layer.opacity_percent,
				blend_mode: manifest_layer.blend_mode,
				mask,
				offset_x: manifest_layer.offset_x,
				offset_y: manifest_layer.offset_y,
				tiles,
				dirty_tiles: std::collections::HashSet::new(),
			});
		}

		Ok(Document {
			id: common::DocumentId::new(),
			canvas_size: manifest.canvas_size,
			layer_hierarchy: restored_layers
				.iter()
				.map(|layer| LayerHierarchyNode::Layer(layer.id))
				.collect(),
			layers: restored_layers,
			active_layer_index: 0,
			active_edit_target: manifest.active_edit_target,
			tile_size: common::DEFAULT_TILE_SIZE,
			selection: None,
			selection_inverted: false,
		})
	}
}

pub fn save_document_to_path(path: &Path, document: &Document) -> anyhow::Result<()> {
	let project_file = ProjectFile::from(document);
	let json = serde_json::to_vec_pretty(&project_file)?;

	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	let temp_path = path.with_extension(format!("{}.tmp", PROJECT_FILE_EXTENSION));
	fs::write(&temp_path, json)?;
	fs::rename(&temp_path, path)?;
	Ok(())
}

pub fn load_document_from_path(path: &Path) -> anyhow::Result<Document> {
	let bytes = fs::read(path)?;
	let project_file: ProjectFile = serde_json::from_slice(&bytes)?;
	project_file.try_into()
}

pub fn export_png_to_path(path: &Path, document: &Document) -> anyhow::Result<()> {
	let flattened = flatten_document_rgba(document);
	let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
		document.canvas_size.width,
		document.canvas_size.height,
		flattened,
	)
	.ok_or_else(|| anyhow::anyhow!("failed to build PNG image buffer from flattened document"))?;

	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	image.save_with_format(path, ImageFormat::Png)?;
	Ok(())
}

pub fn export_jpeg_to_path(path: &Path, document: &Document) -> anyhow::Result<()> {
	let flattened = flatten_document_rgb(document, [255, 255, 255]);
	let image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(
		document.canvas_size.width,
		document.canvas_size.height,
		flattened,
	)
	.ok_or_else(|| anyhow::anyhow!("failed to build JPEG image buffer from flattened document"))?;

	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	image.save_with_format(path, ImageFormat::Jpeg)?;
	Ok(())
}

pub fn export_webp_to_path(path: &Path, document: &Document) -> anyhow::Result<()> {
	let flattened = flatten_document_rgba(document);
	let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
		document.canvas_size.width,
		document.canvas_size.height,
		flattened,
	)
	.ok_or_else(|| anyhow::anyhow!("failed to build WebP image buffer from flattened document"))?;

	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	image.save_with_format(path, ImageFormat::WebP)?;
	Ok(())
}

pub fn import_png_from_path(path: &Path) -> anyhow::Result<Document> {
	import_raster_document_from_path(path)
		.with_context(|| format!("failed to import PNG from {}", path.display()))
}

pub fn import_jpeg_from_path(path: &Path) -> anyhow::Result<Document> {
	import_raster_document_from_path(path)
		.with_context(|| format!("failed to import JPEG from {}", path.display()))
}

pub fn import_webp_from_path(path: &Path) -> anyhow::Result<Document> {
	import_raster_document_from_path(path)
		.with_context(|| format!("failed to import WebP from {}", path.display()))
}

fn import_raster_document_from_path(path: &Path) -> anyhow::Result<Document> {
	let decoded = image::open(path)?.to_rgba8();
	let (width, height) = decoded.dimensions();
	let mut document = Document::new(width, height);
	let tile_size = document.tile_size;

	for (pixel_x, pixel_y, pixel) in decoded.enumerate_pixels() {
		if pixel[3] == 0 {
			continue;
		}

		let coord = document
			.tile_coord_for_pixel(pixel_x, pixel_y)
			.ok_or_else(|| anyhow::anyhow!("decoded PNG pixel fell outside document bounds"))?;
		let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
		let local_x = (pixel_x - tile_origin_x) as usize;
		let local_y = (pixel_y - tile_origin_y) as usize;
		let pixel_index = (local_y * document.tile_size as usize + local_x) * 4;
		let tile = document
			.layer_mut(0)
			.ok_or_else(|| anyhow::anyhow!("imported document is missing its base layer"))?
			.ensure_tile(coord, tile_size);
		tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(&pixel.0);
	}

	Ok(document)
}

pub fn flatten_document_rgba(document: &Document) -> Vec<u8> {
	let mut output = vec![0_u8; (document.canvas_size.width * document.canvas_size.height * 4) as usize];

	for layer in &document.layers {
		if !layer.visible {
			continue;
		}

		let layer_opacity = layer.opacity_percent as f32 / 100.0;
		for (coord, tile) in &layer.tiles {
			let mask_tile = layer
				.mask
				.as_ref()
				.filter(|mask| mask.enabled)
				.and_then(|mask| mask.tiles.get(coord));
			let (tile_origin_x, tile_origin_y) = document.tile_origin(*coord);
			for local_y in 0..document.tile_size as usize {
				let canvas_y = tile_origin_y as i32 + layer.offset_y + local_y as i32;
				if canvas_y < 0 {
					continue;
				}
				if canvas_y >= document.canvas_size.height as i32 {
					break;
				}

				for local_x in 0..document.tile_size as usize {
					let canvas_x = tile_origin_x as i32 + layer.offset_x + local_x as i32;
					if canvas_x < 0 {
						continue;
					}
					if canvas_x >= document.canvas_size.width as i32 {
						break;
					}

					let src_index = (local_y * document.tile_size as usize + local_x) * 4;
					let mask_alpha = mask_tile
						.map(|tile| tile.alpha[local_y * document.tile_size as usize + local_x])
						.unwrap_or(255);
					let dst_index =
						(canvas_y as usize * document.canvas_size.width as usize + canvas_x as usize) * 4;
					composite_masked_pixel(
						&mut output[dst_index..dst_index + 4],
						&tile.pixels[src_index..src_index + 4],
						mask_alpha,
						layer_opacity,
						layer.blend_mode,
					);
				}
			}
		}
	}

	output
}

fn flatten_document_rgb(document: &Document, background_rgb: [u8; 3]) -> Vec<u8> {
	let flattened_rgba = flatten_document_rgba(document);
	let mut flattened_rgb = Vec::with_capacity((document.canvas_size.width * document.canvas_size.height * 3) as usize);

	for pixel in flattened_rgba.chunks_exact(4) {
		let alpha = pixel[3] as f32 / 255.0;
		for channel in 0..3 {
			let output = pixel[channel] as f32 * alpha + background_rgb[channel] as f32 * (1.0 - alpha);
			flattened_rgb.push(output.round().clamp(0.0, 255.0) as u8);
		}
	}

	flattened_rgb
}

fn composite_masked_pixel(
	destination: &mut [u8],
	source: &[u8],
	mask_alpha: u8,
	layer_opacity: f32,
	blend_mode: BlendMode,
) {
	if source[3] == 0 || mask_alpha == 0 {
		return;
	}

	if mask_alpha == 255 {
		composite_pixel(destination, source, layer_opacity, blend_mode);
		return;
	}

	let effective_alpha = ((source[3] as u16 * mask_alpha as u16 + 127) / 255) as u8;
	if effective_alpha == 0 {
		return;
	}

	let masked_source = [source[0], source[1], source[2], effective_alpha];
	composite_pixel(destination, &masked_source, layer_opacity, blend_mode);
}

fn composite_pixel(destination: &mut [u8], source: &[u8], layer_opacity: f32, blend_mode: BlendMode) {
	let result = blend_rgba_over(
		[destination[0], destination[1], destination[2], destination[3]],
		[source[0], source[1], source[2], source[3]],
		layer_opacity,
		match blend_mode {
			BlendMode::Normal => BlendModeMath::Normal,
			BlendMode::Multiply => BlendModeMath::Multiply,
			BlendMode::Screen => BlendModeMath::Screen,
			BlendMode::Overlay => BlendModeMath::Overlay,
			BlendMode::Darken => BlendModeMath::Darken,
			BlendMode::Lighten => BlendModeMath::Lighten,
		},
	);
	destination.copy_from_slice(&result);
}

#[cfg(test)]
mod tests {
	use super::{
		export_jpeg_to_path, export_png_to_path, export_webp_to_path, flatten_document_rgba,
		import_jpeg_from_path, import_png_from_path, import_webp_from_path, load_document_from_path,
		recovery_path_for_project_path, save_document_to_path, update_flattened_region_rgba,
		ProjectFile, ProjectManifest,
		CURRENT_PROJECT_FORMAT_VERSION,
	};
	use color_math::{blend_rgba_over, BlendModeMath};
	use doc_model::{BlendMode, Document, TileCoord};
	use std::fs;
	use std::path::PathBuf;
	use std::time::{SystemTime, UNIX_EPOCH};

	fn set_pixel(document: &mut Document, layer_index: usize, x: u32, y: u32, rgba: [u8; 4]) {
		let tile_size = document.tile_size as usize;
		let coord = document
			.tile_coord_for_pixel(x, y)
			.expect("representative scene pixel should land inside canvas");
		let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
		let tile = document
			.ensure_tile_for_pixel(layer_index, x, y)
			.expect("tile should be created for representative scene");
		let local_x = (x - tile_origin_x) as usize;
		let local_y = (y - tile_origin_y) as usize;
		let pixel_index = (local_y * tile_size + local_x) * 4;
		tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(&rgba);
	}

	fn set_mask_alpha(document: &mut Document, layer_index: usize, x: u32, y: u32, alpha: u8) {
		let tile_size = document.tile_size as usize;
		let tile_size_u32 = document.tile_size;
		let coord = document
			.tile_coord_for_pixel(x, y)
			.expect("mask pixel should land inside canvas");
		let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
		let local_x = (x - tile_origin_x) as usize;
		let local_y = (y - tile_origin_y) as usize;
		let pixel_index = local_y * tile_size + local_x;
		let mask = document
			.layer_mask_mut(layer_index)
			.expect("mask should exist for masked scene");
		let tile = mask.ensure_tile(coord, tile_size_u32);
		tile.alpha[pixel_index] = alpha;
	}

	fn build_representative_scene() -> Document {
		let mut document = Document::new(64, 64);
		document.rename_layer(0, "Background");

		for y in 0..16 {
			for x in 0..16 {
				set_pixel(&mut document, 0, x, y, [40, 80, 120, 255]);
			}
		}
		for y in 20..36 {
			for x in 20..44 {
				set_pixel(&mut document, 0, x, y, [10, 20, 30, 255]);
			}
		}

		document.add_layer("Multiply");
		let multiply_index = document.active_layer_index();
		document.set_layer_blend_mode(multiply_index, BlendMode::Multiply);
		for y in 0..16 {
			for x in 0..16 {
				set_pixel(&mut document, multiply_index, x, y, [200, 128, 100, 255]);
			}
		}

		document.add_layer("Screen Accent");
		let screen_index = document.active_layer_index();
		document.set_layer_blend_mode(screen_index, BlendMode::Screen);
		document.set_layer_opacity(screen_index, 70);
		assert!(document.set_layer_offset(screen_index, 4, 6));
		for y in 8..24 {
			for x in 8..24 {
				set_pixel(&mut document, screen_index, x, y, [120, 180, 220, 220]);
			}
		}

		document.add_layer("Lighten Edge");
		let lighten_index = document.active_layer_index();
		document.set_layer_blend_mode(lighten_index, BlendMode::Lighten);
		for y in 40..56 {
			for x in 6..18 {
				set_pixel(&mut document, lighten_index, x, y, [250, 50, 140, 180]);
			}
		}

		assert!(document.set_active_layer(lighten_index));
		document
	}

	fn build_large_sparse_document() -> Document {
		let mut document = Document::new(2048, 2048);
		document.rename_layer(0, "Background");

		for &(x, y, rgba) in &[
			(0, 0, [25, 30, 35, 255]),
			(255, 255, [50, 80, 120, 255]),
			(512, 256, [90, 110, 130, 255]),
			(1024, 1024, [120, 20, 90, 255]),
			(2047, 2047, [200, 210, 220, 255]),
		] {
			set_pixel(&mut document, 0, x, y, rgba);
		}

		for layer_number in 0..3 {
			document.add_layer(format!("Sparse Layer {}", layer_number + 1));
			let layer_index = document.active_layer_index();
			let mode = match layer_number {
				0 => BlendMode::Multiply,
				1 => BlendMode::Screen,
				_ => BlendMode::Overlay,
			};
			document.set_layer_blend_mode(layer_index, mode);
			document.set_layer_opacity(layer_index, 65 + layer_number as u8 * 10);
			assert!(document.set_layer_offset(layer_index, 12 * (layer_number as i32 + 1), -7 * layer_number as i32));

			for &(x, y, rgba) in &[
				(64 + layer_number as u32 * 128, 96, [220, 40, 40, 255]),
				(700, 600 + layer_number as u32 * 40, [40, 220, 120, 200]),
				(1536, 1536, [80, 120, 240, 255]),
				(1900, 300 + layer_number as u32 * 20, [200, 160, 40, 180]),
			] {
				set_pixel(&mut document, layer_index, x, y, rgba);
			}
		}

		document
	}

	fn build_masked_scene() -> Document {
		let mut document = Document::new(32, 32);
		document.rename_layer(0, "Background");

		for y in 0..32 {
			for x in 0..32 {
				set_pixel(&mut document, 0, x, y, [30, 50, 80, 255]);
			}
		}

		document.add_layer("Masked Paint");
		let masked_index = document.active_layer_index();
		for y in 6..26 {
			for x in 6..26 {
				set_pixel(&mut document, masked_index, x, y, [220, 120, 60, 255]);
			}
		}
		assert!(document.add_layer_mask(masked_index));

		for y in 6..26 {
			for x in 6..26 {
				let alpha = if x < 12 {
					0
				} else if x < 18 {
					128
				} else {
					255
				};
				set_mask_alpha(&mut document, masked_index, x, y, alpha);
			}
		}

		document
	}

	#[test]
	fn project_manifest_uses_current_version() {
		let document = Document::new(1920, 1080);
		let manifest = ProjectManifest::from(&document);

		assert_eq!(manifest.format_version, CURRENT_PROJECT_FORMAT_VERSION);
		assert_eq!(manifest.canvas_size.width, 1920);
		assert_eq!(manifest.active_edit_target, doc_model::LayerEditTarget::LayerPixels);
		assert_eq!(manifest.layers.len(), 1);
	}

	#[test]
	fn project_manifest_roundtrips_through_json() {
		let mut document = Document::new(800, 600);
		document.add_layer("Paint");
		assert!(document.add_layer_mask(1));
		assert!(document.set_layer_mask_enabled(1, false));

		let manifest = ProjectManifest::from(&document);
		let json = serde_json::to_string_pretty(&manifest).expect("manifest should serialize");
		let restored: ProjectManifest = serde_json::from_str(&json).expect("manifest should deserialize");

		assert_eq!(restored.layers.len(), 2);
		assert_eq!(restored.active_edit_target, doc_model::LayerEditTarget::LayerPixels);
		assert_eq!(restored.layers[1].name, "Paint");
		assert!(!restored.layers[1].mask_enabled);
		assert!(restored.layers[1].mask_payload_path.is_some());
		assert_eq!(restored.layers[1].offset_x, 0);
		assert!(restored.layers[1].payload_path.starts_with("layers/"));
		assert!(restored.layers[1].payload_path.ends_with(".png"));
	}

	#[test]
	fn project_file_roundtrips_tile_payloads() {
		let mut document = Document::new(512, 512);
		let tile = document
			.ensure_tile_for_pixel(0, 42, 11)
			.expect("tile should be created");
		tile.pixels[0] = 255;
		tile.pixels[3] = 255;
		assert!(document.add_layer_mask(0));
		let tile_size = document.tile_size as usize;
		let mask_tile_size = document.tile_size;
		let mask = document.layer_mask_mut(0).expect("mask should exist");
		let mask_tile = mask.ensure_tile(TileCoord::new(0, 0), mask_tile_size);
		mask_tile.alpha[11 * tile_size + 42] = 128;
		assert!(document.set_active_edit_target(doc_model::LayerEditTarget::LayerMask));

		let project_file = ProjectFile::from(&document);
		let restored = Document::try_from(project_file).expect("project file should restore document");

		assert_eq!(restored.canvas_size.width, 512);
		assert_eq!(restored.layers.len(), 1);
		assert_eq!(restored.layers[0].tiles.len(), 1);
		assert_eq!(restored.active_edit_target, doc_model::LayerEditTarget::LayerMask);
		assert!(restored.layer_mask(0).is_some());
		assert_eq!(
			restored.layer_mask(0).expect("mask exists").tiles[&TileCoord::new(0, 0)].alpha[11 * tile_size + 42],
			128
		);
	}

	#[test]
	fn save_and_load_document_roundtrip() {
		let mut document = Document::new(512, 512);
		document.add_layer("Ink");
		let layer_index = document.active_layer_index();
		let tile = document
			.ensure_tile_for_pixel(layer_index, 300, 300)
			.expect("tile should be created");
		tile.pixels[1] = 200;
		tile.pixels[3] = 255;

		let path = temporary_project_path();
		save_document_to_path(&path, &document).expect("save should succeed");
		let restored = load_document_from_path(&path).expect("load should succeed");
		fs::remove_file(&path).expect("temporary project file should be removed");

		assert_eq!(restored.canvas_size, document.canvas_size);
		assert_eq!(restored.layers.len(), document.layers.len());
		assert_eq!(restored.layers[1].name, "Ink");
		assert_eq!(restored.layers[1].tiles.len(), 1);
	}

	#[test]
	fn flatten_document_includes_layer_pixels() {
		let mut document = Document::new(4, 4);
		let tile_size = document.tile_size as usize;
		let tile = document
			.ensure_tile_for_pixel(0, 1, 1)
			.expect("tile should be created");
		let pixel_index = (1 * tile_size + 1) * 4;
		tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(&[10, 20, 30, 255]);

		let flattened = flatten_document_rgba(&document);
		let output_index = (1 * 4 + 1) * 4;
		assert_eq!(&flattened[output_index..output_index + 4], &[10, 20, 30, 255]);
	}

	#[test]
	fn export_and_import_png_roundtrip() {
		let mut document = Document::new(8, 8);
		let tile_size = document.tile_size as usize;
		let tile = document
			.ensure_tile_for_pixel(0, 2, 3)
			.expect("tile should be created");
		let pixel_index = (3 * tile_size + 2) * 4;
		tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(&[200, 100, 50, 255]);

		let path = std::env::temp_dir().join(format!("phototux-png-{}.png", temporary_suffix()));
		export_png_to_path(&path, &document).expect("png export should succeed");
		let restored = import_png_from_path(&path).expect("png import should succeed");
		fs::remove_file(&path).expect("temporary png should be removed");

		assert_eq!(restored.canvas_size.width, 8);
		assert_eq!(restored.canvas_size.height, 8);
		let restored_tile = restored.layer(0).expect("layer exists").tiles.get(&doc_model::TileCoord::new(0, 0)).expect("tile exists");
		assert_eq!(&restored_tile.pixels[pixel_index..pixel_index + 4], &[200, 100, 50, 255]);
	}

	#[test]
	fn export_and_import_jpeg_roundtrip() {
		let mut document = Document::new(8, 8);
		let tile_size = document.tile_size as usize;
		let tile = document
			.ensure_tile_for_pixel(0, 2, 3)
			.expect("tile should be created");
		for pixel_y in 0..8 {
			for pixel_x in 0..8 {
				let pixel_index = (pixel_y * tile_size + pixel_x) * 4;
				tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(&[200, 100, 50, 255]);
			}
		}

		let path = std::env::temp_dir().join(format!("phototux-jpeg-{}.jpg", temporary_suffix()));
		export_jpeg_to_path(&path, &document).expect("jpeg export should succeed");
		let restored = import_jpeg_from_path(&path).expect("jpeg import should succeed");
		fs::remove_file(&path).expect("temporary jpeg should be removed");

		assert_eq!(restored.canvas_size.width, 8);
		assert_eq!(restored.canvas_size.height, 8);
		let restored_tile = restored
			.layer(0)
			.expect("layer exists")
			.tiles
			.get(&doc_model::TileCoord::new(0, 0))
			.expect("tile exists");
		let pixel_index = (3 * tile_size + 2) * 4;
		let restored = &restored_tile.pixels[pixel_index..pixel_index + 4];
		assert!((restored[0] as i16 - 200).abs() <= 20);
		assert!((restored[1] as i16 - 100).abs() <= 20);
		assert!((restored[2] as i16 - 50).abs() <= 20);
		assert_eq!(restored[3], 255);
	}

	#[test]
	fn export_and_import_webp_roundtrip() {
		let mut document = Document::new(8, 8);
		let tile_size = document.tile_size as usize;
		let tile = document
			.ensure_tile_for_pixel(0, 2, 3)
			.expect("tile should be created");
		let pixel_index = (3 * tile_size + 2) * 4;
		tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(&[40, 160, 220, 200]);

		let path = std::env::temp_dir().join(format!("phototux-webp-{}.webp", temporary_suffix()));
		export_webp_to_path(&path, &document).expect("webp export should succeed");
		let restored = import_webp_from_path(&path).expect("webp import should succeed");
		fs::remove_file(&path).expect("temporary webp should be removed");

		assert_eq!(restored.canvas_size.width, 8);
		assert_eq!(restored.canvas_size.height, 8);
		let restored_tile = restored
			.layer(0)
			.expect("layer exists")
			.tiles
			.get(&doc_model::TileCoord::new(0, 0))
			.expect("tile exists");
		let restored = &restored_tile.pixels[pixel_index..pixel_index + 4];
		assert!((restored[0] as i16 - 40).abs() <= 10);
		assert!((restored[1] as i16 - 160).abs() <= 10);
		assert!((restored[2] as i16 - 220).abs() <= 10);
		assert!((restored[3] as i16 - 200).abs() <= 15);
	}

	#[test]
	fn malformed_jpeg_and_webp_imports_return_contextual_errors() {
		let jpeg_path = std::env::temp_dir().join(format!("phototux-bad-{}.jpg", temporary_suffix()));
		fs::write(&jpeg_path, b"not a jpeg").expect("invalid jpeg fixture should be written");
		let jpeg_error = import_jpeg_from_path(&jpeg_path).expect_err("invalid jpeg should fail");
		fs::remove_file(&jpeg_path).expect("temporary jpeg should be removed");
		assert!(jpeg_error.to_string().contains("failed to import JPEG"));

		let webp_path = std::env::temp_dir().join(format!("phototux-bad-{}.webp", temporary_suffix()));
		fs::write(&webp_path, b"not a webp").expect("invalid webp fixture should be written");
		let webp_error = import_webp_from_path(&webp_path).expect_err("invalid webp should fail");
		fs::remove_file(&webp_path).expect("temporary webp should be removed");
		assert!(webp_error.to_string().contains("failed to import WebP"));
	}

	#[test]
	fn flatten_document_respects_layer_order_visibility_and_opacity() {
		let mut document = Document::new(4, 4);
		let tile_size = document.tile_size as usize;
		let base_tile = document
			.ensure_tile_for_pixel(0, 1, 1)
			.expect("base tile should exist");
		base_tile.pixels[(1 * tile_size + 1) * 4..(1 * tile_size + 1) * 4 + 4]
			.copy_from_slice(&[40, 80, 120, 255]);

		document.add_layer("Top");
		let top_index = document.active_layer_index();
		let top_tile = document
			.ensure_tile_for_pixel(top_index, 1, 1)
			.expect("top tile should exist");
		top_tile.pixels[(1 * tile_size + 1) * 4..(1 * tile_size + 1) * 4 + 4]
			.copy_from_slice(&[200, 10, 10, 255]);
		document.set_layer_opacity(top_index, 50);

		let flattened = flatten_document_rgba(&document);
		let index = (1 * 4 + 1) * 4;
		assert_eq!(&flattened[index..index + 4], &[120, 45, 65, 255]);

		document.set_layer_visibility(top_index, false);
		let hidden = flatten_document_rgba(&document);
		assert_eq!(&hidden[index..index + 4], &[40, 80, 120, 255]);
	}

	#[test]
	fn flatten_document_applies_initial_blend_modes() {
		let mut document = Document::new(4, 4);
		let tile_size = document.tile_size as usize;
		let base_tile = document
			.ensure_tile_for_pixel(0, 1, 1)
			.expect("base tile should exist");
		base_tile.pixels[(1 * tile_size + 1) * 4..(1 * tile_size + 1) * 4 + 4]
			.copy_from_slice(&[128, 128, 128, 255]);

		document.add_layer("Top");
		let top_index = document.active_layer_index();
		let top_tile = document
			.ensure_tile_for_pixel(top_index, 1, 1)
			.expect("top tile should exist");
		top_tile.pixels[(1 * tile_size + 1) * 4..(1 * tile_size + 1) * 4 + 4]
			.copy_from_slice(&[128, 64, 255, 255]);

		let index = (1 * 4 + 1) * 4;

		document.set_layer_blend_mode(top_index, BlendMode::Multiply);
		let multiply = flatten_document_rgba(&document);
		assert_eq!(&multiply[index..index + 4], &[64, 32, 128, 255]);

		document.set_layer_blend_mode(top_index, BlendMode::Darken);
		let darken = flatten_document_rgba(&document);
		assert_eq!(&darken[index..index + 4], &[128, 64, 128, 255]);

		document.set_layer_blend_mode(top_index, BlendMode::Lighten);
		let lighten = flatten_document_rgba(&document);
		assert_eq!(&lighten[index..index + 4], &[128, 128, 255, 255]);
	}

	#[test]
	fn project_roundtrip_preserves_multiple_layers() {
		let mut document = Document::new(16, 16);
		let tile_size = document.tile_size as usize;
		document.rename_layer(0, "Background");
		document.add_layer("Highlights");
		let top_index = document.active_layer_index();
		document.set_layer_opacity(top_index, 75);
		assert!(document.add_layer_mask(top_index));
		let tile = document
			.ensure_tile_for_pixel(top_index, 2, 2)
			.expect("top layer tile should exist");
		tile.pixels[(2 * tile_size + 2) * 4..(2 * tile_size + 2) * 4 + 4]
			.copy_from_slice(&[250, 250, 250, 255]);

		let path = temporary_project_path();
		save_document_to_path(&path, &document).expect("save should succeed");
		let restored = load_document_from_path(&path).expect("load should succeed");
		fs::remove_file(&path).expect("temporary project file should be removed");

		assert_eq!(restored.layers.len(), 2);
		assert_eq!(restored.layers[0].name, "Background");
		assert_eq!(restored.layers[1].name, "Highlights");
		assert_eq!(restored.layers[1].opacity_percent, 75);
		assert_eq!(restored.layers[1].offset_x, 0);
		assert_eq!(restored.layers[1].tiles.len(), 1);
		assert!(restored.layer_mask(1).is_some());
	}

	#[test]
	fn flatten_document_applies_layer_offsets() {
		let mut document = Document::new(6, 6);
		let tile_size = document.tile_size as usize;
		let tile = document
			.ensure_tile_for_pixel(0, 1, 1)
			.expect("tile should be created");
		tile.pixels[(1 * tile_size + 1) * 4..(1 * tile_size + 1) * 4 + 4]
			.copy_from_slice(&[220, 50, 50, 255]);
		assert!(document.set_layer_offset(0, 2, 1));

		let flattened = flatten_document_rgba(&document);
		let shifted_index = (2 * 6 + 3) * 4;
		assert_eq!(&flattened[shifted_index..shifted_index + 4], &[220, 50, 50, 255]);
	}

	#[test]
	fn flatten_document_applies_enabled_layer_masks() {
		let mut document = Document::new(4, 2);
		for y in 0..2 {
			for x in 0..4 {
				set_pixel(&mut document, 0, x, y, [20, 40, 60, 255]);
			}
		}

		document.add_layer("Masked");
		let layer_index = document.active_layer_index();
		for y in 0..2 {
			for x in 0..4 {
				set_pixel(&mut document, layer_index, x, y, [200, 100, 50, 255]);
			}
		}
		assert!(document.add_layer_mask(layer_index));
		set_mask_alpha(&mut document, layer_index, 0, 0, 0);
		set_mask_alpha(&mut document, layer_index, 1, 0, 128);

		let flattened = flatten_document_rgba(&document);
		assert_eq!(&flattened[0..4], &[20, 40, 60, 255]);

		let expected_half = blend_rgba_over(
			[20, 40, 60, 255],
			[200, 100, 50, 128],
			1.0,
			BlendModeMath::Normal,
		);
		assert_eq!(&flattened[4..8], &expected_half);
		assert_eq!(&flattened[8..12], &[200, 100, 50, 255]);
	}

	#[test]
	fn flatten_document_ignores_disabled_layer_masks() {
		let mut document = Document::new(2, 1);
		set_pixel(&mut document, 0, 0, 0, [20, 40, 60, 255]);
		document.add_layer("Masked");
		let layer_index = document.active_layer_index();
		set_pixel(&mut document, layer_index, 0, 0, [200, 100, 50, 255]);
		assert!(document.add_layer_mask(layer_index));
		set_mask_alpha(&mut document, layer_index, 0, 0, 0);
		assert!(document.set_layer_mask_enabled(layer_index, false));

		let flattened = flatten_document_rgba(&document);
		assert_eq!(&flattened[0..4], &[200, 100, 50, 255]);
	}

	#[test]
	fn masked_scene_roundtrip_preserves_flattened_output() {
		let document = build_masked_scene();
		let expected = flatten_document_rgba(&document);

		let path = temporary_project_path();
		save_document_to_path(&path, &document).expect("masked scene should save");
		let restored = load_document_from_path(&path).expect("masked scene should load");
		fs::remove_file(&path).expect("temporary project file should be removed");

		assert_eq!(flatten_document_rgba(&restored), expected);
	}

	#[test]
	fn update_flattened_region_matches_full_flatten_for_masked_scene() {
		let document = build_masked_scene();
		let expected = flatten_document_rgba(&document);
		let mut partial = expected.clone();
		let rect = common::CanvasRect::new(6, 6, 20, 20);

		for y in rect.y as usize..(rect.y + rect.height as i32) as usize {
			for x in rect.x as usize..(rect.x + rect.width as i32) as usize {
				let index = (y * document.canvas_size.width as usize + x) * 4;
				partial[index..index + 4].copy_from_slice(&[1, 2, 3, 4]);
			}
		}

		update_flattened_region_rgba(&document, &mut partial, rect);
		assert_eq!(partial, expected);
	}

	#[test]
	fn representative_scene_roundtrip_preserves_flattened_output() {
		let document = build_representative_scene();
		let expected = flatten_document_rgba(&document);

		let path = temporary_project_path();
		save_document_to_path(&path, &document).expect("representative scene should save");
		let restored = load_document_from_path(&path).expect("representative scene should load");
		fs::remove_file(&path).expect("temporary project file should be removed");

		assert_eq!(flatten_document_rgba(&restored), expected);
	}

	#[test]
	fn repeated_project_roundtrip_preserves_representative_scene() {
		let mut document = build_representative_scene();
		let expected = flatten_document_rgba(&document);

		for _ in 0..3 {
			let path = temporary_project_path();
			save_document_to_path(&path, &document).expect("representative scene should save during repeated roundtrip");
			document = load_document_from_path(&path).expect("representative scene should load during repeated roundtrip");
			fs::remove_file(&path).expect("temporary project file should be removed");
		}

		assert_eq!(flatten_document_rgba(&document), expected);
	}

	#[test]
	fn representative_scene_png_export_matches_flattened_output() {
		let document = build_representative_scene();
		let expected = flatten_document_rgba(&document);

		let path = std::env::temp_dir().join(format!("phototux-scene-{}.png", temporary_suffix()));
		export_png_to_path(&path, &document).expect("representative scene png export should succeed");
		let restored = import_png_from_path(&path).expect("representative scene png import should succeed");
		fs::remove_file(&path).expect("temporary png should be removed");

		assert_eq!(flatten_document_rgba(&restored), expected);
	}

	#[test]
	fn large_sparse_document_save_load_and_export_remain_consistent() {
		let document = build_large_sparse_document();
		let expected = flatten_document_rgba(&document);

		let project_path = temporary_project_path();
		save_document_to_path(&project_path, &document).expect("large sparse document should save");
		let restored = load_document_from_path(&project_path).expect("large sparse document should load");
		fs::remove_file(&project_path).expect("temporary project file should be removed");
		assert_eq!(flatten_document_rgba(&restored), expected);

		let png_path = std::env::temp_dir().join(format!("phototux-large-{}.png", temporary_suffix()));
		export_png_to_path(&png_path, &document).expect("large sparse document png export should succeed");
		let restored_png = import_png_from_path(&png_path).expect("large sparse document png import should succeed");
		fs::remove_file(&png_path).expect("temporary png should be removed");
		assert_eq!(flatten_document_rgba(&restored_png), expected);
	}

	fn temporary_project_path() -> PathBuf {
		let unique = temporary_suffix();
		std::env::temp_dir().join(format!("phototux-{unique}.ptx"))
	}

	#[test]
	fn recovery_path_appends_expected_suffix() {
		let project_path = PathBuf::from("/tmp/example.ptx");
		assert_eq!(
			recovery_path_for_project_path(&project_path),
			PathBuf::from("/tmp/example.ptx.autosave")
		);
	}

	fn temporary_suffix() -> u128 {
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("system time should be after epoch")
			.as_nanos()
	}
}

pub fn update_flattened_region_rgba(document: &Document, output: &mut [u8], rect: common::CanvasRect) {
    // Clear the region
    let start_y = rect.y.max(0) as usize;
    let end_y = (rect.y + rect.height as i32).min(document.canvas_size.height as i32).max(0) as usize;
    let start_x = rect.x.max(0) as usize;
    let end_x = (rect.x + rect.width as i32).min(document.canvas_size.width as i32).max(0) as usize;
    let row_len = (end_x.saturating_sub(start_x)) * 4;

    if row_len == 0 {
        return;
    }

    for y in start_y..end_y {
        let dst_offset = (y * document.canvas_size.width as usize + start_x) * 4;
        output[dst_offset..dst_offset + row_len].fill(0);
    }

    for layer in &document.layers {
        if !layer.visible {
            continue;
        }

        let layer_opacity = layer.opacity_percent as f32 / 100.0;
        for (coord, tile) in &layer.tiles {
			let mask_tile = layer
				.mask
				.as_ref()
				.filter(|mask| mask.enabled)
				.and_then(|mask| mask.tiles.get(coord));
            let (tile_origin_x, tile_origin_y) = document.tile_origin(*coord);
            let tile_canvas_x = tile_origin_x as i32 + layer.offset_x;
            let tile_canvas_y = tile_origin_y as i32 + layer.offset_y;

            if tile_canvas_x + document.tile_size as i32 <= rect.x || tile_canvas_x >= rect.x + rect.width as i32 {
                continue;
            }
            if tile_canvas_y + document.tile_size as i32 <= rect.y || tile_canvas_y >= rect.y + rect.height as i32 {
                continue;
            }

            let canvas_clip_y = tile_canvas_y.max(rect.y).max(0);
            let canvas_clip_h = (tile_canvas_y + document.tile_size as i32).min(rect.y + rect.height as i32).min(document.canvas_size.height as i32);
            let canvas_clip_x = tile_canvas_x.max(rect.x).max(0);
            let canvas_clip_w = (tile_canvas_x + document.tile_size as i32).min(rect.x + rect.width as i32).min(document.canvas_size.width as i32);

            for canvas_y in canvas_clip_y..canvas_clip_h {
                let local_y = (canvas_y - tile_canvas_y) as usize;
                let canvas_y_usize = canvas_y as usize;
                
                for canvas_x in canvas_clip_x..canvas_clip_w {
                    let local_x = (canvas_x - tile_canvas_x) as usize;
                    let src_index = (local_y * document.tile_size as usize + local_x) * 4;
					let mask_alpha = mask_tile
						.map(|tile| tile.alpha[local_y * document.tile_size as usize + local_x])
						.unwrap_or(255);
                    let dst_index = (canvas_y_usize * document.canvas_size.width as usize + canvas_x as usize) * 4;
                    
					composite_masked_pixel(
                        &mut output[dst_index..dst_index + 4],
                        &tile.pixels[src_index..src_index + 4],
						mask_alpha,
                        layer_opacity,
                        layer.blend_mode,
                    );
                }
            }
        }
    }
}
