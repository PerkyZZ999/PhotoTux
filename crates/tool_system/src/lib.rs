//! Tool dispatch and interaction scaffolding for PhotoTux.

use common::{LayerId, Point, round_to_pixel};
use doc_model::{RasterSurface, SelectionBounds, SelectionMask};
use history_engine::{PixelChange, StrokeHistoryEntry};
use image_ops::{
    BrushApplicationMode, BrushPreview, PixelChangeRecord, RoundBrushDab, ScaleTransformPreview,
    apply_round_brush_dab, brush_preview, preview_scale_surface_nearest, scale_surface_nearest,
    translate_selection, translate_surface,
};
use std::collections::BTreeMap;

/// Active stroke tool kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrokeToolKind {
    /// Standard paint brush.
    Brush,
    /// Alpha-reducing eraser.
    Eraser,
}

/// A sampled pointer position for stroke generation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeSample {
    /// Sample position in document coordinates.
    pub position: Point,
}

impl StrokeSample {
    /// Create a stroke sample.
    #[must_use]
    pub const fn new(position: Point) -> Self {
        Self { position }
    }
}

/// Adjustable round-brush settings for the feasibility prototype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSettings {
    /// Brush diameter in pixels.
    pub size: f32,
    /// Hardness in the range `[0.0, 1.0]`.
    pub hardness: f32,
    /// Opacity in the range `[0.0, 1.0]`.
    pub opacity: f32,
    /// Flow in the range `[0.0, 1.0]`.
    pub flow: f32,
    /// Spacing as a fraction of brush diameter.
    pub spacing: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 24.0,
            hardness: 0.85,
            opacity: 1.0,
            flow: 1.0,
            spacing: 0.2,
        }
    }
}

impl BrushSettings {
    fn spacing_pixels(self) -> f32 {
        (self.size * self.spacing.clamp(0.05, 2.0)).max(1.0)
    }
}

/// Collects pointer samples for a single stroke.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StrokeCollector {
    samples: Vec<StrokeSample>,
}

impl StrokeCollector {
    /// Create an empty stroke collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pointer sample.
    pub fn push_sample(&mut self, sample: StrokeSample) {
        self.samples.push(sample);
    }

    /// Return the collected samples.
    #[must_use]
    pub fn samples(&self) -> &[StrokeSample] {
        &self.samples
    }

    /// Finish and return the collected samples.
    #[must_use]
    pub fn finish(self) -> Vec<StrokeSample> {
        self.samples
    }
}

/// Result of applying a stroke to a surface.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeApplication {
    /// Grouped undo entry for the full stroke.
    pub history_entry: StrokeHistoryEntry,
    /// Number of dabs emitted for the stroke.
    pub dab_count: usize,
}

/// Result of applying a move transform.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveToolApplication {
    /// Grouped undo entry for the move.
    pub history_entry: StrokeHistoryEntry,
    /// Horizontal delta in pixels.
    pub delta_x: i32,
    /// Vertical delta in pixels.
    pub delta_y: i32,
}

/// Result of committing a scale transform.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleTransformApplication {
    /// Grouped undo entry for the scale.
    pub history_entry: StrokeHistoryEntry,
    /// Preview metadata describing the committed scale transform.
    pub preview: ScaleTransformPreview,
}

/// Round brush and eraser tool implementation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushTool {
    /// Active tool kind.
    pub kind: StrokeToolKind,
    /// Brush settings.
    pub settings: BrushSettings,
    /// Brush color for paint mode.
    pub color: [u8; 4],
}

/// Minimal move-tool implementation for layer and selection translation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MoveTool;

/// Minimal scale-transform tool implementation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScaleTransformTool;

impl BrushTool {
    /// Create a paint brush tool.
    #[must_use]
    pub fn brush(settings: BrushSettings, color: [u8; 4]) -> Self {
        Self {
            kind: StrokeToolKind::Brush,
            settings,
            color,
        }
    }

    /// Create an eraser tool.
    #[must_use]
    pub fn eraser(settings: BrushSettings) -> Self {
        Self {
            kind: StrokeToolKind::Eraser,
            settings,
            color: [0, 0, 0, 0],
        }
    }

    /// Build preview geometry for the current brush.
    #[must_use]
    pub fn preview(&self) -> BrushPreview {
        brush_preview(self.settings.size, self.settings.hardness)
    }

    /// Expand stroke samples into interpolated dab positions.
    #[must_use]
    pub fn interpolated_dabs(&self, samples: &[StrokeSample]) -> Vec<Point> {
        interpolate_stroke_samples(samples, self.settings.spacing_pixels())
    }

    /// Apply a full stroke and return a single grouped history entry.
    pub fn apply_stroke(
        &self,
        surface: &mut doc_model::RasterSurface,
        samples: &[StrokeSample],
    ) -> StrokeApplication {
        let dab_positions = self.interpolated_dabs(samples);
        let mut changes_by_pixel = BTreeMap::<(u32, u32), PixelChange>::new();

        for position in &dab_positions {
            let mode = match self.kind {
                StrokeToolKind::Brush => BrushApplicationMode::Paint { color: self.color },
                StrokeToolKind::Eraser => BrushApplicationMode::Erase,
            };

            let records = apply_round_brush_dab(
                surface,
                RoundBrushDab {
                    center: *position,
                    size: self.settings.size,
                    hardness: self.settings.hardness,
                    opacity: self.settings.opacity,
                    flow: self.settings.flow,
                    mode,
                },
            );

            merge_pixel_changes(&mut changes_by_pixel, &records);
        }

        StrokeApplication {
            history_entry: StrokeHistoryEntry::new(changes_by_pixel.into_values().collect()),
            dab_count: dab_positions.len(),
        }
    }
}

impl MoveTool {
    /// Convert drag endpoints into a pixel-aligned delta.
    #[must_use]
    pub fn drag_delta(&self, start: Point, current: Point) -> (i32, i32) {
        (
            round_to_pixel(current.x - start.x),
            round_to_pixel(current.y - start.y),
        )
    }

    /// Translate an entire layer surface.
    pub fn move_layer(
        &self,
        layer_id: LayerId,
        surface: &mut RasterSurface,
        start: Point,
        current: Point,
    ) -> MoveToolApplication {
        let (delta_x, delta_y) = self.drag_delta(start, current);
        let records = translate_surface(surface, delta_x, delta_y);

        MoveToolApplication {
            history_entry: StrokeHistoryEntry::for_layer(
                layer_id,
                records_to_history_changes(&records),
            ),
            delta_x,
            delta_y,
        }
    }

    /// Translate only the selected pixels in a layer and move the selection mask with them.
    pub fn move_selection(
        &self,
        layer_id: LayerId,
        surface: &mut RasterSurface,
        selection_mask: &mut SelectionMask,
        start: Point,
        current: Point,
    ) -> MoveToolApplication {
        let (delta_x, delta_y) = self.drag_delta(start, current);
        let records = translate_selection(surface, selection_mask, delta_x, delta_y);
        selection_mask.translate(delta_x, delta_y);

        MoveToolApplication {
            history_entry: StrokeHistoryEntry::for_layer(
                layer_id,
                records_to_history_changes(&records),
            ),
            delta_x,
            delta_y,
        }
    }
}

impl ScaleTransformTool {
    /// Preview a deterministic nearest-neighbor scale transform for a layer or selection region.
    #[must_use]
    pub fn preview_scale(
        &self,
        surface: &RasterSurface,
        bounds: Option<SelectionBounds>,
        scale_x: f32,
        scale_y: f32,
    ) -> ScaleTransformPreview {
        let source_bounds = bounds.unwrap_or_else(|| {
            SelectionBounds::new(0, 0, surface.width().max(1), surface.height().max(1))
        });

        preview_scale_surface_nearest(surface, source_bounds, scale_x, scale_y)
    }

    /// Commit a deterministic nearest-neighbor scale transform.
    pub fn commit_scale(
        &self,
        layer_id: LayerId,
        surface: &mut RasterSurface,
        bounds: Option<SelectionBounds>,
        scale_x: f32,
        scale_y: f32,
    ) -> ScaleTransformApplication {
        let preview = self.preview_scale(surface, bounds, scale_x, scale_y);
        let records = scale_surface_nearest(surface, preview.source_bounds, scale_x, scale_y);

        ScaleTransformApplication {
            history_entry: StrokeHistoryEntry::for_layer(
                layer_id,
                records_to_history_changes(&records),
            ),
            preview,
        }
    }
}

/// Interpolate a stroke into evenly-spaced dab positions.
#[must_use]
pub fn interpolate_stroke_samples(samples: &[StrokeSample], spacing_pixels: f32) -> Vec<Point> {
    if samples.is_empty() {
        return Vec::new();
    }
    if samples.len() == 1 {
        return vec![samples[0].position];
    }

    let spacing_pixels = spacing_pixels.max(1.0);
    let estimated_capacity = estimate_dab_capacity(samples, spacing_pixels);
    let mut dabs = Vec::with_capacity(estimated_capacity);
    dabs.push(samples[0].position);

    for window in samples.windows(2) {
        let start = window[0].position;
        let end = window[1].position;
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance <= f32::EPSILON {
            continue;
        }

        let mut traveled = spacing_pixels;
        while traveled < distance {
            let t = traveled / distance;
            dabs.push(Point::new(start.x + dx * t, start.y + dy * t));
            traveled += spacing_pixels;
        }

        if dabs
            .last()
            .is_none_or(|last| (last.x - end.x).abs() > 0.001 || (last.y - end.y).abs() > 0.001)
        {
            dabs.push(end);
        }
    }

    dabs
}

fn estimate_dab_capacity(samples: &[StrokeSample], spacing_pixels: f32) -> usize {
    let mut capacity = 1usize;

    for window in samples.windows(2) {
        let start = window[0].position;
        let end = window[1].position;
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance <= f32::EPSILON {
            continue;
        }

        capacity += (distance / spacing_pixels).ceil() as usize;
    }

    capacity.max(samples.len())
}

fn merge_pixel_changes(
    changes_by_pixel: &mut BTreeMap<(u32, u32), PixelChange>,
    records: &[PixelChangeRecord],
) {
    for record in records {
        changes_by_pixel
            .entry((record.x, record.y))
            .and_modify(|change| change.after = record.after)
            .or_insert_with(|| PixelChange::new(record.x, record.y, record.before, record.after));
    }
}

fn records_to_history_changes(records: &[PixelChangeRecord]) -> Vec<PixelChange> {
    records
        .iter()
        .map(|record| PixelChange::new(record.x, record.y, record.before, record.after))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        BrushSettings, BrushTool, MoveTool, ScaleTransformTool, StrokeCollector, StrokeSample,
        StrokeToolKind, interpolate_stroke_samples,
    };
    use common::{DocumentId, LayerId, Point};
    use doc_model::{
        Canvas, Document, DocumentMetadata, LayeredRasterDocument, RasterLayer, RasterSurface,
    };
    use history_engine::HistoryStack;

    #[test]
    fn stroke_collector_keeps_samples_in_order() {
        let mut collector = StrokeCollector::new();
        collector.push_sample(StrokeSample::new(Point::new(1.0, 2.0)));
        collector.push_sample(StrokeSample::new(Point::new(3.0, 4.0)));

        assert_eq!(collector.samples().len(), 2);
        assert_eq!(collector.samples()[1].position, Point::new(3.0, 4.0));
    }

    #[test]
    fn interpolation_adds_dabs_between_samples() {
        let points = interpolate_stroke_samples(
            &[
                StrokeSample::new(Point::new(0.0, 0.0)),
                StrokeSample::new(Point::new(20.0, 0.0)),
            ],
            5.0,
        );

        assert!(points.len() >= 5);
        assert_eq!(points.first().copied(), Some(Point::new(0.0, 0.0)));
        assert_eq!(points.last().copied(), Some(Point::new(20.0, 0.0)));
    }

    #[test]
    fn brush_stroke_groups_changes_into_single_history_entry() {
        let mut surface = RasterSurface::new(64, 64);
        let tool = BrushTool::brush(BrushSettings::default(), [255, 0, 0, 255]);

        let stroke = tool.apply_stroke(
            &mut surface,
            &[
                StrokeSample::new(Point::new(8.0, 8.0)),
                StrokeSample::new(Point::new(24.0, 8.0)),
            ],
        );

        assert!(stroke.dab_count >= 2);
        assert!(!stroke.history_entry.is_empty());
    }

    #[test]
    fn eraser_tool_preserves_tool_kind() {
        let tool = BrushTool::eraser(BrushSettings::default());

        assert_eq!(tool.kind, StrokeToolKind::Eraser);
        assert_eq!(tool.preview().outline_points.len(), 32);
    }

    #[test]
    fn move_tool_drag_rounds_to_pixel_delta() {
        let tool = MoveTool;

        assert_eq!(
            tool.drag_delta(Point::new(10.2, 20.1), Point::new(14.7, 17.8)),
            (5, -2)
        );
    }

    #[test]
    fn move_tool_translates_entire_layer_surface() {
        let tool = MoveTool;
        let mut surface = RasterSurface::new(16, 16);
        let _ = surface.write_pixel(2, 3, [255, 0, 0, 255]);

        let application = tool.move_layer(
            LayerId::new(1),
            &mut surface,
            Point::new(2.0, 3.0),
            Point::new(5.0, 7.0),
        );

        assert_eq!((application.delta_x, application.delta_y), (3, 4));
        assert_eq!(surface.pixel(5, 7), [255, 0, 0, 255]);
        assert_eq!(surface.pixel(2, 3), [0, 0, 0, 0]);
    }

    #[test]
    fn move_tool_translates_selected_pixels_and_mask() {
        let tool = MoveTool;
        let mut document = Document::new(
            DocumentId::new(1),
            Canvas::new(16, 16),
            DocumentMetadata {
                title: "move selection".to_string(),
            },
        );
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));
        document.select_rect(2, 2, 4, 4);

        let mut layered_document = LayeredRasterDocument::from_document(document);
        {
            let (surface, selection_mask) = layered_document
                .surface_and_selection_mut(LayerId::new(1))
                .expect("surface and selection should exist");
            let _ = surface.write_pixel(2, 2, [255, 0, 0, 255]);

            let application = tool.move_selection(
                LayerId::new(1),
                surface,
                selection_mask,
                Point::new(2.0, 2.0),
                Point::new(5.0, 4.0),
            );

            assert_eq!((application.delta_x, application.delta_y), (3, 2));
        }
        assert_eq!(
            layered_document
                .surface(LayerId::new(1))
                .map(|surface| surface.pixel(5, 4)),
            Some([255, 0, 0, 255])
        );
        assert_eq!(
            layered_document.document().selection_mask().bounds(),
            Some(doc_model::SelectionBounds::new(5, 4, 2, 2))
        );
    }

    #[test]
    fn scale_transform_preview_reports_scaled_bounds() {
        let tool = ScaleTransformTool;
        let mut surface = RasterSurface::new(8, 8);
        let _ = surface.write_pixel(1, 1, [255, 0, 0, 255]);

        let preview = tool.preview_scale(
            &surface,
            Some(doc_model::SelectionBounds::new(0, 0, 4, 4)),
            2.0,
            1.5,
        );

        assert_eq!(preview.target_bounds.width, 8);
        assert_eq!(preview.target_bounds.height, 6);
    }

    #[test]
    fn scale_transform_commit_is_deterministic() {
        let tool = ScaleTransformTool;
        let mut left = RasterSurface::new(8, 8);
        let mut right = RasterSurface::new(8, 8);
        let _ = left.write_pixel(1, 1, [255, 0, 0, 255]);
        let _ = right.write_pixel(1, 1, [255, 0, 0, 255]);

        let _ = tool.commit_scale(LayerId::new(1), &mut left, None, 2.0, 2.0);
        let _ = tool.commit_scale(LayerId::new(1), &mut right, None, 2.0, 2.0);

        assert_eq!(left, right);
    }

    #[test]
    fn transform_history_undo_and_redo_restore_layer_pixels() {
        let move_tool = MoveTool;
        let scale_tool = ScaleTransformTool;
        let mut history = HistoryStack::new();
        let mut document = Document::new(
            DocumentId::new(7),
            Canvas::new(16, 16),
            DocumentMetadata {
                title: "transform history".to_string(),
            },
        );
        document.add_layer(RasterLayer::new(LayerId::new(2), "Paint"));

        let mut layered_document = LayeredRasterDocument::from_document(document);
        let surface = layered_document
            .surface_mut(LayerId::new(2))
            .expect("surface should exist");
        let _ = surface.write_pixel(2, 2, [255, 0, 0, 255]);

        let moved = move_tool.move_layer(
            LayerId::new(2),
            surface,
            Point::new(2.0, 2.0),
            Point::new(4.0, 5.0),
        );
        history.push_brush_stroke(moved.history_entry);

        let scaled = scale_tool.commit_scale(
            LayerId::new(2),
            layered_document
                .surface_mut(LayerId::new(2))
                .expect("surface should exist"),
            None,
            1.5,
            1.5,
        );
        history.push_brush_stroke(scaled.history_entry);
        let transformed_surface = layered_document
            .surface(LayerId::new(2))
            .cloned()
            .expect("surface should exist after transform");

        assert!(history.undo_layered(&mut layered_document));
        assert!(history.undo_layered(&mut layered_document));
        assert_eq!(
            layered_document
                .surface(LayerId::new(2))
                .map(|surface| surface.pixel(2, 2)),
            Some([255, 0, 0, 255])
        );

        assert!(history.redo_layered(&mut layered_document));
        assert!(history.redo_layered(&mut layered_document));
        assert_eq!(
            layered_document.surface(LayerId::new(2)).cloned(),
            Some(transformed_surface)
        );
    }
}
