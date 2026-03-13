//! Document-session orchestration for the current app shell slice.

use common::{DocumentId, LayerId, Point, Size, Vector};
use doc_model::{BlendMode, Canvas, Document, DocumentMetadata, RasterLayer, RasterSurface};
use file_io::{export_surface_as_png, load_single_surface_document, save_single_surface_document};
use history_engine::{HistoryStack, PixelChange, StrokeHistoryEntry};
use render_wgpu::{ViewportSize, ViewportState};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;
use tool_system::{BrushSettings, BrushTool, MoveTool, StrokeCollector, StrokeSample};
use tracing::info;
use ui_shell::{UiShellDelegate, UiShellState};

const PREVIEW_MAX_WIDTH: u32 = 960;
const CHECKER_LIGHT: [u8; 4] = [0x50, 0x55, 0x5F, 0xFF];
const CHECKER_DARK: [u8; 4] = [0x3E, 0x43, 0x4C, 0xFF];
const CHECKER_CELL_SIZE: u32 = 16;

const SAVE_DIR_NAME: &str = "phototux-session";
const SAVE_FILE_NAME: &str = "session.ptx";
const EXPORT_FILE_NAME: &str = "visible.png";
const DEFAULT_FOREGROUND_COLOR: [u8; 4] = [79, 140, 255, 255];
const DEFAULT_BACKGROUND_COLOR: [u8; 4] = [0x10, 0x12, 0x16, 0xFF];
const INTERACTIVE_PREVIEW_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Errors raised while running the app session.
#[derive(Debug, Error)]
pub enum SessionError {
    /// Save, load, or export work failed.
    #[error(transparent)]
    FileIo(#[from] file_io::FileIoError),
}

/// Delegate-backed app session driving the current shell slice.
#[derive(Debug)]
pub struct AppSession {
    document: Document,
    surface: RasterSurface,
    history: HistoryStack,
    document_dirty: bool,
    recent_actions: Vec<String>,
    active_tool: ShellTool,
    brush_tool: BrushTool,
    eraser_tool: BrushTool,
    background_color: [u8; 4],
    workspace_dir: PathBuf,
    viewport: ViewportState,
    preview_cache: RefCell<Option<PreviewCache>>,
    last_interactive_preview_at: Cell<Option<Instant>>,
    last_cursor_position: Option<(u32, u32)>,
    canvas_interaction: Option<CanvasInteraction>,
}

#[derive(Clone, Debug)]
struct PreviewCache {
    width: u32,
    height: u32,
    scale_factor: f32,
    zoom: f32,
    pan: Vector,
    pixel_buffer: slint::SharedPixelBuffer<slint::Rgba8Pixel>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShellTool {
    Move,
    Marq,
    Lasso,
    Select,
    Crop,
    Dropper,
    Brush,
    Clone,
    History,
    Eras,
    Gradient,
    Blur,
    Dodge,
    Pen,
    Type,
    Path,
    Shape,
    Hand,
    Zoom,
}

#[derive(Debug)]
enum CanvasInteraction {
    Stroke(LiveStroke),
    Move { start: Point },
    Marquee { start: Point },
    Crop { start: Point },
    Hand { last_screen: Point },
}

#[derive(Debug, Default)]
struct LiveStroke {
    collector: StrokeCollector,
    changes_by_pixel: HashMap<(u32, u32), PixelChange>,
    dab_count: usize,
}

impl AppSession {
    /// Create a new document session.
    #[must_use]
    pub fn new() -> Self {
        let (document, surface) = new_blank_document_state("untitled.ptx");
        let viewport = default_viewport_state(&surface);

        Self {
            document,
            surface,
            history: HistoryStack::new(),
            document_dirty: false,
            recent_actions: vec!["Session ready".to_string()],
            active_tool: ShellTool::Brush,
            brush_tool: BrushTool::brush(BrushSettings::default(), DEFAULT_FOREGROUND_COLOR),
            eraser_tool: BrushTool::eraser(BrushSettings::default()),
            background_color: DEFAULT_BACKGROUND_COLOR,
            workspace_dir: std::env::temp_dir().join(SAVE_DIR_NAME),
            viewport,
            preview_cache: RefCell::new(None),
            last_interactive_preview_at: Cell::new(None),
            last_cursor_position: None,
            canvas_interaction: None,
        }
    }

    fn project_path(&self) -> PathBuf {
        self.workspace_dir.join(SAVE_FILE_NAME)
    }

    fn export_path(&self) -> PathBuf {
        self.workspace_dir.join(EXPORT_FILE_NAME)
    }

    fn active_stroke_tool(&self) -> Option<BrushTool> {
        match self.active_tool {
            ShellTool::Brush => Some(self.brush_tool),
            ShellTool::Eras => Some(self.eraser_tool),
            ShellTool::Move
            | ShellTool::Marq
            | ShellTool::Lasso
            | ShellTool::Select
            | ShellTool::Crop
            | ShellTool::Dropper
            | ShellTool::Clone
            | ShellTool::History
            | ShellTool::Gradient
            | ShellTool::Blur
            | ShellTool::Dodge
            | ShellTool::Pen
            | ShellTool::Type
            | ShellTool::Path
            | ShellTool::Shape
            | ShellTool::Hand
            | ShellTool::Zoom => None,
        }
    }

    fn active_stroke_tool_mut(&mut self) -> Option<&mut BrushTool> {
        match self.active_tool {
            ShellTool::Brush => Some(&mut self.brush_tool),
            ShellTool::Eras => Some(&mut self.eraser_tool),
            ShellTool::Move
            | ShellTool::Marq
            | ShellTool::Lasso
            | ShellTool::Select
            | ShellTool::Crop
            | ShellTool::Dropper
            | ShellTool::Clone
            | ShellTool::History
            | ShellTool::Gradient
            | ShellTool::Blur
            | ShellTool::Dodge
            | ShellTool::Pen
            | ShellTool::Type
            | ShellTool::Path
            | ShellTool::Shape
            | ShellTool::Hand
            | ShellTool::Zoom => None,
        }
    }

    fn active_tool_label(&self) -> String {
        match self.active_tool {
            ShellTool::Move => "Move Tool".to_string(),
            ShellTool::Marq => "Marquee Tool".to_string(),
            ShellTool::Lasso => "Lasso Tool".to_string(),
            ShellTool::Select => "Quick Selection Tool".to_string(),
            ShellTool::Crop => "Crop Tool".to_string(),
            ShellTool::Dropper => "Eyedropper Tool".to_string(),
            ShellTool::Brush => "Brush Tool".to_string(),
            ShellTool::Clone => "Clone Stamp Tool".to_string(),
            ShellTool::History => "History Brush Tool".to_string(),
            ShellTool::Eras => "Eraser Tool".to_string(),
            ShellTool::Gradient => "Gradient Tool".to_string(),
            ShellTool::Blur => "Blur Tool".to_string(),
            ShellTool::Dodge => "Dodge Tool".to_string(),
            ShellTool::Pen => "Pen Tool".to_string(),
            ShellTool::Type => "Type Tool".to_string(),
            ShellTool::Path => "Path Selection Tool".to_string(),
            ShellTool::Shape => "Shape Tool".to_string(),
            ShellTool::Hand => "Hand Tool".to_string(),
            ShellTool::Zoom => "Zoom Tool".to_string(),
        }
    }

    fn active_tool_key(&self) -> &'static str {
        match self.active_tool {
            ShellTool::Move => "Move",
            ShellTool::Marq => "Marq",
            ShellTool::Lasso => "Lasso",
            ShellTool::Select => "Select",
            ShellTool::Crop => "Crop",
            ShellTool::Dropper => "Dropper",
            ShellTool::Brush => "Brush",
            ShellTool::Clone => "Clone",
            ShellTool::History => "History",
            ShellTool::Eras => "Eras",
            ShellTool::Gradient => "Gradient",
            ShellTool::Blur => "Blur",
            ShellTool::Dodge => "Dodge",
            ShellTool::Pen => "Pen",
            ShellTool::Type => "Type",
            ShellTool::Path => "Path",
            ShellTool::Shape => "Shape",
            ShellTool::Hand => "Hand",
            ShellTool::Zoom => "Zoom",
        }
    }

    fn tool_preset_label(&self) -> &'static str {
        match self.active_tool {
            ShellTool::Move => "Layer Move",
            ShellTool::Marq => "Rectangular",
            ShellTool::Lasso => "Polygonal",
            ShellTool::Select => "Contiguous",
            ShellTool::Crop => "Classic Crop",
            ShellTool::Dropper => "Point Sample",
            ShellTool::Brush => "Round Brush",
            ShellTool::Clone => "Aligned Stamp",
            ShellTool::History => "Source Snapshot",
            ShellTool::Eras => "Round Eraser",
            ShellTool::Gradient => "Linear Gradient",
            ShellTool::Blur => "Soften",
            ShellTool::Dodge => "Midtones",
            ShellTool::Pen => "Path Mode",
            ShellTool::Type => "Horizontal Type",
            ShellTool::Path => "Layer Path",
            ShellTool::Shape => "Rectangle",
            ShellTool::Hand => "Viewport Pan",
            ShellTool::Zoom => "Incremental",
        }
    }

    fn tool_option_rows(&self) -> [(String, String); 4] {
        match self.active_tool {
            ShellTool::Move => [
                ("Auto-select".to_string(), "On".to_string()),
                ("Transform".to_string(), "On".to_string()),
                ("Snap".to_string(), "On".to_string()),
                ("Align".to_string(), "Later".to_string()),
            ],
            ShellTool::Marq => [
                ("Feather".to_string(), "0 px".to_string()),
                ("Anti-alias".to_string(), "On".to_string()),
                ("Mode".to_string(), "Replace".to_string()),
                ("Ratio".to_string(), "Free".to_string()),
            ],
            ShellTool::Lasso => [
                ("Feather".to_string(), "0 px".to_string()),
                ("Anti-alias".to_string(), "On".to_string()),
                ("Mode".to_string(), "Add".to_string()),
                ("Edge".to_string(), "Normal".to_string()),
            ],
            ShellTool::Select => [
                ("Sample All".to_string(), "Off".to_string()),
                ("Tolerance".to_string(), "32".to_string()),
                ("Contiguous".to_string(), "On".to_string()),
                ("Subject".to_string(), "Later".to_string()),
            ],
            ShellTool::Crop => [
                ("Ratio".to_string(), "Original".to_string()),
                ("Delete Crop".to_string(), "Off".to_string()),
                ("Straighten".to_string(), "Ready".to_string()),
                ("Overlay".to_string(), "Rule of Thirds".to_string()),
            ],
            ShellTool::Dropper => [
                ("Sample".to_string(), "Point".to_string()),
                ("Readout".to_string(), "RGB".to_string()),
                ("Layer".to_string(), "Current".to_string()),
                ("Info".to_string(), "Panel".to_string()),
            ],
            ShellTool::Brush | ShellTool::Eras => {
                let settings = self
                    .active_stroke_tool()
                    .expect("paint tools must expose brush settings")
                    .settings;

                [
                    ("Size".to_string(), format!("{:.0} px", settings.size)),
                    (
                        "Hardness".to_string(),
                        format!("{:.0}%", settings.hardness * 100.0),
                    ),
                    (
                        "Opacity".to_string(),
                        format!("{:.0}%", settings.opacity * 100.0),
                    ),
                    ("Flow".to_string(), format!("{:.0}%", settings.flow * 100.0)),
                ]
            }
            ShellTool::Clone => [
                ("Sample".to_string(), "Current".to_string()),
                ("Aligned".to_string(), "On".to_string()),
                ("Opacity".to_string(), "100%".to_string()),
                ("Flow".to_string(), "100%".to_string()),
            ],
            ShellTool::History => [
                ("Mode".to_string(), "Normal".to_string()),
                ("Source".to_string(), "Current".to_string()),
                ("Opacity".to_string(), "100%".to_string()),
                ("Area".to_string(), "Tight".to_string()),
            ],
            ShellTool::Gradient => [
                ("Type".to_string(), "Linear".to_string()),
                ("Blend".to_string(), "Normal".to_string()),
                ("Opacity".to_string(), "100%".to_string()),
                ("Reverse".to_string(), "Off".to_string()),
            ],
            ShellTool::Blur => [
                ("Strength".to_string(), "50%".to_string()),
                ("Mode".to_string(), "Blur".to_string()),
                ("Sample All".to_string(), "Off".to_string()),
                ("Protect".to_string(), "Detail".to_string()),
            ],
            ShellTool::Dodge => [
                ("Range".to_string(), "Midtones".to_string()),
                ("Exposure".to_string(), "12%".to_string()),
                ("Protect".to_string(), "On".to_string()),
                ("Brush".to_string(), "Soft".to_string()),
            ],
            ShellTool::Pen => [
                ("Mode".to_string(), "Path".to_string()),
                ("Combine".to_string(), "Add".to_string()),
                ("Auto Add".to_string(), "On".to_string()),
                ("Rubber".to_string(), "Off".to_string()),
            ],
            ShellTool::Type => [
                ("Font".to_string(), "Plex Sans".to_string()),
                ("Size".to_string(), "24 pt".to_string()),
                ("AA".to_string(), "Sharp".to_string()),
                ("Align".to_string(), "Left".to_string()),
            ],
            ShellTool::Path => [
                ("Mode".to_string(), "Layer".to_string()),
                ("Transform".to_string(), "Ready".to_string()),
                ("Arrange".to_string(), "Front".to_string()),
                ("Selection".to_string(), "Path".to_string()),
            ],
            ShellTool::Shape => [
                ("Fill".to_string(), self.foreground_color_label()),
                ("Stroke".to_string(), "0 px".to_string()),
                ("Radius".to_string(), "0 px".to_string()),
                ("Combine".to_string(), "Add".to_string()),
            ],
            ShellTool::Hand => [
                ("Mode".to_string(), "Canvas".to_string()),
                ("Drag".to_string(), "Hold Space".to_string()),
                ("Fit".to_string(), "Double-click".to_string()),
                ("Cursor".to_string(), "Open Hand".to_string()),
            ],
            ShellTool::Zoom => [
                ("Step".to_string(), "100%".to_string()),
                ("Target".to_string(), "Cursor".to_string()),
                ("Smooth".to_string(), "Off".to_string()),
                ("Range".to_string(), "25-3200%".to_string()),
            ],
        }
    }

    fn foreground_color_label(&self) -> String {
        rgba_hex_label(self.brush_tool.color)
    }

    fn active_layer_id(&self) -> Option<LayerId> {
        self.document.active_layer().map(|layer| layer.id)
    }

    fn canvas_point(&mut self, normalized_x: f32, normalized_y: f32) -> (u32, u32, Point) {
        let screen_point = self.preview_screen_point(normalized_x, normalized_y);
        let document_point = self.viewport.screen_to_document(screen_point);
        let x = document_point
            .x
            .round()
            .clamp(0.0, self.document.canvas.width.saturating_sub(1) as f32) as u32;
        let y = document_point
            .y
            .round()
            .clamp(0.0, self.document.canvas.height.saturating_sub(1) as f32) as u32;

        self.last_cursor_position = Some((x, y));
        (x, y, Point::new(x as f32, y as f32))
    }

    fn preview_screen_point(&self, normalized_x: f32, normalized_y: f32) -> Point {
        Point::new(
            normalized_x.clamp(0.0, 1.0) * self.viewport.size.logical_width as f32,
            normalized_y.clamp(0.0, 1.0) * self.viewport.size.logical_height as f32,
        )
    }

    fn update_viewport_for_surface(&mut self) {
        self.viewport = default_viewport_state(&self.surface);
        self.invalidate_preview_cache();
    }

    fn zoom_label(&self) -> String {
        format!("{:.0}%", self.viewport.zoom * 100.0)
    }

    fn zoom_view_at(&mut self, screen_point: Point, zoom_in: bool) {
        let document_before = self.viewport.screen_to_document(screen_point);

        if zoom_in {
            self.viewport.zoom_in();
        } else {
            self.viewport.zoom_out();
        }

        let scale_factor = self.viewport.size.scale_factor.max(1.0);
        self.viewport.pan.dx = (screen_point.x / scale_factor) - document_before.x * self.viewport.zoom;
        self.viewport.pan.dy = (screen_point.y / scale_factor) - document_before.y * self.viewport.zoom;
        self.invalidate_preview_cache();
    }

    fn invalidate_preview_cache(&self) {
        *self.preview_cache.borrow_mut() = None;
        self.last_interactive_preview_at.set(None);
    }

    fn should_publish_interactive_preview(&self) -> bool {
        self.last_interactive_preview_at
            .get()
            .is_none_or(|instant| instant.elapsed() >= INTERACTIVE_PREVIEW_FRAME_INTERVAL)
    }

    fn update_preview_for_changes(&self, changes: &[PixelChange]) {
        let Some((min_x, min_y, max_x, max_y)) = preview_document_bounds(changes) else {
            return;
        };

        let mut preview_cache = self.preview_cache.borrow_mut();
        if preview_cache
            .as_ref()
            .is_none_or(|cache| !preview_cache_matches(cache, &self.viewport))
        {
            *preview_cache = Some(build_preview_cache(&self.surface, &self.viewport));
        }

        if let Some(cache) = preview_cache.as_mut() {
            repaint_preview_document_region(
                cache,
                &self.surface,
                &self.viewport,
                min_x,
                min_y,
                max_x,
                max_y,
            );
        }
    }

    fn cached_canvas_preview(&self) -> slint::Image {
        let mut preview_cache = self.preview_cache.borrow_mut();
        if preview_cache
            .as_ref()
            .is_none_or(|cache| !preview_cache_matches(cache, &self.viewport))
        {
            *preview_cache = Some(build_preview_cache(&self.surface, &self.viewport));
        }

        slint::Image::from_rgba8(
            preview_cache
                .as_ref()
                .expect("preview cache should exist")
                .pixel_buffer
                .clone(),
        )
    }

    fn interactive_drag_shell_state(
        &self,
        current_state: &UiShellState,
        status_message: String,
        cursor_x: u32,
        cursor_y: u32,
        preview_published: bool,
    ) -> UiShellState {
        if preview_published {
            return self.shell_state(status_message);
        }

        UiShellState {
            document_dirty: self.document_dirty,
            status_message,
            cursor_position_label: format!("Cursor {}, {}", cursor_x, cursor_y),
            canvas_preview: current_state.canvas_preview.clone(),
            ..current_state.clone()
        }
    }

    fn shell_state(&self, status_message: String) -> UiShellState {
        let active_layer = self.document.active_layer();
        let option_rows = self.tool_option_rows();
        let selection_bounds_label = self
            .document
            .selection_mask()
            .bounds()
            .map(|bounds| format!("Sel {}x{}", bounds.width, bounds.height))
            .unwrap_or_else(|| "No Selection".to_string());
        let history_entries = self.history_entries();

        UiShellState {
            document_title: self.document.metadata.title.clone(),
            document_dirty: self.document_dirty,
            active_tool_label: self.active_tool_label(),
            status_message,
            active_tool_key: self.active_tool_key().to_string(),
            tool_preset_label: self.tool_preset_label().to_string(),
            tool_option_primary_label: option_rows[0].0.clone(),
            tool_option_secondary_label: option_rows[1].0.clone(),
            tool_option_tertiary_label: option_rows[2].0.clone(),
            tool_option_quaternary_label: option_rows[3].0.clone(),
            tool_size_label: option_rows[0].1.clone(),
            tool_hardness_label: option_rows[1].1.clone(),
            tool_opacity_label: option_rows[2].1.clone(),
            tool_flow_label: option_rows[3].1.clone(),
            zoom_label: self.zoom_label(),
            canvas_size_label: format!(
                "{}x{} px",
                self.document.canvas.width, self.document.canvas.height
            ),
            canvas_width_px: self.document.canvas.width as i32,
            canvas_height_px: self.document.canvas.height as i32,
            cursor_position_label: self
                .last_cursor_position
                .map(|(x, y)| format!("Cursor {}, {}", x, y))
                .unwrap_or_else(|| "Cursor -, -".to_string()),
            selection_bounds_label,
            foreground_color_label: self.foreground_color_label(),
            background_color_label: rgba_hex_label(self.background_color),
            active_layer_name: active_layer
                .map(|layer| layer.name.clone())
                .unwrap_or_else(|| "No Active Layer".to_string()),
            layer_blend_mode_label: active_layer
                .map(|layer| blend_mode_label(layer.blend_mode).to_string())
                .unwrap_or_else(|| "Normal".to_string()),
            layer_opacity_label: active_layer
                .map(|layer| format!("{:.0}%", layer.opacity * 100.0))
                .unwrap_or_else(|| "100%".to_string()),
            layer_visibility_label: active_layer
                .map(|layer| if layer.visible { "Visible" } else { "Hidden" }.to_string())
                .unwrap_or_else(|| "Hidden".to_string()),
            layer_count_label: format!(
                "{} {}",
                self.document.layers().len(),
                if self.document.layers().len() == 1 {
                    "Layer"
                } else {
                    "Layers"
                }
            ),
            history_entry_primary: history_entries[0].clone(),
            history_entry_secondary: history_entries[1].clone(),
            history_entry_tertiary: history_entries[2].clone(),
            history_entry_quaternary: history_entries[3].clone(),
            history_entry_quinary: history_entries[4].clone(),
            canvas_preview: self.cached_canvas_preview(),
        }
    }

    fn history_entries(&self) -> [String; 5] {
        let mut entries = [
            "No actions yet".to_string(),
            "Brush strokes and edits appear here".to_string(),
            "Undo and redo update this list".to_string(),
            "Save and export remain visible in status".to_string(),
            "Keyboard shortcuts stay available".to_string(),
        ];

        for (index, action) in self.recent_actions.iter().rev().take(5).enumerate() {
            entries[index] = action.clone();
        }

        entries
    }

    fn push_recent_action(&mut self, label: impl Into<String>) {
        self.recent_actions.push(label.into());
        if self.recent_actions.len() > 5 {
            let excess = self.recent_actions.len() - 5;
            self.recent_actions.drain(0..excess);
        }
    }

    fn adjust_active_tool_option(&mut self, option: &str, delta: i32) -> UiShellState {
        let active_tool_label = self.active_tool_label();
        let delta = delta as f32;
        let Some(tool) = self.active_stroke_tool_mut() else {
            return self.shell_state(format!(
                "{} options are fixed in this shell pass",
                active_tool_label
            ));
        };

        let status_message = match option {
            "size" => {
                tool.settings.size = (tool.settings.size + delta * 4.0).clamp(1.0, 256.0);
                format!(
                    "{} size set to {:.0} px",
                    active_tool_label, tool.settings.size
                )
            }
            "hardness" => {
                tool.settings.hardness = (tool.settings.hardness + delta * 0.05).clamp(0.0, 1.0);
                format!(
                    "{} hardness set to {:.0}%",
                    active_tool_label,
                    tool.settings.hardness * 100.0
                )
            }
            "opacity" => {
                tool.settings.opacity = (tool.settings.opacity + delta * 0.05).clamp(0.0, 1.0);
                format!(
                    "{} opacity set to {:.0}%",
                    active_tool_label,
                    tool.settings.opacity * 100.0
                )
            }
            "flow" => {
                tool.settings.flow = (tool.settings.flow + delta * 0.05).clamp(0.0, 1.0);
                format!(
                    "{} flow set to {:.0}%",
                    active_tool_label,
                    tool.settings.flow * 100.0
                )
            }
            _ => format!("{} option is not wired yet", option),
        };

        self.shell_state(status_message)
    }

    fn reset_active_tool_options(&mut self) -> UiShellState {
        let active_tool_label = self.active_tool_label();
        let Some(tool) = self.active_stroke_tool_mut() else {
            return self.shell_state(format!(
                "{} has no adjustable options yet",
                active_tool_label
            ));
        };

        let default_settings = BrushSettings::default();
        tool.settings = default_settings;

        self.shell_state(format!(
            "reset {} settings",
            self.active_tool_label().to_lowercase()
        ))
    }

    fn apply_demo_stroke(&mut self) -> UiShellState {
        let started_at = Instant::now();
        let Some(tool) = self.active_stroke_tool() else {
            return self.shell_state(format!(
                "{} demo actions are not wired yet",
                self.active_tool_label()
            ));
        };

        let center_y = if self.active_tool == ShellTool::Brush {
            420.0
        } else {
            460.0
        };
        let stroke = tool.apply_stroke(
            &mut self.surface,
            &[
                StrokeSample::new(Point::new(320.0, center_y)),
                StrokeSample::new(Point::new(960.0, center_y)),
                StrokeSample::new(Point::new(1440.0, center_y + 48.0)),
            ],
        );
        self.update_preview_for_changes(stroke.history_entry.changes());

        self.history.push_brush_stroke(stroke.history_entry);
        self.document_dirty = true;
        self.push_recent_action(format!("{} stroke", self.active_tool_label()));
        let elapsed_ms = elapsed_milliseconds(started_at);
        log_timing(
            "brush_commit",
            started_at,
            &[
                ("tool", self.active_tool_label()),
                ("dab_count", stroke.dab_count.to_string()),
                (
                    "canvas",
                    format!("{}x{}", self.surface.width(), self.surface.height()),
                ),
            ],
        );

        self.shell_state(format!(
            "applied {} sample stroke with {} dabs in {:.2} ms",
            self.active_tool_label().to_lowercase(),
            stroke.dab_count,
            elapsed_ms,
        ))
    }

    fn save_project(&mut self) -> Result<UiShellState, SessionError> {
        let started_at = Instant::now();
        let project_path = self.project_path();
        save_single_surface_document(&project_path, &self.document, &self.surface)?;
        self.document_dirty = false;
        self.push_recent_action("Saved document");
        let elapsed_ms = elapsed_milliseconds(started_at);
        log_timing(
            "save_project",
            started_at,
            &[("path", project_path.display().to_string())],
        );

        let mut state = self.shell_state(format!(
            "saved document to {} in {:.2} ms",
            project_path.display(),
            elapsed_ms
        ));
        state.document_title = project_file_name(&project_path);

        Ok(state)
    }

    fn load_project(&mut self) -> Result<UiShellState, SessionError> {
        let started_at = Instant::now();
        let project_path = self.project_path();
        let (document, surface) = load_single_surface_document(&project_path)?;
        self.document = document;
        self.surface = surface;
        self.update_viewport_for_surface();
        self.history = HistoryStack::new();
        self.document_dirty = false;
        self.recent_actions.clear();
        self.push_recent_action("Reopened document");
        let elapsed_ms = elapsed_milliseconds(started_at);
        log_timing(
            "load_project",
            started_at,
            &[("path", project_path.display().to_string())],
        );

        let mut state = self.shell_state(format!(
            "reopened document from {} in {:.2} ms",
            project_path.display(),
            elapsed_ms
        ));
        state.document_title = project_file_name(&project_path);

        Ok(state)
    }

    fn export_png(&mut self) -> Result<UiShellState, SessionError> {
        let started_at = Instant::now();
        let export_path = self.export_path();
        export_surface_as_png(&export_path, &self.surface)?;
        self.push_recent_action("Exported PNG");
        let elapsed_ms = elapsed_milliseconds(started_at);
        log_timing(
            "export_png",
            started_at,
            &[("path", export_path.display().to_string())],
        );

        Ok(self.shell_state(format!(
            "exported visible canvas to {} in {:.2} ms",
            export_path.display(),
            elapsed_ms
        )))
    }

    fn new_document(&mut self) -> UiShellState {
        let (document, surface) = new_blank_document_state("untitled.ptx");
        self.document = document;
        self.surface = surface;
        self.update_viewport_for_surface();
        self.history = HistoryStack::new();
        self.document_dirty = false;
        self.recent_actions.clear();
        self.push_recent_action("Created new document");

        self.shell_state("opened a fresh untitled canvas".to_string())
    }

    fn close_document(&mut self) -> UiShellState {
        let (document, surface) = new_blank_document_state("untitled.ptx");
        self.document = document;
        self.surface = surface;
        self.update_viewport_for_surface();
        self.history = HistoryStack::new();
        self.document_dirty = false;
        self.recent_actions.clear();
        self.push_recent_action("Closed active document");

        self.shell_state("closed the active document and opened a clean canvas".to_string())
    }

    fn delete_active_layer(&mut self) -> UiShellState {
        if self.document.layers().len() <= 1 {
            return self.shell_state("cannot delete the last remaining layer".to_string());
        }

        let Some(active_layer_id) = self.document.active_layer().map(|layer| layer.id) else {
            return self.shell_state("no active layer is available".to_string());
        };

        let Some(removed_layer) = self.document.delete_layer(active_layer_id) else {
            return self.shell_state("failed to delete the active layer".to_string());
        };

        self.document_dirty = true;
        self.push_recent_action(format!("Deleted layer {}", removed_layer.name));

        self.shell_state(format!("deleted layer {}", removed_layer.name))
    }

    fn swap_colors(&mut self) -> UiShellState {
        std::mem::swap(&mut self.brush_tool.color, &mut self.background_color);
        self.push_recent_action("Swapped foreground and background colors");

        self.shell_state("swapped foreground and background colors".to_string())
    }

    fn reset_colors(&mut self) -> UiShellState {
        self.brush_tool.color = DEFAULT_FOREGROUND_COLOR;
        self.background_color = DEFAULT_BACKGROUND_COLOR;
        self.push_recent_action("Reset colors to default swatches");

        self.shell_state("restored the default foreground and background colors".to_string())
    }

    fn undo(&mut self) -> UiShellState {
        let started_at = Instant::now();
        let status_message = if self.history.undo(&mut self.surface) {
            self.document_dirty = true;
            self.invalidate_preview_cache();
            self.push_recent_action("Undo");
            "undid last stroke".to_string()
        } else {
            "nothing to undo".to_string()
        };
        let elapsed_ms = elapsed_milliseconds(started_at);
        log_timing("undo", started_at, &[("result", status_message.clone())]);

        self.shell_state(format!("{} in {:.2} ms", status_message, elapsed_ms))
    }

    fn redo(&mut self) -> UiShellState {
        let started_at = Instant::now();
        let status_message = if self.history.redo(&mut self.surface) {
            self.document_dirty = true;
            self.invalidate_preview_cache();
            self.push_recent_action("Redo");
            "redid last stroke".to_string()
        } else {
            "nothing to redo".to_string()
        };
        let elapsed_ms = elapsed_milliseconds(started_at);
        log_timing("redo", started_at, &[("result", status_message.clone())]);

        self.shell_state(format!("{} in {:.2} ms", status_message, elapsed_ms))
    }

    fn select_tool(&mut self, tool: &str) -> UiShellState {
        self.canvas_interaction = None;
        self.active_tool = match tool {
            "Move" => ShellTool::Move,
            "Marq" => ShellTool::Marq,
            "Lasso" => ShellTool::Lasso,
            "Select" => ShellTool::Select,
            "Crop" => ShellTool::Crop,
            "Dropper" => ShellTool::Dropper,
            "Brush" => ShellTool::Brush,
            "Clone" => ShellTool::Clone,
            "History" => ShellTool::History,
            "Eras" => ShellTool::Eras,
            "Gradient" => ShellTool::Gradient,
            "Blur" => ShellTool::Blur,
            "Dodge" => ShellTool::Dodge,
            "Pen" => ShellTool::Pen,
            "Type" => ShellTool::Type,
            "Path" => ShellTool::Path,
            "Shape" => ShellTool::Shape,
            "Hand" => ShellTool::Hand,
            "Zoom" => ShellTool::Zoom,
            _ => self.active_tool,
        };

        self.shell_state(format!(
            "selected {}",
            self.active_tool_label().to_lowercase()
        ))
    }

    fn select_all(&mut self) -> UiShellState {
        self.document.select_all();
        self.push_recent_action("Selected full canvas");
        self.shell_state("selected the full canvas".to_string())
    }

    fn clear_selection(&mut self) -> UiShellState {
        self.document.clear_selection();
        self.push_recent_action("Cleared selection");
        self.shell_state("cleared the current selection".to_string())
    }

    fn invert_selection(&mut self) -> UiShellState {
        self.document.invert_selection();
        self.push_recent_action("Inverted selection");
        self.shell_state("inverted the current selection".to_string())
    }

    fn zoom_in(&mut self) -> UiShellState {
        let center = Point::new(
            self.viewport.size.logical_width as f32 / 2.0,
            self.viewport.size.logical_height as f32 / 2.0,
        );
        self.zoom_view_at(center, true);
        self.push_recent_action("Zoomed in");
        self.shell_state(format!("zoomed to {}", self.zoom_label()))
    }

    fn zoom_out(&mut self) -> UiShellState {
        let center = Point::new(
            self.viewport.size.logical_width as f32 / 2.0,
            self.viewport.size.logical_height as f32 / 2.0,
        );
        self.zoom_view_at(center, false);
        self.push_recent_action("Zoomed out");
        self.shell_state(format!("zoomed to {}", self.zoom_label()))
    }

    fn reset_view(&mut self) -> UiShellState {
        self.update_viewport_for_surface();
        self.push_recent_action("Reset viewport");
        self.shell_state("reset the viewport to fit the canvas".to_string())
    }

    fn apply_crop(&mut self, start: Point, end: Point) -> UiShellState {
        self.document
            .select_rect(start.x as u32, start.y as u32, end.x as u32, end.y as u32);
        let Some(bounds) = self.document.selection_mask().bounds() else {
            return self.shell_state("crop region is empty".to_string());
        };

        let cropped_surface = crop_surface(&self.surface, bounds);
        let layer_template = self
            .document
            .active_layer()
            .cloned()
            .unwrap_or_else(|| RasterLayer::new(LayerId::new(1), "Surface"));
        let mut document = Document::new(
            self.document.id,
            Canvas::new(bounds.width, bounds.height),
            DocumentMetadata {
                title: self.document.metadata.title.clone(),
            },
        );
        document.add_layer(layer_template);

        self.document = document;
        self.surface = cropped_surface;
        self.history = HistoryStack::new();
        self.document_dirty = true;
        self.update_viewport_for_surface();
        self.push_recent_action(format!("Cropped canvas to {}x{}", bounds.width, bounds.height));

        self.shell_state(format!("cropped canvas to {}x{} px", bounds.width, bounds.height))
    }

    fn canvas_pressed(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let (x, y, point) = self.canvas_point(normalized_x, normalized_y);

        match self.active_tool {
            ShellTool::Brush | ShellTool::Eras => {
                let mut live_stroke = LiveStroke::default();
                live_stroke.collector.push_sample(StrokeSample::new(point));
                self.canvas_interaction = Some(CanvasInteraction::Stroke(live_stroke));
                self.shell_state(format!(
                    "started {} at {}, {}",
                    self.active_tool_label().to_lowercase(),
                    x,
                    y,
                ))
            }
            ShellTool::Move => {
                self.canvas_interaction = Some(CanvasInteraction::Move { start: point });
                self.shell_state(format!("move drag started at {}, {}", x, y))
            }
            ShellTool::Marq => {
                self.document.select_rect(x, y, x, y);
                self.canvas_interaction = Some(CanvasInteraction::Marquee { start: point });
                self.shell_state(format!("started marquee selection at {}, {}", x, y))
            }
            ShellTool::Crop => {
                self.document.select_rect(x, y, x, y);
                self.canvas_interaction = Some(CanvasInteraction::Crop { start: point });
                self.shell_state(format!("started crop region at {}, {}", x, y))
            }
            ShellTool::Hand => {
                self.canvas_interaction = Some(CanvasInteraction::Hand {
                    last_screen: self.preview_screen_point(normalized_x, normalized_y),
                });
                self.shell_state(format!("started panning at {}, {}", x, y))
            }
            ShellTool::Zoom => {
                let screen_point = self.preview_screen_point(normalized_x, normalized_y);
                self.zoom_view_at(screen_point, true);
                self.push_recent_action("Zoomed in");
                self.shell_state(format!("zoomed to {}", self.zoom_label()))
            }
            ShellTool::Dropper => {
                self.brush_tool.color = self.surface.pixel(x, y);
                self.push_recent_action(format!("Sampled {}", self.foreground_color_label()));
                self.shell_state(format!(
                    "sampled {} at {}, {}",
                    self.foreground_color_label(),
                    x,
                    y,
                ))
            }
            _ => self.shell_state(current_state.status_message.clone()),
        }
    }



    fn canvas_dragged_batch(
        &mut self,
        points: &[(f32, f32)],
        current_state: &UiShellState,
    ) -> UiShellState {
        if points.is_empty() {
            return current_state.clone();
        }
        let drag_started_at = Instant::now();

        let mapped_points: Vec<_> = points
            .iter()
            .map(|&(nx, ny)| {
                let (cx, cy, pt) = self.canvas_point(nx, ny);
                let current_screen = self.preview_screen_point(nx, ny);
                (nx, ny, cx, cy, pt, current_screen)
            })
            .collect();

        let (_, _, last_cx, last_cy, _, _) = *mapped_points.last().unwrap();

        let mut preview_changes: Option<Vec<PixelChange>> = None;
        let mut stroke_apply_metrics: Option<(f64, usize, usize)> = None;
        let active_stroke_tool = self.active_stroke_tool();

        let status_message = match self.canvas_interaction.as_mut() {
            Some(CanvasInteraction::Stroke(live_stroke)) => {
                let previous_sample = live_stroke.collector.samples().last().copied();
                let mut stroke_samples = Vec::with_capacity(mapped_points.len() + 1);
                if let Some(prev) = previous_sample {
                    stroke_samples.push(prev);
                }

                for &(_, _, _, _, pt, _) in &mapped_points {
                    let sample = StrokeSample::new(pt);
                    stroke_samples.push(sample);
                    live_stroke.collector.push_sample(sample);
                }

                if stroke_samples.len() > 1 {
                    let stroke_tool = active_stroke_tool
                        .expect("paint tools must expose stroke tools");
                    let apply_started_at = Instant::now();
                    let application =
                        stroke_tool.apply_stroke(&mut self.surface, &stroke_samples);
                    stroke_apply_metrics = Some((
                        elapsed_milliseconds(apply_started_at),
                        application.dab_count,
                        application.history_entry.changes().len(),
                    ));
                    preview_changes = Some(application.history_entry.changes().to_vec());
                    merge_pixel_changes(
                        &mut live_stroke.changes_by_pixel,
                        application.history_entry.changes(),
                    );
                    live_stroke.dab_count += application.dab_count;
                    self.document_dirty = true;
                }

                format!(
                    "dragging {} at {}, {}",
                    self.active_tool_label().to_lowercase(),
                    last_cx,
                    last_cy,
                )
            }
            Some(CanvasInteraction::Move { start }) => {
                let (_, _, _, _, pt, _) = *mapped_points.last().unwrap();
                let move_tool = MoveTool;
                let (delta_x, delta_y) = move_tool.drag_delta(*start, pt);
                return self.shell_state(format!("move delta {} px, {} px", delta_x, delta_y));
            }
            Some(CanvasInteraction::Marquee { start }) => {
                self.document.select_rect(start.x as u32, start.y as u32, last_cx, last_cy);
                return self.shell_state(format!("marquee selection to {}, {}", last_cx, last_cy));
            }
            Some(CanvasInteraction::Crop { start }) => {
                self.document.select_rect(start.x as u32, start.y as u32, last_cx, last_cy);
                return self.shell_state(format!("crop region to {}, {}", last_cx, last_cy));
            }
            Some(CanvasInteraction::Hand { last_screen }) => {
                let (_, _, _, _, _, current_screen) = *mapped_points.last().unwrap();
                self.viewport.pan_by_screen_delta(Vector::new(
                    current_screen.x - last_screen.x,
                    current_screen.y - last_screen.y,
                ));
                *last_screen = current_screen;
                return self.shell_state(format!("panning view at {}, {}", last_cx, last_cy));
            }
            None => return self.shell_state(current_state.status_message.clone()),
        };

        let mut preview_update_ms = None;
        let mut preview_published = false;
        if let Some(changes) = preview_changes {
            if self.should_publish_interactive_preview() {
                let preview_started_at = Instant::now();
                self.update_preview_for_changes(&changes);
                self.last_interactive_preview_at.set(Some(Instant::now()));
                preview_update_ms = Some(elapsed_milliseconds(preview_started_at));
                preview_published = true;
            }
        }

        if let Some((apply_ms, dab_count, changed_pixels)) = stroke_apply_metrics {
            log_elapsed_ms(
                "brush_drag_segment",
                elapsed_milliseconds(drag_started_at),
                &[
                    ("apply_ms", format!("{apply_ms:.2}")),
                    (
                        "preview_ms",
                        format!("{:.2}", preview_update_ms.unwrap_or_default()),
                    ),
                    ("preview_published", preview_published.to_string()),
                    ("dabs", dab_count.to_string()),
                    ("changed_pixels", changed_pixels.to_string()),
                    ("cursor", format!("{last_cx},{last_cy}")),
                    ("batch_size", mapped_points.len().to_string()),
                ],
            );
        }

        self.interactive_drag_shell_state(
            current_state,
            status_message,
            last_cx,
            last_cy,
            preview_published,
        )
    }
    fn canvas_released(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let (x, y, point) = self.canvas_point(normalized_x, normalized_y);
        let Some(interaction) = self.canvas_interaction.take() else {
            return self.shell_state(current_state.status_message.clone());
        };

        match interaction {
            CanvasInteraction::Stroke(mut live_stroke) => {
                if live_stroke.changes_by_pixel.is_empty() {
                    let stroke_tool = self
                        .active_stroke_tool()
                        .expect("paint tools must expose stroke tools");
                    let samples = live_stroke.collector.finish();
                    let application = stroke_tool.apply_stroke(&mut self.surface, &samples);
                    merge_pixel_changes(
                        &mut live_stroke.changes_by_pixel,
                        application.history_entry.changes(),
                    );
                    live_stroke.dab_count += application.dab_count;
                }

                let history_entry = StrokeHistoryEntry::new(
                    live_stroke.changes_by_pixel.into_values().collect(),
                );

                if history_entry.is_empty() {
                    return self.shell_state(current_state.status_message.clone());
                }

                self.update_preview_for_changes(history_entry.changes());
                self.last_interactive_preview_at.set(None);
                self.history.push_brush_stroke(history_entry);
                self.document_dirty = true;
                self.push_recent_action(format!("{} stroke", self.active_tool_label()));

                self.shell_state(format!(
                    "applied {} with {} dabs",
                    self.active_tool_label().to_lowercase(),
                    live_stroke.dab_count.max(1),
                ))
            }
            CanvasInteraction::Move { start } => {
                let Some(layer_id) = self.active_layer_id() else {
                    return self.shell_state("no active layer is available".to_string());
                };

                let move_tool = MoveTool;
                let application = if self.document.selection_mask().is_empty() {
                    move_tool.move_layer(layer_id, &mut self.surface, start, point)
                } else {
                    move_tool.move_selection(
                        layer_id,
                        &mut self.surface,
                        self.document.selection_mask_mut(),
                        start,
                        point,
                    )
                };

                if application.history_entry.is_empty() {
                    return self.shell_state(format!("move ended at {}, {}", x, y));
                }

                self.invalidate_preview_cache();
                self.history.push_brush_stroke(application.history_entry);
                self.document_dirty = true;
                self.push_recent_action(format!(
                    "Moved content by {}, {}",
                    application.delta_x, application.delta_y
                ));

                self.shell_state(format!(
                    "moved content by {} px, {} px",
                    application.delta_x, application.delta_y,
                ))
            }
            CanvasInteraction::Marquee { start } => {
                self.document
                    .select_rect(start.x as u32, start.y as u32, x, y);
                self.push_recent_action("Updated marquee selection");
                self.shell_state(format!("finished marquee selection at {}, {}", x, y))
            }
            CanvasInteraction::Crop { start } => self.apply_crop(start, point),
            CanvasInteraction::Hand { .. } => {
                self.push_recent_action("Panned viewport");
                self.shell_state(format!("panned viewport at {}", self.zoom_label()))
            }
        }
    }

    fn canvas_hovered(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let _ = self.canvas_point(normalized_x, normalized_y);
        self.shell_state(current_state.status_message.clone())
    }
}

impl UiShellDelegate for AppSession {
    fn initial_state(&self) -> UiShellState {
        self.shell_state("session ready".to_string())
    }

    fn on_command(&mut self, command: &str, current_state: &UiShellState) -> UiShellState {
        match command {
            "File" | "File::Save" => self.save_project().unwrap_or_else(|error| UiShellState {
                status_message: format!("save failed: {error}"),
                ..current_state.clone()
            }),
            "File::New" => self.new_document(),
            "File::Reopen" => self.load_project().unwrap_or_else(|error| UiShellState {
                status_message: format!("reopen failed: {error}"),
                ..current_state.clone()
            }),
            "File::ExportPng" => self.export_png().unwrap_or_else(|error| UiShellState {
                status_message: format!("export failed: {error}"),
                ..current_state.clone()
            }),
            "Edit" | "Edit::Undo" => self.undo(),
            "Edit::Redo" => self.redo(),
            "Image" | "Image::SampleStroke" => self.apply_demo_stroke(),
            "View::ZoomIn" => self.zoom_in(),
            "View::ZoomOut" => self.zoom_out(),
            "View::Reset" => self.reset_view(),
            "Search" => self.shell_state(
                "command access is staged here until the palette surface is implemented"
                    .to_string(),
            ),
            "New Document" => self.new_document(),
            "Close Document" => self.close_document(),
            "Delete Layer" | "Layer::Delete" => self.delete_active_layer(),
            "Swap Colors" => self.swap_colors(),
            "Reset Colors" => self.reset_colors(),
            "Select::All" => self.select_all(),
            "Select::Clear" => self.clear_selection(),
            "Select::Invert" => self.invert_selection(),
            "Layer" => self.shell_state(
                "layer menu commands will expand in the next workflow slice".to_string(),
            ),
            "Select" => self.shell_state(
                "selection menu commands will expand in the next workflow slice".to_string(),
            ),
            "Filter" => self.shell_state(
                "filter commands will arrive after the core editing loop stabilizes".to_string(),
            ),
            "Help" => self.shell_state(
                "help content will land after the command palette and docs surfaces".to_string(),
            ),
            "View" => self.shell_state(format!("current zoom is {}", self.zoom_label())),
            "Window" => self.shell_state(
                "window menu content is staged behind the fixed-shell MVP interactions".to_string(),
            ),
            _ => UiShellState {
                status_message: format!("{command} is not available in this shell pass"),
                ..current_state.clone()
            },
        }
    }

    fn on_tool(&mut self, tool: &str, _current_state: &UiShellState) -> UiShellState {
        self.select_tool(tool)
    }

    fn on_tool_option_adjusted(
        &mut self,
        option: &str,
        delta: i32,
        _current_state: &UiShellState,
    ) -> UiShellState {
        self.adjust_active_tool_option(option, delta)
    }

    fn on_tool_options_reset(&mut self, _current_state: &UiShellState) -> UiShellState {
        self.reset_active_tool_options()
    }

    fn on_canvas_pressed(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        self.canvas_pressed(normalized_x, normalized_y, current_state)
    }

    fn on_canvas_dragged(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        self.canvas_dragged_batch(&[(normalized_x, normalized_y)], current_state)
    }

    fn on_canvas_dragged_batch(
        &mut self,
        points: &[(f32, f32)],
        current_state: &UiShellState,
    ) -> UiShellState {
        self.canvas_dragged_batch(points, current_state)
    }

    fn on_canvas_released(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        self.canvas_released(normalized_x, normalized_y, current_state)
    }

    fn on_canvas_hovered(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        self.canvas_hovered(normalized_x, normalized_y, current_state)
    }
}

fn merge_pixel_changes(
    changes_by_pixel: &mut HashMap<(u32, u32), PixelChange>,
    changes: &[PixelChange],
) {
    for change in changes {
        changes_by_pixel
            .entry((change.x, change.y))
            .and_modify(|existing| {
                existing.after = change.after;
            })
            .or_insert(*change);
    }
}

fn project_file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "untitled.ptx".to_string())
}

fn elapsed_milliseconds(started_at: Instant) -> f64 {
    started_at.elapsed().as_secs_f64() * 1000.0
}

fn build_preview_cache(surface: &RasterSurface, viewport: &ViewportState) -> PreviewCache {
    let width = viewport.size.logical_width.max(1);
    let height = viewport.size.logical_height.max(1);
    let mut cache = PreviewCache {
        width,
        height,
        scale_factor: viewport.size.scale_factor,
        zoom: viewport.zoom,
        pan: viewport.pan,
        pixel_buffer: slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height),
    };

    repaint_preview_region(
        cache.pixel_buffer.make_mut_bytes(),
        width,
        height,
        surface,
        viewport,
        0,
        0,
        width - 1,
        height - 1,
    );
    cache
}

fn preview_cache_matches(cache: &PreviewCache, viewport: &ViewportState) -> bool {
    cache.width == viewport.size.logical_width.max(1)
        && cache.height == viewport.size.logical_height.max(1)
        && cache.scale_factor == viewport.size.scale_factor
        && cache.zoom == viewport.zoom
        && cache.pan == viewport.pan
}

fn preview_document_bounds(changes: &[PixelChange]) -> Option<(u32, u32, u32, u32)> {
    let mut iter = changes.iter();
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;

    for change in iter {
        min_x = min_x.min(change.x);
        min_y = min_y.min(change.y);
        max_x = max_x.max(change.x);
        max_y = max_y.max(change.y);
    }

    Some((min_x, min_y, max_x, max_y))
}

fn repaint_preview_document_region(
    cache: &mut PreviewCache,
    surface: &RasterSurface,
    viewport: &ViewportState,
    min_doc_x: u32,
    min_doc_y: u32,
    max_doc_x: u32,
    max_doc_y: u32,
) {
    let left = ((min_doc_x as f32) * viewport.zoom + viewport.pan.dx).floor() as i32 - 1;
    let top = ((min_doc_y as f32) * viewport.zoom + viewport.pan.dy).floor() as i32 - 1;
    let right = (((max_doc_x + 1) as f32) * viewport.zoom + viewport.pan.dx).ceil() as i32 + 1;
    let bottom = (((max_doc_y + 1) as f32) * viewport.zoom + viewport.pan.dy).ceil() as i32 + 1;

    let start_x = left.clamp(0, cache.width.saturating_sub(1) as i32) as u32;
    let start_y = top.clamp(0, cache.height.saturating_sub(1) as i32) as u32;
    let end_x = right.clamp(0, cache.width.saturating_sub(1) as i32) as u32;
    let end_y = bottom.clamp(0, cache.height.saturating_sub(1) as i32) as u32;

    repaint_preview_region(
        cache.pixel_buffer.make_mut_bytes(),
        cache.width,
        cache.height,
        surface,
        viewport,
        start_x,
        start_y,
        end_x,
        end_y,
    );
}

fn repaint_preview_region(
    pixels: &mut [u8],
    preview_width: u32,
    _preview_height: u32,
    surface: &RasterSurface,
    viewport: &ViewportState,
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
) {
    let inv_zoom = 1.0 / viewport.zoom;
    let pan_x = viewport.pan.dx;
    let pan_y = viewport.pan.dy;
    let surface_width = surface.width() as usize;
    let surface_height = surface.height() as i32;
    let flat_surface = surface.to_flat_rgba();

    for preview_y in start_y..=end_y {
        let logical_y = preview_y as f32;
        let document_y = ((logical_y - pan_y) * inv_zoom).floor() as i32;
        for preview_x in start_x..=end_x {
            let checker_pixel = checkerboard_pixel(preview_x, preview_y);
            let logical_x = preview_x as f32;
            let document_x = ((logical_x - pan_x) * inv_zoom).floor() as i32;
            let composed_pixel = if document_x >= 0
                && document_y >= 0
                && document_x < surface.width() as i32
                && document_y < surface_height
            {
                let index = ((document_y as usize * surface_width) + document_x as usize) * 4;
                let document_pixel = [
                    flat_surface[index],
                    flat_surface[index + 1],
                    flat_surface[index + 2],
                    flat_surface[index + 3],
                ];
                blend_over_checkerboard(document_pixel, checker_pixel)
            } else {
                checker_pixel
            };
            let index = ((preview_y * preview_width + preview_x) * 4) as usize;

            pixels[index] = composed_pixel[0];
            pixels[index + 1] = composed_pixel[1];
            pixels[index + 2] = composed_pixel[2];
            pixels[index + 3] = composed_pixel[3];
        }
    }
}

fn default_viewport_state(surface: &RasterSurface) -> ViewportState {
    let preview_width = surface.width().min(PREVIEW_MAX_WIDTH).max(1);
    let preview_height = ((preview_width as f64 / surface.width() as f64) * surface.height() as f64)
        .round()
        .max(1.0) as u32;
    let size = ViewportSize::new(preview_width, preview_height, 1.0);
    let document_size = Size::new(surface.width() as f32, surface.height() as f32);
    let mut viewport = ViewportState::new(size, document_size);

    viewport.zoom = (preview_width as f32 / document_size.width)
        .min(preview_height as f32 / document_size.height)
        .clamp(ViewportState::MIN_ZOOM, ViewportState::MAX_ZOOM);
    viewport.pan.dx = (preview_width as f32 - document_size.width * viewport.zoom) / 2.0;
    viewport.pan.dy = (preview_height as f32 - document_size.height * viewport.zoom) / 2.0;

    viewport
}

fn crop_surface(surface: &RasterSurface, bounds: doc_model::SelectionBounds) -> RasterSurface {
    let mut cropped = RasterSurface::new(bounds.width, bounds.height);

    for target_y in 0..bounds.height {
        for target_x in 0..bounds.width {
            let source_x = bounds.x + target_x;
            let source_y = bounds.y + target_y;
            let pixel = surface.pixel(source_x, source_y);
            let _ = cropped.write_pixel(target_x, target_y, pixel);
        }
    }

    cropped
}

fn checkerboard_pixel(x: u32, y: u32) -> [u8; 4] {
    let checker_x = x / CHECKER_CELL_SIZE.max(1);
    let checker_y = y / CHECKER_CELL_SIZE.max(1);

    if (checker_x + checker_y) % 2 == 0 {
        CHECKER_LIGHT
    } else {
        CHECKER_DARK
    }
}

fn blend_over_checkerboard(foreground: [u8; 4], background: [u8; 4]) -> [u8; 4] {
    let alpha = foreground[3] as f32 / 255.0;
    let inverse_alpha = 1.0 - alpha;

    [
        ((foreground[0] as f32 * alpha) + (background[0] as f32 * inverse_alpha)).round() as u8,
        ((foreground[1] as f32 * alpha) + (background[1] as f32 * inverse_alpha)).round() as u8,
        ((foreground[2] as f32 * alpha) + (background[2] as f32 * inverse_alpha)).round() as u8,
        0xFF,
    ]
}

fn rgba_hex_label(color: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}

fn blend_mode_label(blend_mode: BlendMode) -> &'static str {
    match blend_mode {
        BlendMode::Normal => "Normal",
        BlendMode::Multiply => "Multiply",
        BlendMode::Screen => "Screen",
        BlendMode::Overlay => "Overlay",
        BlendMode::Darken => "Darken",
        BlendMode::Lighten => "Lighten",
    }
}

fn new_blank_document_state(title: &str) -> (Document, RasterSurface) {
    let mut document = Document::new(
        DocumentId::new(1),
        Canvas::new(1920, 1080),
        DocumentMetadata {
            title: title.to_string(),
        },
    );
    document.add_layer(RasterLayer::new(LayerId::new(1), "Surface"));

    (document, RasterSurface::new(1920, 1080))
}

fn log_timing(operation: &str, started_at: Instant, fields: &[(impl AsRef<str>, String)]) {
    log_elapsed_ms(operation, elapsed_milliseconds(started_at), fields);
}

fn log_elapsed_ms(operation: &str, elapsed_ms: f64, fields: &[(impl AsRef<str>, String)]) {
    if !cfg!(debug_assertions) {
        return;
    }

    let extra = fields
        .iter()
        .map(|(key, value)| format!("{}={}", key.as_ref(), value))
        .collect::<Vec<_>>()
        .join(" ");

    if extra.is_empty() {
        info!(
            target: "phototux::profiling",
            operation,
            elapsed_ms,
            "shell timing"
        );
    } else {
        info!(
            target: "phototux::profiling",
            operation,
            elapsed_ms,
            fields = %extra,
            "shell timing"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::AppSession;
    use ui_shell::{UiShellDelegate, UiShellState};

    #[test]
    fn selecting_eraser_updates_tool_label() {
        let mut session = AppSession::new();

        let state = session.on_tool("Eras", &UiShellState::default());

        assert_eq!(state.active_tool_label, "Eraser Tool");
    }

    #[test]
    fn selecting_move_updates_tool_label_and_key() {
        let mut session = AppSession::new();

        let state = session.on_tool("Move", &UiShellState::default());

        assert_eq!(state.active_tool_label, "Move Tool");
        assert_eq!(state.active_tool_key, "Move");
        assert_eq!(state.tool_option_primary_label, "Auto-select");
    }

    #[test]
    fn image_command_applies_demo_stroke() {
        let mut session = AppSession::new();
        let state = session.on_command("Image", &UiShellState::default());

        assert!(state.status_message.contains("sample stroke"));
    }

    #[test]
    fn file_command_saves_project() {
        let mut session = AppSession::new();
        let state = session.on_command("File", &UiShellState::default());

        assert_eq!(state.document_title, "session.ptx");
        assert!(state.status_message.contains("saved document"));
    }

    #[test]
    fn search_command_returns_shell_status_message() {
        let mut session = AppSession::new();

        let state = session.on_command("Search", &UiShellState::default());

        assert!(state.status_message.contains("command access"));
    }

    #[test]
    fn tool_option_adjustment_updates_size_label() {
        let mut session = AppSession::new();

        let state = session.on_tool_option_adjusted("size", 1, &UiShellState::default());

        assert_eq!(state.tool_size_label, "28 px");
        assert!(state.status_message.contains("size set to 28 px"));
    }

    #[test]
    fn resetting_tool_options_restores_defaults() {
        let mut session = AppSession::new();
        let _ = session.on_tool_option_adjusted("size", 2, &UiShellState::default());

        let state = session.on_tool_options_reset(&UiShellState::default());

        assert_eq!(state.tool_size_label, "24 px");
        assert!(state.status_message.contains("reset brush tool settings"));
    }

    #[test]
    fn brush_canvas_interaction_updates_surface_and_history() {
        let mut session = AppSession::new();

        let _ = session.on_canvas_pressed(0.1, 0.1, &UiShellState::default());
        let _ = session.on_canvas_dragged(0.2, 0.1, &UiShellState::default());
        let state = session.on_canvas_released(0.2, 0.1, &UiShellState::default());

        assert!(state.status_message.contains("applied brush tool"));
        assert!(session.history.can_undo());
        assert_ne!(session.surface.pixel(192, 108), [0, 0, 0, 0]);
    }

    #[test]
    fn select_all_command_updates_selection_bounds() {
        let mut session = AppSession::new();

        let state = session.on_command("Select::All", &UiShellState::default());

        assert_eq!(state.selection_bounds_label, "Sel 1920x1080");
    }

    #[test]
    fn zoom_command_updates_zoom_label() {
        let mut session = AppSession::new();

        let initial = session.initial_state().zoom_label;
        let state = session.on_command("View::ZoomIn", &UiShellState::default());

        assert_ne!(state.zoom_label, initial);
    }

    #[test]
    fn eyedropper_samples_foreground_color() {
        let mut session = AppSession::new();
        let _ = session.surface.write_pixel(200, 120, [255, 64, 32, 255]);
        let _ = session.on_tool("Dropper", &UiShellState::default());

        let state = session.on_canvas_pressed(
            200.0 / session.document.canvas.width as f32,
            120.0 / session.document.canvas.height as f32,
            &UiShellState::default(),
        );

        assert_eq!(state.foreground_color_label, "#FF4020");
    }

    #[test]
    fn crop_tool_reduces_canvas_and_preserves_pixels() {
        let mut session = AppSession::new();
        let _ = session.surface.write_pixel(50, 60, [12, 34, 56, 255]);
        let _ = session.on_tool("Crop", &UiShellState::default());

        let _ = session.on_canvas_pressed(
            40.0 / session.document.canvas.width as f32,
            50.0 / session.document.canvas.height as f32,
            &UiShellState::default(),
        );
        let state = session.on_canvas_released(
            70.0 / session.document.canvas.width as f32,
            90.0 / session.document.canvas.height as f32,
            &UiShellState::default(),
        );

        assert_eq!(state.canvas_size_label, "30x40 px");
        assert_eq!(session.surface.pixel(10, 10), [12, 34, 56, 255]);
        assert!(state.status_message.contains("cropped canvas"));
    }
}
