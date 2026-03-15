use common::{CanvasRect, LayerId};
use doc_model::{Document, RasterTile, RectSelection, TileCoord};
use image_ops::{apply_round_brush_dab_clipped, apply_round_eraser_dab_clipped, BrushDab};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Brush,
    Eraser,
    Move,
    RectangularMarquee,
    Hand,
    Zoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushToolMode {
    Paint,
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSettings {
    pub radius: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub spacing: f32,
    pub color: [u8; 4],
}

impl BrushSettings {
    pub fn to_dab(self) -> BrushDab {
        BrushDab::new(self.radius, self.hardness, self.opacity, self.color)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileChange {
    pub layer_id: LayerId,
    pub coord: TileCoord,
    pub before: Option<RasterTile>,
    pub after: Option<RasterTile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrushStrokeRecord {
    pub layer_id: LayerId,
    pub dab_count: usize,
    pub changes: Vec<TileChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveLayerRecord {
    pub layer_id: LayerId,
    pub before_offset: (i32, i32),
    pub after_offset: (i32, i32),
}

impl MoveLayerRecord {
    pub fn undo(&self, document: &mut Document) {
        if let Some(layer_index) = document.layer_index_by_id(self.layer_id) {
            let _ = document.set_layer_offset(layer_index, self.before_offset.0, self.before_offset.1);
        }
    }

    pub fn redo(&self, document: &mut Document) {
        if let Some(layer_index) = document.layer_index_by_id(self.layer_id) {
            let _ = document.set_layer_offset(layer_index, self.after_offset.0, self.after_offset.1);
        }
    }
}

impl BrushStrokeRecord {
    pub fn undo(&self, document: &mut Document) {
        for change in &self.changes {
            let _ = document.apply_tile_snapshot(change.layer_id, change.coord, change.before.clone());
        }
    }

    pub fn redo(&self, document: &mut Document) {
        for change in &self.changes {
            let _ = document.apply_tile_snapshot(change.layer_id, change.coord, change.after.clone());
        }
    }
}

pub struct BrushTool;
pub struct MoveTool;
pub struct RectangularMarqueeTool;

impl BrushTool {
    pub fn apply_stroke(
        document: &mut Document,
        layer_index: usize,
        points: &[(f32, f32)],
        settings: BrushSettings,
        mode: BrushToolMode,
    ) -> Option<BrushStrokeRecord> {
        if points.is_empty() || settings.radius <= 0.0 {
            return None;
        }

        let layer_id = document.layer(layer_index)?.id;
        let tile_size = document.tile_size;
        let dab = settings.to_dab();
        let dab_positions = interpolate_dab_positions(points, settings.spacing.max(1.0));
        let clip_rect = document.selection();
        let clip_inverted = document.selection_inverted();
        let mut changes = Vec::<TileChange>::new();

        for &(dab_x, dab_y) in &dab_positions {
            let touched_coords = document.tile_coords_in_radius(dab_x, dab_y, settings.radius);

            for coord in touched_coords {
                if changes.iter().all(|change| !(change.layer_id == layer_id && change.coord == coord)) {
                    changes.push(TileChange {
                        layer_id,
                        coord,
                        before: document.tile_snapshot(layer_index, coord),
                        after: None,
                    });
                }

                let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
                let changed = {
                    let tile = document.layer_mut(layer_index)?.ensure_tile(coord, tile_size);
                    match mode {
                        BrushToolMode::Paint => apply_round_brush_dab_clipped(
                            &mut tile.pixels,
                            tile_size,
                            tile_origin_x,
                            tile_origin_y,
                            dab_x,
                            dab_y,
                            dab,
                            clip_rect,
                            clip_inverted,
                        ),
                        BrushToolMode::Erase => apply_round_eraser_dab_clipped(
                            &mut tile.pixels,
                            tile_size,
                            tile_origin_x,
                            tile_origin_y,
                            dab_x,
                            dab_y,
                            dab,
                            clip_rect,
                            clip_inverted,
                        ),
                    }
                };

                if !changed {
                    if document.layer(layer_index)?.tiles.get(&coord).is_some_and(|tile| tile.pixels.iter().all(|value| *value == 0))
                    {
                        document.layer_mut(layer_index)?.tiles.remove(&coord);
                    }
                    continue;
                }
            }
        }

        changes.retain(|change| {
            let after = document
                .layer_index_by_id(change.layer_id)
                .and_then(|idx| document.tile_snapshot(idx, change.coord));
            change.before != after
        });

        if changes.is_empty() {
            return None;
        }

        for change in &mut changes {
            let Some(layer_index) = document.layer_index_by_id(change.layer_id) else {
                continue;
            };
            change.after = document.tile_snapshot(layer_index, change.coord);
        }

        Some(BrushStrokeRecord {
            layer_id,
            dab_count: dab_positions.len(),
            changes,
        })
    }
}

impl MoveTool {
    pub fn move_layer(
        document: &mut Document,
        layer_index: usize,
        delta_x: i32,
        delta_y: i32,
    ) -> Option<MoveLayerRecord> {
        if delta_x == 0 && delta_y == 0 {
            return None;
        }

        let layer = document.layer(layer_index)?;
        let before_offset = (layer.offset_x, layer.offset_y);
        let after_offset = (before_offset.0 + delta_x, before_offset.1 + delta_y);
        let layer_id = layer.id;

        document.set_layer_offset(layer_index, after_offset.0, after_offset.1);

        Some(MoveLayerRecord {
            layer_id,
            before_offset,
            after_offset,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RectangularSelectionRecord {
    pub before: Option<RectSelection>,
    pub before_inverted: bool,
    pub after: Option<RectSelection>,
    pub after_inverted: bool,
}

impl RectangularSelectionRecord {
    pub fn undo(&self, document: &mut Document) {
        document.set_selection_state(self.before, self.before_inverted);
    }

    pub fn redo(&self, document: &mut Document) {
        document.set_selection_state(self.after, self.after_inverted);
    }
}

impl RectangularMarqueeTool {
    pub fn preview_rect(start_x: i32, start_y: i32, end_x: i32, end_y: i32) -> Option<CanvasRect> {
        let left = start_x.min(end_x);
        let top = start_y.min(end_y);
        let right = start_x.max(end_x);
        let bottom = start_y.max(end_y);

        let width = (right - left) as u32;
        let height = (bottom - top) as u32;
        if width == 0 || height == 0 {
            return None;
        }

        Some(CanvasRect::new(left, top, width, height))
    }

    pub fn apply_selection(
        document: &mut Document,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
    ) -> Option<RectangularSelectionRecord> {
        let before = document.selection();
        let before_inverted = document.selection_inverted();
        let after = Self::preview_rect(start_x, start_y, end_x, end_y);

        if before == after && !before_inverted {
            return None;
        }

        match after {
            Some(selection) => document.set_selection(selection),
            None => document.clear_selection(),
        }

        Some(RectangularSelectionRecord {
            before,
            before_inverted,
            after,
            after_inverted: false,
        })
    }
}

fn interpolate_dab_positions(points: &[(f32, f32)], spacing: f32) -> Vec<(f32, f32)> {
    let mut positions = vec![points[0]];

    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        let delta_x = end.0 - start.0;
        let delta_y = end.1 - start.1;
        let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();

        if distance == 0.0 {
            continue;
        }

        let steps = (distance / spacing).ceil() as usize;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            positions.push((start.0 + delta_x * t, start.1 + delta_y * t));
        }
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::{BrushSettings, BrushTool, BrushToolMode, MoveTool, RectangularMarqueeTool};
    use common::CanvasRect;
    use doc_model::{Document, TileCoord};
    use history_engine::HistoryStack;

    fn brush_settings() -> BrushSettings {
        BrushSettings {
            radius: 6.0,
            hardness: 0.8,
            opacity: 1.0,
            spacing: 4.0,
            color: [255, 0, 0, 255],
        }
    }

    fn pixel_alpha(document: &Document, layer_index: usize, pixel_x: u32, pixel_y: u32) -> u8 {
        let coord = document
            .tile_coord_for_pixel(pixel_x, pixel_y)
            .unwrap_or(TileCoord::new(0, 0));
        let tile = document
            .layer(layer_index)
            .expect("layer exists")
            .tiles
            .get(&coord)
            .expect("tile exists");
        let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
        let local_x = (pixel_x - tile_origin_x) as usize;
        let local_y = (pixel_y - tile_origin_y) as usize;
        tile.pixels[(local_y * document.tile_size as usize + local_x) * 4 + 3]
    }

    #[test]
    fn brush_stroke_updates_document_tiles() {
        let mut document = Document::new(512, 512);

        let record = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(10.0, 10.0), (30.0, 10.0)],
            brush_settings(),
            BrushToolMode::Paint,
        )
        .expect("stroke should produce a history record");

        assert!(record.dab_count >= 2);
        assert!(!record.changes.is_empty());
        assert!(document.layer(0).expect("layer exists").tiles.len() >= 1);
    }

    #[test]
    fn brush_stroke_can_be_undone_and_redone() {
        let mut document = Document::new(512, 512);

        let record = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(40.0, 40.0), (80.0, 40.0)],
            brush_settings(),
            BrushToolMode::Paint,
        )
        .expect("stroke should modify the layer");

        let painted_tile_count = document.layer(0).expect("layer exists").tiles.len();
        record.undo(&mut document);
        assert_eq!(document.layer(0).expect("layer exists").tiles.len(), 0);

        record.redo(&mut document);
        assert_eq!(document.layer(0).expect("layer exists").tiles.len(), painted_tile_count);
    }

    #[test]
    fn eraser_stroke_reduces_existing_content() {
        let mut document = Document::new(512, 512);
        let _paint = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(64.0, 64.0)],
            brush_settings(),
            BrushToolMode::Paint,
        )
        .expect("paint stroke should apply");

        let coord = document
            .tile_coord_for_pixel(64, 64)
            .expect("stroke position should map to a tile");
        let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
        let local_x = (64 - tile_origin_x) as usize;
        let local_y = (64 - tile_origin_y) as usize;
        let pixel_index = (local_y * document.tile_size as usize + local_x) * 4 + 3;

        let alpha_before = document
            .layer(0)
            .expect("layer exists")
            .tiles
            .get(&coord)
            .expect("tile exists")
            .pixels[pixel_index];

        let _erase = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(64.0, 64.0)],
            brush_settings(),
            BrushToolMode::Erase,
        )
        .expect("eraser stroke should apply");

        let alpha_after = document
            .layer(0)
            .expect("layer exists")
            .tiles
            .get(&coord)
            .expect("tile exists")
            .pixels[pixel_index];
        assert!(alpha_after < alpha_before);
    }

    #[test]
    fn history_stack_undoes_and_redoes_a_stroke_as_one_action() {
        let mut document = Document::new(512, 512);
        let mut history = HistoryStack::default();

        let stroke = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(24.0, 24.0), (72.0, 24.0), (96.0, 48.0)],
            brush_settings(),
            BrushToolMode::Paint,
        )
        .expect("stroke should create a history record");

        let tile_count_after_stroke = document.layer(0).expect("layer exists").tiles.len();
        history.push(stroke);

        let undone = history.undo().expect("stroke should be undoable");
        undone.undo(&mut document);
        assert_eq!(document.layer(0).expect("layer exists").tiles.len(), 0);

        let redone = history.redo().expect("stroke should be redoable");
        redone.redo(&mut document);
        assert_eq!(document.layer(0).expect("layer exists").tiles.len(), tile_count_after_stroke);
    }

    #[test]
    fn move_tool_updates_layer_offset() {
        let mut document = Document::new(512, 512);

        let record = MoveTool::move_layer(&mut document, 0, 12, -4)
            .expect("move record should be produced for non-zero motion");

        assert_eq!(record.before_offset, (0, 0));
        assert_eq!(record.after_offset, (12, -4));
        assert_eq!(document.layer_offset(0), Some((12, -4)));
    }

    #[test]
    fn move_tool_can_be_undone_and_redone() {
        let mut document = Document::new(512, 512);
        let record = MoveTool::move_layer(&mut document, 0, 20, 30)
            .expect("move record should exist");

        record.undo(&mut document);
        assert_eq!(document.layer_offset(0), Some((0, 0)));

        record.redo(&mut document);
        assert_eq!(document.layer_offset(0), Some((20, 30)));
    }

    #[test]
    fn rectangular_marquee_sets_normalized_selection() {
        let mut document = Document::new(512, 512);
        let record = RectangularMarqueeTool::apply_selection(&mut document, 60, 80, 20, 30)
            .expect("selection record should exist");

        assert_eq!(record.after, Some(CanvasRect::new(20, 30, 40, 50)));
        assert!(!record.after_inverted);
        assert_eq!(document.selection(), Some(CanvasRect::new(20, 30, 40, 50)));
    }

    #[test]
    fn rectangular_marquee_can_be_undone() {
        let mut document = Document::new(512, 512);
        let record = RectangularMarqueeTool::apply_selection(&mut document, 10, 10, 30, 40)
            .expect("selection should be created");

        record.undo(&mut document);
        assert_eq!(document.selection(), None);
        assert!(!document.selection_inverted());

        record.redo(&mut document);
        assert_eq!(document.selection(), Some(CanvasRect::new(10, 10, 20, 30)));
        assert!(!document.selection_inverted());
    }

    #[test]
    fn brush_stroke_respects_rectangular_selection() {
        let mut document = Document::new(128, 128);
        document.set_selection(CanvasRect::new(60, 56, 16, 16));

        let _record = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(64.0, 64.0)],
            brush_settings(),
            BrushToolMode::Paint,
        )
        .expect("selected brush stroke should produce a record");

        let inside_alpha = pixel_alpha(&document, 0, 64, 64);
        let outside_alpha = pixel_alpha(&document, 0, 58, 64);
        assert!(inside_alpha > 0);
        assert_eq!(outside_alpha, 0);
    }

    #[test]
    fn eraser_stroke_respects_inverted_selection() {
        let mut document = Document::new(128, 128);
        let _paint = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(64.0, 64.0)],
            brush_settings(),
            BrushToolMode::Paint,
        )
        .expect("paint stroke should apply");
        document.set_selection(CanvasRect::new(60, 56, 16, 16));
        assert!(document.invert_selection());

        let _erase = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(64.0, 64.0)],
            brush_settings(),
            BrushToolMode::Erase,
        )
        .expect("inverted eraser stroke should apply");

        let center_alpha = pixel_alpha(&document, 0, 64, 64);
        assert_eq!(center_alpha, 255);
    }
}
