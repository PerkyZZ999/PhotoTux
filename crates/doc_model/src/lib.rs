use common::{CanvasSize, DocumentId, LayerId, DEFAULT_TILE_SIZE};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterTile {
    pub pixels: Vec<u8>,
}

impl RasterTile {
    pub fn new(tile_size: u32) -> Self {
        let pixel_count = tile_size as usize * tile_size as usize * 4;
        Self {
            pixels: vec![0; pixel_count],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterLayer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub opacity_percent: u8,
    pub blend_mode: BlendMode,
    pub tiles: HashMap<TileCoord, RasterTile>,
    pub dirty_tiles: HashSet<TileCoord>,
}

impl RasterLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            opacity_percent: 100,
            blend_mode: BlendMode::Normal,
            tiles: HashMap::new(),
            dirty_tiles: HashSet::new(),
        }
    }

    pub fn ensure_tile(&mut self, coord: TileCoord, tile_size: u32) -> &mut RasterTile {
        self.dirty_tiles.insert(coord);
        self.tiles
            .entry(coord)
            .or_insert_with(|| RasterTile::new(tile_size))
    }

    pub fn mark_tile_dirty(&mut self, coord: TileCoord) {
        self.dirty_tiles.insert(coord);
    }

    pub fn take_dirty_tiles(&mut self) -> Vec<TileCoord> {
        let mut dirty_tiles = self.dirty_tiles.drain().collect::<Vec<_>>();
        dirty_tiles.sort_by_key(|coord| (coord.y, coord.x));
        dirty_tiles
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGridSize {
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub canvas_size: CanvasSize,
    pub layers: Vec<RasterLayer>,
    pub active_layer_index: usize,
    pub tile_size: u32,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            id: DocumentId::new(),
            canvas_size: CanvasSize::new(width, height),
            layers: vec![RasterLayer::new("Background")],
            active_layer_index: 0,
            tile_size: DEFAULT_TILE_SIZE,
        }
    }

    pub fn tile_grid_size(&self) -> TileGridSize {
        TileGridSize {
            columns: self.canvas_size.width.div_ceil(self.tile_size),
            rows: self.canvas_size.height.div_ceil(self.tile_size),
        }
    }

    pub fn tile_coord_for_pixel(&self, pixel_x: u32, pixel_y: u32) -> Option<TileCoord> {
        if pixel_x >= self.canvas_size.width || pixel_y >= self.canvas_size.height {
            return None;
        }

        Some(TileCoord::new(pixel_x / self.tile_size, pixel_y / self.tile_size))
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn active_layer(&self) -> &RasterLayer {
        &self.layers[self.active_layer_index]
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let layer = RasterLayer::new(name);
        let layer_id = layer.id;
        self.layers.insert(self.active_layer_index + 1, layer);
        self.active_layer_index += 1;
        layer_id
    }

    pub fn ensure_tile_for_pixel(
        &mut self,
        layer_index: usize,
        pixel_x: u32,
        pixel_y: u32,
    ) -> Option<&mut RasterTile> {
        let coord = self.tile_coord_for_pixel(pixel_x, pixel_y)?;
        let tile_size = self.tile_size;
        let layer = self.layers.get_mut(layer_index)?;
        Some(layer.ensure_tile(coord, tile_size))
    }

    pub fn dirty_tiles(&self, layer_index: usize) -> Option<&HashSet<TileCoord>> {
        Some(&self.layers.get(layer_index)?.dirty_tiles)
    }

    pub fn rename_layer(&mut self, index: usize, name: impl Into<String>) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.name = name.into();
        }
    }

    pub fn set_layer_visibility(&mut self, index: usize, visible: bool) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.visible = visible;
        }
    }

    pub fn set_layer_opacity(&mut self, index: usize, opacity_percent: u8) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.opacity_percent = opacity_percent.min(100);
        }
    }

    pub fn move_layer(&mut self, from_index: usize, to_index: usize) -> bool {
        if from_index >= self.layers.len() || to_index >= self.layers.len() || from_index == to_index {
            return false;
        }

        let layer = self.layers.remove(from_index);
        self.layers.insert(to_index, layer);
        self.active_layer_index = to_index;
        true
    }

    pub fn delete_layer(&mut self, index: usize) -> bool {
        if self.layers.len() <= 1 || index >= self.layers.len() {
            return false;
        }

        self.layers.remove(index);
        if self.active_layer_index >= self.layers.len() {
            self.active_layer_index = self.layers.len() - 1;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::{BlendMode, Document, TileCoord};

    #[test]
    fn new_document_starts_with_background_layer() {
        let document = Document::new(1920, 1080);

        assert_eq!(document.canvas_size.width, 1920);
        assert_eq!(document.canvas_size.height, 1080);
        assert_eq!(document.layer_count(), 1);
        assert_eq!(document.active_layer().name, "Background");
        assert_eq!(document.active_layer().blend_mode, BlendMode::Normal);
        assert_eq!(document.tile_grid_size().columns, 8);
        assert_eq!(document.tile_grid_size().rows, 5);
    }

    #[test]
    fn add_layer_inserts_after_active_layer() {
        let mut document = Document::new(640, 480);

        document.add_layer("Sketch");

        assert_eq!(document.layer_count(), 2);
        assert_eq!(document.active_layer_index, 1);
        assert_eq!(document.active_layer().name, "Sketch");
    }

    #[test]
    fn move_layer_reorders_layers() {
        let mut document = Document::new(640, 480);
        document.add_layer("Sketch");
        document.add_layer("Highlights");

        let moved = document.move_layer(2, 0);

        assert!(moved);
        assert_eq!(document.layers[0].name, "Highlights");
        assert_eq!(document.active_layer_index, 0);
    }

    #[test]
    fn delete_layer_keeps_at_least_one_layer() {
        let mut document = Document::new(640, 480);

        assert!(!document.delete_layer(0));

        document.add_layer("Sketch");
        assert!(document.delete_layer(1));
        assert_eq!(document.layer_count(), 1);
    }

    #[test]
    fn layer_property_updates_are_clamped_and_applied() {
        let mut document = Document::new(640, 480);

        document.rename_layer(0, "Base");
        document.set_layer_visibility(0, false);
        document.set_layer_opacity(0, 255);

        assert_eq!(document.layers[0].name, "Base");
        assert!(!document.layers[0].visible);
        assert_eq!(document.layers[0].opacity_percent, 100);
    }

    #[test]
    fn tile_coord_for_pixel_maps_pixels_to_tile_grid() {
        let document = Document::new(1024, 1024);

        assert_eq!(document.tile_coord_for_pixel(0, 0), Some(TileCoord::new(0, 0)));
        assert_eq!(document.tile_coord_for_pixel(255, 255), Some(TileCoord::new(0, 0)));
        assert_eq!(document.tile_coord_for_pixel(256, 256), Some(TileCoord::new(1, 1)));
        assert_eq!(document.tile_coord_for_pixel(1024, 0), None);
    }

    #[test]
    fn ensure_tile_for_pixel_creates_and_marks_dirty_tile() {
        let mut document = Document::new(512, 512);

        let tile = document
            .ensure_tile_for_pixel(0, 300, 20)
            .expect("tile should exist for a valid layer and pixel");

        assert_eq!(tile.pixels.len(), 256 * 256 * 4);
        assert!(document
            .dirty_tiles(0)
            .expect("layer should exist")
            .contains(&TileCoord::new(1, 0)));
    }

    #[test]
    fn take_dirty_tiles_returns_sorted_coordinates() {
        let mut document = Document::new(512, 512);
        let layer = &mut document.layers[0];
        layer.mark_tile_dirty(TileCoord::new(1, 1));
        layer.mark_tile_dirty(TileCoord::new(0, 0));
        layer.mark_tile_dirty(TileCoord::new(1, 0));

        let dirty_tiles = layer.take_dirty_tiles();

        assert_eq!(
            dirty_tiles,
            vec![TileCoord::new(0, 0), TileCoord::new(1, 0), TileCoord::new(1, 1)]
        );
        assert!(layer.dirty_tiles.is_empty());
    }
}
