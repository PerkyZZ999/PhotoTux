use std::cell::RefCell;
use std::rc::Rc;

use common::{CanvasRaster, CanvasRect};
use doc_model::Document;
use file_io::flatten_document_rgba;
use history_engine::HistoryStack;
use tool_system::{
    BrushSettings, BrushStrokeRecord, BrushTool, BrushToolMode, MoveLayerRecord,
    RectangularMarqueeTool, RectangularSelectionRecord,
};
use ui_shell::{LayerPanelItem, ShellController, ShellSnapshot, ShellToolKind};

pub fn build_shell_controller() -> Rc<RefCell<dyn ShellController>> {
    Rc::new(RefCell::new(PhotoTuxController::new()))
}

#[derive(Debug)]
struct PhotoTuxController {
    document: Document,
    history: HistoryStack<EditorHistoryEntry>,
    foreground_color: [u8; 4],
    background_color: [u8; 4],
    document_title: String,
    next_layer_number: usize,
    active_tool: ShellToolKind,
    canvas_revision: u64,
    interaction: Option<CanvasInteraction>,
}

#[derive(Debug, Clone)]
enum CanvasInteraction {
    Move {
        layer_id: common::LayerId,
        start_canvas_x: i32,
        start_canvas_y: i32,
        start_offset_x: i32,
        start_offset_y: i32,
    },
    Marquee {
        before: Option<common::CanvasRect>,
        before_inverted: bool,
        start_canvas_x: i32,
        start_canvas_y: i32,
    },
    Brush {
        mode: BrushToolMode,
        last_canvas_x: i32,
        last_canvas_y: i32,
        aggregate: Option<BrushStrokeRecord>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EditorHistoryEntry {
    label: String,
    operation: Option<EditorOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EditorOperation {
    BrushStroke(BrushStrokeRecord),
    MoveLayer(MoveLayerRecord),
    Selection(RectangularSelectionRecord),
}

impl PhotoTuxController {
    fn new() -> Self {
        let mut document = Document::new(1920, 1080);
        document.rename_layer(0, "Background");
        let background_tile = document
            .ensure_tile_for_pixel(0, 32, 32)
            .expect("background tile should be created");
        background_tile.pixels[3] = 255;
        document.add_layer("Sketch");
        let sketch_index = document.active_layer_index();
        let sketch_tile = document
            .ensure_tile_for_pixel(sketch_index, 180, 140)
            .expect("sketch tile should be created");
        sketch_tile.pixels[0] = 120;
        sketch_tile.pixels[3] = 255;
        document.add_layer("Highlights");
        let highlights_index = document.active_layer_index();
        let highlights_tile = document
            .ensure_tile_for_pixel(highlights_index, 260, 180)
            .expect("highlights tile should be created");
        highlights_tile.pixels[0] = 220;
        highlights_tile.pixels[1] = 220;
        highlights_tile.pixels[3] = 255;

        let mut history = HistoryStack::default();
        history.push(EditorHistoryEntry {
            label: "Open Document".to_string(),
            operation: None,
        });

        Self {
            document,
            history,
            foreground_color: [232, 236, 243, 255],
            background_color: [27, 29, 33, 255],
            document_title: "untitled.ptx".to_string(),
            next_layer_number: 4,
            active_tool: ShellToolKind::Brush,
            canvas_revision: 1,
            interaction: None,
        }
    }

    fn active_layer_name(&self) -> String {
        self.document.active_layer().name.clone()
    }

    fn push_history(&mut self, entry: impl Into<String>) {
        self.history.push(EditorHistoryEntry {
            label: entry.into(),
            operation: None,
        });
    }

    fn push_operation(&mut self, label: impl Into<String>, operation: EditorOperation) {
        self.history.push(EditorHistoryEntry {
            label: label.into(),
            operation: Some(operation),
        });
    }

    fn bump_canvas_revision(&mut self) {
        self.canvas_revision = self.canvas_revision.saturating_add(1);
    }

    fn current_brush_settings(&self, mode: BrushToolMode) -> BrushSettings {
        BrushSettings {
            radius: 12.0,
            hardness: 0.8,
            opacity: 1.0,
            spacing: 6.0,
            color: match mode {
                BrushToolMode::Paint => self.foreground_color,
                BrushToolMode::Erase => [0, 0, 0, 255],
            },
        }
    }

    fn apply_active_layer_stroke_segment(
        &mut self,
        mode: BrushToolMode,
        points: &[(f32, f32)],
    ) -> Option<BrushStrokeRecord> {
        let layer_index = self.document.active_layer_index();
        let settings = self.current_brush_settings(mode);
        let record = BrushTool::apply_stroke(&mut self.document, layer_index, points, settings, mode)?;
        self.bump_canvas_revision();
        Some(record)
    }

    fn merge_brush_records(aggregate: &mut BrushStrokeRecord, segment: BrushStrokeRecord) {
        aggregate.dab_count += segment.dab_count;

        for change in segment.changes {
            if let Some(existing) = aggregate
                .changes
                .iter_mut()
                .find(|existing| existing.layer_id == change.layer_id && existing.coord == change.coord)
            {
                existing.after = change.after;
            } else {
                aggregate.changes.push(change);
            }
        }
    }

    fn layer_items(&self) -> Vec<LayerPanelItem> {
        self.document
            .layers
            .iter()
            .enumerate()
            .rev()
            .map(|(index, layer)| LayerPanelItem {
                index,
                name: layer.name.clone(),
                visible: layer.visible,
                opacity_percent: layer.opacity_percent,
                is_active: index == self.document.active_layer_index(),
            })
            .collect()
    }

    fn move_active_layer_by(&mut self, delta: isize) {
        let current = self.document.active_layer_index() as isize;
        let target = (current + delta).clamp(0, self.document.layer_count().saturating_sub(1) as isize);
        if current == target {
            return;
        }

        let active_name = self.active_layer_name();
        if self.document.move_layer(current as usize, target as usize) {
            self.bump_canvas_revision();
            self.push_history(format!("Move Layer {}", active_name));
        }
    }

    fn active_layer_bounds(&self) -> Option<CanvasRect> {
        self.document.layer_canvas_bounds(self.document.active_layer_index())
    }
}

impl ShellController for PhotoTuxController {
    fn snapshot(&self) -> ShellSnapshot {
        let active_layer = self.document.active_layer();
        ShellSnapshot {
            document_title: self.document_title.clone(),
            canvas_size: self.document.canvas_size,
            canvas_revision: self.canvas_revision,
            active_tool_name: self.active_tool.label().to_string(),
            active_tool: self.active_tool,
            layers: self.layer_items(),
            active_layer_name: active_layer.name.clone(),
            active_layer_opacity_percent: active_layer.opacity_percent,
            active_layer_visible: active_layer.visible,
            active_layer_blend_mode: format!("{:?}", active_layer.blend_mode),
            active_layer_bounds: self.active_layer_bounds(),
            selection_rect: self.document.selection(),
            selection_inverted: self.document.selection_inverted(),
            foreground_color: self.foreground_color,
            background_color: self.background_color,
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            history_entries: self
                .history
                .undo_entries()
                .iter()
                .rev()
                .map(|entry| entry.label.clone())
                .collect(),
        }
    }

    fn canvas_raster(&self) -> CanvasRaster {
        CanvasRaster {
            size: self.document.canvas_size,
            pixels: flatten_document_rgba(&self.document),
        }
    }

    fn add_layer(&mut self) {
        let layer_name = format!("Layer {}", self.next_layer_number);
        self.next_layer_number += 1;
        self.document.add_layer(layer_name.clone());
        self.bump_canvas_revision();
        self.push_history(format!("Add Layer {}", layer_name));
    }

    fn duplicate_active_layer(&mut self) {
        let active_index = self.document.active_layer_index();
        let active_name = self.active_layer_name();
        if self.document.duplicate_layer(active_index).is_some() {
            self.bump_canvas_revision();
            self.push_history(format!("Duplicate Layer {}", active_name));
        }
    }

    fn delete_active_layer(&mut self) {
        let active_index = self.document.active_layer_index();
        let active_name = self.active_layer_name();
        if self.document.delete_layer(active_index) {
            self.bump_canvas_revision();
            self.push_history(format!("Delete Layer {}", active_name));
        }
    }

    fn select_layer(&mut self, index: usize) {
        let _ = self.document.set_active_layer(index);
    }

    fn toggle_layer_visibility(&mut self, index: usize) {
        if let Some(layer) = self.document.layer(index) {
            let visible = !layer.visible;
            let layer_name = layer.name.clone();
            self.document.set_layer_visibility(index, visible);
            self.bump_canvas_revision();
            self.push_history(format!("Toggle Visibility {}", layer_name));
        }
    }

    fn increase_active_layer_opacity(&mut self) {
        let active_index = self.document.active_layer_index();
        let next_opacity = (self.document.active_layer().opacity_percent + 10).min(100);
        self.document.set_layer_opacity(active_index, next_opacity);
        self.bump_canvas_revision();
        self.push_history(format!("Increase Opacity {}", self.active_layer_name()));
    }

    fn decrease_active_layer_opacity(&mut self) {
        let active_index = self.document.active_layer_index();
        let next_opacity = self.document.active_layer().opacity_percent.saturating_sub(10);
        self.document.set_layer_opacity(active_index, next_opacity);
        self.bump_canvas_revision();
        self.push_history(format!("Decrease Opacity {}", self.active_layer_name()));
    }

    fn move_active_layer_up(&mut self) {
        self.move_active_layer_by(1);
    }

    fn move_active_layer_down(&mut self) {
        self.move_active_layer_by(-1);
    }

    fn swap_colors(&mut self) {
        std::mem::swap(&mut self.foreground_color, &mut self.background_color);
        self.push_history("Swap Colors");
    }

    fn reset_colors(&mut self) {
        self.foreground_color = [232, 236, 243, 255];
        self.background_color = [27, 29, 33, 255];
        self.push_history("Reset Colors");
    }

    fn clear_selection(&mut self) {
        let before = self.document.selection();
        let before_inverted = self.document.selection_inverted();
        if before.is_none() {
            return;
        }

        self.document.clear_selection();
        self.push_operation(
            "Clear Selection",
            EditorOperation::Selection(RectangularSelectionRecord {
                before,
                before_inverted,
                after: None,
                after_inverted: false,
            }),
        );
    }

    fn invert_selection(&mut self) {
        let before = self.document.selection();
        let before_inverted = self.document.selection_inverted();
        if !self.document.invert_selection() {
            return;
        }

        self.push_operation(
            "Invert Selection",
            EditorOperation::Selection(RectangularSelectionRecord {
                before,
                before_inverted,
                after: self.document.selection(),
                after_inverted: self.document.selection_inverted(),
            }),
        );
    }

    fn undo(&mut self) {
        let Some(entry) = self.history.undo().cloned() else {
            return;
        };

        if let Some(operation) = entry.operation {
            match operation {
                EditorOperation::BrushStroke(record) => {
                    record.undo(&mut self.document);
                    self.bump_canvas_revision();
                }
                EditorOperation::MoveLayer(record) => {
                    record.undo(&mut self.document);
                    self.bump_canvas_revision();
                }
                EditorOperation::Selection(record) => record.undo(&mut self.document),
            }
        }
    }

    fn redo(&mut self) {
        let Some(entry) = self.history.redo().cloned() else {
            return;
        };

        if let Some(operation) = entry.operation {
            match operation {
                EditorOperation::BrushStroke(record) => {
                    record.redo(&mut self.document);
                    self.bump_canvas_revision();
                }
                EditorOperation::MoveLayer(record) => {
                    record.redo(&mut self.document);
                    self.bump_canvas_revision();
                }
                EditorOperation::Selection(record) => record.redo(&mut self.document),
            }
        }
    }

    fn select_tool(&mut self, tool: ShellToolKind) {
        self.active_tool = tool;
    }

    fn begin_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32) {
        match self.active_tool {
            ShellToolKind::Move => {
                let (start_offset_x, start_offset_y) = self
                    .document
                    .layer_offset(self.document.active_layer_index())
                    .unwrap_or((0, 0));
                self.interaction = Some(CanvasInteraction::Move {
                    layer_id: self.document.active_layer().id,
                    start_canvas_x: canvas_x,
                    start_canvas_y: canvas_y,
                    start_offset_x,
                    start_offset_y,
                });
            }
            ShellToolKind::RectangularMarquee => {
                self.interaction = Some(CanvasInteraction::Marquee {
                    before: self.document.selection(),
                    before_inverted: self.document.selection_inverted(),
                    start_canvas_x: canvas_x,
                    start_canvas_y: canvas_y,
                });
            }
            ShellToolKind::Brush | ShellToolKind::Eraser => {
                let mode = if self.active_tool == ShellToolKind::Brush {
                    BrushToolMode::Paint
                } else {
                    BrushToolMode::Erase
                };
                let aggregate =
                    self.apply_active_layer_stroke_segment(mode, &[(canvas_x as f32, canvas_y as f32)]);
                self.interaction = Some(CanvasInteraction::Brush {
                    mode,
                    last_canvas_x: canvas_x,
                    last_canvas_y: canvas_y,
                    aggregate,
                });
            }
            _ => {}
        }
    }

    fn update_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32) {
        let Some(interaction) = self.interaction.take() else {
            return;
        };

        self.interaction = match interaction {
            CanvasInteraction::Move {
                layer_id,
                start_canvas_x,
                start_canvas_y,
                start_offset_x,
                start_offset_y,
            } => {
                let delta_x = canvas_x - start_canvas_x;
                let delta_y = canvas_y - start_canvas_y;
                let _ = self.document.set_layer_offset(
                    self.document.active_layer_index(),
                    start_offset_x + delta_x,
                    start_offset_y + delta_y,
                );
                self.bump_canvas_revision();
                Some(CanvasInteraction::Move {
                    layer_id,
                    start_canvas_x,
                    start_canvas_y,
                    start_offset_x,
                    start_offset_y,
                })
            }
            CanvasInteraction::Marquee {
                before,
                before_inverted,
                start_canvas_x,
                start_canvas_y,
            } => {
                if let Some(rect) =
                    RectangularMarqueeTool::preview_rect(start_canvas_x, start_canvas_y, canvas_x, canvas_y)
                {
                    self.document.set_selection(rect);
                } else {
                    self.document.clear_selection();
                }
                Some(CanvasInteraction::Marquee {
                    before,
                    before_inverted,
                    start_canvas_x,
                    start_canvas_y,
                })
            }
            CanvasInteraction::Brush {
                mode,
                last_canvas_x,
                last_canvas_y,
                mut aggregate,
            } => {
                if last_canvas_x != canvas_x || last_canvas_y != canvas_y {
                    if let Some(segment) = self.apply_active_layer_stroke_segment(
                        mode,
                        &[
                            (last_canvas_x as f32, last_canvas_y as f32),
                            (canvas_x as f32, canvas_y as f32),
                        ],
                    ) {
                        if let Some(existing) = &mut aggregate {
                            Self::merge_brush_records(existing, segment);
                        } else {
                            aggregate = Some(segment);
                        }
                    }
                }

                Some(CanvasInteraction::Brush {
                    mode,
                    last_canvas_x: canvas_x,
                    last_canvas_y: canvas_y,
                    aggregate,
                })
            }
        };
    }

    fn end_canvas_interaction(&mut self) {
        match self.interaction.take() {
            Some(CanvasInteraction::Move {
                layer_id,
                start_offset_x,
                start_offset_y,
                ..
            }) => {
                let (current_x, current_y) = self
                    .document
                    .layer_offset(self.document.active_layer_index())
                    .unwrap_or((0, 0));
                let delta_x = current_x - start_offset_x;
                let delta_y = current_y - start_offset_y;
                if delta_x != 0 || delta_y != 0 {
                    self.push_operation(
                        format!("Move Layer {} ({}, {})", self.active_layer_name(), delta_x, delta_y),
                        EditorOperation::MoveLayer(MoveLayerRecord {
                            layer_id,
                            before_offset: (start_offset_x, start_offset_y),
                            after_offset: (current_x, current_y),
                        }),
                    );
                }
            }
            Some(CanvasInteraction::Marquee {
                before,
                before_inverted,
                start_canvas_x,
                start_canvas_y,
            }) => {
                if let Some(selection) = self.document.selection() {
                    self.push_operation(
                        "Rectangular Selection",
                        EditorOperation::Selection(RectangularSelectionRecord {
                            before,
                            before_inverted,
                            after: Some(selection),
                            after_inverted: self.document.selection_inverted(),
                        }),
                    );
                } else if before.is_some() {
                    let _ = RectangularMarqueeTool::apply_selection(
                        &mut self.document,
                        start_canvas_x,
                        start_canvas_y,
                        start_canvas_x,
                        start_canvas_y,
                    );
                }
            }
            Some(CanvasInteraction::Brush { mode, aggregate, .. }) => {
                if let Some(record) = aggregate {
                    let label = match mode {
                        BrushToolMode::Paint => "Brush Stroke",
                        BrushToolMode::Erase => "Erase Stroke",
                    };
                    self.push_operation(label, EditorOperation::BrushStroke(record));
                }
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PhotoTuxController;
    use common::CanvasRect;
    use ui_shell::{ShellController, ShellToolKind};

    #[test]
    fn layer_actions_update_snapshot() {
        let mut controller = PhotoTuxController::new();
        let initial_count = controller.snapshot().layers.len();

        controller.add_layer();
        controller.duplicate_active_layer();

        let snapshot = controller.snapshot();
        assert!(snapshot.layers.len() >= initial_count + 2);
        assert!(snapshot.history_entries.iter().any(|entry| entry.contains("Add Layer")));
        assert!(snapshot
            .history_entries
            .iter()
            .any(|entry| entry.contains("Duplicate Layer")));
    }

    #[test]
    fn color_actions_update_snapshot() {
        let mut controller = PhotoTuxController::new();
        let before = controller.snapshot();

        controller.swap_colors();
        let swapped = controller.snapshot();
        assert_eq!(swapped.foreground_color, before.background_color);
        assert_eq!(swapped.background_color, before.foreground_color);

        controller.reset_colors();
        let reset = controller.snapshot();
        assert_eq!(reset.foreground_color, [232, 236, 243, 255]);
        assert_eq!(reset.background_color, [27, 29, 33, 255]);
    }

    #[test]
    fn move_interaction_updates_active_layer_bounds() {
        let mut controller = PhotoTuxController::new();
        let before = controller
            .snapshot()
            .active_layer_bounds
            .map(|rect| (rect.x, rect.y))
            .expect("active layer should have bounds");
        controller.select_tool(ShellToolKind::Move);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(30, 15);
        controller.end_canvas_interaction();

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.active_layer_bounds.map(|rect| (rect.x, rect.y)),
            Some((before.0 + 30, before.1 + 15))
        );
        assert!(snapshot.history_entries.iter().any(|entry| entry.contains("Move Layer")));
    }

    #[test]
    fn marquee_interaction_sets_selection_rect() {
        let mut controller = PhotoTuxController::new();
        controller.select_tool(ShellToolKind::RectangularMarquee);
        controller.begin_canvas_interaction(10, 20);
        controller.update_canvas_interaction(50, 70);
        controller.end_canvas_interaction();

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.selection_rect, Some(CanvasRect::new(10, 20, 40, 50)));
    }

    #[test]
    fn brush_interaction_updates_canvas_and_history() {
        let mut controller = PhotoTuxController::new();
        let before = controller.canvas_raster();

        controller.select_tool(ShellToolKind::Brush);
        controller.begin_canvas_interaction(120, 120);
        controller.update_canvas_interaction(144, 132);
        controller.end_canvas_interaction();

        let after = controller.canvas_raster();
        assert_ne!(before.pixels, after.pixels);
        assert!(controller
            .snapshot()
            .history_entries
            .iter()
            .any(|entry| entry.contains("Brush Stroke")));
    }

    #[test]
    fn undo_redo_restores_brush_move_and_selection_state() {
        let mut controller = PhotoTuxController::new();
        let original_canvas = controller.canvas_raster();

        controller.select_tool(ShellToolKind::Brush);
        controller.begin_canvas_interaction(120, 120);
        controller.update_canvas_interaction(144, 132);
        controller.end_canvas_interaction();
        let painted_canvas = controller.canvas_raster();
        let painted_bounds = controller.snapshot().active_layer_bounds;
        assert_ne!(painted_canvas.pixels, original_canvas.pixels);

        controller.undo();
        assert_eq!(controller.canvas_raster().pixels, original_canvas.pixels);
        controller.redo();
        assert_eq!(controller.canvas_raster().pixels, painted_canvas.pixels);

        controller.select_tool(ShellToolKind::Move);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(25, 10);
        controller.end_canvas_interaction();
        let moved_bounds = controller.snapshot().active_layer_bounds;
        assert_ne!(moved_bounds, painted_bounds);

        controller.undo();
        assert_eq!(controller.snapshot().active_layer_bounds, painted_bounds);
        controller.redo();
        assert_eq!(controller.snapshot().active_layer_bounds, moved_bounds);

        controller.select_tool(ShellToolKind::RectangularMarquee);
        controller.begin_canvas_interaction(10, 10);
        controller.update_canvas_interaction(30, 40);
        controller.end_canvas_interaction();
        assert_eq!(controller.snapshot().selection_rect, Some(CanvasRect::new(10, 10, 20, 30)));

        controller.undo();
        assert_eq!(controller.snapshot().selection_rect, None);
        controller.redo();
        assert_eq!(controller.snapshot().selection_rect, Some(CanvasRect::new(10, 10, 20, 30)));
    }
}