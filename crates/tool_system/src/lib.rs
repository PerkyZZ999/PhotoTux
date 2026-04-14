use common::{CanvasRect, LayerId};
use doc_model::{
    Document, FreeformSelection, LayerEditTarget, LayerStateSnapshot, MaskTile, RasterTile,
    SelectionPoint, SelectionShape, TileCoord,
};
use image_ops::{
    BrushDab, BrushTileContext, apply_round_brush_dab_clipped, apply_round_eraser_dab_clipped,
    apply_round_mask_hide_dab_clipped, apply_round_mask_reveal_dab_clipped,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Brush,
    Eraser,
    Move,
    RectangularMarquee,
    Lasso,
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
    pub flow: f32,
    pub color: [u8; 4],
    pub pressure_size_enabled: bool,
    pub pressure_opacity_enabled: bool,
}

impl BrushSettings {
    pub fn validated(self) -> Self {
        Self {
            radius: self.radius.clamp(1.0, 128.0),
            hardness: self.hardness.clamp(0.0, 1.0),
            opacity: self.opacity.clamp(0.0, 1.0),
            spacing: self.spacing.clamp(1.0, 64.0),
            flow: self.flow.clamp(0.05, 1.0),
            ..self
        }
    }

    pub fn to_dab(self) -> BrushDab {
        let settings = self.validated();
        BrushDab::new(
            settings.radius,
            settings.hardness,
            settings.opacity,
            settings.flow,
            settings.color,
        )
    }

    pub fn to_dab_with_pressure(self, pressure: f32) -> BrushDab {
        let settings = self.validated();
        let normalized_pressure = pressure.clamp(0.0, 1.0);
        let radius = if settings.pressure_size_enabled {
            settings.radius * (0.35 + 0.65 * normalized_pressure)
        } else {
            settings.radius
        };
        let opacity = if settings.pressure_opacity_enabled {
            settings.opacity * (0.2 + 0.8 * normalized_pressure)
        } else {
            settings.opacity
        };

        BrushDab::new(
            radius,
            settings.hardness,
            opacity,
            settings.flow,
            settings.color,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSample {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

impl BrushSample {
    pub fn new(x: f32, y: f32, pressure: f32) -> Self {
        Self {
            x,
            y,
            pressure: pressure.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrushChange {
    Pixels {
        layer_id: LayerId,
        coord: TileCoord,
        before: Option<RasterTile>,
        after: Option<RasterTile>,
    },
    Mask {
        layer_id: LayerId,
        coord: TileCoord,
        before: Option<MaskTile>,
        after: Option<MaskTile>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrushStrokeRecord {
    pub layer_id: LayerId,
    pub target: LayerEditTarget,
    pub dab_count: usize,
    pub changes: Vec<BrushChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerTransformRecord {
    pub layer_id: LayerId,
    pub before: LayerStateSnapshot,
    pub after: LayerStateSnapshot,
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
            let _ =
                document.set_layer_offset(layer_index, self.before_offset.0, self.before_offset.1);
        }
    }

    pub fn redo(&self, document: &mut Document) {
        if let Some(layer_index) = document.layer_index_by_id(self.layer_id) {
            let _ =
                document.set_layer_offset(layer_index, self.after_offset.0, self.after_offset.1);
        }
    }
}

impl BrushStrokeRecord {
    pub fn undo(&self, document: &mut Document) {
        for change in &self.changes {
            match change {
                BrushChange::Pixels {
                    layer_id,
                    coord,
                    before,
                    ..
                } => {
                    let _ = document.apply_tile_snapshot(*layer_id, *coord, before.clone());
                }
                BrushChange::Mask {
                    layer_id,
                    coord,
                    before,
                    ..
                } => {
                    let _ = document.apply_mask_tile_snapshot(*layer_id, *coord, before.clone());
                }
            }
        }
    }

    pub fn redo(&self, document: &mut Document) {
        for change in &self.changes {
            match change {
                BrushChange::Pixels {
                    layer_id,
                    coord,
                    after,
                    ..
                } => {
                    let _ = document.apply_tile_snapshot(*layer_id, *coord, after.clone());
                }
                BrushChange::Mask {
                    layer_id,
                    coord,
                    after,
                    ..
                } => {
                    let _ = document.apply_mask_tile_snapshot(*layer_id, *coord, after.clone());
                }
            }
        }
    }
}

impl BrushChange {
    pub fn layer_id(&self) -> LayerId {
        match self {
            Self::Pixels { layer_id, .. } | Self::Mask { layer_id, .. } => *layer_id,
        }
    }

    pub fn coord(&self) -> TileCoord {
        match self {
            Self::Pixels { coord, .. } | Self::Mask { coord, .. } => *coord,
        }
    }
}

impl LayerTransformRecord {
    pub fn undo(&self, document: &mut Document) {
        let _ = document.apply_layer_state_snapshot(self.layer_id, self.before.clone());
    }

    pub fn redo(&self, document: &mut Document) {
        let _ = document.apply_layer_state_snapshot(self.layer_id, self.after.clone());
    }
}

pub struct BrushTool;
pub struct MoveTool;
pub struct RectangularMarqueeTool;
pub struct LassoTool;
pub struct SimpleTransformTool;

#[derive(Debug, Clone, Copy, PartialEq)]
struct TransformDelta {
    scale_x: f32,
    scale_y: f32,
    rotate_quadrants: i32,
    translate_x: i32,
    translate_y: i32,
}

impl BrushTool {
    pub fn apply_stroke(
        document: &mut Document,
        layer_index: usize,
        points: &[(f32, f32)],
        settings: BrushSettings,
        mode: BrushToolMode,
        target: LayerEditTarget,
    ) -> Option<BrushStrokeRecord> {
        let samples = points
            .iter()
            .map(|(x, y)| BrushSample::new(*x, *y, 1.0))
            .collect::<Vec<_>>();
        Self::apply_stroke_with_samples(document, layer_index, &samples, settings, mode, target)
    }

    pub fn apply_stroke_with_samples(
        document: &mut Document,
        layer_index: usize,
        samples: &[BrushSample],
        settings: BrushSettings,
        mode: BrushToolMode,
        target: LayerEditTarget,
    ) -> Option<BrushStrokeRecord> {
        if samples.is_empty() || settings.radius <= 0.0 {
            return None;
        }

        let layer_id = document.layer(layer_index)?.id;
        let (layer_offset_x, layer_offset_y) = document.layer_offset(layer_index)?;
        let tile_size = document.tile_size;
        let settings = settings.validated();
        let dab_positions = interpolate_brush_samples(samples, effective_spacing(settings));
        let selection_shape = document.selection_shape().cloned();
        let clip_rect = selection_shape.as_ref().map(SelectionShape::bounds);
        let clip_inverted = document.selection_inverted();
        let mut changes = Vec::<BrushChange>::new();

        for sample in &dab_positions {
            let dab_x = sample.x;
            let dab_y = sample.y;
            let dab = settings.to_dab_with_pressure(sample.pressure);
            let local_dab_x = dab_x - layer_offset_x as f32;
            let local_dab_y = dab_y - layer_offset_y as f32;
            let touched_coords =
                document.tile_coords_in_radius(local_dab_x, local_dab_y, dab.radius);

            for coord in touched_coords {
                let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
                let canvas_tile_origin_x = tile_origin_x as i32 + layer_offset_x;
                let canvas_tile_origin_y = tile_origin_y as i32 + layer_offset_y;
                match target {
                    LayerEditTarget::LayerPixels => {
                        if changes.iter().all(|change| {
                            !(change.layer_id() == layer_id && change.coord() == coord)
                        }) {
                            changes.push(BrushChange::Pixels {
                                layer_id,
                                coord,
                                before: document.tile_snapshot(layer_index, coord),
                                after: None,
                            });
                        }

                        let changed = {
                            let tile = document
                                .layer_mut(layer_index)?
                                .ensure_tile(coord, tile_size);
                            let context = BrushTileContext::new(
                                tile_size,
                                canvas_tile_origin_x,
                                canvas_tile_origin_y,
                                dab_x,
                                dab_y,
                            )
                            .with_clip(clip_rect, clip_inverted);
                            match mode {
                                BrushToolMode::Paint => {
                                    apply_round_brush_dab_clipped(&mut tile.pixels, context, dab)
                                }
                                BrushToolMode::Erase => {
                                    apply_round_eraser_dab_clipped(&mut tile.pixels, context, dab)
                                }
                            }
                        };

                        if let Some(selection_shape) = &selection_shape
                            && matches!(selection_shape, SelectionShape::Freeform(_))
                        {
                            let before_tile = changes.iter().find_map(|change| match change {
                                BrushChange::Pixels {
                                    layer_id: changed_layer_id,
                                    coord: changed_coord,
                                    before,
                                    ..
                                } if *changed_layer_id == layer_id && *changed_coord == coord => {
                                    before.clone()
                                }
                                _ => None,
                            });
                            if let Some(tile) =
                                document.layer_mut(layer_index)?.tiles.get_mut(&coord)
                            {
                                restore_pixels_outside_selection(
                                    &mut tile.pixels,
                                    before_tile.as_ref(),
                                    tile_size,
                                    canvas_tile_origin_x,
                                    canvas_tile_origin_y,
                                    selection_shape,
                                    clip_inverted,
                                );
                            }
                        }

                        if !changed
                            && document
                                .layer(layer_index)?
                                .tiles
                                .get(&coord)
                                .is_some_and(|tile| tile.pixels.iter().all(|value| *value == 0))
                        {
                            document.layer_mut(layer_index)?.tiles.remove(&coord);
                        }
                    }
                    LayerEditTarget::LayerMask => {
                        if document.layer_mask(layer_index).is_none() {
                            continue;
                        }

                        if changes.iter().all(|change| {
                            !(change.layer_id() == layer_id && change.coord() == coord)
                        }) {
                            changes.push(BrushChange::Mask {
                                layer_id,
                                coord,
                                before: document.mask_tile_snapshot(layer_index, coord),
                                after: None,
                            });
                        }

                        let changed = {
                            let tile = document.ensure_mask_tile_for_pixel(
                                layer_index,
                                coord.x * tile_size,
                                coord.y * tile_size,
                            )?;
                            let context = BrushTileContext::new(
                                tile_size,
                                canvas_tile_origin_x,
                                canvas_tile_origin_y,
                                dab_x,
                                dab_y,
                            )
                            .with_clip(clip_rect, clip_inverted);
                            match mode {
                                BrushToolMode::Paint => {
                                    apply_round_mask_hide_dab_clipped(&mut tile.alpha, context, dab)
                                }
                                BrushToolMode::Erase => apply_round_mask_reveal_dab_clipped(
                                    &mut tile.alpha,
                                    context,
                                    dab,
                                ),
                            }
                        };

                        if let Some(selection_shape) = &selection_shape
                            && matches!(selection_shape, SelectionShape::Freeform(_))
                        {
                            let before_tile = changes.iter().find_map(|change| match change {
                                BrushChange::Mask {
                                    layer_id: changed_layer_id,
                                    coord: changed_coord,
                                    before,
                                    ..
                                } if *changed_layer_id == layer_id && *changed_coord == coord => {
                                    before.clone()
                                }
                                _ => None,
                            });
                            if let Some(mask) = document.layer_mask_mut(layer_index)
                                && let Some(tile) = mask.tiles.get_mut(&coord)
                            {
                                restore_mask_outside_selection(
                                    &mut tile.alpha,
                                    before_tile.as_ref(),
                                    tile_size,
                                    canvas_tile_origin_x,
                                    canvas_tile_origin_y,
                                    selection_shape,
                                    clip_inverted,
                                );
                            }
                        }

                        if document
                            .mask_tile_snapshot(layer_index, coord)
                            .is_some_and(|tile| tile.alpha.iter().all(|value| *value == 255))
                        {
                            let _ = document.apply_mask_tile_snapshot(layer_id, coord, None);
                        }

                        if !changed {
                            continue;
                        }
                    }
                }
            }
        }

        changes.retain(|change| {
            let after =
                document
                    .layer_index_by_id(change.layer_id())
                    .and_then(|idx| match change {
                        BrushChange::Pixels { coord, .. } => document
                            .tile_snapshot(idx, *coord)
                            .map(BrushChangeSnapshot::Pixels),
                        BrushChange::Mask { coord, .. } => document
                            .mask_tile_snapshot(idx, *coord)
                            .map(BrushChangeSnapshot::Mask),
                    });

            match (change, after) {
                (BrushChange::Pixels { before, .. }, Some(BrushChangeSnapshot::Pixels(after))) => {
                    *before != Some(after)
                }
                (BrushChange::Pixels { before, .. }, None) => before.is_some(),
                (BrushChange::Mask { before, .. }, Some(BrushChangeSnapshot::Mask(after))) => {
                    *before != Some(after)
                }
                (BrushChange::Mask { before, .. }, None) => before.is_some(),
                _ => false,
            }
        });

        if changes.is_empty() {
            return None;
        }

        for change in &mut changes {
            let Some(layer_index) = document.layer_index_by_id(change.layer_id()) else {
                continue;
            };
            match change {
                BrushChange::Pixels { coord, after, .. } => {
                    *after = document.tile_snapshot(layer_index, *coord);
                }
                BrushChange::Mask { coord, after, .. } => {
                    *after = document.mask_tile_snapshot(layer_index, *coord);
                }
            }
        }

        Some(BrushStrokeRecord {
            layer_id,
            target,
            dab_count: dab_positions.len(),
            changes,
        })
    }
}

enum BrushChangeSnapshot {
    Pixels(RasterTile),
    Mask(MaskTile),
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

        if document.selection_shape().is_some() {
            let before = document.layer_state_snapshot(layer_index)?;
            let layer_id = document.layer(layer_index)?.id;
            let _record = SimpleTransformTool::transform_layer(
                document,
                layer_index,
                1.0,
                1.0,
                0,
                delta_x,
                delta_y,
            )?;
            let after = document.layer_state_snapshot(layer_index)?;
            if before == after {
                return None;
            }
            return Some(MoveLayerRecord {
                layer_id,
                before_offset: (before.offset_x, before.offset_y),
                after_offset: (after.offset_x, after.offset_y),
            });
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
pub struct SelectionRecord {
    pub before: Option<SelectionShape>,
    pub before_inverted: bool,
    pub after: Option<SelectionShape>,
    pub after_inverted: bool,
}

impl SelectionRecord {
    pub fn undo(&self, document: &mut Document) {
        document.set_selection_shape_state(self.before.clone(), self.before_inverted);
    }

    pub fn redo(&self, document: &mut Document) {
        document.set_selection_shape_state(self.after.clone(), self.after_inverted);
    }
}

fn apply_selection_shape(
    document: &mut Document,
    after: Option<SelectionShape>,
) -> Option<SelectionRecord> {
    let before = document.selection_shape().cloned();
    let before_inverted = document.selection_inverted();

    if before == after && !before_inverted {
        return None;
    }

    match after.clone() {
        Some(selection) => document.set_selection_shape_state(Some(selection), false),
        None => document.clear_selection(),
    }

    Some(SelectionRecord {
        before,
        before_inverted,
        after,
        after_inverted: false,
    })
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
    ) -> Option<SelectionRecord> {
        let after =
            Self::preview_rect(start_x, start_y, end_x, end_y).map(SelectionShape::Rectangular);
        apply_selection_shape(document, after)
    }
}

impl LassoTool {
    pub fn preview_selection(points: &[(i32, i32)]) -> Option<FreeformSelection> {
        let mut deduped = Vec::<SelectionPoint>::new();
        for &(x, y) in points {
            if deduped.last().copied() != Some(SelectionPoint::new(x, y)) {
                deduped.push(SelectionPoint::new(x, y));
            }
        }

        FreeformSelection::new(deduped)
    }

    pub fn apply_selection(
        document: &mut Document,
        points: &[(i32, i32)],
    ) -> Option<SelectionRecord> {
        let after = Self::preview_selection(points).map(SelectionShape::Freeform);
        apply_selection_shape(document, after)
    }
}

fn for_each_tile_pixel_outside_selection(
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    selection_shape: &SelectionShape,
    clip_inverted: bool,
    mut visit: impl FnMut(usize, usize),
) {
    let tile_size = tile_size as usize;

    for local_y in 0..tile_size {
        for local_x in 0..tile_size {
            let canvas_x = tile_origin_x + local_x as i32;
            let canvas_y = tile_origin_y + local_y as i32;
            let inside = selection_shape.contains_pixel(canvas_x, canvas_y) != clip_inverted;
            if inside {
                continue;
            }

            visit(local_x, local_y);
        }
    }
}

fn restore_pixels_outside_selection(
    tile_pixels: &mut [u8],
    before: Option<&RasterTile>,
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    selection_shape: &SelectionShape,
    clip_inverted: bool,
) {
    let tile_size_usize = tile_size as usize;

    for_each_tile_pixel_outside_selection(
        tile_size,
        tile_origin_x,
        tile_origin_y,
        selection_shape,
        clip_inverted,
        |local_x, local_y| {
            let index = (local_y * tile_size_usize + local_x) * 4;
            if let Some(before) = before {
                tile_pixels[index..index + 4].copy_from_slice(&before.pixels[index..index + 4]);
            } else {
                tile_pixels[index..index + 4].fill(0);
            }
        },
    );
}

fn restore_mask_outside_selection(
    tile_alpha: &mut [u8],
    before: Option<&MaskTile>,
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    selection_shape: &SelectionShape,
    clip_inverted: bool,
) {
    let tile_size_usize = tile_size as usize;

    for_each_tile_pixel_outside_selection(
        tile_size,
        tile_origin_x,
        tile_origin_y,
        selection_shape,
        clip_inverted,
        |local_x, local_y| {
            let index = local_y * tile_size_usize + local_x;
            tile_alpha[index] = before.map(|tile| tile.alpha[index]).unwrap_or(255);
        },
    );
}

impl SimpleTransformTool {
    pub fn preview_bounds(
        document: &Document,
        layer_index: usize,
        scale_x: f32,
        scale_y: f32,
        rotate_quadrants: i32,
        translate_x: i32,
        translate_y: i32,
    ) -> Option<CanvasRect> {
        if scale_x <= 0.0 || scale_y <= 0.0 {
            return None;
        }

        let bounds = transform_target_bounds(document, layer_index)?;
        let scaled_width = ((bounds.width as f32) * scale_x).round().max(1.0) as u32;
        let scaled_height = ((bounds.height as f32) * scale_y).round().max(1.0) as u32;
        let rotate_quadrants = rotate_quadrants.rem_euclid(4);
        let (preview_width, preview_height) = if rotate_quadrants % 2 == 0 {
            (scaled_width, scaled_height)
        } else {
            (scaled_height, scaled_width)
        };

        Some(CanvasRect::new(
            bounds.x + translate_x,
            bounds.y + translate_y,
            preview_width,
            preview_height,
        ))
    }

    pub fn transform_layer(
        document: &mut Document,
        layer_index: usize,
        scale_x: f32,
        scale_y: f32,
        rotate_quadrants: i32,
        translate_x: i32,
        translate_y: i32,
    ) -> Option<LayerTransformRecord> {
        if scale_x <= 0.0 || scale_y <= 0.0 {
            return None;
        }

        let before = document.layer_state_snapshot(layer_index)?;
        if before.tiles.is_empty() {
            return None;
        }

        let layer_id = document.layer(layer_index)?.id;
        let source_bounds = document.layer_canvas_bounds(layer_index)?;
        let rotate_quadrants = rotate_quadrants.rem_euclid(4);
        let transform = TransformDelta {
            scale_x,
            scale_y,
            rotate_quadrants,
            translate_x,
            translate_y,
        };

        if document.selection_shape().is_some() {
            let after =
                transform_selected_layer_state(document, &before, source_bounds, transform)?;
            if before == after {
                return None;
            }

            let _ = document.apply_layer_state_snapshot(layer_id, after.clone());
            return Some(LayerTransformRecord {
                layer_id,
                before,
                after,
            });
        }

        let (source_width, source_height, source_pixels) =
            rasterize_layer_snapshot(&before, document.tile_size, source_bounds)?;
        let scaled_width = ((source_width as f32) * scale_x).round().max(1.0) as u32;
        let scaled_height = ((source_height as f32) * scale_y).round().max(1.0) as u32;
        let scaled_pixels = resample_nearest_rgba(
            &source_pixels,
            source_width,
            source_height,
            scaled_width,
            scaled_height,
        );
        let (target_width, target_height, transformed_pixels) = rotate_rgba_quadrants(
            &scaled_pixels,
            scaled_width,
            scaled_height,
            rotate_quadrants,
        );
        let after = layer_state_from_pixels(
            document.tile_size,
            source_bounds.x + translate_x,
            source_bounds.y + translate_y,
            target_width,
            target_height,
            &transformed_pixels,
        );

        if before == after {
            return None;
        }

        let _ = document.apply_layer_state_snapshot(layer_id, after.clone());
        Some(LayerTransformRecord {
            layer_id,
            before,
            after,
        })
    }
}

fn transform_target_bounds(document: &Document, layer_index: usize) -> Option<CanvasRect> {
    let source_bounds = document.layer_canvas_bounds(layer_index)?;
    let Some(selection_shape) = document.selection_shape() else {
        return Some(source_bounds);
    };

    let selection_bounds = selection_shape.bounds();
    let left = source_bounds.x.max(selection_bounds.x);
    let top = source_bounds.y.max(selection_bounds.y);
    let right = source_bounds.right().min(selection_bounds.right());
    let bottom = source_bounds.bottom().min(selection_bounds.bottom());

    if left >= right || top >= bottom {
        return Some(source_bounds);
    }

    Some(CanvasRect::new(
        left,
        top,
        (right - left) as u32,
        (bottom - top) as u32,
    ))
}

fn transform_selected_layer_state(
    document: &Document,
    before: &LayerStateSnapshot,
    source_bounds: CanvasRect,
    transform: TransformDelta,
) -> Option<LayerStateSnapshot> {
    let (_source_width, _source_height, source_pixels) =
        rasterize_layer_snapshot(before, document.tile_size, source_bounds)?;
    let selected_bounds = selected_pixel_bounds(document, source_bounds, &source_pixels)?;
    let selected_pixels =
        extract_selected_pixels(document, source_bounds, selected_bounds, &source_pixels);
    let base_pixels = clear_selected_pixels(document, source_bounds, &source_pixels);

    let scaled_width = ((selected_bounds.width as f32) * transform.scale_x)
        .round()
        .max(1.0) as u32;
    let scaled_height = ((selected_bounds.height as f32) * transform.scale_y)
        .round()
        .max(1.0) as u32;
    let scaled_pixels = resample_nearest_rgba(
        &selected_pixels,
        selected_bounds.width,
        selected_bounds.height,
        scaled_width,
        scaled_height,
    );
    let (target_width, target_height, transformed_pixels) = rotate_rgba_quadrants(
        &scaled_pixels,
        scaled_width,
        scaled_height,
        transform.rotate_quadrants,
    );

    let destination_bounds = CanvasRect::new(
        selected_bounds.x + transform.translate_x,
        selected_bounds.y + transform.translate_y,
        target_width,
        target_height,
    );
    let union_bounds = union_rect(source_bounds, destination_bounds);
    let mut output = vec![0_u8; (union_bounds.width * union_bounds.height * 4) as usize];

    blit_rgba(&mut output, union_bounds, source_bounds, &base_pixels);
    composite_rgba(
        &mut output,
        union_bounds,
        destination_bounds,
        &transformed_pixels,
    );

    Some(layer_state_from_pixels(
        document.tile_size,
        union_bounds.x,
        union_bounds.y,
        union_bounds.width,
        union_bounds.height,
        &output,
    ))
}

fn selected_pixel_bounds(
    document: &Document,
    source_bounds: CanvasRect,
    source_pixels: &[u8],
) -> Option<CanvasRect> {
    let width = source_bounds.width as usize;
    let height = source_bounds.height as usize;
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for local_y in 0..height {
        for local_x in 0..width {
            let index = (local_y * width + local_x) * 4 + 3;
            if source_pixels[index] == 0 {
                continue;
            }
            let canvas_x = source_bounds.x + local_x as i32;
            let canvas_y = source_bounds.y + local_y as i32;
            if !document.allows_pixel_edit(canvas_x, canvas_y) {
                continue;
            }
            min_x = min_x.min(canvas_x);
            min_y = min_y.min(canvas_y);
            max_x = max_x.max(canvas_x);
            max_y = max_y.max(canvas_y);
        }
    }

    if min_x == i32::MAX {
        return None;
    }

    Some(CanvasRect::new(
        min_x,
        min_y,
        (max_x - min_x + 1) as u32,
        (max_y - min_y + 1) as u32,
    ))
}

fn extract_selected_pixels(
    document: &Document,
    source_bounds: CanvasRect,
    selected_bounds: CanvasRect,
    source_pixels: &[u8],
) -> Vec<u8> {
    let mut selected = vec![0_u8; (selected_bounds.width * selected_bounds.height * 4) as usize];
    let source_width = source_bounds.width as usize;
    let selected_width = selected_bounds.width as usize;

    for local_y in 0..selected_bounds.height as usize {
        for local_x in 0..selected_bounds.width as usize {
            let canvas_x = selected_bounds.x + local_x as i32;
            let canvas_y = selected_bounds.y + local_y as i32;
            if !document.allows_pixel_edit(canvas_x, canvas_y) {
                continue;
            }

            let source_x = (canvas_x - source_bounds.x) as usize;
            let source_y = (canvas_y - source_bounds.y) as usize;
            let source_index = (source_y * source_width + source_x) * 4;
            let target_index = (local_y * selected_width + local_x) * 4;
            selected[target_index..target_index + 4]
                .copy_from_slice(&source_pixels[source_index..source_index + 4]);
        }
    }

    selected
}

fn clear_selected_pixels(
    document: &Document,
    source_bounds: CanvasRect,
    source_pixels: &[u8],
) -> Vec<u8> {
    let mut base = source_pixels.to_vec();
    let width = source_bounds.width as usize;
    let height = source_bounds.height as usize;

    for local_y in 0..height {
        for local_x in 0..width {
            let canvas_x = source_bounds.x + local_x as i32;
            let canvas_y = source_bounds.y + local_y as i32;
            if !document.allows_pixel_edit(canvas_x, canvas_y) {
                continue;
            }
            let index = (local_y * width + local_x) * 4;
            base[index..index + 4].fill(0);
        }
    }

    base
}

fn union_rect(left: CanvasRect, right: CanvasRect) -> CanvasRect {
    let min_x = left.x.min(right.x);
    let min_y = left.y.min(right.y);
    let max_x = left.right().max(right.right());
    let max_y = left.bottom().max(right.bottom());
    CanvasRect::new(min_x, min_y, (max_x - min_x) as u32, (max_y - min_y) as u32)
}

fn blit_rgba(
    destination: &mut [u8],
    union_bounds: CanvasRect,
    destination_bounds: CanvasRect,
    source: &[u8],
) {
    let destination_width = union_bounds.width;
    let destination_height = union_bounds.height;
    let source_width = destination_bounds.width;
    let source_height = destination_bounds.height;
    for y in 0..source_height as usize {
        for x in 0..source_width as usize {
            let canvas_x = destination_bounds.x + x as i32;
            let canvas_y = destination_bounds.y + y as i32;
            let target_x = canvas_x - union_bounds.x;
            let target_y = canvas_y - union_bounds.y;
            if target_x < 0
                || target_y < 0
                || target_x >= destination_width as i32
                || target_y >= destination_height as i32
            {
                continue;
            }
            let source_index = (y * source_width as usize + x) * 4;
            let destination_index =
                ((target_y as usize * destination_width as usize) + target_x as usize) * 4;
            destination[destination_index..destination_index + 4]
                .copy_from_slice(&source[source_index..source_index + 4]);
        }
    }
}

fn composite_rgba(
    destination: &mut [u8],
    union_bounds: CanvasRect,
    destination_bounds: CanvasRect,
    source: &[u8],
) {
    let destination_width = union_bounds.width;
    let destination_height = union_bounds.height;
    let source_width = destination_bounds.width;
    let source_height = destination_bounds.height;
    for y in 0..source_height as usize {
        for x in 0..source_width as usize {
            let canvas_x = destination_bounds.x + x as i32;
            let canvas_y = destination_bounds.y + y as i32;
            let target_x = canvas_x - union_bounds.x;
            let target_y = canvas_y - union_bounds.y;
            if target_x < 0
                || target_y < 0
                || target_x >= destination_width as i32
                || target_y >= destination_height as i32
            {
                continue;
            }
            let source_index = (y * source_width as usize + x) * 4;
            let alpha = source[source_index + 3] as f32 / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let destination_index =
                ((target_y as usize * destination_width as usize) + target_x as usize) * 4;
            composite_pixel_over(
                &mut destination[destination_index..destination_index + 4],
                [
                    source[source_index],
                    source[source_index + 1],
                    source[source_index + 2],
                    source[source_index + 3],
                ],
                alpha,
            );
        }
    }
}

fn composite_pixel_over(destination: &mut [u8], source: [u8; 4], alpha: f32) {
    let source_alpha = alpha.clamp(0.0, 1.0);
    let destination_alpha = destination[3] as f32 / 255.0;
    let out_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    for channel in 0..3 {
        let src = source[channel] as f32 / 255.0;
        let dst = destination[channel] as f32 / 255.0;
        let out = src * source_alpha + dst * (1.0 - source_alpha);
        destination[channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    destination[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn rasterize_layer_snapshot(
    snapshot: &LayerStateSnapshot,
    tile_size: u32,
    bounds: CanvasRect,
) -> Option<(u32, u32, Vec<u8>)> {
    let width = bounds.width;
    let height = bounds.height;
    if width == 0 || height == 0 {
        return None;
    }

    let mut pixels = vec![0_u8; (width * height * 4) as usize];
    for (coord, tile) in &snapshot.tiles {
        let tile_origin_x = coord.x as i32 * tile_size as i32 + snapshot.offset_x;
        let tile_origin_y = coord.y as i32 * tile_size as i32 + snapshot.offset_y;

        for local_y in 0..tile_size as usize {
            for local_x in 0..tile_size as usize {
                let canvas_x = tile_origin_x + local_x as i32;
                let canvas_y = tile_origin_y + local_y as i32;
                if canvas_x < bounds.x
                    || canvas_y < bounds.y
                    || canvas_x >= bounds.x + width as i32
                    || canvas_y >= bounds.y + height as i32
                {
                    continue;
                }

                let source_index = (local_y * tile_size as usize + local_x) * 4;
                let target_x = (canvas_x - bounds.x) as usize;
                let target_y = (canvas_y - bounds.y) as usize;
                let target_index = (target_y * width as usize + target_x) * 4;
                pixels[target_index..target_index + 4]
                    .copy_from_slice(&tile.pixels[source_index..source_index + 4]);
            }
        }
    }

    Some((width, height, pixels))
}

fn resample_nearest_rgba(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> Vec<u8> {
    let mut output = vec![0_u8; (target_width * target_height * 4) as usize];

    for target_y in 0..target_height {
        for target_x in 0..target_width {
            let source_x = ((target_x as f32 + 0.5) * source_width as f32 / target_width as f32)
                .floor()
                .clamp(0.0, source_width.saturating_sub(1) as f32)
                as u32;
            let source_y = ((target_y as f32 + 0.5) * source_height as f32 / target_height as f32)
                .floor()
                .clamp(0.0, source_height.saturating_sub(1) as f32)
                as u32;
            let source_index = ((source_y * source_width + source_x) * 4) as usize;
            let target_index = ((target_y * target_width + target_x) * 4) as usize;
            output[target_index..target_index + 4]
                .copy_from_slice(&source[source_index..source_index + 4]);
        }
    }

    output
}

fn rotate_rgba_quadrants(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    rotate_quadrants: i32,
) -> (u32, u32, Vec<u8>) {
    let rotate_quadrants = rotate_quadrants.rem_euclid(4);
    if rotate_quadrants == 0 {
        return (source_width, source_height, source.to_vec());
    }

    let (target_width, target_height) = if rotate_quadrants % 2 == 0 {
        (source_width, source_height)
    } else {
        (source_height, source_width)
    };
    let mut output = vec![0_u8; (target_width * target_height * 4) as usize];

    for source_y in 0..source_height {
        for source_x in 0..source_width {
            let (target_x, target_y) = match rotate_quadrants {
                1 => (source_height - 1 - source_y, source_x),
                2 => (source_width - 1 - source_x, source_height - 1 - source_y),
                3 => (source_y, source_width - 1 - source_x),
                _ => (source_x, source_y),
            };
            let source_index = ((source_y * source_width + source_x) * 4) as usize;
            let target_index = ((target_y * target_width + target_x) * 4) as usize;
            output[target_index..target_index + 4]
                .copy_from_slice(&source[source_index..source_index + 4]);
        }
    }

    (target_width, target_height, output)
}

fn layer_state_from_pixels(
    tile_size: u32,
    offset_x: i32,
    offset_y: i32,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> LayerStateSnapshot {
    let mut tiles = HashMap::new();
    let tile_columns = width.div_ceil(tile_size);
    let tile_rows = height.div_ceil(tile_size);

    for tile_y in 0..tile_rows {
        for tile_x in 0..tile_columns {
            let mut tile = RasterTile::new(tile_size);
            let mut has_content = false;
            for local_y in 0..tile_size as usize {
                let pixel_y = tile_y * tile_size + local_y as u32;
                if pixel_y >= height {
                    break;
                }
                for local_x in 0..tile_size as usize {
                    let pixel_x = tile_x * tile_size + local_x as u32;
                    if pixel_x >= width {
                        break;
                    }
                    let source_index = ((pixel_y * width + pixel_x) * 4) as usize;
                    let target_index = (local_y * tile_size as usize + local_x) * 4;
                    tile.pixels[target_index..target_index + 4]
                        .copy_from_slice(&pixels[source_index..source_index + 4]);
                    if pixels[source_index + 3] != 0 {
                        has_content = true;
                    }
                }
            }

            if has_content {
                tiles.insert(TileCoord::new(tile_x, tile_y), tile);
            }
        }
    }

    LayerStateSnapshot {
        offset_x,
        offset_y,
        tiles,
        mask: None,
    }
}

fn interpolate_brush_samples(samples: &[BrushSample], spacing: f32) -> Vec<BrushSample> {
    let mut positions = vec![samples[0]];

    for window in samples.windows(2) {
        let start = window[0];
        let end = window[1];
        let delta_x = end.x - start.x;
        let delta_y = end.y - start.y;
        let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();

        if distance == 0.0 {
            continue;
        }

        let steps = (distance / spacing).ceil() as usize;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            positions.push(BrushSample::new(
                start.x + delta_x * t,
                start.y + delta_y * t,
                start.pressure + (end.pressure - start.pressure) * t,
            ));
        }
    }

    positions
}

fn effective_spacing(settings: BrushSettings) -> f32 {
    let max_spacing = (settings.radius * 1.5).max(1.0);
    settings.spacing.clamp(1.0, max_spacing)
}

#[cfg(test)]
mod tests {
    use super::{
        BrushSample, BrushSettings, BrushTool, BrushToolMode, LassoTool, MoveTool,
        RectangularMarqueeTool, SimpleTransformTool,
    };
    use common::CanvasRect;
    use doc_model::{
        Document, FreeformSelection, LayerEditTarget, SelectionPoint, SelectionShape, TileCoord,
    };
    use history_engine::HistoryStack;

    fn brush_settings() -> BrushSettings {
        BrushSettings {
            radius: 6.0,
            hardness: 0.8,
            opacity: 1.0,
            spacing: 4.0,
            flow: 1.0,
            color: [255, 0, 0, 255],
            pressure_size_enabled: false,
            pressure_opacity_enabled: false,
        }
    }

    #[test]
    fn pressure_samples_interpolate_along_brush_segments() {
        let samples = super::interpolate_brush_samples(
            &[
                BrushSample::new(0.0, 0.0, 0.2),
                BrushSample::new(8.0, 0.0, 0.8),
            ],
            4.0,
        );

        assert_eq!(samples.first().map(|sample| sample.pressure), Some(0.2));
        assert_eq!(samples.last().map(|sample| sample.pressure), Some(0.8));
        assert!(samples.len() >= 3);
    }

    #[test]
    fn pressure_mapping_can_reduce_radius_and_opacity() {
        let mut settings = brush_settings();
        settings.pressure_size_enabled = true;
        settings.pressure_opacity_enabled = true;

        let light = settings.to_dab_with_pressure(0.25);
        let heavy = settings.to_dab_with_pressure(1.0);

        assert!(light.radius < heavy.radius);
        assert!(light.opacity < heavy.opacity);
    }

    #[test]
    fn brush_settings_validation_clamps_dynamic_parameters() {
        let validated = BrushSettings {
            radius: 500.0,
            hardness: -1.0,
            opacity: 1.5,
            spacing: 0.0,
            flow: 0.0,
            ..brush_settings()
        }
        .validated();

        assert_eq!(validated.radius, 128.0);
        assert_eq!(validated.hardness, 0.0);
        assert_eq!(validated.opacity, 1.0);
        assert_eq!(validated.spacing, 1.0);
        assert_eq!(validated.flow, 0.05);
    }

    #[test]
    fn low_flow_reduces_dab_opacity() {
        let light = BrushSettings {
            flow: 0.25,
            ..brush_settings()
        }
        .to_dab();
        let heavy = BrushSettings {
            flow: 1.0,
            ..brush_settings()
        }
        .to_dab();

        assert!(light.flow < heavy.flow);
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
            LayerEditTarget::LayerPixels,
        )
        .expect("stroke should produce a history record");

        assert!(record.dab_count >= 2);
        assert!(!record.changes.is_empty());
        assert!(!document.layer(0).expect("layer exists").tiles.is_empty());
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
            LayerEditTarget::LayerPixels,
        )
        .expect("stroke should modify the layer");

        let painted_tile_count = document.layer(0).expect("layer exists").tiles.len();
        record.undo(&mut document);
        assert_eq!(document.layer(0).expect("layer exists").tiles.len(), 0);

        record.redo(&mut document);
        assert_eq!(
            document.layer(0).expect("layer exists").tiles.len(),
            painted_tile_count
        );
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
            LayerEditTarget::LayerPixels,
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
            LayerEditTarget::LayerPixels,
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
            LayerEditTarget::LayerPixels,
        )
        .expect("stroke should create a history record");

        let tile_count_after_stroke = document.layer(0).expect("layer exists").tiles.len();
        history.push(stroke);

        let undone = history.undo().expect("stroke should be undoable");
        undone.undo(&mut document);
        assert_eq!(document.layer(0).expect("layer exists").tiles.len(), 0);

        let redone = history.redo().expect("stroke should be redoable");
        redone.redo(&mut document);
        assert_eq!(
            document.layer(0).expect("layer exists").tiles.len(),
            tile_count_after_stroke
        );
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
        let record =
            MoveTool::move_layer(&mut document, 0, 20, 30).expect("move record should exist");

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

        assert_eq!(
            record.after,
            Some(SelectionShape::Rectangular(CanvasRect::new(20, 30, 40, 50)))
        );
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
            LayerEditTarget::LayerPixels,
        )
        .expect("selected brush stroke should produce a record");

        let inside_alpha = pixel_alpha(&document, 0, 64, 64);
        let outside_alpha = pixel_alpha(&document, 0, 58, 64);
        assert!(inside_alpha > 0);
        assert_eq!(outside_alpha, 0);
    }

    #[test]
    fn lasso_selection_creates_freeform_shape() {
        let mut document = Document::new(512, 512);
        let record = LassoTool::apply_selection(&mut document, &[(10, 10), (40, 10), (25, 40)])
            .expect("lasso selection should produce a record");

        assert!(matches!(record.after, Some(SelectionShape::Freeform(_))));
        assert!(document.selection_contains_pixel(25, 20));
        assert!(!document.selection_contains_pixel(5, 5));
    }

    #[test]
    fn brush_stroke_respects_freeform_selection() {
        let mut document = Document::new(128, 128);
        let selection = FreeformSelection::new(vec![
            SelectionPoint::new(60, 56),
            SelectionPoint::new(76, 56),
            SelectionPoint::new(68, 72),
        ])
        .expect("triangle selection should be valid");
        document.set_freeform_selection(selection);

        let _record = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(68.0, 64.0)],
            brush_settings(),
            BrushToolMode::Paint,
            LayerEditTarget::LayerPixels,
        )
        .expect("selected brush stroke should produce a record");

        let inside_alpha = pixel_alpha(&document, 0, 68, 64);
        let outside_alpha = pixel_alpha(&document, 0, 58, 64);
        assert!(inside_alpha > 0);
        assert_eq!(outside_alpha, 0);
    }

    #[test]
    fn move_tool_moves_only_selected_pixels() {
        let mut document = Document::new(128, 128);
        let _ = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(20.0, 20.0)],
            brush_settings(),
            BrushToolMode::Paint,
            LayerEditTarget::LayerPixels,
        );
        let _ = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(60.0, 20.0)],
            brush_settings(),
            BrushToolMode::Paint,
            LayerEditTarget::LayerPixels,
        );
        document.set_selection(CanvasRect::new(16, 16, 16, 16));

        let _record =
            MoveTool::move_layer(&mut document, 0, 10, 0).expect("selected move should apply");

        assert_eq!(pixel_alpha(&document, 0, 20, 20), 0);
        assert!(pixel_alpha(&document, 0, 30, 20) > 0);
        assert!(pixel_alpha(&document, 0, 60, 20) > 0);
    }

    #[test]
    fn transform_tool_transforms_only_selected_pixels() {
        let mut document = Document::new(128, 128);
        let _ = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(20.0, 20.0)],
            brush_settings(),
            BrushToolMode::Paint,
            LayerEditTarget::LayerPixels,
        );
        let _ = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(60.0, 20.0)],
            brush_settings(),
            BrushToolMode::Paint,
            LayerEditTarget::LayerPixels,
        );
        document.set_selection(CanvasRect::new(16, 16, 16, 16));

        let _record = SimpleTransformTool::transform_layer(&mut document, 0, 1.0, 1.0, 0, 10, 0)
            .expect("selected transform should apply");

        assert_eq!(pixel_alpha(&document, 0, 20, 20), 0);
        assert!(pixel_alpha(&document, 0, 30, 20) > 0);
        assert!(pixel_alpha(&document, 0, 60, 20) > 0);
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
            LayerEditTarget::LayerPixels,
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
            LayerEditTarget::LayerPixels,
        )
        .expect("inverted eraser stroke should apply");

        let center_alpha = pixel_alpha(&document, 0, 64, 64);
        assert_eq!(center_alpha, 255);
    }

    #[test]
    fn mask_stroke_updates_alpha_and_roundtrips_through_history() {
        let mut document = Document::new(128, 128);
        assert!(document.add_layer_mask(0));

        let hide = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(32.0, 32.0)],
            brush_settings(),
            BrushToolMode::Paint,
            LayerEditTarget::LayerMask,
        )
        .expect("mask hide stroke should create a record");

        assert_eq!(hide.target, LayerEditTarget::LayerMask);
        let coord = document
            .tile_coord_for_pixel(32, 32)
            .expect("mask stroke should map to tile");
        let tile = document
            .layer_mask(0)
            .expect("mask exists")
            .tiles
            .get(&coord)
            .expect("mask tile exists");
        let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
        let local_x = (32 - tile_origin_x) as usize;
        let local_y = (32 - tile_origin_y) as usize;
        let mask_index = local_y * document.tile_size as usize + local_x;
        assert!(tile.alpha[mask_index] < 255);

        hide.undo(&mut document);
        assert!(
            document
                .layer_mask(0)
                .expect("mask exists")
                .tiles
                .is_empty()
        );

        hide.redo(&mut document);
        assert!(
            !document
                .layer_mask(0)
                .expect("mask exists")
                .tiles
                .is_empty()
        );

        let reveal = BrushTool::apply_stroke(
            &mut document,
            0,
            &[(32.0, 32.0)],
            brush_settings(),
            BrushToolMode::Erase,
            LayerEditTarget::LayerMask,
        )
        .expect("mask reveal stroke should create a record");

        reveal.undo(&mut document);
        assert!(
            !document
                .layer_mask(0)
                .expect("mask exists")
                .tiles
                .is_empty()
        );

        reveal.redo(&mut document);
        let revealed_tile = document
            .layer_mask(0)
            .expect("mask exists")
            .tiles
            .get(&coord)
            .expect("mask tile exists after reveal");
        assert_eq!(revealed_tile.alpha[mask_index], 255);
    }

    #[test]
    fn simple_transform_preview_bounds_include_scale_and_translation() {
        let mut document = Document::new(512, 512);
        let _ = document.ensure_tile_for_pixel(0, 32, 32);

        let bounds = SimpleTransformTool::preview_bounds(&document, 0, 1.5, 1.5, 0, 20, -10)
            .expect("preview bounds should exist");

        assert_eq!(bounds, CanvasRect::new(20, -10, 384, 384));
    }

    #[test]
    fn simple_transform_preview_bounds_support_non_uniform_scale_and_rotation() {
        let mut document = Document::new(512, 512);
        let _ = document.ensure_tile_for_pixel(0, 32, 32);

        let bounds = SimpleTransformTool::preview_bounds(&document, 0, 2.0, 0.5, 1, 20, -10)
            .expect("preview bounds should exist");

        assert_eq!(bounds, CanvasRect::new(20, -10, 128, 512));
    }

    #[test]
    fn simple_transform_can_be_undone_and_redone() {
        let mut document = Document::new(512, 512);
        let tile_size = document.tile_size as usize;
        let tile = document
            .ensure_tile_for_pixel(0, 8, 8)
            .expect("tile should be created");
        tile.pixels[(8 * tile_size + 8) * 4 + 3] = 255;

        let before = document
            .layer_state_snapshot(0)
            .expect("snapshot should exist");
        let record = SimpleTransformTool::transform_layer(&mut document, 0, 2.0, 2.0, 0, 15, 5)
            .expect("transform should produce a record");
        let after = document
            .layer_state_snapshot(0)
            .expect("snapshot should exist");

        assert_ne!(before, after);
        record.undo(&mut document);
        assert_eq!(document.layer_state_snapshot(0), Some(before.clone()));
        record.redo(&mut document);
        assert_eq!(document.layer_state_snapshot(0), Some(after));
    }

    #[test]
    fn simple_transform_can_rotate_quarter_turns() {
        let mut document = Document::new(512, 512);
        let tile_size = document.tile_size as usize;
        let tile = document
            .ensure_tile_for_pixel(0, 8, 8)
            .expect("tile should be created");
        tile.pixels[(8 * tile_size + 8) * 4 + 3] = 255;

        let record = SimpleTransformTool::transform_layer(&mut document, 0, 1.0, 1.0, 1, 0, 0)
            .expect("rotation should produce a record");

        assert_eq!(pixel_alpha(&document, 0, 247, 8), 255);
        record.undo(&mut document);
        assert_eq!(pixel_alpha(&document, 0, 8, 8), 255);
    }
}
