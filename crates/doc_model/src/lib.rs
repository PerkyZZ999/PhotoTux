use common::{CanvasRect, CanvasSize, DocumentId, LayerId, DEFAULT_TILE_SIZE};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub offset_x: i32,
    pub offset_y: i32,
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
            offset_x: 0,
            offset_y: 0,
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

pub type RectSelection = CanvasRect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub canvas_size: CanvasSize,
    pub layers: Vec<RasterLayer>,
    pub active_layer_index: usize,
    pub tile_size: u32,
    pub selection: Option<RectSelection>,
    pub selection_inverted: bool,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            id: DocumentId::new(),
            canvas_size: CanvasSize::new(width, height),
            layers: vec![RasterLayer::new("Background")],
            active_layer_index: 0,
            tile_size: DEFAULT_TILE_SIZE,
            selection: None,
            selection_inverted: false,
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

    pub fn active_layer_index(&self) -> usize {
        self.active_layer_index
    }

    pub fn layer(&self, index: usize) -> Option<&RasterLayer> {
        self.layers.get(index)
    }

    pub fn layer_mut(&mut self, index: usize) -> Option<&mut RasterLayer> {
        self.layers.get_mut(index)
    }

    pub fn layer_index_by_id(&self, layer_id: LayerId) -> Option<usize> {
        self.layers.iter().position(|layer| layer.id == layer_id)
    }

    pub fn tile_origin(&self, coord: TileCoord) -> (u32, u32) {
        (coord.x * self.tile_size, coord.y * self.tile_size)
    }

    pub fn layer_offset(&self, index: usize) -> Option<(i32, i32)> {
        let layer = self.layers.get(index)?;
        Some((layer.offset_x, layer.offset_y))
    }

    pub fn layer_canvas_bounds(&self, index: usize) -> Option<CanvasRect> {
        let layer = self.layers.get(index)?;
        let mut tile_iter = layer.tiles.keys();
        let first = *tile_iter.next()?;
        let mut min_x = first.x;
        let mut max_x = first.x;
        let mut min_y = first.y;
        let mut max_y = first.y;

        for coord in tile_iter {
            min_x = min_x.min(coord.x);
            max_x = max_x.max(coord.x);
            min_y = min_y.min(coord.y);
            max_y = max_y.max(coord.y);
        }

        Some(CanvasRect::new(
            min_x as i32 * self.tile_size as i32 + layer.offset_x,
            min_y as i32 * self.tile_size as i32 + layer.offset_y,
            (max_x - min_x + 1) * self.tile_size,
            (max_y - min_y + 1) * self.tile_size,
        ))
    }

    pub fn selection(&self) -> Option<RectSelection> {
        self.selection
    }

    pub fn selection_inverted(&self) -> bool {
        self.selection_inverted
    }

    pub fn set_selection_state(&mut self, selection: Option<RectSelection>, inverted: bool) {
        self.selection = selection;
        self.selection_inverted = selection.is_some() && inverted;
    }

    pub fn set_selection(&mut self, selection: RectSelection) {
        self.set_selection_state(Some(selection), false);
    }

    pub fn clear_selection(&mut self) {
        self.set_selection_state(None, false);
    }

    pub fn invert_selection(&mut self) -> bool {
        if self.selection.is_none() {
            return false;
        }

        self.selection_inverted = !self.selection_inverted;
        true
    }

    pub fn selection_contains_pixel(&self, pixel_x: i32, pixel_y: i32) -> bool {
        let Some(selection) = self.selection else {
            return false;
        };

        let right = selection.x + selection.width as i32;
        let bottom = selection.y + selection.height as i32;
        pixel_x >= selection.x && pixel_x < right && pixel_y >= selection.y && pixel_y < bottom
    }

    pub fn allows_pixel_edit(&self, pixel_x: i32, pixel_y: i32) -> bool {
        if self.selection.is_none() {
            return true;
        }

        self.selection_contains_pixel(pixel_x, pixel_y) != self.selection_inverted
    }

    pub fn tile_coords_in_radius(&self, center_x: f32, center_y: f32, radius: f32) -> Vec<TileCoord> {
        if radius <= 0.0 {
            return Vec::new();
        }

        let min_x = center_x - radius;
        let min_y = center_y - radius;
        let max_x = center_x + radius;
        let max_y = center_y + radius;

        if max_x < 0.0
            || max_y < 0.0
            || min_x >= self.canvas_size.width as f32
            || min_y >= self.canvas_size.height as f32
        {
            return Vec::new();
        }

        let start_tile_x = (min_x.max(0.0) as u32) / self.tile_size;
        let start_tile_y = (min_y.max(0.0) as u32) / self.tile_size;
        let end_tile_x = (max_x.floor().max(0.0) as u32).min(self.canvas_size.width.saturating_sub(1)) / self.tile_size;
        let end_tile_y = (max_y.floor().max(0.0) as u32).min(self.canvas_size.height.saturating_sub(1)) / self.tile_size;

        let mut coords = Vec::new();
        for tile_y in start_tile_y..=end_tile_y {
            for tile_x in start_tile_x..=end_tile_x {
                coords.push(TileCoord::new(tile_x, tile_y));
            }
        }

        coords
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let layer = RasterLayer::new(name);
        let layer_id = layer.id;
        self.layers.insert(self.active_layer_index + 1, layer);
        self.active_layer_index += 1;
        layer_id
    }

    pub fn set_active_layer(&mut self, index: usize) -> bool {
        if index >= self.layers.len() {
            return false;
        }

        self.active_layer_index = index;
        true
    }

    pub fn duplicate_layer(&mut self, index: usize) -> Option<LayerId> {
        let source = self.layers.get(index)?.clone();
        let duplicate_id = LayerId::new();
        let duplicate_name = format!("{} copy", source.name);

        let duplicate = RasterLayer {
            id: duplicate_id,
            name: duplicate_name,
            visible: source.visible,
            opacity_percent: source.opacity_percent,
            blend_mode: source.blend_mode,
            offset_x: source.offset_x,
            offset_y: source.offset_y,
            tiles: source.tiles,
            dirty_tiles: HashSet::new(),
        };

        self.layers.insert(index + 1, duplicate);
        self.active_layer_index = index + 1;
        Some(duplicate_id)
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

    pub fn tile_snapshot(&self, layer_index: usize, coord: TileCoord) -> Option<RasterTile> {
        self.layers.get(layer_index)?.tiles.get(&coord).cloned()
    }

    pub fn apply_tile_snapshot(
        &mut self,
        layer_id: LayerId,
        coord: TileCoord,
        tile: Option<RasterTile>,
    ) -> bool {
        let Some(layer_index) = self.layer_index_by_id(layer_id) else {
            return false;
        };

        let Some(layer) = self.layers.get_mut(layer_index) else {
            return false;
        };

        match tile {
            Some(tile) => {
                layer.tiles.insert(coord, tile);
                layer.mark_tile_dirty(coord);
            }
            None => {
                layer.tiles.remove(&coord);
                layer.mark_tile_dirty(coord);
            }
        }

        true
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

    pub fn set_layer_offset(&mut self, index: usize, offset_x: i32, offset_y: i32) -> bool {
        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };

        layer.offset_x = offset_x;
        layer.offset_y = offset_y;
        true
    }

    pub fn translate_layer(&mut self, index: usize, delta_x: i32, delta_y: i32) -> bool {
        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };

        layer.offset_x += delta_x;
        layer.offset_y += delta_y;
        true
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
    use super::{BlendMode, Document, RasterTile, TileCoord};
    use common::CanvasRect;

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
    fn duplicate_layer_clones_tiles_and_activates_copy() {
        let mut document = Document::new(512, 512);
        document.rename_layer(0, "Paint");
        let tile = document
            .ensure_tile_for_pixel(0, 25, 25)
            .expect("tile should be created");
        tile.pixels[0] = 120;
        tile.pixels[3] = 255;

        let duplicate_id = document
            .duplicate_layer(0)
            .expect("layer duplication should succeed");

        assert_eq!(document.layer_count(), 2);
        assert_eq!(document.active_layer_index(), 1);
        assert_eq!(document.layers[1].name, "Paint copy");
        assert_ne!(document.layers[0].id, duplicate_id);
        assert_eq!(document.layers[1].id, duplicate_id);
        assert_eq!(document.layers[1].tiles, document.layers[0].tiles);
    }

    #[test]
    fn set_active_layer_rejects_invalid_indices() {
        let mut document = Document::new(320, 240);
        document.add_layer("Top");

        assert!(document.set_active_layer(0));
        assert_eq!(document.active_layer_index(), 0);
        assert!(!document.set_active_layer(99));
        assert_eq!(document.active_layer_index(), 0);
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
    fn layer_offset_updates_and_translates() {
        let mut document = Document::new(640, 480);

        assert!(document.set_layer_offset(0, 12, -8));
        assert_eq!(document.layer_offset(0), Some((12, -8)));

        assert!(document.translate_layer(0, 5, 10));
        assert_eq!(document.layer_offset(0), Some((17, 2)));
    }

    #[test]
    fn layer_canvas_bounds_include_tile_region_and_offset() {
        let mut document = Document::new(1024, 1024);
        let _ = document.ensure_tile_for_pixel(0, 300, 20);
        let _ = document.ensure_tile_for_pixel(0, 700, 500);
        assert!(document.set_layer_offset(0, 5, -3));

        let bounds = document.layer_canvas_bounds(0).expect("bounds should exist");

        assert_eq!(bounds, CanvasRect::new(261, -3, 512, 512));
    }

    #[test]
    fn selection_can_be_set_and_cleared() {
        let mut document = Document::new(320, 240);
        let selection = CanvasRect::new(10, 20, 30, 40);

        document.set_selection(selection);
        assert_eq!(document.selection(), Some(selection));
        assert!(!document.selection_inverted());

        document.clear_selection();
        assert_eq!(document.selection(), None);
        assert!(!document.selection_inverted());
    }

    #[test]
    fn selection_can_be_inverted() {
        let mut document = Document::new(320, 240);
        document.set_selection(CanvasRect::new(5, 6, 7, 8));

        assert!(document.invert_selection());
        assert!(document.selection_inverted());

        assert!(document.invert_selection());
        assert!(!document.selection_inverted());
    }

    #[test]
    fn selection_pixel_tests_use_exclusive_bottom_right_edge() {
        let mut document = Document::new(320, 240);
        document.set_selection(CanvasRect::new(10, 20, 30, 40));

        assert!(document.selection_contains_pixel(10, 20));
        assert!(document.selection_contains_pixel(39, 59));
        assert!(!document.selection_contains_pixel(40, 59));
        assert!(!document.selection_contains_pixel(39, 60));
    }

    #[test]
    fn allows_pixel_edit_respects_normal_and_inverted_selection() {
        let mut document = Document::new(320, 240);

        assert!(document.allows_pixel_edit(2, 3));

        document.set_selection(CanvasRect::new(10, 20, 30, 40));
        assert!(document.allows_pixel_edit(20, 30));
        assert!(!document.allows_pixel_edit(2, 3));

        assert!(document.invert_selection());
        assert!(!document.allows_pixel_edit(20, 30));
        assert!(document.allows_pixel_edit(2, 3));
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

    #[test]
    fn tile_coords_in_radius_returns_touched_tiles() {
        let document = Document::new(600, 600);

        let coords = document.tile_coords_in_radius(260.0, 260.0, 40.0);

        assert_eq!(
            coords,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(0, 1),
                TileCoord::new(1, 1),
            ]
        );
    }

    #[test]
    fn apply_tile_snapshot_restores_tile_presence() {
        let mut document = Document::new(512, 512);
        let layer_id = document.layers[0].id;
        let coord = TileCoord::new(1, 1);
        let tile = RasterTile::new(document.tile_size);

        assert!(document.apply_tile_snapshot(layer_id, coord, Some(tile.clone())));
        assert!(document.tile_snapshot(0, coord).is_some());
        assert!(document.apply_tile_snapshot(layer_id, coord, None));
        assert!(document.tile_snapshot(0, coord).is_none());
    }
}
