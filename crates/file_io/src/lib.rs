use common::{CanvasSize, LayerId};
use doc_model::{BlendMode, Document, RasterLayer, RasterTile, TileCoord};
use image::{ImageBuffer, ImageFormat, Rgba};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PROJECT_FILE_EXTENSION: &str = "ptx";
pub const CURRENT_PROJECT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
	pub format_version: u32,
	pub canvas_size: CanvasSize,
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
				offset_x: layer.offset_x,
				offset_y: layer.offset_y,
				payload_path: format!("layers/{}.png", layer.id.0),
			})
			.collect();

		Self {
			format_version: CURRENT_PROJECT_FORMAT_VERSION,
			canvas_size: document.canvas_size,
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
	pub offset_x: i32,
	pub offset_y: i32,
	pub payload_path: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilePayload {
	pub coord: TileCoord,
	pub pixels: Vec<u8>,
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

			restored_layers.push(RasterLayer {
				id: manifest_layer.id,
				name: manifest_layer.name,
				visible: manifest_layer.visible,
				opacity_percent: manifest_layer.opacity_percent,
				blend_mode: manifest_layer.blend_mode,
				offset_x: manifest_layer.offset_x,
				offset_y: manifest_layer.offset_y,
				tiles,
				dirty_tiles: std::collections::HashSet::new(),
			});
		}

		Ok(Document {
			id: common::DocumentId::new(),
			canvas_size: manifest.canvas_size,
			layers: restored_layers,
			active_layer_index: 0,
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

pub fn import_png_from_path(path: &Path) -> anyhow::Result<Document> {
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
					let dst_index =
						(canvas_y as usize * document.canvas_size.width as usize + canvas_x as usize) * 4;
					composite_pixel(
						&mut output[dst_index..dst_index + 4],
						&tile.pixels[src_index..src_index + 4],
						layer_opacity,
						layer.blend_mode,
					);
				}
			}
		}
	}

	output
}

fn composite_pixel(destination: &mut [u8], source: &[u8], layer_opacity: f32, blend_mode: BlendMode) {
	let effective_alpha = (source[3] as f32 / 255.0) * layer_opacity;
	if effective_alpha <= 0.0 {
		return;
	}

	let destination_alpha = destination[3] as f32 / 255.0;
	let output_alpha = effective_alpha + destination_alpha * (1.0 - effective_alpha);

	for channel in 0..3 {
		let source_value = source[channel] as f32 / 255.0;
		let destination_value = destination[channel] as f32 / 255.0;
		let blended = match blend_mode {
			BlendMode::Normal
			| BlendMode::Multiply
			| BlendMode::Screen
			| BlendMode::Overlay
			| BlendMode::Darken
			| BlendMode::Lighten => source_value,
		};
		let output = blended * effective_alpha + destination_value * (1.0 - effective_alpha);
		destination[channel] = (output * 255.0).round().clamp(0.0, 255.0) as u8;
	}

	destination[3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
	use super::{
		export_png_to_path, flatten_document_rgba, import_png_from_path, load_document_from_path,
		save_document_to_path, ProjectFile, ProjectManifest, CURRENT_PROJECT_FORMAT_VERSION,
	};
	use doc_model::Document;
	use std::fs;
	use std::path::PathBuf;
	use std::time::{SystemTime, UNIX_EPOCH};

	#[test]
	fn project_manifest_uses_current_version() {
		let document = Document::new(1920, 1080);
		let manifest = ProjectManifest::from(&document);

		assert_eq!(manifest.format_version, CURRENT_PROJECT_FORMAT_VERSION);
		assert_eq!(manifest.canvas_size.width, 1920);
		assert_eq!(manifest.layers.len(), 1);
	}

	#[test]
	fn project_manifest_roundtrips_through_json() {
		let mut document = Document::new(800, 600);
		document.add_layer("Paint");

		let manifest = ProjectManifest::from(&document);
		let json = serde_json::to_string_pretty(&manifest).expect("manifest should serialize");
		let restored: ProjectManifest = serde_json::from_str(&json).expect("manifest should deserialize");

		assert_eq!(restored.layers.len(), 2);
		assert_eq!(restored.layers[1].name, "Paint");
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

		let project_file = ProjectFile::from(&document);
		let restored = Document::try_from(project_file).expect("project file should restore document");

		assert_eq!(restored.canvas_size.width, 512);
		assert_eq!(restored.layers.len(), 1);
		assert_eq!(restored.layers[0].tiles.len(), 1);
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
	fn project_roundtrip_preserves_multiple_layers() {
		let mut document = Document::new(16, 16);
		let tile_size = document.tile_size as usize;
		document.rename_layer(0, "Background");
		document.add_layer("Highlights");
		let top_index = document.active_layer_index();
		document.set_layer_opacity(top_index, 75);
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

	fn temporary_project_path() -> PathBuf {
		let unique = temporary_suffix();
		std::env::temp_dir().join(format!("phototux-{unique}.ptx"))
	}

	fn temporary_suffix() -> u128 {
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("system time should be after epoch")
			.as_nanos()
	}
}
