//! Headless document model primitives for PhotoTux.

use common::{DocumentId, LayerId, Size, clamp_f32};
use std::collections::{BTreeMap, BTreeSet};

/// Locked tile size for MVP raster editing.
pub const TILE_SIZE: u32 = 256;

const BYTES_PER_PIXEL: usize = 4;
const TRANSPARENT_PIXEL: [u8; 4] = [0, 0, 0, 0];
const SELECTION_BYTES_PER_PIXEL: usize = 1;

/// Pixel-space bounds for a document selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionBounds {
    /// Left-most selected pixel.
    pub x: u32,
    /// Top-most selected pixel.
    pub y: u32,
    /// Width of the selected region.
    pub width: u32,
    /// Height of the selected region.
    pub height: u32,
}

impl SelectionBounds {
    /// Create selection bounds.
    #[must_use]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Tile-backed raster mask storing selected pixels in document space.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionMask {
    width: u32,
    height: u32,
    tiles: BTreeMap<TileCoord, Box<[u8]>>,
}

impl SelectionMask {
    /// Create an empty selection mask aligned to a canvas.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            tiles: BTreeMap::new(),
        }
    }

    /// Return whether the selection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Clear the selection.
    pub fn clear(&mut self) {
        self.tiles.clear();
    }

    /// Select the full canvas.
    pub fn select_all(&mut self) {
        self.clear();

        for y in 0..self.height {
            for x in 0..self.width {
                self.set_selected(x, y, true);
            }
        }
    }

    /// Invert the current selection across the full canvas.
    pub fn invert(&mut self) {
        let mut inverted = SelectionMask::new(self.width, self.height);

        for y in 0..self.height {
            for x in 0..self.width {
                inverted.set_selected(x, y, !self.is_selected(x, y));
            }
        }

        *self = inverted;
    }

    /// Translate the current selection mask by a pixel delta.
    pub fn translate(&mut self, delta_x: i32, delta_y: i32) {
        let mut translated = SelectionMask::new(self.width, self.height);

        for y in 0..self.height {
            for x in 0..self.width {
                if !self.is_selected(x, y) {
                    continue;
                }

                let target_x = x as i32 + delta_x;
                let target_y = y as i32 + delta_y;
                if target_x < 0
                    || target_y < 0
                    || target_x >= self.width as i32
                    || target_y >= self.height as i32
                {
                    continue;
                }

                translated.set_selected(target_x as u32, target_y as u32, true);
            }
        }

        *self = translated;
    }

    /// Replace the selection with a rectangular marquee between two points.
    pub fn set_rect(&mut self, start_x: u32, start_y: u32, end_x: u32, end_y: u32) {
        self.clear();

        let min_x = start_x.min(end_x).min(self.width);
        let min_y = start_y.min(end_y).min(self.height);
        let max_x = start_x.max(end_x).min(self.width);
        let max_y = start_y.max(end_y).min(self.height);

        if min_x == max_x || min_y == max_y {
            return;
        }

        for y in min_y..max_y {
            for x in min_x..max_x {
                self.set_selected(x, y, true);
            }
        }
    }

    /// Return whether a pixel is selected.
    #[must_use]
    pub fn is_selected(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }

        let tile_coord = TileCoord::new(x / TILE_SIZE, y / TILE_SIZE);
        let Some(tile) = self.tiles.get(&tile_coord) else {
            return false;
        };

        let index = selection_local_offset(x % TILE_SIZE, y % TILE_SIZE);
        tile[index] != 0
    }

    /// Return the bounds of the current selection if any pixels are selected.
    #[must_use]
    pub fn bounds(&self) -> Option<SelectionBounds> {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut any_selected = false;

        for y in 0..self.height {
            for x in 0..self.width {
                if self.is_selected(x, y) {
                    any_selected = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        any_selected
            .then(|| SelectionBounds::new(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
    }

    fn set_selected(&mut self, x: u32, y: u32, selected: bool) {
        if x >= self.width || y >= self.height {
            return;
        }

        let tile_coord = TileCoord::new(x / TILE_SIZE, y / TILE_SIZE);
        let local_index = selection_local_offset(x % TILE_SIZE, y % TILE_SIZE);

        if selected {
            let tile = self
                .tiles
                .entry(tile_coord)
                .or_insert_with(new_selection_tile);
            tile[local_index] = 255;
            return;
        }

        let mut remove_tile = false;
        if let Some(tile) = self.tiles.get_mut(&tile_coord) {
            tile[local_index] = 0;
            remove_tile = tile.iter().all(|value| *value == 0);
        }

        if remove_tile {
            self.tiles.remove(&tile_coord);
        }
    }
}

/// The raster canvas size for a document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Canvas {
    /// Canvas width in pixels.
    pub width: u32,
    /// Canvas height in pixels.
    pub height: u32,
}

impl Canvas {
    /// Create a canvas with pixel dimensions.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Return the canvas size as floating-point geometry.
    #[must_use]
    pub fn size(self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }
}

/// Pixel dimensions for a generated document thumbnail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThumbnailSize {
    /// Thumbnail width in pixels.
    pub width: u32,
    /// Thumbnail height in pixels.
    pub height: u32,
}

impl ThumbnailSize {
    /// Create thumbnail dimensions.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Strategy used to derive a saved-document preview thumbnail size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThumbnailStrategy {
    /// Maximum size of the longest thumbnail edge.
    pub max_edge: u32,
}

impl Default for ThumbnailStrategy {
    fn default() -> Self {
        Self { max_edge: 256 }
    }
}

impl ThumbnailStrategy {
    /// Create a thumbnail strategy.
    #[must_use]
    pub const fn new(max_edge: u32) -> Self {
        Self { max_edge }
    }

    /// Return preview dimensions while preserving aspect ratio.
    #[must_use]
    pub fn size_for_canvas(self, canvas: Canvas) -> ThumbnailSize {
        let max_edge = self.max_edge.max(1);

        if canvas.width == 0 || canvas.height == 0 {
            return ThumbnailSize::new(max_edge, max_edge);
        }

        if canvas.width >= canvas.height {
            let height = ((canvas.height as f64 / canvas.width as f64) * max_edge as f64)
                .round()
                .max(1.0) as u32;
            ThumbnailSize::new(max_edge, height)
        } else {
            let width = ((canvas.width as f64 / canvas.height as f64) * max_edge as f64)
                .round()
                .max(1.0) as u32;
            ThumbnailSize::new(width, max_edge)
        }
    }
}

/// Supported early blend modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlendMode {
    /// Standard source-over compositing.
    Normal,
    /// Multiply blend mode.
    Multiply,
    /// Screen blend mode.
    Screen,
    /// Overlay blend mode.
    Overlay,
    /// Darken blend mode.
    Darken,
    /// Lighten blend mode.
    Lighten,
}

/// Metadata stored alongside document state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocumentMetadata {
    /// Human-friendly document title.
    pub title: String,
}

/// A raster layer in the document model.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterLayer {
    /// Stable layer identifier.
    pub id: LayerId,
    /// Layer display name.
    pub name: String,
    /// Whether the layer is visible.
    pub visible: bool,
    /// Layer opacity in the range `[0.0, 1.0]`.
    pub opacity: f32,
    /// Layer blend mode.
    pub blend_mode: BlendMode,
}

impl RasterLayer {
    /// Create a new visible raster layer with normal blending.
    #[must_use]
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
        }
    }

    /// Set the layer opacity.
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = clamp_f32(opacity, 0.0, 1.0);
    }
}

/// Tile coordinate within a tiled raster surface.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TileCoord {
    /// Tile column.
    pub x: u32,
    /// Tile row.
    pub y: u32,
}

impl TileCoord {
    /// Create a tile coordinate.
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// CPU-side raster tile storage.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterTile {
    pixels: Box<[u8]>,
}

impl RasterTile {
    fn new() -> Self {
        Self {
            pixels: vec![0; tile_byte_len()].into_boxed_slice(),
        }
    }

    /// Return the tile bytes in tightly-packed RGBA8 order.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    fn pixel(&self, local_x: u32, local_y: u32) -> [u8; 4] {
        let start = local_pixel_offset(local_x, local_y);
        let mut pixel = [0; 4];
        pixel.copy_from_slice(&self.pixels[start..start + BYTES_PER_PIXEL]);
        pixel
    }

    fn write_pixel(&mut self, local_x: u32, local_y: u32, rgba: [u8; 4]) {
        let start = local_pixel_offset(local_x, local_y);
        self.pixels[start..start + BYTES_PER_PIXEL].copy_from_slice(&rgba);
    }
}

/// A single editable tiled raster surface for the feasibility prototype.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterSurface {
    width: u32,
    height: u32,
    tiles: BTreeMap<TileCoord, RasterTile>,
    dirty_tiles: BTreeSet<TileCoord>,
}

impl RasterSurface {
    /// Create an empty raster surface.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            tiles: BTreeMap::new(),
            dirty_tiles: BTreeSet::new(),
        }
    }

    /// Return the locked tile size.
    #[must_use]
    pub const fn tile_size(&self) -> u32 {
        TILE_SIZE
    }

    /// Return the surface width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Return the surface height.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Write a pixel and mark the owning tile dirty.
    pub fn write_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }

        let tile_coord = TileCoord::new(x / TILE_SIZE, y / TILE_SIZE);
        let local_x = x % TILE_SIZE;
        let local_y = y % TILE_SIZE;
        let tile = self.tiles.entry(tile_coord).or_insert_with(RasterTile::new);
        tile.write_pixel(local_x, local_y, rgba);
        self.dirty_tiles.insert(tile_coord);
        true
    }

    /// Read a pixel from the surface.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return TRANSPARENT_PIXEL;
        }

        let tile_coord = TileCoord::new(x / TILE_SIZE, y / TILE_SIZE);
        let Some(tile) = self.tiles.get(&tile_coord) else {
            return TRANSPARENT_PIXEL;
        };

        tile.pixel(x % TILE_SIZE, y % TILE_SIZE)
    }

    /// Return a tile by coordinate.
    #[must_use]
    pub fn tile(&self, coord: TileCoord) -> Option<&RasterTile> {
        self.tiles.get(&coord)
    }

    /// Export the current surface into a tightly-packed flat RGBA8 buffer.
    #[must_use]
    pub fn to_flat_rgba(&self) -> Vec<u8> {
        let mut flat_rgba = vec![0; (self.width as usize) * (self.height as usize) * BYTES_PER_PIXEL];

        for (&tile_coord, tile) in &self.tiles {
            let tile_origin_x = tile_coord.x * TILE_SIZE;
            let tile_origin_y = tile_coord.y * TILE_SIZE;
            if tile_origin_x >= self.width || tile_origin_y >= self.height {
                continue;
            }

            let copy_width = TILE_SIZE.min(self.width - tile_origin_x) as usize;
            let copy_height = TILE_SIZE.min(self.height - tile_origin_y) as usize;
            let tile_bytes = tile.as_bytes();

            for local_y in 0..copy_height {
                let src_start = local_y * TILE_SIZE as usize * BYTES_PER_PIXEL;
                let src_end = src_start + copy_width * BYTES_PER_PIXEL;
                let dst_start = (((tile_origin_y as usize + local_y) * self.width as usize)
                    + tile_origin_x as usize)
                    * BYTES_PER_PIXEL;
                let dst_end = dst_start + copy_width * BYTES_PER_PIXEL;
                flat_rgba[dst_start..dst_end].copy_from_slice(&tile_bytes[src_start..src_end]);
            }
        }

        flat_rgba
    }

    /// Return the number of dirty tiles awaiting upload.
    #[must_use]
    pub fn dirty_tile_count(&self) -> usize {
        self.dirty_tiles.len()
    }

    /// Drain the current dirty tile set in stable order.
    pub fn take_dirty_tiles(&mut self) -> Vec<TileCoord> {
        let dirty_tiles = self.dirty_tiles.iter().copied().collect();
        self.dirty_tiles.clear();
        dirty_tiles
    }
}

fn tile_byte_len() -> usize {
    (TILE_SIZE as usize) * (TILE_SIZE as usize) * BYTES_PER_PIXEL
}

fn local_pixel_offset(local_x: u32, local_y: u32) -> usize {
    ((local_y as usize) * (TILE_SIZE as usize) + (local_x as usize)) * BYTES_PER_PIXEL
}

fn new_selection_tile() -> Box<[u8]> {
    vec![0; (TILE_SIZE as usize) * (TILE_SIZE as usize) * SELECTION_BYTES_PER_PIXEL]
        .into_boxed_slice()
}

fn selection_local_offset(local_x: u32, local_y: u32) -> usize {
    (local_y as usize) * (TILE_SIZE as usize) + (local_x as usize)
}

/// The authoritative headless document state.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    /// Stable document identifier.
    pub id: DocumentId,
    /// Canvas dimensions.
    pub canvas: Canvas,
    /// Document metadata.
    pub metadata: DocumentMetadata,
    layers: Vec<RasterLayer>,
    active_layer_id: Option<LayerId>,
    selection_mask: SelectionMask,
}

impl Document {
    /// Create a new empty document.
    #[must_use]
    pub fn new(id: DocumentId, canvas: Canvas, metadata: DocumentMetadata) -> Self {
        Self {
            id,
            canvas,
            metadata,
            layers: Vec::new(),
            active_layer_id: None,
            selection_mask: SelectionMask::new(canvas.width, canvas.height),
        }
    }

    /// Return all layers in front-to-back stack order.
    #[must_use]
    pub fn layers(&self) -> &[RasterLayer] {
        &self.layers
    }

    /// Return the currently active layer.
    #[must_use]
    pub fn active_layer(&self) -> Option<&RasterLayer> {
        let active_layer_id = self.active_layer_id?;
        self.layers.iter().find(|layer| layer.id == active_layer_id)
    }

    /// Return the raster-backed selection mask.
    #[must_use]
    pub const fn selection_mask(&self) -> &SelectionMask {
        &self.selection_mask
    }

    /// Return mutable access to the raster-backed selection mask.
    #[must_use]
    pub fn selection_mask_mut(&mut self) -> &mut SelectionMask {
        &mut self.selection_mask
    }

    /// Return the active layer id.
    #[must_use]
    pub const fn active_layer_id(&self) -> Option<LayerId> {
        self.active_layer_id
    }

    /// Return the current stack index for a layer.
    #[must_use]
    pub fn layer_index(&self, layer_id: LayerId) -> Option<usize> {
        self.layers.iter().position(|layer| layer.id == layer_id)
    }

    /// Add a layer to the top of the stack.
    pub fn add_layer(&mut self, layer: RasterLayer) {
        let layer_id = layer.id;
        self.layers.push(layer);

        if self.active_layer_id.is_none() {
            self.active_layer_id = Some(layer_id);
        }
    }

    /// Insert a layer at a target stack index.
    pub fn insert_layer(&mut self, target_index: usize, layer: RasterLayer) {
        let layer_id = layer.id;
        let new_index = target_index.min(self.layers.len());
        self.layers.insert(new_index, layer);

        if self.active_layer_id.is_none() {
            self.active_layer_id = Some(layer_id);
        }
    }

    /// Rename a layer by identifier.
    pub fn rename_layer(&mut self, layer_id: LayerId, name: impl Into<String>) -> bool {
        let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == layer_id) else {
            return false;
        };

        layer.name = name.into();
        true
    }

    /// Move a layer to a new stack position.
    pub fn reorder_layer(&mut self, layer_id: LayerId, target_index: usize) -> bool {
        let Some(current_index) = self.layers.iter().position(|layer| layer.id == layer_id) else {
            return false;
        };

        let layer = self.layers.remove(current_index);
        let new_index = target_index.min(self.layers.len());
        self.layers.insert(new_index, layer);
        true
    }

    /// Delete a layer and return it if found.
    pub fn delete_layer(&mut self, layer_id: LayerId) -> Option<RasterLayer> {
        let layer_index = self.layers.iter().position(|layer| layer.id == layer_id)?;
        let removed = self.layers.remove(layer_index);

        if self.active_layer_id == Some(layer_id) {
            self.active_layer_id = self.layers.last().map(|layer| layer.id);
        }

        Some(removed)
    }

    /// Set the active layer.
    pub fn set_active_layer(&mut self, layer_id: LayerId) -> bool {
        if self.layers.iter().any(|layer| layer.id == layer_id) {
            self.active_layer_id = Some(layer_id);
            return true;
        }

        false
    }

    /// Set layer visibility.
    pub fn set_layer_visibility(&mut self, layer_id: LayerId, visible: bool) -> bool {
        let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == layer_id) else {
            return false;
        };

        layer.visible = visible;
        true
    }

    /// Set layer opacity.
    pub fn set_layer_opacity(&mut self, layer_id: LayerId, opacity: f32) -> bool {
        let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == layer_id) else {
            return false;
        };

        layer.set_opacity(opacity);
        true
    }

    /// Replace the current selection with a rectangular marquee.
    pub fn select_rect(&mut self, start_x: u32, start_y: u32, end_x: u32, end_y: u32) {
        self.selection_mask.set_rect(start_x, start_y, end_x, end_y);
    }

    /// Clear the current selection.
    pub fn clear_selection(&mut self) {
        self.selection_mask.clear();
    }

    /// Select the full canvas.
    pub fn select_all(&mut self) {
        self.selection_mask.select_all();
    }

    /// Invert the current selection.
    pub fn invert_selection(&mut self) {
        self.selection_mask.invert();
    }

    /// Translate the current selection by a pixel delta.
    pub fn translate_selection(&mut self, delta_x: i32, delta_y: i32) {
        self.selection_mask.translate(delta_x, delta_y);
    }
}

/// Headless raster document state that carries one editable surface per raster layer.
#[derive(Clone, Debug, PartialEq)]
pub struct LayeredRasterDocument {
    document: Document,
    layer_surfaces: BTreeMap<LayerId, RasterSurface>,
}

impl LayeredRasterDocument {
    /// Create layered raster state for an existing document.
    #[must_use]
    pub fn from_document(document: Document) -> Self {
        let mut layer_surfaces = BTreeMap::new();

        for layer in document.layers() {
            layer_surfaces.insert(
                layer.id,
                RasterSurface::new(document.canvas.width, document.canvas.height),
            );
        }

        Self {
            document,
            layer_surfaces,
        }
    }

    /// Return the headless document metadata and layer stack.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Return mutable access to the document metadata and layer stack.
    #[must_use]
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// Return the number of tracked raster layer surfaces.
    #[must_use]
    pub fn layer_surface_count(&self) -> usize {
        self.layer_surfaces.len()
    }

    /// Return the active layer id.
    #[must_use]
    pub const fn active_layer_id(&self) -> Option<LayerId> {
        self.document.active_layer_id()
    }

    /// Return the current stack index for a layer.
    #[must_use]
    pub fn layer_index(&self, layer_id: LayerId) -> Option<usize> {
        self.document.layer_index(layer_id)
    }

    /// Return the raster surface for a layer.
    #[must_use]
    pub fn surface(&self, layer_id: LayerId) -> Option<&RasterSurface> {
        self.layer_surfaces.get(&layer_id)
    }

    /// Return mutable raster surface access for a layer.
    #[must_use]
    pub fn surface_mut(&mut self, layer_id: LayerId) -> Option<&mut RasterSurface> {
        self.layer_surfaces.get_mut(&layer_id)
    }

    /// Return mutable access to a layer surface and the shared selection mask.
    #[must_use]
    pub fn surface_and_selection_mut(
        &mut self,
        layer_id: LayerId,
    ) -> Option<(&mut RasterSurface, &mut SelectionMask)> {
        let layer_surfaces = &mut self.layer_surfaces;
        let document = &mut self.document;
        let surface = layer_surfaces.get_mut(&layer_id)?;
        let selection_mask = document.selection_mask_mut();

        Some((surface, selection_mask))
    }

    /// Return the currently active layer surface.
    #[must_use]
    pub fn active_surface(&self) -> Option<&RasterSurface> {
        let active_layer_id = self.document.active_layer()?.id;
        self.layer_surfaces.get(&active_layer_id)
    }

    /// Return mutable access to the currently active layer surface.
    #[must_use]
    pub fn active_surface_mut(&mut self) -> Option<&mut RasterSurface> {
        let active_layer_id = self.document.active_layer()?.id;
        self.layer_surfaces.get_mut(&active_layer_id)
    }

    /// Add a new blank raster layer and matching surface.
    pub fn create_layer(&mut self, layer: RasterLayer) -> bool {
        if self.layer_surfaces.contains_key(&layer.id) {
            return false;
        }

        let layer_id = layer.id;
        self.document.add_layer(layer);
        self.layer_surfaces.insert(
            layer_id,
            RasterSurface::new(self.document.canvas.width, self.document.canvas.height),
        );
        true
    }

    /// Insert a raster layer and matching surface at a target index.
    pub fn insert_layer_with_surface(
        &mut self,
        target_index: usize,
        layer: RasterLayer,
        surface: RasterSurface,
    ) -> bool {
        if self.layer_surfaces.contains_key(&layer.id) {
            return false;
        }

        let layer_id = layer.id;
        self.document.insert_layer(target_index, layer);
        self.layer_surfaces.insert(layer_id, surface);
        true
    }

    /// Delete a raster layer and return its metadata plus surface.
    pub fn delete_layer(&mut self, layer_id: LayerId) -> Option<(RasterLayer, RasterSurface)> {
        let layer = self.document.delete_layer(layer_id)?;
        let surface = self.layer_surfaces.remove(&layer_id).unwrap_or_else(|| {
            RasterSurface::new(self.document.canvas.width, self.document.canvas.height)
        });

        Some((layer, surface))
    }

    /// Duplicate a layer and its raster surface into a new layer id.
    pub fn duplicate_layer(
        &mut self,
        source_layer_id: LayerId,
        duplicate_layer: RasterLayer,
    ) -> bool {
        if self.layer_surfaces.contains_key(&duplicate_layer.id) {
            return false;
        }

        let Some(source_surface) = self.layer_surfaces.get(&source_layer_id).cloned() else {
            return false;
        };

        let duplicate_layer_id = duplicate_layer.id;
        self.document.add_layer(duplicate_layer);
        self.layer_surfaces
            .insert(duplicate_layer_id, source_surface);
        true
    }

    /// Rename an existing layer.
    pub fn rename_layer(&mut self, layer_id: LayerId, name: impl Into<String>) -> bool {
        self.document.rename_layer(layer_id, name)
    }

    /// Reorder an existing layer.
    pub fn reorder_layer(&mut self, layer_id: LayerId, target_index: usize) -> bool {
        self.document.reorder_layer(layer_id, target_index)
    }

    /// Set the active layer.
    pub fn set_active_layer(&mut self, layer_id: LayerId) -> bool {
        self.document.set_active_layer(layer_id)
    }

    /// Update a layer visibility flag.
    pub fn set_layer_visibility(&mut self, layer_id: LayerId, visible: bool) -> bool {
        self.document.set_layer_visibility(layer_id, visible)
    }

    /// Update a layer opacity value.
    pub fn set_layer_opacity(&mut self, layer_id: LayerId, opacity: f32) -> bool {
        self.document.set_layer_opacity(layer_id, opacity)
    }

    /// Return the current selection mask.
    #[must_use]
    pub const fn selection_mask(&self) -> &SelectionMask {
        self.document.selection_mask()
    }

    /// Replace the current selection with a rectangular marquee.
    pub fn select_rect(&mut self, start_x: u32, start_y: u32, end_x: u32, end_y: u32) {
        self.document.select_rect(start_x, start_y, end_x, end_y);
    }

    /// Clear the current selection.
    pub fn clear_selection(&mut self) {
        self.document.clear_selection();
    }

    /// Select the full canvas.
    pub fn select_all(&mut self) {
        self.document.select_all();
    }

    /// Invert the current selection.
    pub fn invert_selection(&mut self) {
        self.document.invert_selection();
    }

    /// Translate the current selection by a pixel delta.
    pub fn translate_selection(&mut self, delta_x: i32, delta_y: i32) {
        self.document.translate_selection(delta_x, delta_y);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Canvas, Document, DocumentMetadata, LayeredRasterDocument, RasterLayer, RasterSurface,
        SelectionBounds, SelectionMask, TILE_SIZE, ThumbnailSize, ThumbnailStrategy, TileCoord,
    };
    use common::{DocumentId, LayerId};

    fn test_document() -> Document {
        Document::new(
            DocumentId::new(1),
            Canvas::new(1920, 1080),
            DocumentMetadata {
                title: "Test Document".to_string(),
            },
        )
    }

    #[test]
    fn creates_empty_document_with_expected_canvas() {
        let document = test_document();

        assert_eq!(document.canvas.width, 1920);
        assert_eq!(document.canvas.height, 1080);
        assert!(document.layers().is_empty());
        assert!(document.active_layer().is_none());
    }

    #[test]
    fn thumbnail_strategy_preserves_landscape_aspect_ratio() {
        let strategy = ThumbnailStrategy::default();

        assert_eq!(
            strategy.size_for_canvas(Canvas::new(1920, 1080)),
            ThumbnailSize::new(256, 144)
        );
    }

    #[test]
    fn thumbnail_strategy_preserves_portrait_aspect_ratio() {
        let strategy = ThumbnailStrategy::default();

        assert_eq!(
            strategy.size_for_canvas(Canvas::new(900, 1600)),
            ThumbnailSize::new(144, 256)
        );
    }

    #[test]
    fn thumbnail_strategy_keeps_square_canvas_square() {
        let strategy = ThumbnailStrategy::default();

        assert_eq!(
            strategy.size_for_canvas(Canvas::new(1024, 1024)),
            ThumbnailSize::new(256, 256)
        );
    }

    #[test]
    fn selection_mask_sets_rectangular_marquee_bounds() {
        let mut selection = SelectionMask::new(512, 512);
        selection.set_rect(300, 260, 260, 300);

        assert!(selection.is_selected(260, 260));
        assert!(selection.is_selected(299, 299));
        assert!(!selection.is_selected(300, 300));
        assert_eq!(
            selection.bounds(),
            Some(SelectionBounds::new(260, 260, 40, 40))
        );
    }

    #[test]
    fn selection_mask_can_clear_select_all_and_invert() {
        let mut selection = SelectionMask::new(8, 6);
        selection.select_all();

        assert_eq!(selection.bounds(), Some(SelectionBounds::new(0, 0, 8, 6)));
        selection.invert();
        assert!(selection.is_empty());

        selection.set_rect(1, 1, 3, 3);
        selection.clear();
        assert!(selection.is_empty());
    }

    #[test]
    fn document_selection_operations_update_raster_mask() {
        let mut document = test_document();

        document.select_rect(10, 12, 16, 18);
        assert_eq!(
            document.selection_mask().bounds(),
            Some(SelectionBounds::new(10, 12, 6, 6))
        );

        document.invert_selection();
        assert!(!document.selection_mask().is_selected(10, 12));
        assert!(document.selection_mask().is_selected(0, 0));

        document.clear_selection();
        assert!(document.selection_mask().is_empty());
    }

    #[test]
    fn layered_document_exposes_selection_operations() {
        let document = test_document();
        let mut layered_document = LayeredRasterDocument::from_document(document);

        layered_document.select_rect(4, 5, 9, 9);

        assert_eq!(
            layered_document.selection_mask().bounds(),
            Some(SelectionBounds::new(4, 5, 5, 4))
        );
    }

    #[test]
    fn first_added_layer_becomes_active() {
        let mut document = test_document();
        let layer = RasterLayer::new(LayerId::new(10), "Background");

        document.add_layer(layer);

        assert_eq!(document.layers().len(), 1);
        assert_eq!(
            document.active_layer().map(|layer| layer.name.as_str()),
            Some("Background")
        );
    }

    #[test]
    fn renames_and_reorders_layers() {
        let mut document = test_document();
        let first = RasterLayer::new(LayerId::new(1), "First");
        let second = RasterLayer::new(LayerId::new(2), "Second");

        document.add_layer(first);
        document.add_layer(second);

        assert!(document.rename_layer(LayerId::new(2), "Paint"));
        assert!(document.reorder_layer(LayerId::new(2), 0));
        assert_eq!(document.layers()[0].name, "Paint");
    }

    #[test]
    fn deleting_active_layer_selects_last_remaining_layer() {
        let mut document = test_document();
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        document.add_layer(RasterLayer::new(LayerId::new(2), "Paint"));
        assert!(document.set_active_layer(LayerId::new(2)));

        let removed = document.delete_layer(LayerId::new(2));

        assert_eq!(removed.map(|layer| layer.name), Some("Paint".to_string()));
        assert_eq!(
            document.active_layer().map(|layer| layer.id),
            Some(LayerId::new(1))
        );
    }

    #[test]
    fn layered_raster_document_creates_surface_for_each_layer() {
        let mut document = test_document();
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        document.add_layer(RasterLayer::new(LayerId::new(2), "Overlay"));

        let layered_document = LayeredRasterDocument::from_document(document);

        assert_eq!(layered_document.layer_surface_count(), 2);
        assert_eq!(
            layered_document
                .surface(LayerId::new(1))
                .map(RasterSurface::width),
            Some(1920)
        );
        assert_eq!(
            layered_document
                .surface(LayerId::new(2))
                .map(RasterSurface::height),
            Some(1080)
        );
    }

    #[test]
    fn layered_raster_document_targets_active_layer_surface() {
        let mut document = test_document();
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        document.add_layer(RasterLayer::new(LayerId::new(2), "Paint"));
        assert!(document.set_active_layer(LayerId::new(2)));

        let mut layered_document = LayeredRasterDocument::from_document(document);
        let active_surface = layered_document
            .active_surface_mut()
            .expect("active layer surface should exist");
        let _ = active_surface.write_pixel(8, 8, [255, 0, 0, 255]);

        assert_eq!(
            layered_document
                .surface(LayerId::new(1))
                .map(|surface| surface.pixel(8, 8)),
            Some([0, 0, 0, 0])
        );
        assert_eq!(
            layered_document
                .surface(LayerId::new(2))
                .map(|surface| surface.pixel(8, 8)),
            Some([255, 0, 0, 255])
        );
    }

    #[test]
    fn layered_raster_document_can_create_layer() {
        let mut document = test_document();
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        let mut layered_document = LayeredRasterDocument::from_document(document);

        assert!(layered_document.create_layer(RasterLayer::new(LayerId::new(2), "Sketch")));
        assert_eq!(layered_document.layer_surface_count(), 2);
        assert_eq!(layered_document.document().layers()[1].name, "Sketch");
        assert_eq!(
            layered_document
                .surface(LayerId::new(2))
                .map(RasterSurface::width),
            Some(1920)
        );
    }

    #[test]
    fn layered_raster_document_can_delete_layer() {
        let mut document = test_document();
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        document.add_layer(RasterLayer::new(LayerId::new(2), "Paint"));
        let mut layered_document = LayeredRasterDocument::from_document(document);

        let removed = layered_document.delete_layer(LayerId::new(2));

        assert_eq!(
            removed.map(|(layer, _)| layer.name),
            Some("Paint".to_string())
        );
        assert_eq!(layered_document.layer_surface_count(), 1);
        assert!(layered_document.surface(LayerId::new(2)).is_none());
    }

    #[test]
    fn layered_raster_document_can_duplicate_layer_with_pixels() {
        let mut document = test_document();
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        let mut layered_document = LayeredRasterDocument::from_document(document);

        let _ = layered_document
            .surface_mut(LayerId::new(1))
            .expect("base surface should exist")
            .write_pixel(12, 14, [10, 20, 30, 255]);

        assert!(layered_document.duplicate_layer(
            LayerId::new(1),
            RasterLayer::new(LayerId::new(2), "Base Copy")
        ));

        assert_eq!(layered_document.layer_surface_count(), 2);
        assert_eq!(
            layered_document
                .surface(LayerId::new(2))
                .map(|surface| surface.pixel(12, 14)),
            Some([10, 20, 30, 255])
        );
    }

    #[test]
    fn layered_raster_document_can_update_layer_metadata() {
        let mut document = test_document();
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        document.add_layer(RasterLayer::new(LayerId::new(2), "Paint"));
        let mut layered_document = LayeredRasterDocument::from_document(document);

        assert!(layered_document.rename_layer(LayerId::new(2), "Ink"));
        assert!(layered_document.reorder_layer(LayerId::new(2), 0));
        assert!(layered_document.set_active_layer(LayerId::new(2)));
        assert!(layered_document.set_layer_visibility(LayerId::new(2), false));
        assert!(layered_document.set_layer_opacity(LayerId::new(2), 0.35));

        let first_layer = &layered_document.document().layers()[0];
        assert_eq!(first_layer.id, LayerId::new(2));
        assert_eq!(first_layer.name, "Ink");
        assert!(!first_layer.visible);
        assert_eq!(first_layer.opacity, 0.35);
        assert_eq!(
            layered_document
                .document()
                .active_layer()
                .map(|layer| layer.id),
            Some(LayerId::new(2))
        );
    }

    #[test]
    fn raster_surface_uses_locked_tile_size() {
        let surface = RasterSurface::new(2048, 1024);

        assert_eq!(surface.tile_size(), TILE_SIZE);
    }

    #[test]
    fn raster_surface_tracks_dirty_tiles_when_pixels_change() {
        let mut surface = RasterSurface::new(512, 512);

        assert!(surface.write_pixel(1, 1, [255, 0, 0, 255]));
        assert!(surface.write_pixel(300, 300, [0, 255, 0, 255]));

        let dirty_tiles = surface.take_dirty_tiles();
        assert_eq!(
            dirty_tiles,
            vec![TileCoord::new(0, 0), TileCoord::new(1, 1)]
        );
        assert_eq!(surface.dirty_tile_count(), 0);
    }

    #[test]
    fn raster_surface_reads_transparent_pixels_from_unallocated_tiles() {
        let surface = RasterSurface::new(512, 512);

        assert_eq!(surface.pixel(10, 10), [0, 0, 0, 0]);
    }

    #[test]
    fn raster_surface_writes_and_reads_pixels() {
        let mut surface = RasterSurface::new(512, 512);

        assert!(surface.write_pixel(255, 255, [10, 20, 30, 255]));
        assert_eq!(surface.pixel(255, 255), [10, 20, 30, 255]);
        assert!(surface.tile(TileCoord::new(0, 0)).is_some());
    }
}
