//! Native file, import, export, and recovery scaffolding for PhotoTux.

use color_math::{
    blend_darken_rgba8, blend_lighten_rgba8, blend_multiply_rgba8, blend_normal_rgba8,
    blend_overlay_rgba8, blend_screen_rgba8,
};
use common::{DocumentId, LayerId};
use doc_model::{
    BlendMode, Canvas, Document, DocumentMetadata, LayeredRasterDocument, RasterLayer,
    RasterSurface, ThumbnailStrategy,
};
use image::{DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const PROTOTYPE_FORMAT_VERSION: u32 = 1;
const DOCUMENT_VERSION: u32 = 1;
const MANIFEST_FILE_NAME: &str = "manifest.json";
const LAYERS_DIR_NAME: &str = "layers";
const THUMB_DIR_NAME: &str = "thumb";
const PREVIEW_FILE_NAME: &str = "preview.png";

/// Errors raised while saving, loading, or exporting prototype documents.
#[derive(Debug, Error)]
pub enum FileIoError {
    /// Underlying filesystem operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Manifest serialization or deserialization failed.
    #[error(transparent)]
    Manifest(#[from] serde_json::Error),
    /// PNG encoding failed.
    #[error(transparent)]
    PngEncoding(#[from] png::EncodingError),
    /// PNG decoding failed.
    #[error(transparent)]
    PngDecoding(#[from] png::DecodingError),
    /// Image decoding failed.
    #[error(transparent)]
    Image(#[from] image::ImageError),
    /// The saved project was not in the expected prototype format.
    #[error("unsupported prototype format version: {version}")]
    UnsupportedFormatVersion {
        /// Manifest format version.
        version: u32,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct PrototypeManifest {
    format_version: u32,
    document_id: u128,
    title: String,
    canvas_width: u32,
    canvas_height: u32,
    layer: ManifestLayer,
    surface_file: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct LayeredManifest {
    format_version: u32,
    document_version: u32,
    app_version: String,
    document_id: u128,
    title: String,
    canvas_width: u32,
    canvas_height: u32,
    active_layer_id: Option<u128>,
    layer_order: Vec<u128>,
    layers: Vec<ManifestLayerWithPayload>,
    preview_file: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct ManifestLayerWithPayload {
    id: u128,
    name: String,
    visible: bool,
    opacity: f32,
    blend_mode: ManifestBlendMode,
    surface_file: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct ManifestLayer {
    id: u128,
    name: String,
    visible: bool,
    opacity: f32,
    blend_mode: ManifestBlendMode,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
enum ManifestBlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

impl From<BlendMode> for ManifestBlendMode {
    fn from(value: BlendMode) -> Self {
        match value {
            BlendMode::Normal => Self::Normal,
            BlendMode::Multiply => Self::Multiply,
            BlendMode::Screen => Self::Screen,
            BlendMode::Overlay => Self::Overlay,
            BlendMode::Darken => Self::Darken,
            BlendMode::Lighten => Self::Lighten,
        }
    }
}

impl From<ManifestBlendMode> for BlendMode {
    fn from(value: ManifestBlendMode) -> Self {
        match value {
            ManifestBlendMode::Normal => Self::Normal,
            ManifestBlendMode::Multiply => Self::Multiply,
            ManifestBlendMode::Screen => Self::Screen,
            ManifestBlendMode::Overlay => Self::Overlay,
            ManifestBlendMode::Darken => Self::Darken,
            ManifestBlendMode::Lighten => Self::Lighten,
        }
    }
}

/// Import a PNG image into a single-layer layered document.
pub fn import_png(path: &Path) -> Result<LayeredRasterDocument, FileIoError> {
    import_raster_image(path, ImageFormat::Png)
}

/// Import a JPEG image into a single-layer layered document.
pub fn import_jpeg(path: &Path) -> Result<LayeredRasterDocument, FileIoError> {
    import_raster_image(path, ImageFormat::Jpeg)
}

/// Import a WebP image into a single-layer layered document.
pub fn import_webp(path: &Path) -> Result<LayeredRasterDocument, FileIoError> {
    import_raster_image(path, ImageFormat::WebP)
}

/// Save a layered document into a `.ptx` directory container.
pub fn save_layered_document(
    root: &Path,
    layered_document: &LayeredRasterDocument,
) -> Result<(), FileIoError> {
    let temp_root = temporary_output_root(root);
    fs::create_dir_all(temp_root.join(LAYERS_DIR_NAME))?;
    fs::create_dir_all(temp_root.join(THUMB_DIR_NAME))?;

    let document = layered_document.document();
    let layers = document.layers();
    let preview_relative_path = format!("{THUMB_DIR_NAME}/{PREVIEW_FILE_NAME}");
    let preview_surface = build_preview_surface(layered_document);

    let manifest = LayeredManifest {
        format_version: PROTOTYPE_FORMAT_VERSION,
        document_version: DOCUMENT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        document_id: document.id.get(),
        title: document.metadata.title.clone(),
        canvas_width: document.canvas.width,
        canvas_height: document.canvas.height,
        active_layer_id: document.active_layer().map(|layer| layer.id.get()),
        layer_order: layers.iter().map(|layer| layer.id.get()).collect(),
        layers: layers
            .iter()
            .map(|layer| ManifestLayerWithPayload {
                id: layer.id.get(),
                name: layer.name.clone(),
                visible: layer.visible,
                opacity: layer.opacity,
                blend_mode: layer.blend_mode.into(),
                surface_file: format!("{LAYERS_DIR_NAME}/{}.png", layer.id.get()),
            })
            .collect(),
        preview_file: preview_relative_path.clone(),
    };

    for manifest_layer in &manifest.layers {
        let layer_id = LayerId::new(manifest_layer.id);
        let surface = layered_document.surface(layer_id).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("missing raster surface for layer {}", manifest_layer.id),
            )
        })?;
        write_surface_png(&temp_root.join(&manifest_layer.surface_file), surface)?;
    }

    write_surface_png(&temp_root.join(&preview_relative_path), &preview_surface)?;
    let manifest_writer = BufWriter::new(File::create(temp_root.join(MANIFEST_FILE_NAME))?);
    serde_json::to_writer_pretty(manifest_writer, &manifest)?;

    finalize_directory_save(root, &temp_root)?;
    Ok(())
}

/// Load a layered document from a `.ptx` directory container.
pub fn load_layered_document(root: &Path) -> Result<LayeredRasterDocument, FileIoError> {
    let manifest_reader = BufReader::new(File::open(root.join(MANIFEST_FILE_NAME))?);
    let manifest: LayeredManifest = serde_json::from_reader(manifest_reader)?;
    if manifest.format_version != PROTOTYPE_FORMAT_VERSION {
        return Err(FileIoError::UnsupportedFormatVersion {
            version: manifest.format_version,
        });
    }

    let mut layers_by_id = manifest
        .layers
        .iter()
        .map(|layer| (layer.id, layer.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut document = Document::new(
        DocumentId::new(manifest.document_id),
        Canvas::new(manifest.canvas_width, manifest.canvas_height),
        DocumentMetadata {
            title: manifest.title,
        },
    );

    for layer_id in &manifest.layer_order {
        let manifest_layer = layers_by_id.remove(layer_id).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("missing manifest layer entry for id {layer_id}"),
            )
        })?;
        let mut layer = RasterLayer::new(LayerId::new(manifest_layer.id), manifest_layer.name);
        layer.visible = manifest_layer.visible;
        layer.opacity = manifest_layer.opacity;
        layer.blend_mode = manifest_layer.blend_mode.into();
        document.add_layer(layer);
    }

    if let Some(active_layer_id) = manifest.active_layer_id {
        let _ = document.set_active_layer(LayerId::new(active_layer_id));
    }

    let mut layered_document = LayeredRasterDocument::from_document(document);
    for manifest_layer in manifest.layers {
        let surface = read_surface_png(&root.join(manifest_layer.surface_file))?;
        let Some(target_surface) = layered_document.surface_mut(LayerId::new(manifest_layer.id))
        else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("loaded layer {} has no target surface", manifest_layer.id),
            )
            .into());
        };
        *target_surface = surface;
    }

    Ok(layered_document)
}

/// Save a single-surface prototype document into a `.ptx` directory container.
pub fn save_single_surface_document(
    root: &Path,
    document: &Document,
    surface: &RasterSurface,
) -> Result<(), FileIoError> {
    fs::create_dir_all(root)?;
    fs::create_dir_all(root.join("layers"))?;

    let layer = document
        .active_layer()
        .or_else(|| document.layers().first())
        .cloned()
        .unwrap_or_else(|| RasterLayer::new(LayerId::new(1), "Surface"));
    let surface_file = format!("layers/{}.png", layer.id.get());
    let manifest = PrototypeManifest {
        format_version: PROTOTYPE_FORMAT_VERSION,
        document_id: document.id.get(),
        title: document.metadata.title.clone(),
        canvas_width: document.canvas.width,
        canvas_height: document.canvas.height,
        layer: ManifestLayer {
            id: layer.id.get(),
            name: layer.name,
            visible: layer.visible,
            opacity: layer.opacity,
            blend_mode: layer.blend_mode.into(),
        },
        surface_file: surface_file.clone(),
    };

    write_surface_png(&root.join(surface_file), surface)?;
    let manifest_writer = BufWriter::new(File::create(root.join(MANIFEST_FILE_NAME))?);
    serde_json::to_writer_pretty(manifest_writer, &manifest)?;
    Ok(())
}

/// Load a single-surface prototype document from a `.ptx` directory container.
pub fn load_single_surface_document(root: &Path) -> Result<(Document, RasterSurface), FileIoError> {
    let manifest_reader = BufReader::new(File::open(root.join(MANIFEST_FILE_NAME))?);
    let manifest: PrototypeManifest = serde_json::from_reader(manifest_reader)?;
    if manifest.format_version != PROTOTYPE_FORMAT_VERSION {
        return Err(FileIoError::UnsupportedFormatVersion {
            version: manifest.format_version,
        });
    }

    let mut document = Document::new(
        DocumentId::new(manifest.document_id),
        Canvas::new(manifest.canvas_width, manifest.canvas_height),
        DocumentMetadata {
            title: manifest.title,
        },
    );
    let mut layer = RasterLayer::new(LayerId::new(manifest.layer.id), manifest.layer.name);
    layer.visible = manifest.layer.visible;
    layer.opacity = manifest.layer.opacity;
    layer.blend_mode = manifest.layer.blend_mode.into();
    document.add_layer(layer);

    let surface = read_surface_png(&root.join(manifest.surface_file))?;
    Ok((document, surface))
}

/// Export a single surface as a flattened PNG image.
pub fn export_surface_as_png(path: &Path, surface: &RasterSurface) -> Result<(), FileIoError> {
    write_surface_png(path, surface)
}

fn write_surface_png(path: &Path, surface: &RasterSurface) -> Result<(), FileIoError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let writer = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(writer, surface.width(), surface.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header()?;

    let mut bytes = Vec::with_capacity((surface.width() * surface.height() * 4) as usize);
    for y in 0..surface.height() {
        for x in 0..surface.width() {
            bytes.extend_from_slice(&surface.pixel(x, y));
        }
    }

    png_writer.write_image_data(&bytes)?;
    Ok(())
}

fn read_surface_png(path: &Path) -> Result<RasterSurface, FileIoError> {
    let decoder = png::Decoder::new(BufReader::new(File::open(path)?));
    let mut reader = decoder.read_info()?;
    let output_buffer_size = reader.output_buffer_size().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "png output buffer too large",
        )
    })?;
    let mut buffer = vec![0; output_buffer_size];
    let info = reader.next_frame(&mut buffer)?;
    let bytes = &buffer[..info.buffer_size()];

    let mut surface = RasterSurface::new(info.width, info.height);
    for y in 0..info.height {
        for x in 0..info.width {
            let index = ((y as usize) * (info.width as usize) + (x as usize)) * 4;
            let pixel = [
                bytes[index],
                bytes[index + 1],
                bytes[index + 2],
                bytes[index + 3],
            ];
            if pixel != [0, 0, 0, 0] {
                let _ = surface.write_pixel(x, y, pixel);
            }
        }
    }

    Ok(surface)
}

fn import_raster_image(
    path: &Path,
    expected_format: ImageFormat,
) -> Result<LayeredRasterDocument, FileIoError> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    let detected_format = reader.format();

    if detected_format != Some(expected_format) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "expected {:?} import source but detected {:?}",
                expected_format, detected_format
            ),
        )
        .into());
    }

    let image = reader.decode()?;
    Ok(normalize_imported_image(path, image))
}

fn normalize_imported_image(path: &Path, image: DynamicImage) -> LayeredRasterDocument {
    let rgba_image = image.to_rgba8();
    let (width, height) = rgba_image.dimensions();
    let title = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "imported-image".to_string());
    let layer_name = path
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported Layer".to_string());

    let mut document = Document::new(
        DocumentId::new(1),
        Canvas::new(width, height),
        DocumentMetadata { title },
    );
    document.add_layer(RasterLayer::new(LayerId::new(1), layer_name));

    let mut layered_document = LayeredRasterDocument::from_document(document);
    let surface = layered_document
        .surface_mut(LayerId::new(1))
        .expect("imported layer surface should exist");

    for y in 0..height {
        for x in 0..width {
            let pixel = rgba_image.get_pixel(x, y).0;
            if pixel != [0, 0, 0, 0] {
                let _ = surface.write_pixel(x, y, pixel);
            }
        }
    }

    layered_document
}

fn build_preview_surface(layered_document: &LayeredRasterDocument) -> RasterSurface {
    let document = layered_document.document();
    let composited_surface = composite_visible_surface(layered_document);
    let preview_size = ThumbnailStrategy::default().size_for_canvas(document.canvas);

    if preview_size.width == composited_surface.width()
        && preview_size.height == composited_surface.height()
    {
        return composited_surface;
    }

    resize_surface_nearest(&composited_surface, preview_size.width, preview_size.height)
}

fn composite_visible_surface(layered_document: &LayeredRasterDocument) -> RasterSurface {
    let document = layered_document.document();
    let mut composited = RasterSurface::new(document.canvas.width, document.canvas.height);

    for y in 0..document.canvas.height {
        for x in 0..document.canvas.width {
            let mut pixel = [0, 0, 0, 0];

            for layer in document.layers() {
                if !layer.visible {
                    continue;
                }

                let Some(surface) = layered_document.surface(layer.id) else {
                    continue;
                };
                let source_pixel = surface.pixel(x, y);
                pixel = blend_pixel(pixel, source_pixel, layer.blend_mode, layer.opacity);
            }

            if pixel != [0, 0, 0, 0] {
                let _ = composited.write_pixel(x, y, pixel);
            }
        }
    }

    composited
}

fn resize_surface_nearest(
    surface: &RasterSurface,
    target_width: u32,
    target_height: u32,
) -> RasterSurface {
    let target_width = target_width.max(1);
    let target_height = target_height.max(1);
    let mut resized = RasterSurface::new(target_width, target_height);
    let scale_x = surface.width() as f32 / target_width as f32;
    let scale_y = surface.height() as f32 / target_height as f32;

    for target_y in 0..target_height {
        for target_x in 0..target_width {
            let source_x = ((target_x as f32) * scale_x)
                .floor()
                .clamp(0.0, surface.width().saturating_sub(1) as f32)
                as u32;
            let source_y = ((target_y as f32) * scale_y)
                .floor()
                .clamp(0.0, surface.height().saturating_sub(1) as f32)
                as u32;
            let pixel = surface.pixel(source_x, source_y);

            if pixel != [0, 0, 0, 0] {
                let _ = resized.write_pixel(target_x, target_y, pixel);
            }
        }
    }

    resized
}

fn blend_pixel(
    destination: [u8; 4],
    source: [u8; 4],
    blend_mode: BlendMode,
    opacity: f32,
) -> [u8; 4] {
    match blend_mode {
        BlendMode::Normal => blend_normal_rgba8(destination, source, opacity),
        BlendMode::Multiply => blend_multiply_rgba8(destination, source, opacity),
        BlendMode::Screen => blend_screen_rgba8(destination, source, opacity),
        BlendMode::Overlay => blend_overlay_rgba8(destination, source, opacity),
        BlendMode::Darken => blend_darken_rgba8(destination, source, opacity),
        BlendMode::Lighten => blend_lighten_rgba8(destination, source, opacity),
    }
}

fn temporary_output_root(root: &Path) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let file_name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.ptx".to_string());

    root.with_file_name(format!(".{file_name}.tmp-{unique_suffix}"))
}

fn backup_output_root(root: &Path) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let file_name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.ptx".to_string());

    root.with_file_name(format!(".{file_name}.bak-{unique_suffix}"))
}

fn finalize_directory_save(root: &Path, temp_root: &Path) -> Result<(), FileIoError> {
    if !root.exists() {
        fs::rename(temp_root, root)?;
        return Ok(());
    }

    let backup_root = backup_output_root(root);
    fs::rename(root, &backup_root)?;

    match fs::rename(temp_root, root) {
        Ok(()) => {
            fs::remove_dir_all(backup_root)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup_root, root);
            Err(error.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        export_surface_as_png, import_jpeg, import_png, import_webp, load_layered_document,
        load_single_surface_document, save_layered_document, save_single_surface_document,
    };
    use common::{DocumentId, LayerId};
    use doc_model::{
        BlendMode, Canvas, Document, DocumentMetadata, LayeredRasterDocument, RasterLayer,
        RasterSurface,
    };
    use image::{DynamicImage, ExtendedColorType, ImageBuffer, ImageEncoder, Rgba};
    use std::fs::{self, File};
    use std::io::BufWriter;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("phototux-{name}-{nanos}"))
    }

    fn test_document_and_surface() -> (Document, RasterSurface) {
        let mut document = Document::new(
            DocumentId::new(7),
            Canvas::new(16, 16),
            DocumentMetadata {
                title: "Prototype".to_string(),
            },
        );
        let mut layer = RasterLayer::new(LayerId::new(9), "Surface");
        layer.blend_mode = BlendMode::Normal;
        document.add_layer(layer);

        let mut surface = RasterSurface::new(16, 16);
        let _ = surface.write_pixel(1, 1, [255, 0, 0, 255]);
        let _ = surface.write_pixel(2, 2, [0, 255, 0, 255]);
        (document, surface)
    }

    fn test_layered_document() -> LayeredRasterDocument {
        let mut document = Document::new(
            DocumentId::new(11),
            Canvas::new(8, 8),
            DocumentMetadata {
                title: "Layered Prototype".to_string(),
            },
        );

        let mut base_layer = RasterLayer::new(LayerId::new(101), "Base");
        base_layer.blend_mode = BlendMode::Normal;
        let mut accent_layer = RasterLayer::new(LayerId::new(202), "Accent");
        accent_layer.opacity = 0.5;
        accent_layer.blend_mode = BlendMode::Screen;
        document.add_layer(base_layer);
        document.add_layer(accent_layer);
        let _ = document.set_active_layer(LayerId::new(202));

        let mut layered_document = LayeredRasterDocument::from_document(document);
        let _ = layered_document
            .surface_mut(LayerId::new(101))
            .expect("base layer surface should exist")
            .write_pixel(1, 1, [255, 0, 0, 255]);
        let _ = layered_document
            .surface_mut(LayerId::new(202))
            .expect("accent layer surface should exist")
            .write_pixel(1, 1, [0, 0, 255, 128]);

        layered_document
    }

    fn write_png_fixture(path: &Path) {
        let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(3, 2, |x, y| match (x, y) {
            (0, 0) => Rgba([255, 0, 0, 128]),
            (1, 0) => Rgba([0, 255, 0, 255]),
            _ => Rgba([0, 0, 0, 0]),
        });
        image
            .save_with_format(path, image::ImageFormat::Png)
            .expect("png fixture should save");
    }

    fn write_jpeg_fixture(path: &Path) {
        let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(3, 2, |x, y| match (x, y) {
            (0, 0) => Rgba([250, 10, 20, 255]),
            (1, 0) => Rgba([20, 200, 40, 255]),
            _ => Rgba([30, 30, 30, 255]),
        });
        let rgb = DynamicImage::ImageRgba8(image).to_rgb8();
        rgb.save_with_format(path, image::ImageFormat::Jpeg)
            .expect("jpeg fixture should save");
    }

    fn write_webp_fixture(path: &Path) {
        let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(3, 2, |x, y| match (x, y) {
            (0, 0) => Rgba([10, 20, 250, 128]),
            (1, 0) => Rgba([200, 180, 20, 255]),
            _ => Rgba([0, 0, 0, 0]),
        });
        let writer = BufWriter::new(File::create(path).expect("webp fixture file should create"));
        image::codecs::webp::WebPEncoder::new_lossless(writer)
            .write_image(image.as_raw(), 3, 2, ExtendedColorType::Rgba8)
            .expect("webp fixture should save");
    }

    #[test]
    fn saves_and_loads_single_surface_document_roundtrip() {
        let root = unique_test_dir("save-load");
        let (document, surface) = test_document_and_surface();

        save_single_surface_document(&root, &document, &surface).expect("save should succeed");
        let (loaded_document, loaded_surface) =
            load_single_surface_document(&root).expect("load should succeed");

        assert_eq!(loaded_document.metadata.title, "Prototype");
        assert_eq!(loaded_surface.pixel(1, 1), [255, 0, 0, 255]);
        assert_eq!(loaded_surface.pixel(2, 2), [0, 255, 0, 255]);

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }

    #[test]
    fn exported_png_matches_surface_pixels() {
        let root = unique_test_dir("export");
        let export_path = root.join("visible.png");
        let (_, surface) = test_document_and_surface();

        export_surface_as_png(&export_path, &surface).expect("export should succeed");
        let loaded_surface =
            super::read_surface_png(&export_path).expect("exported png should decode");

        assert_eq!(loaded_surface.pixel(1, 1), [255, 0, 0, 255]);
        assert_eq!(loaded_surface.pixel(2, 2), [0, 255, 0, 255]);

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }

    #[test]
    fn saves_and_loads_layered_document_roundtrip() {
        let root = unique_test_dir("layered-save-load");
        let layered_document = test_layered_document();

        save_layered_document(&root, &layered_document).expect("layered save should succeed");
        let loaded_document = load_layered_document(&root).expect("layered load should succeed");

        assert_eq!(
            loaded_document.document().metadata.title,
            "Layered Prototype"
        );
        assert_eq!(loaded_document.document().layers().len(), 2);
        assert_eq!(
            loaded_document
                .document()
                .active_layer()
                .map(|layer| layer.id),
            Some(LayerId::new(202))
        );
        assert_eq!(
            loaded_document
                .surface(LayerId::new(101))
                .map(|surface| surface.pixel(1, 1)),
            Some([255, 0, 0, 255])
        );
        assert_eq!(
            loaded_document
                .surface(LayerId::new(202))
                .map(|surface| surface.pixel(1, 1)),
            Some([0, 0, 255, 128])
        );

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }

    #[test]
    fn layered_save_writes_manifest_and_preview_thumbnail() {
        let root = unique_test_dir("layered-preview");
        let layered_document = test_layered_document();

        save_layered_document(&root, &layered_document).expect("layered save should succeed");

        let manifest_path = root.join("manifest.json");
        let preview_path = root.join("thumb/preview.png");
        let manifest_json = fs::read_to_string(&manifest_path).expect("manifest should exist");

        assert!(manifest_json.contains("\"document_version\": 1"));
        assert!(manifest_json.contains("\"layer_order\""));
        assert!(manifest_json.contains("\"surface_file\": \"layers/101.png\""));
        assert!(preview_path.exists());

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }

    #[test]
    fn layered_save_replaces_existing_document_atomically() {
        let root = unique_test_dir("layered-replace");
        let first_document = test_layered_document();
        save_layered_document(&root, &first_document).expect("first layered save should succeed");

        let mut second_document = test_layered_document();
        let _ = second_document
            .surface_mut(LayerId::new(101))
            .expect("base layer surface should exist")
            .write_pixel(2, 2, [0, 255, 0, 255]);

        save_layered_document(&root, &second_document)
            .expect("second layered save should replace existing document");
        let loaded_document = load_layered_document(&root).expect("layered load should succeed");

        assert_eq!(
            loaded_document
                .surface(LayerId::new(101))
                .map(|surface| surface.pixel(2, 2)),
            Some([0, 255, 0, 255])
        );

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }

    #[test]
    fn png_import_preserves_alpha_and_canvas_size() {
        let root = unique_test_dir("import-png");
        fs::create_dir_all(&root).expect("test directory should create");
        let png_path = root.join("imported.png");
        write_png_fixture(&png_path);

        let imported = import_png(&png_path).expect("png import should succeed");

        assert_eq!(imported.document().canvas.width, 3);
        assert_eq!(imported.document().canvas.height, 2);
        assert_eq!(imported.document().metadata.title, "imported.png");
        assert_eq!(
            imported
                .surface(LayerId::new(1))
                .map(|surface| surface.pixel(0, 0)),
            Some([255, 0, 0, 128])
        );

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }

    #[test]
    fn jpeg_import_normalizes_to_opaque_rgba_document_pixels() {
        let root = unique_test_dir("import-jpeg");
        fs::create_dir_all(&root).expect("test directory should create");
        let jpeg_path = root.join("imported.jpg");
        write_jpeg_fixture(&jpeg_path);

        let imported = import_jpeg(&jpeg_path).expect("jpeg import should succeed");
        let imported_pixel = imported
            .surface(LayerId::new(1))
            .map(|surface| surface.pixel(0, 0))
            .expect("imported surface should exist");

        assert_eq!(imported.document().canvas.width, 3);
        assert_eq!(imported.document().canvas.height, 2);
        assert_eq!(imported_pixel[3], 255);
        assert!(imported_pixel[0] > 200);

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }

    #[test]
    fn webp_import_preserves_alpha_and_canvas_size() {
        let root = unique_test_dir("import-webp");
        fs::create_dir_all(&root).expect("test directory should create");
        let webp_path = root.join("imported.webp");
        write_webp_fixture(&webp_path);

        let imported = import_webp(&webp_path).expect("webp import should succeed");

        assert_eq!(imported.document().canvas.width, 3);
        assert_eq!(imported.document().canvas.height, 2);
        assert_eq!(
            imported
                .surface(LayerId::new(1))
                .map(|surface| surface.pixel(0, 0)),
            Some([10, 20, 250, 128])
        );

        fs::remove_dir_all(root).expect("test directory cleanup should succeed");
    }
}
