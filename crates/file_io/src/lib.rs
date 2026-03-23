use anyhow::Context;
use color_math::{blend_rgba_over, BlendModeMath};
use common::{CanvasSize, GroupId, LayerId};
use doc_model::{
	BlendMode, Document, Guide, GuideOrientation, LayerEditTarget, LayerGroup,
	LayerHierarchyNode, MaskTile, RasterLayer, RasterTile, TileCoord,
};
use image::{ImageBuffer, ImageFormat, Rgb, Rgba};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const PROJECT_FILE_EXTENSION: &str = "ptx";
pub const CURRENT_PROJECT_FORMAT_VERSION: u32 = 1;
pub const CURRENT_PSD_IMPORT_MANIFEST_VERSION: u32 = 1;
pub const RECOVERY_FILE_SUFFIX: &str = ".autosave";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PsdImportSourceKind {
	Psd,
	#[serde(other)]
	Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PsdImportColorMode {
	Rgb,
	Grayscale,
	Indexed,
	Cmyk,
	Multichannel,
	Duotone,
	Lab,
	Bitmap,
	#[serde(other)]
	Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PsdImportLayerKind {
	Raster,
	Group,
	Text,
	SmartObject,
	Adjustment,
	ClippingMask,
	#[serde(other)]
	Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PsdImportDiagnosticSeverity {
	Info,
	Warning,
	Error,
	#[serde(other)]
	Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdImportCanvasRecord {
	pub width_px: u32,
	pub height_px: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdImportOffsetRecord {
	pub x: i32,
	pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdImportBoundsRecord {
	pub left: i32,
	pub top: i32,
	pub width: u32,
	pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdImportCompositeRecord {
	pub available: bool,
	pub asset_relpath: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdImportDiagnostic {
	pub severity: PsdImportDiagnosticSeverity,
	pub code: String,
	pub message: String,
	pub source_index: Option<usize>,
}

impl PsdImportDiagnostic {
	fn warning(code: impl Into<String>, message: impl Into<String>, source_index: Option<usize>) -> Self {
		Self {
			severity: PsdImportDiagnosticSeverity::Warning,
			code: code.into(),
			message: message.into(),
			source_index,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdImportLayerRecord {
	pub source_index: usize,
	pub kind: PsdImportLayerKind,
	pub name: String,
	pub visible: bool,
	pub opacity_0_255: u8,
	pub blend_key: String,
	pub offset_px: PsdImportOffsetRecord,
	pub bounds_px: PsdImportBoundsRecord,
	pub raster_asset_relpath: Option<String>,
	#[serde(default)]
	pub unsupported_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdImportManifest {
	pub manifest_version: u32,
	pub source_kind: PsdImportSourceKind,
	pub source_color_mode: PsdImportColorMode,
	pub source_depth_bits: u8,
	pub canvas: PsdImportCanvasRecord,
	pub composite: PsdImportCompositeRecord,
	#[serde(default)]
	pub diagnostics: Vec<PsdImportDiagnostic>,
	#[serde(default)]
	pub layers: Vec<PsdImportLayerRecord>,
}

#[derive(Debug, Clone)]
pub struct PsdImportResult {
	pub document: Document,
	pub diagnostics: Vec<PsdImportDiagnostic>,
	pub used_flattened_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct PsdImportSidecar {
	executable_path: PathBuf,
	base_args: Vec<OsString>,
}

impl PsdImportSidecar {
	pub fn new(executable_path: impl Into<PathBuf>) -> Self {
		Self {
			executable_path: executable_path.into(),
			base_args: Vec::new(),
		}
	}

	pub fn with_arg(mut self, arg: impl Into<OsString>) -> Self {
		self.base_args.push(arg.into());
		self
	}

	pub fn with_args<I, S>(mut self, args: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<OsString>,
	{
		self.base_args.extend(args.into_iter().map(Into::into));
		self
	}

	pub fn executable_path(&self) -> &Path {
		&self.executable_path
	}

	pub fn base_args(&self) -> &[OsString] {
		&self.base_args
	}
	}

#[derive(Debug)]
struct PsdImportWorkspace {
	root_dir: PathBuf,
	manifest_path: PathBuf,
}

impl PsdImportWorkspace {
	fn create() -> anyhow::Result<Self> {
		let unique_suffix = format!(
			"{}-{}",
			std::process::id(),
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.expect("system time should be after epoch")
				.as_nanos()
		);
		let root_dir = std::env::temp_dir().join(format!("phototux-psd-import-{unique_suffix}"));
		fs::create_dir_all(&root_dir).with_context(|| {
			format!("failed to create PSD import workspace {}", root_dir.display())
		})?;
		let manifest_path = root_dir.join("manifest.json");
		Ok(Self {
			root_dir,
			manifest_path,
		})
	}

	fn root_dir(&self) -> &Path {
		&self.root_dir
	}

	fn manifest_path(&self) -> &Path {
		&self.manifest_path
	}
}

impl Drop for PsdImportWorkspace {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.root_dir);
	}
}

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
	#[serde(default)]
	pub layer_hierarchy: Vec<ManifestHierarchyNode>,
	#[serde(default)]
	pub guides: Vec<ManifestGuideRecord>,
	#[serde(default = "default_guides_visible")]
	pub guides_visible: bool,
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
			layer_hierarchy: document
				.layer_hierarchy
				.iter()
				.map(ManifestHierarchyNode::from)
				.collect(),
			guides: document.guides().iter().map(ManifestGuideRecord::from).collect(),
			guides_visible: document.guides_visible(),
		}
	}
}

fn default_guides_visible() -> bool {
	true
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestGuideRecord {
	pub orientation: GuideOrientation,
	pub position: i32,
}

impl From<&Guide> for ManifestGuideRecord {
	fn from(guide: &Guide) -> Self {
		Self {
			orientation: guide.orientation,
			position: guide.position,
		}
	}
}

impl From<&ManifestGuideRecord> for Guide {
	fn from(guide: &ManifestGuideRecord) -> Self {
		Guide {
			orientation: guide.orientation,
			position: guide.position,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ManifestHierarchyNode {
	Layer(LayerId),
	Group(ManifestGroupRecord),
}

impl From<&LayerHierarchyNode> for ManifestHierarchyNode {
	fn from(node: &LayerHierarchyNode) -> Self {
		match node {
			LayerHierarchyNode::Layer(layer_id) => Self::Layer(*layer_id),
			LayerHierarchyNode::Group(group) => Self::Group(ManifestGroupRecord::from(group)),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestGroupRecord {
	pub id: GroupId,
	pub name: String,
	pub visible: bool,
	pub opacity_percent: u8,
	pub children: Vec<ManifestHierarchyNode>,
}

impl From<&LayerGroup> for ManifestGroupRecord {
	fn from(group: &LayerGroup) -> Self {
		Self {
			id: group.id,
			name: group.name.clone(),
			visible: group.visible,
			opacity_percent: group.opacity_percent,
			children: group.children.iter().map(ManifestHierarchyNode::from).collect(),
		}
	}
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

		let fallback_hierarchy = restored_layers
			.iter()
			.map(|layer| LayerHierarchyNode::Layer(layer.id))
			.collect::<Vec<_>>();
		let restored_hierarchy = if manifest.layer_hierarchy.is_empty() {
			fallback_hierarchy.clone()
		} else {
			manifest
				.layer_hierarchy
				.iter()
				.map(manifest_hierarchy_to_document)
				.collect()
		};

		let mut document = Document {
			id: common::DocumentId::new(),
			canvas_size: manifest.canvas_size,
			layer_hierarchy: fallback_hierarchy,
			layers: restored_layers,
			active_layer_index: 0,
			active_edit_target: manifest.active_edit_target,
			tile_size: common::DEFAULT_TILE_SIZE,
			selection: None,
			selection_inverted: false,
			guides: manifest.guides.iter().map(Guide::from).collect(),
			guides_visible: manifest.guides_visible,
		};
		document
			.set_layer_hierarchy(restored_hierarchy)
			.map_err(|error| anyhow::anyhow!(error))?;

		Ok(document)
	}
}

fn manifest_hierarchy_to_document(node: &ManifestHierarchyNode) -> LayerHierarchyNode {
	match node {
		ManifestHierarchyNode::Layer(layer_id) => LayerHierarchyNode::Layer(*layer_id),
		ManifestHierarchyNode::Group(group) => LayerHierarchyNode::Group(LayerGroup {
			id: group.id,
			name: group.name.clone(),
			visible: group.visible,
			opacity_percent: group.opacity_percent,
			children: group.children.iter().map(manifest_hierarchy_to_document).collect(),
		}),
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

pub fn load_psd_import_manifest_from_path(path: &Path) -> anyhow::Result<PsdImportManifest> {
	let bytes = fs::read(path)
		.with_context(|| format!("failed to read PSD import manifest from {}", path.display()))?;
	serde_json::from_slice(&bytes)
		.with_context(|| format!("failed to parse PSD import manifest from {}", path.display()))
}

pub fn import_psd_from_manifest_path(path: &Path) -> anyhow::Result<PsdImportResult> {
	let manifest = load_psd_import_manifest_from_path(path)?;
	let manifest_dir = path.parent().ok_or_else(|| {
		anyhow::anyhow!("PSD import manifest path {} has no parent directory", path.display())
	})?;

	validate_psd_import_manifest(&manifest)?;

	let mut diagnostics = manifest.diagnostics.clone();
	let layered_limitations = collect_psd_layered_import_limitations(&manifest);
	if layered_limitations.is_empty() {
		let document = import_psd_layers_from_manifest(&manifest, manifest_dir)?;
		return Ok(PsdImportResult {
			document,
			diagnostics,
			used_flattened_fallback: false,
		});
	}

	diagnostics.extend(layered_limitations.iter().cloned());
	if manifest.composite.available {
		let document = import_psd_composite_fallback(&manifest, manifest_dir)?;
		diagnostics.push(PsdImportDiagnostic::warning(
			"flattened_fallback_used",
			"Imported the flattened PSD composite because the source exceeded PhotoTux's currently supported layered PSD subset.",
			None,
		));
		return Ok(PsdImportResult {
			document,
			diagnostics,
			used_flattened_fallback: true,
		});
	}

	anyhow::bail!(
		"PSD import exceeds the supported subset and no flattened composite fallback is available: {}",
		summarize_psd_diagnostics(&layered_limitations)
	);
}

pub fn import_psd_from_path_with_sidecar(
	psd_path: &Path,
	sidecar: &PsdImportSidecar,
) -> anyhow::Result<PsdImportResult> {
	if !psd_path.exists() {
		anyhow::bail!("PSD source file does not exist: {}", psd_path.display());
	}

	let workspace = PsdImportWorkspace::create()?;
	run_psd_import_sidecar(sidecar, psd_path, &workspace)?;
	if !workspace.manifest_path().exists() {
		anyhow::bail!(
			"PSD sidecar completed without producing a manifest at {}",
			workspace.manifest_path().display()
		);
	}

	import_psd_from_manifest_path(workspace.manifest_path())
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

fn validate_psd_import_manifest(manifest: &PsdImportManifest) -> anyhow::Result<()> {
	if manifest.manifest_version != CURRENT_PSD_IMPORT_MANIFEST_VERSION {
		anyhow::bail!(
			"unsupported PSD import manifest version: expected {}, got {}",
			CURRENT_PSD_IMPORT_MANIFEST_VERSION,
			manifest.manifest_version
		);
	}
	if manifest.source_kind != PsdImportSourceKind::Psd {
		anyhow::bail!("unsupported PSD import manifest source kind");
	}
	if manifest.canvas.width_px == 0 || manifest.canvas.height_px == 0 {
		anyhow::bail!("PSD import manifest canvas dimensions must be non-zero");
	}
	if manifest.composite.available && manifest.composite.asset_relpath.is_none() {
		anyhow::bail!("PSD import manifest marks a composite fallback as available but omits its asset path");
	}
	Ok(())
}

fn run_psd_import_sidecar(
	sidecar: &PsdImportSidecar,
	psd_path: &Path,
	workspace: &PsdImportWorkspace,
) -> anyhow::Result<()> {
	let output = Command::new(sidecar.executable_path())
		.args(sidecar.base_args())
		.arg(psd_path)
		.arg(workspace.root_dir())
		.arg(workspace.manifest_path())
		.output()
		.with_context(|| {
			format!(
				"failed to start PSD import sidecar {}",
				sidecar.executable_path().display()
			)
		})?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
		let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
		let detail = if !stderr.is_empty() {
			stderr
		} else if !stdout.is_empty() {
			stdout
		} else {
			format!("process exited with status {}", output.status)
		};
		anyhow::bail!(
			"PSD import sidecar failed for {}: {}",
			psd_path.display(),
			detail
		);
	}

	Ok(())
}

fn collect_psd_layered_import_limitations(manifest: &PsdImportManifest) -> Vec<PsdImportDiagnostic> {
	let mut diagnostics = Vec::new();

	if manifest.source_color_mode != PsdImportColorMode::Rgb {
		diagnostics.push(PsdImportDiagnostic::warning(
			"unsupported_color_mode",
			format!(
				"Unsupported PSD color mode {:?}; current layered PSD import supports RGB only.",
				manifest.source_color_mode
			),
			None,
		));
	}
	if manifest.source_depth_bits != 8 {
		diagnostics.push(PsdImportDiagnostic::warning(
			"unsupported_bit_depth",
			format!(
				"Unsupported PSD bit depth {}; current layered PSD import supports 8-bit documents only.",
				manifest.source_depth_bits
			),
			None,
		));
	}
	if manifest.layers.is_empty() {
		diagnostics.push(PsdImportDiagnostic::warning(
			"no_raster_layers_available",
			"The PSD manifest does not expose any importable layers for the current subset.",
			None,
		));
	}

	for layer in &manifest.layers {
		if layer.kind != PsdImportLayerKind::Raster {
			diagnostics.push(PsdImportDiagnostic::warning(
				"unsupported_layer_kind",
				format!(
					"Layer '{}' uses unsupported kind {:?} for the current layered PSD subset.",
					layer.name, layer.kind
				),
				Some(layer.source_index),
			));
		}
		if !layer.unsupported_features.is_empty() {
			diagnostics.push(PsdImportDiagnostic::warning(
				"unsupported_layer_features",
				format!(
					"Layer '{}' includes unsupported features: {}.",
					layer.name,
					layer.unsupported_features.join(", ")
				),
				Some(layer.source_index),
			));
		}
		if map_psd_blend_key(&layer.blend_key).is_none() {
			diagnostics.push(PsdImportDiagnostic::warning(
				"unsupported_blend_mode",
				format!(
					"Layer '{}' uses unsupported PSD blend key '{}'.",
					layer.name, layer.blend_key
				),
				Some(layer.source_index),
			));
		}
	}

	diagnostics
}

fn summarize_psd_diagnostics(diagnostics: &[PsdImportDiagnostic]) -> String {
	diagnostics
		.iter()
		.map(|diagnostic| diagnostic.message.clone())
		.collect::<Vec<_>>()
		.join("; ")
}

fn import_psd_layers_from_manifest(
	manifest: &PsdImportManifest,
	manifest_dir: &Path,
) -> anyhow::Result<Document> {
	let mut document = Document::new(manifest.canvas.width_px, manifest.canvas.height_px);
	for (layer_position, layer_record) in manifest.layers.iter().enumerate() {
		let layer_index = ensure_import_layer_slot(&mut document, layer_position, &layer_record.name)?;
		let blend_mode = map_psd_blend_key(&layer_record.blend_key).ok_or_else(|| {
			anyhow::anyhow!(
				"layer '{}' uses unsupported PSD blend key '{}'",
				layer_record.name,
				layer_record.blend_key
			)
		})?;
		let raster_asset_relpath = layer_record.raster_asset_relpath.as_ref().ok_or_else(|| {
			anyhow::anyhow!(
				"layer '{}' is missing its raster asset path in the PSD import manifest",
				layer_record.name
			)
		})?;
		let raster_asset_path = manifest_dir.join(raster_asset_relpath);
		let decoded = image::open(&raster_asset_path)
			.with_context(|| format!("failed to load PSD layer asset {}", raster_asset_path.display()))?
			.to_rgba8();
		let (asset_width, asset_height) = decoded.dimensions();
		validate_psd_layer_asset_bounds(layer_record, asset_width, asset_height)?;

		document.rename_layer(layer_index, layer_record.name.clone());
		document.set_layer_visibility(layer_index, layer_record.visible);
		document.set_layer_opacity(layer_index, psd_opacity_to_percent(layer_record.opacity_0_255));
		document.set_layer_blend_mode(layer_index, blend_mode);
		let _ = document.set_layer_offset(layer_index, layer_record.offset_px.x, layer_record.offset_px.y);
		write_rgba_image_into_layer_local(
			&mut document,
			layer_index,
			asset_width,
			asset_height,
			decoded.as_raw(),
		)?;
	}

	let top_index = manifest.layers.len().saturating_sub(1);
	let _ = document.set_active_layer(top_index);
	Ok(document)
}

fn import_psd_composite_fallback(
	manifest: &PsdImportManifest,
	manifest_dir: &Path,
) -> anyhow::Result<Document> {
	let composite_relpath = manifest.composite.asset_relpath.as_ref().ok_or_else(|| {
		anyhow::anyhow!("PSD import manifest requested composite fallback but omitted its asset path")
	})?;
	let composite_path = manifest_dir.join(composite_relpath);
	let decoded = image::open(&composite_path)
		.with_context(|| format!("failed to load PSD composite fallback {}", composite_path.display()))?
		.to_rgba8();
	let (width, height) = decoded.dimensions();
	if width != manifest.canvas.width_px || height != manifest.canvas.height_px {
		anyhow::bail!(
			"PSD composite fallback dimensions {}x{} do not match manifest canvas {}x{}",
			width,
			height,
			manifest.canvas.width_px,
			manifest.canvas.height_px
		);
	}

	let mut document = Document::new(width, height);
	document.rename_layer(0, "Flattened PSD Import");
	write_rgba_image_into_layer_local(&mut document, 0, width, height, decoded.as_raw())?;
	Ok(document)
}

fn ensure_import_layer_slot(
	document: &mut Document,
	layer_position: usize,
	layer_name: &str,
) -> anyhow::Result<usize> {
	if layer_position == 0 {
		return Ok(0);
	}

	let active_index = layer_position - 1;
	if !document.set_active_layer(active_index) {
		anyhow::bail!("failed to select layer {} before inserting PSD layer '{}'", active_index, layer_name);
	}
	document.add_layer(layer_name.to_string());
	Ok(document.active_layer_index())
}

fn validate_psd_layer_asset_bounds(
	layer_record: &PsdImportLayerRecord,
	asset_width: u32,
	asset_height: u32,
) -> anyhow::Result<()> {
	if layer_record.bounds_px.left != layer_record.offset_px.x
		|| layer_record.bounds_px.top != layer_record.offset_px.y
	{
		anyhow::bail!(
			"layer '{}' has inconsistent offset and bounds origin in the PSD import manifest",
			layer_record.name
		);
	}
	if layer_record.bounds_px.width != asset_width || layer_record.bounds_px.height != asset_height {
		anyhow::bail!(
			"layer '{}' asset dimensions {}x{} do not match manifest bounds {}x{}",
			layer_record.name,
			asset_width,
			asset_height,
			layer_record.bounds_px.width,
			layer_record.bounds_px.height
		);
	}
	Ok(())
}

fn write_rgba_image_into_layer_local(
	document: &mut Document,
	layer_index: usize,
	width: u32,
	height: u32,
	pixels: &[u8],
) -> anyhow::Result<()> {
	let expected_len = (width as usize)
		.checked_mul(height as usize)
		.and_then(|pixel_count| pixel_count.checked_mul(4))
		.ok_or_else(|| anyhow::anyhow!("PSD raster asset dimensions overflowed pixel buffer sizing"))?;
	if pixels.len() != expected_len {
		anyhow::bail!(
			"PSD raster asset buffer length {} does not match expected RGBA length {}",
			pixels.len(),
			expected_len
		);
	}

	let tile_size = document.tile_size;
	for pixel_y in 0..height {
		for pixel_x in 0..width {
			let source_index = ((pixel_y * width + pixel_x) as usize) * 4;
			let rgba = &pixels[source_index..source_index + 4];
			if rgba[3] == 0 {
				continue;
			}

			let coord = TileCoord::new(pixel_x / tile_size, pixel_y / tile_size);
			let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
			let local_x = (pixel_x - tile_origin_x) as usize;
			let local_y = (pixel_y - tile_origin_y) as usize;
			let pixel_index = (local_y * tile_size as usize + local_x) * 4;
			let layer = document.layer_mut(layer_index).ok_or_else(|| {
				anyhow::anyhow!("PSD import target layer {} is unavailable", layer_index)
			})?;
			let tile = layer.ensure_tile(coord, tile_size);
			tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(rgba);
		}
	}

	Ok(())
}

fn map_psd_blend_key(key: &str) -> Option<BlendMode> {
	match key.trim_end() {
		"norm" => Some(BlendMode::Normal),
		"mul" => Some(BlendMode::Multiply),
		"scrn" => Some(BlendMode::Screen),
		"over" => Some(BlendMode::Overlay),
		"dark" => Some(BlendMode::Darken),
		"lite" => Some(BlendMode::Lighten),
		_ => None,
	}
}

fn psd_opacity_to_percent(opacity_0_255: u8) -> u8 {
	((u16::from(opacity_0_255) * 100 + 127) / 255) as u8
}

pub fn flatten_document_rgba(document: &Document) -> Vec<u8> {
	let mut output = vec![0_u8; (document.canvas_size.width * document.canvas_size.height * 4) as usize];
	composite_hierarchy_into(document, &mut output, None);

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
		import_jpeg_from_path, import_png_from_path, import_psd_from_manifest_path,
		import_psd_from_path_with_sidecar, import_webp_from_path, load_document_from_path,
		recovery_path_for_project_path, save_document_to_path, update_flattened_region_rgba,
		ManifestHierarchyNode, ProjectFile, ProjectManifest, PsdImportBoundsRecord,
		PsdImportCanvasRecord, PsdImportColorMode, PsdImportCompositeRecord,
		PsdImportDiagnostic, PsdImportDiagnosticSeverity, PsdImportLayerKind,
		PsdImportLayerRecord, PsdImportManifest, PsdImportOffsetRecord,
		PsdImportSidecar, PsdImportSourceKind, CURRENT_PROJECT_FORMAT_VERSION,
		CURRENT_PSD_IMPORT_MANIFEST_VERSION,
	};
	use color_math::{blend_rgba_over, BlendModeMath};
	use doc_model::{BlendMode, Document, LayerGroup, LayerHierarchyNode, TileCoord};
	use image::{ImageBuffer, ImageFormat, Rgba};
	#[cfg(unix)]
	use std::os::unix::fs::PermissionsExt;
	use std::fs;
	use std::path::{Path, PathBuf};
	#[cfg(unix)]
	use std::process::Command;
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

	fn build_grouped_scene() -> Document {
		let mut document = Document::new(16, 16);
		document.rename_layer(0, "Background");
		for y in 0..16 {
			for x in 0..16 {
				set_pixel(&mut document, 0, x, y, [20, 30, 40, 255]);
			}
		}

		document.add_layer("Warm Tint");
		let warm_index = document.active_layer_index();
		for y in 2..14 {
			for x in 2..14 {
				set_pixel(&mut document, warm_index, x, y, [180, 80, 40, 255]);
			}
		}

		document.add_layer("Cool Accent");
		let cool_index = document.active_layer_index();
		for y in 4..12 {
			for x in 4..12 {
				set_pixel(&mut document, cool_index, x, y, [40, 140, 220, 255]);
			}
		}

		document.add_layer("Top Highlight");
		let top_index = document.active_layer_index();
		for y in 6..10 {
			for x in 6..10 {
				set_pixel(&mut document, top_index, x, y, [250, 250, 250, 255]);
			}
		}

		let hierarchy = vec![
			LayerHierarchyNode::Layer(document.layers[0].id),
			LayerHierarchyNode::Group(LayerGroup {
				id: common::GroupId::new(),
				name: "Color Stack".to_string(),
				visible: true,
				opacity_percent: 60,
				children: vec![
					LayerHierarchyNode::Layer(document.layers[warm_index].id),
					LayerHierarchyNode::Group(LayerGroup {
						id: common::GroupId::new(),
						name: "Accents".to_string(),
						visible: true,
						opacity_percent: 50,
						children: vec![
							LayerHierarchyNode::Layer(document.layers[cool_index].id),
							LayerHierarchyNode::Layer(document.layers[top_index].id),
						],
					}),
				],
			}),
		];

		document
			.set_layer_hierarchy(hierarchy)
			.expect("grouped scene hierarchy should be valid");
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
		let layer_ids = document.layers.iter().map(|layer| layer.id).collect::<Vec<_>>();
		document
			.set_layer_hierarchy(vec![LayerHierarchyNode::Group(LayerGroup {
				id: common::GroupId::new(),
				name: "Stack".to_string(),
				visible: true,
				opacity_percent: 85,
				children: layer_ids.into_iter().map(LayerHierarchyNode::Layer).collect(),
			})])
			.expect("manifest test hierarchy should be valid");

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
		assert_eq!(restored.layer_hierarchy.len(), 1);
		assert!(matches!(restored.layer_hierarchy[0], ManifestHierarchyNode::Group(_)));
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
	fn save_and_load_document_roundtrip_preserves_guides() {
		let mut document = Document::new(512, 512);
		document.add_guide(doc_model::Guide::horizontal(120));
		document.add_guide(doc_model::Guide::vertical(256));
		document.toggle_guides_visible();

		let path = temporary_project_path();
		save_document_to_path(&path, &document).expect("save should succeed");
		let restored = load_document_from_path(&path).expect("load should succeed");
		fs::remove_file(&path).expect("temporary project file should be removed");

		assert_eq!(
			restored.guides(),
			&[
				doc_model::Guide::horizontal(120),
				doc_model::Guide::vertical(256),
			]
		);
		assert!(!restored.guides_visible());
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
	fn flatten_document_propagates_group_visibility_and_opacity() {
		let mut document = build_grouped_scene();
		let index = (8 * document.canvas_size.width as usize + 8) * 4;

		let grouped = flatten_document_rgba(&document);
		assert_ne!(&grouped[index..index + 4], &[20, 30, 40, 255]);

		let mut hidden_hierarchy = document.layer_hierarchy().to_vec();
		if let LayerHierarchyNode::Group(group) = &mut hidden_hierarchy[1] {
			group.visible = false;
		}
		document
			.set_layer_hierarchy(hidden_hierarchy)
			.expect("hidden hierarchy should remain valid");
		let hidden = flatten_document_rgba(&document);
		assert_eq!(&hidden[index..index + 4], &[20, 30, 40, 255]);
	}

	#[test]
	fn grouped_scene_roundtrip_preserves_hierarchy_and_flattened_output() {
		let document = build_grouped_scene();
		let expected = flatten_document_rgba(&document);

		let path = temporary_project_path();
		save_document_to_path(&path, &document).expect("grouped scene should save");
		let restored = load_document_from_path(&path).expect("grouped scene should load");
		fs::remove_file(&path).expect("temporary project file should be removed");

		assert_eq!(restored.group_count(), 2);
		assert_eq!(restored.layer_hierarchy(), document.layer_hierarchy());
		assert_eq!(flatten_document_rgba(&restored), expected);
	}

	#[test]
	fn grouped_scene_png_export_matches_flattened_output() {
		let document = build_grouped_scene();
		let expected = flatten_document_rgba(&document);

		let path = std::env::temp_dir().join(format!("phototux-grouped-{}.png", temporary_suffix()));
		export_png_to_path(&path, &document).expect("grouped scene png export should succeed");
		let restored = import_png_from_path(&path).expect("grouped scene png import should succeed");
		fs::remove_file(&path).expect("temporary png should be removed");

		assert_eq!(flatten_document_rgba(&restored), expected);
	}

	#[test]
	fn update_flattened_region_matches_full_flatten_for_grouped_scene() {
		let document = build_grouped_scene();
		let expected = flatten_document_rgba(&document);
		let mut partial = expected.clone();
		let rect = common::CanvasRect::new(2, 2, 12, 12);

		for y in rect.y as usize..(rect.y + rect.height as i32) as usize {
			for x in rect.x as usize..(rect.x + rect.width as i32) as usize {
				let index = (y * document.canvas_size.width as usize + x) * 4;
				partial[index..index + 4].copy_from_slice(&[9, 8, 7, 6]);
			}
		}

		update_flattened_region_rgba(&document, &mut partial, rect);
		assert_eq!(partial, expected);
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

	fn temporary_workspace_path(prefix: &str) -> PathBuf {
		let unique = temporary_suffix();
		std::env::temp_dir().join(format!("phototux-{prefix}-{unique}"))
	}

	fn create_psd_import_workspace(manifest: &PsdImportManifest) -> PathBuf {
		let workspace = temporary_workspace_path("psd-import");
		fs::create_dir_all(&workspace).expect("PSD import workspace should be created");
		let manifest_path = workspace.join("manifest.json");
		let json = serde_json::to_vec_pretty(manifest).expect("PSD import manifest should serialize");
		fs::write(&manifest_path, json).expect("PSD import manifest should be written");
		manifest_path
	}

	#[cfg(unix)]
	fn write_shell_script(path: &Path, contents: &str) {
		fs::write(path, contents).expect("shell script should be written");
		let mut permissions = fs::metadata(path)
			.expect("shell script metadata should exist")
			.permissions();
		permissions.set_mode(0o755);
		fs::set_permissions(path, permissions).expect("shell script permissions should be updated");
	}

	fn write_rgba_asset(path: &Path, width: u32, height: u32, pixels: Vec<u8>) {
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).expect("PSD asset directory should be created");
		}
		let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, pixels)
			.expect("RGBA asset buffer should match dimensions");
		image
			.save_with_format(path, ImageFormat::Png)
			.expect("PSD asset PNG should be written");
	}

	fn build_supported_psd_manifest() -> PsdImportManifest {
		PsdImportManifest {
			manifest_version: CURRENT_PSD_IMPORT_MANIFEST_VERSION,
			source_kind: PsdImportSourceKind::Psd,
			source_color_mode: PsdImportColorMode::Rgb,
			source_depth_bits: 8,
			canvas: PsdImportCanvasRecord {
				width_px: 6,
				height_px: 6,
			},
			composite: PsdImportCompositeRecord {
				available: false,
				asset_relpath: None,
			},
			diagnostics: vec![PsdImportDiagnostic {
				severity: PsdImportDiagnosticSeverity::Info,
				code: "source_loaded".to_string(),
				message: "PSD manifest decoded successfully.".to_string(),
				source_index: None,
			}],
			layers: vec![
				PsdImportLayerRecord {
					source_index: 0,
					kind: PsdImportLayerKind::Raster,
					name: "Background".to_string(),
					visible: true,
					opacity_0_255: 255,
					blend_key: "norm".to_string(),
					offset_px: PsdImportOffsetRecord { x: 0, y: 0 },
					bounds_px: PsdImportBoundsRecord {
						left: 0,
						top: 0,
						width: 3,
						height: 3,
					},
					raster_asset_relpath: Some("layers/000-background.png".to_string()),
					unsupported_features: Vec::new(),
				},
				PsdImportLayerRecord {
					source_index: 1,
					kind: PsdImportLayerKind::Raster,
					name: "Screen Accent".to_string(),
					visible: true,
					opacity_0_255: 128,
					blend_key: "scrn".to_string(),
					offset_px: PsdImportOffsetRecord { x: 2, y: 1 },
					bounds_px: PsdImportBoundsRecord {
						left: 2,
						top: 1,
						width: 2,
						height: 2,
					},
					raster_asset_relpath: Some("layers/001-screen.png".to_string()),
					unsupported_features: Vec::new(),
				},
			],
		}
	}

	fn build_expected_supported_psd_document() -> Document {
		let mut document = Document::new(6, 6);
		document.rename_layer(0, "Background");
		for y in 0..3 {
			for x in 0..3 {
				set_pixel(&mut document, 0, x, y, [20, 40, 80, 255]);
			}
		}

		document.add_layer("Screen Accent");
		let top_index = document.active_layer_index();
		document.set_layer_blend_mode(top_index, BlendMode::Screen);
		document.set_layer_opacity(top_index, 50);
		assert!(document.set_layer_offset(top_index, 2, 1));
		for y in 0..2 {
			for x in 0..2 {
				set_pixel(&mut document, top_index, x, y, [240, 180, 100, 255]);
			}
		}
		document
	}

	#[cfg(unix)]
	fn repo_root() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../..")
			.canonicalize()
			.expect("repository root should resolve from the file_io crate")
	}

	#[cfg(unix)]
	fn repo_psd_fixture_path(file_name: &str) -> PathBuf {
		repo_root().join("tests/fixtures/psd").join(file_name)
	}

	#[cfg(unix)]
	fn repo_psd_sidecar_script_path() -> PathBuf {
		repo_root()
			.join("tools/psd_import_sidecar/phototux_psd_sidecar.py")
	}

	#[cfg(unix)]
	fn repo_psd_sidecar_runtime_available() -> bool {
		if !repo_psd_sidecar_script_path().is_file() {
			return false;
		}

		match Command::new("python3")
			.args(["-c", "import psd_tools"])
			.output()
		{
			Ok(output) => output.status.success(),
			Err(_) => false,
		}
	}

	#[test]
	fn psd_manifest_import_builds_supported_layer_stack() {
		let manifest = build_supported_psd_manifest();
		let manifest_path = create_psd_import_workspace(&manifest);
		let workspace_dir = manifest_path.parent().expect("manifest should have a parent").to_path_buf();
		write_rgba_asset(
			&workspace_dir.join("layers/000-background.png"),
			3,
			3,
			vec![
				20, 40, 80, 255, 20, 40, 80, 255, 20, 40, 80, 255,
				20, 40, 80, 255, 20, 40, 80, 255, 20, 40, 80, 255,
				20, 40, 80, 255, 20, 40, 80, 255, 20, 40, 80, 255,
			],
		);
		write_rgba_asset(
			&workspace_dir.join("layers/001-screen.png"),
			2,
			2,
			vec![
				240, 180, 100, 255, 240, 180, 100, 255,
				240, 180, 100, 255, 240, 180, 100, 255,
			],
		);

		let imported = import_psd_from_manifest_path(&manifest_path).expect("supported PSD manifest should import");
		let expected = build_expected_supported_psd_document();

		assert!(!imported.used_flattened_fallback);
		assert_eq!(imported.document.layers.len(), 2);
		assert_eq!(imported.document.layers[0].name, "Background");
		assert_eq!(imported.document.layers[1].name, "Screen Accent");
		assert_eq!(imported.document.layers[1].blend_mode, BlendMode::Screen);
		assert_eq!(imported.document.layers[1].opacity_percent, 50);
		assert_eq!(imported.document.layer_offset(1), Some((2, 1)));
		assert_eq!(flatten_document_rgba(&imported.document), flatten_document_rgba(&expected));
		assert_eq!(imported.diagnostics.len(), 1);

		fs::remove_dir_all(workspace_dir).expect("PSD import workspace should be removed");
	}

	#[test]
	fn psd_manifest_import_uses_flattened_fallback_for_unsupported_structure() {
		let mut manifest = build_supported_psd_manifest();
		manifest.composite = PsdImportCompositeRecord {
			available: true,
			asset_relpath: Some("composite.png".to_string()),
		};
		manifest.layers[1].kind = PsdImportLayerKind::Text;
		manifest.layers[1].unsupported_features = vec!["text_engine_data".to_string()];

		let manifest_path = create_psd_import_workspace(&manifest);
		let workspace_dir = manifest_path.parent().expect("manifest should have a parent").to_path_buf();
		write_rgba_asset(
			&workspace_dir.join("composite.png"),
			6,
			6,
			vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 15, 25, 35, 255, 45, 55, 65, 255, 75, 85, 95, 255, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 20, 30, 40, 255, 50, 60, 70, 255, 80, 90, 100, 255, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			],
		);

		let imported = import_psd_from_manifest_path(&manifest_path).expect("unsupported PSD manifest should fall back to composite");

		assert!(imported.used_flattened_fallback);
		assert_eq!(imported.document.layers.len(), 1);
		assert_eq!(imported.document.layers[0].name, "Flattened PSD Import");
		assert!(imported
			.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code == "flattened_fallback_used"));

		fs::remove_dir_all(workspace_dir).expect("PSD import workspace should be removed");
	}

	#[test]
	fn psd_manifest_import_fails_without_truthful_fallback() {
		let mut manifest = build_supported_psd_manifest();
		manifest.source_color_mode = PsdImportColorMode::Cmyk;

		let manifest_path = create_psd_import_workspace(&manifest);
		let workspace_dir = manifest_path.parent().expect("manifest should have a parent").to_path_buf();
		let error = import_psd_from_manifest_path(&manifest_path).expect_err("unsupported PSD manifest should fail without fallback");
		assert!(error.to_string().contains("supported subset"));
		assert!(error.to_string().contains("RGB only"));

		fs::remove_dir_all(workspace_dir).expect("PSD import workspace should be removed");
	}

	#[test]
	fn psd_manifest_import_rejects_unknown_manifest_version() {
		let mut manifest = build_supported_psd_manifest();
		manifest.manifest_version = 99;

		let manifest_path = create_psd_import_workspace(&manifest);
		let workspace_dir = manifest_path.parent().expect("manifest should have a parent").to_path_buf();
		let error = import_psd_from_manifest_path(&manifest_path).expect_err("unknown PSD manifest version should fail");
		assert!(error.to_string().contains("unsupported PSD import manifest version"));

		fs::remove_dir_all(workspace_dir).expect("PSD import workspace should be removed");
	}

	#[cfg(unix)]
	#[test]
	fn psd_sidecar_runtime_imports_manifest_and_cleans_workspace() {
		let manifest = build_supported_psd_manifest();
		let fixture_dir = temporary_workspace_path("psd-sidecar-fixture");
		fs::create_dir_all(&fixture_dir).expect("fixture dir should exist");
		let fixture_manifest = fixture_dir.join("fixture-manifest.json");
		fs::write(
			&fixture_manifest,
			serde_json::to_vec_pretty(&manifest).expect("fixture manifest should serialize"),
		)
		.expect("fixture manifest should be written");
		write_rgba_asset(
			&fixture_dir.join("layers/000-background.png"),
			3,
			3,
			vec![
				20, 40, 80, 255, 20, 40, 80, 255, 20, 40, 80, 255,
				20, 40, 80, 255, 20, 40, 80, 255, 20, 40, 80, 255,
				20, 40, 80, 255, 20, 40, 80, 255, 20, 40, 80, 255,
			],
		);
		write_rgba_asset(
			&fixture_dir.join("layers/001-screen.png"),
			2,
			2,
			vec![
				240, 180, 100, 255, 240, 180, 100, 255,
				240, 180, 100, 255, 240, 180, 100, 255,
			],
		);
		let source_psd = fixture_dir.join("source.psd");
		fs::write(&source_psd, b"placeholder psd source").expect("source psd should be written");
		let workspace_log = fixture_dir.join("workspace-path.txt");
		let script_path = fixture_dir.join("sidecar.sh");
		write_shell_script(
			&script_path,
			&format!(
				"#!/bin/sh\nset -eu\nSOURCE=\"$1\"\nWORKSPACE=\"$2\"\nMANIFEST=\"$3\"\nprintf '%s' \"$WORKSPACE\" > \"{}\"\ncp \"{}\" \"$MANIFEST\"\nmkdir -p \"$WORKSPACE/layers\"\ncp \"{}\" \"$WORKSPACE/layers/000-background.png\"\ncp \"{}\" \"$WORKSPACE/layers/001-screen.png\"\n[ -f \"$SOURCE\" ]\n",
				workspace_log.display(),
				fixture_manifest.display(),
				fixture_dir.join("layers/000-background.png").display(),
				fixture_dir.join("layers/001-screen.png").display(),
			),
		);

		let result = import_psd_from_path_with_sidecar(
			&source_psd,
			&PsdImportSidecar::new("/bin/sh").with_arg(script_path.as_os_str()),
		)
		.expect("sidecar-driven PSD import should succeed");
		let expected = build_expected_supported_psd_document();
		let workspace_path = fs::read_to_string(&workspace_log)
			.expect("workspace path log should be written");
		let workspace_path = PathBuf::from(workspace_path);

		assert_eq!(flatten_document_rgba(&result.document), flatten_document_rgba(&expected));
		assert!(!result.used_flattened_fallback);
		assert!(!workspace_path.exists());

		fs::remove_dir_all(&fixture_dir).expect("fixture dir should be removed");
	}

	#[cfg(unix)]
	#[test]
	fn psd_sidecar_runtime_reports_process_failure_and_cleans_workspace() {
		let fixture_dir = temporary_workspace_path("psd-sidecar-failure");
		fs::create_dir_all(&fixture_dir).expect("fixture dir should exist");
		let source_psd = fixture_dir.join("source.psd");
		fs::write(&source_psd, b"placeholder psd source").expect("source psd should be written");
		let workspace_log = fixture_dir.join("workspace-path.txt");
		let script_path = fixture_dir.join("sidecar-fail.sh");
		write_shell_script(
			&script_path,
			&format!(
				"#!/bin/sh\nset -eu\nprintf '%s' \"$2\" > \"{}\"\necho 'simulated sidecar failure' >&2\nexit 7\n",
				workspace_log.display(),
			),
		);

		let error = import_psd_from_path_with_sidecar(
			&source_psd,
			&PsdImportSidecar::new("/bin/sh").with_arg(script_path.as_os_str()),
		)
		.expect_err("failing sidecar should surface an error");
		let workspace_path = fs::read_to_string(&workspace_log)
			.expect("workspace path log should be written");
		let workspace_path = PathBuf::from(workspace_path);

		assert!(error.to_string().contains("simulated sidecar failure"));
		assert!(!workspace_path.exists());

		fs::remove_dir_all(&fixture_dir).expect("fixture dir should be removed");
	}

	#[cfg(unix)]
	#[test]
	fn psd_repo_fixture_supported_layers_import_through_real_sidecar() {
		if !repo_psd_sidecar_runtime_available() {
			eprintln!("skipping PSD repo fixture import test: python3 with psd_tools is unavailable");
			return;
		}

		let fixture_path = repo_psd_fixture_path("supported-simple-layers.psd");
		let result = import_psd_from_path_with_sidecar(
			&fixture_path,
			&PsdImportSidecar::new("python3").with_arg(repo_psd_sidecar_script_path().as_os_str()),
		)
		.expect("repo PSD fixture should import through the real sidecar");
		let expected = build_expected_supported_psd_document();

		assert!(!result.used_flattened_fallback);
		assert_eq!(result.document.layers.len(), 2);
		assert_eq!(result.document.layers[0].name, "Background");
		assert_eq!(result.document.layers[1].name, "Screen Accent");
		assert_eq!(result.document.layers[1].blend_mode, BlendMode::Screen);
		assert_eq!(result.document.layers[1].opacity_percent, 50);
		assert_eq!(result.document.layer_offset(1), Some((2, 1)));
		assert_eq!(flatten_document_rgba(&result.document), flatten_document_rgba(&expected));
	}

	#[cfg(unix)]
	#[test]
	fn psd_repo_fixture_visibility_and_blend_subset_preserve_import_metadata() {
		if !repo_psd_sidecar_runtime_available() {
			eprintln!("skipping PSD repo fixture metadata test: python3 with psd_tools is unavailable");
			return;
		}

		let visibility_result = import_psd_from_path_with_sidecar(
			&repo_psd_fixture_path("supported-visibility-opacity.psd"),
			&PsdImportSidecar::new("python3").with_arg(repo_psd_sidecar_script_path().as_os_str()),
		)
		.expect("visibility PSD fixture should import through the real sidecar");
		assert!(!visibility_result.used_flattened_fallback);
		assert_eq!(visibility_result.document.layers.len(), 3);
		assert_eq!(visibility_result.document.layers[0].name, "Base Fill");
		assert!(visibility_result.document.layers[0].visible);
		assert_eq!(visibility_result.document.layers[1].name, "Hidden Accent");
		assert!(!visibility_result.document.layers[1].visible);
		assert_eq!(visibility_result.document.layers[2].name, "Soft Overlay");
		assert_eq!(visibility_result.document.layers[2].opacity_percent, 38);

		let blend_result = import_psd_from_path_with_sidecar(
			&repo_psd_fixture_path("supported-blend-subset.psd"),
			&PsdImportSidecar::new("python3").with_arg(repo_psd_sidecar_script_path().as_os_str()),
		)
		.expect("blend subset PSD fixture should import through the real sidecar");
		assert!(!blend_result.used_flattened_fallback);
		assert_eq!(
			blend_result
				.document
				.layers
				.iter()
				.map(|layer| layer.blend_mode)
				.collect::<Vec<_>>(),
			vec![
				BlendMode::Normal,
				BlendMode::Multiply,
				BlendMode::Screen,
				BlendMode::Overlay,
				BlendMode::Darken,
				BlendMode::Lighten,
			],
		);
	}

	#[cfg(unix)]
	#[test]
	fn psd_repo_fixture_grouped_structure_uses_flattened_fallback() {
		if !repo_psd_sidecar_runtime_available() {
			eprintln!("skipping PSD repo fixture fallback test: python3 with psd_tools is unavailable");
			return;
		}

		let result = import_psd_from_path_with_sidecar(
			&repo_psd_fixture_path("flattened-fallback-group.psd"),
			&PsdImportSidecar::new("python3").with_arg(repo_psd_sidecar_script_path().as_os_str()),
		)
		.expect("grouped PSD fixture should import through the real sidecar");

		assert!(result.used_flattened_fallback);
		assert_eq!(result.document.layers.len(), 1);
		assert_eq!(result.document.layers[0].name, "Flattened PSD Import");
		assert!(result
			.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code == "flattened_fallback_used"));
		assert!(result
			.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code == "unsupported_layer_kind"));
	}

	#[cfg(unix)]
	#[test]
	fn psd_repo_fixture_cmyk_source_reports_unsupported_color_mode() {
		if !repo_psd_sidecar_runtime_available() {
			eprintln!("skipping PSD repo fixture CMYK test: python3 with psd_tools is unavailable");
			return;
		}

		let result = import_psd_from_path_with_sidecar(
			&repo_psd_fixture_path("unsupported-cmyk-fallback.psd"),
			&PsdImportSidecar::new("python3").with_arg(repo_psd_sidecar_script_path().as_os_str()),
		)
		.expect("CMYK PSD fixture should import through the real sidecar");

		assert!(result.used_flattened_fallback);
		assert_eq!(result.document.layers.len(), 1);
		assert_eq!(result.document.layers[0].name, "Flattened PSD Import");
		assert!(result
			.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code == "unsupported_color_mode"));
		assert!(result
			.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code == "flattened_fallback_used"));
	}

	#[cfg(unix)]
	#[test]
	fn psd_repo_fixture_unsupported_feature_sources_use_flattened_fallback() {
		if !repo_psd_sidecar_runtime_available() {
			eprintln!("skipping PSD repo fixture unsupported-feature test: python3 with psd_tools is unavailable");
			return;
		}

		let test_cases = [
			(
				"unsupported-text-fallback.psd",
				"unsupported_layer_kind",
				"unsupported kind",
			),
			(
				"unsupported-smart-object-fallback.psd",
				"unsupported_layer_kind",
				"unsupported kind",
			),
			(
				"unsupported-clipping-fallback.psd",
				"unsupported_layer_features",
				"clipping_mask",
			),
			(
				"unsupported-mask-fallback.psd",
				"unsupported_layer_features",
				"mask",
			),
		];

		for (fixture_name, expected_code, expected_message_fragment) in test_cases {
			let result = import_psd_from_path_with_sidecar(
				&repo_psd_fixture_path(fixture_name),
				&PsdImportSidecar::new("python3").with_arg(repo_psd_sidecar_script_path().as_os_str()),
			)
			.expect("unsupported PSD fixture should import through the real sidecar");

			assert!(result.used_flattened_fallback, "{fixture_name} should use flattened fallback");
			assert_eq!(result.document.layers.len(), 1, "{fixture_name} should flatten to one layer");
			assert!(result
				.diagnostics
				.iter()
				.any(|diagnostic| diagnostic.code == "flattened_fallback_used"), "{fixture_name} should report flattened fallback");
			assert!(result
				.diagnostics
				.iter()
				.any(|diagnostic| diagnostic.code == expected_code), "{fixture_name} should report {expected_code}");
			assert!(result
				.diagnostics
				.iter()
				.any(|diagnostic| diagnostic.message.contains(expected_message_fragment)), "{fixture_name} should mention {expected_message_fragment}");
		}
	}

	#[cfg(unix)]
	#[test]
	fn psd_repo_fixture_supported_layers_export_png_matches_flattened_document() {
		if !repo_psd_sidecar_runtime_available() {
			eprintln!("skipping PSD repo fixture export parity test: python3 with psd_tools is unavailable");
			return;
		}

		let result = import_psd_from_path_with_sidecar(
			&repo_psd_fixture_path("supported-simple-layers.psd"),
			&PsdImportSidecar::new("python3").with_arg(repo_psd_sidecar_script_path().as_os_str()),
		)
		.expect("supported PSD fixture should import through the real sidecar");
		let workspace_dir = temporary_workspace_path("psd-export-parity");
		fs::create_dir_all(&workspace_dir).expect("temporary export workspace should be created");
		let export_path = workspace_dir.join("imported-scene.png");

		export_png_to_path(&export_path, &result.document)
			.expect("imported PSD document should export to PNG");
		let exported_pixels = image::open(&export_path)
			.expect("exported PNG should decode")
			.to_rgba8()
			.into_raw();

		assert_eq!(exported_pixels, flatten_document_rgba(&result.document));

		fs::remove_dir_all(&workspace_dir).expect("temporary export workspace should be removed");
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

	composite_hierarchy_into(document, output, Some(rect));
}

fn composite_hierarchy_into(
	document: &Document,
	output: &mut [u8],
	clip_rect: Option<common::CanvasRect>,
) {
	composite_hierarchy_nodes(document, &document.layer_hierarchy, output, clip_rect, true, 1.0);
}

fn composite_hierarchy_nodes(
	document: &Document,
	nodes: &[LayerHierarchyNode],
	output: &mut [u8],
	clip_rect: Option<common::CanvasRect>,
	ancestors_visible: bool,
	ancestor_opacity: f32,
) {
	for node in nodes {
		match node {
			LayerHierarchyNode::Layer(layer_id) => {
				let Some(layer) = document.layer_by_id(*layer_id) else {
					continue;
				};
				if !ancestors_visible || !layer.visible {
					continue;
				}

				let effective_opacity = ancestor_opacity * (layer.opacity_percent as f32 / 100.0);
				if effective_opacity <= 0.0 {
					continue;
				}

				composite_layer_into(document, layer, output, clip_rect, effective_opacity);
			}
			LayerHierarchyNode::Group(group) => {
				if !ancestors_visible || !group.visible {
					continue;
				}
				let effective_opacity = ancestor_opacity * (group.opacity_percent as f32 / 100.0);
				if effective_opacity <= 0.0 {
					continue;
				}
				composite_hierarchy_nodes(
					document,
					&group.children,
					output,
					clip_rect,
					true,
					effective_opacity,
				);
			}
		}
	}
}

fn composite_layer_into(
	document: &Document,
	layer: &RasterLayer,
	output: &mut [u8],
	clip_rect: Option<common::CanvasRect>,
	layer_opacity: f32,
) {
	for (coord, tile) in &layer.tiles {
		let mask_tile = layer
			.mask
			.as_ref()
			.filter(|mask| mask.enabled)
			.and_then(|mask| mask.tiles.get(coord));
		let (tile_origin_x, tile_origin_y) = document.tile_origin(*coord);
		let tile_canvas_x = tile_origin_x as i32 + layer.offset_x;
		let tile_canvas_y = tile_origin_y as i32 + layer.offset_y;

		if let Some(rect) = clip_rect {
			if tile_canvas_x + document.tile_size as i32 <= rect.x || tile_canvas_x >= rect.x + rect.width as i32 {
				continue;
			}
			if tile_canvas_y + document.tile_size as i32 <= rect.y || tile_canvas_y >= rect.y + rect.height as i32 {
				continue;
			}
		}

		let (start_x, end_x, start_y, end_y) = if let Some(rect) = clip_rect {
			(
				tile_canvas_x.max(rect.x).max(0),
				(tile_canvas_x + document.tile_size as i32)
					.min(rect.x + rect.width as i32)
					.min(document.canvas_size.width as i32),
				tile_canvas_y.max(rect.y).max(0),
				(tile_canvas_y + document.tile_size as i32)
					.min(rect.y + rect.height as i32)
					.min(document.canvas_size.height as i32),
			)
		} else {
			(
				tile_canvas_x.max(0),
				(tile_canvas_x + document.tile_size as i32).min(document.canvas_size.width as i32),
				tile_canvas_y.max(0),
				(tile_canvas_y + document.tile_size as i32).min(document.canvas_size.height as i32),
			)
		};

		for canvas_y in start_y..end_y {
			let local_y = (canvas_y - tile_canvas_y) as usize;
			for canvas_x in start_x..end_x {
				let local_x = (canvas_x - tile_canvas_x) as usize;
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
