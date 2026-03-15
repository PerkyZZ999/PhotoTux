use common::{CanvasSize, LayerId};
use doc_model::{BlendMode, Document};
use serde::{Deserialize, Serialize};

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
	pub payload_path: String,
}

#[cfg(test)]
mod tests {
	use super::{ProjectManifest, CURRENT_PROJECT_FORMAT_VERSION};
	use doc_model::Document;

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
		assert!(restored.layers[1].payload_path.starts_with("layers/"));
		assert!(restored.layers[1].payload_path.ends_with(".png"));
	}
}
