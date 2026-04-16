use anyhow::Result;
use common::{
    APP_NAME, CanvasRaster, CanvasRect, CanvasSize, DestructiveFilterKind, GroupId, LayerId,
};
use glib::ControlFlow;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, ComboBoxText,
    CssProvider, Dialog, Entry, EventControllerKey, EventControllerMotion, EventControllerScroll,
    EventControllerScrollFlags, GestureClick, GestureDrag, GestureStylus, HeaderBar, IconTheme,
    Image, Label, MenuButton, MessageDialog, MessageType, Orientation, Paned, Picture, PolicyType,
    Popover, ResponseType, ScrolledWindow, Separator, SpinButton, gdk,
};
use render_wgpu::{
    CanvasOverlayPath, CanvasOverlayRect, OffscreenCanvasRenderer, ViewportRendererConfig,
    ViewportSize, ViewportState,
};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use ui_templates::{
    build_panel_group_shell, load_document_tabs_template, load_info_dialog_template,
    load_status_bar_template, load_titlebar_template,
};

mod canvas_host;
mod file_workflow;
mod layout;
mod menus;
mod panels;
mod shell_chrome;
mod startup;
mod status_presenter;
mod ui_support;
mod ui_templates;

use canvas_host::{CanvasHostState, build_canvas_host};
use status_presenter::{
    apply_status_notice_style, format_import_report_details, format_shell_alert_secondary_text,
    shell_notice_text, shell_status_hint,
};
use ui_support::{
    APP_WINDOW_ICON_NAME, build_icon_label_button, build_icon_label_shortcut_button,
    build_icon_only_button, build_remix_icon, create_menu_popover, set_menu_button_label,
    set_remix_icon_or_fallback, shell_tool_icon, shell_tool_shortcut,
};

const UI_RESOURCE_PREFIX: &str = "/com/phototux";
const OPTIONAL_ICON_FALLBACK_NAME: &str = "image-missing";
const MAIN_WINDOW_DEFAULT_WIDTH: i32 = 1600;
const MAIN_WINDOW_DEFAULT_HEIGHT: i32 = 900;
const STARTUP_WARMUP_WIDTH: u32 = 1280;
const STARTUP_WARMUP_HEIGHT: u32 = 720;

type StartupWindowHook = Box<dyn FnOnce(&ApplicationWindow)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPanelPreview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulerUnit {
    Pixels,
    Inches,
    Centimeters,
}

impl RulerUnit {
    const PIXELS_PER_INCH: f64 = 96.0;
    const CENTIMETERS_PER_INCH: f64 = 2.54;

    fn pixels_per_unit(self) -> f64 {
        match self {
            Self::Pixels => 1.0,
            Self::Inches => Self::PIXELS_PER_INCH,
            Self::Centimeters => Self::PIXELS_PER_INCH / Self::CENTIMETERS_PER_INCH,
        }
    }

    fn format_pixels(self, pixels: f64) -> String {
        match self {
            Self::Pixels => format!("{}", pixels.round() as i32),
            Self::Inches | Self::Centimeters => {
                format_ruler_decimal(pixels / self.pixels_per_unit())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPanelItem {
    pub layer_id: Option<LayerId>,
    pub index: Option<usize>,
    pub group_id: Option<GroupId>,
    pub name: String,
    pub depth: usize,
    pub is_group: bool,
    pub is_text: bool,
    pub visible: bool,
    pub opacity_percent: u8,
    pub has_mask: bool,
    pub mask_enabled: bool,
    pub mask_target_active: bool,
    pub is_selected: bool,
    pub is_active: bool,
    pub preview: Option<LayerPanelPreview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellToolKind {
    Move,
    RectangularMarquee,
    Lasso,
    Transform,
    Text,
    Brush,
    Eraser,
    Hand,
    Zoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellTextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTextSnapshot {
    pub selected: bool,
    pub editing: bool,
    pub request_id: Option<u64>,
    pub is_new_layer: bool,
    pub layer_name: String,
    pub content: String,
    pub font_family: String,
    pub font_size_px: u32,
    pub line_height_percent: u32,
    pub letter_spacing: i32,
    pub fill_rgba: [u8; 4],
    pub alignment: ShellTextAlignment,
    pub origin_x: i32,
    pub origin_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTextUpdate {
    pub content: String,
    pub font_family: String,
    pub font_size_px: u32,
    pub line_height_percent: u32,
    pub letter_spacing: i32,
    pub fill_rgba: [u8; 4],
    pub alignment: ShellTextAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellGuide {
    Horizontal { y: i32 },
    Vertical { x: i32 },
}

impl ShellToolKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Move => "Move Tool",
            Self::RectangularMarquee => "Rectangular Marquee",
            Self::Lasso => "Lasso Tool",
            Self::Transform => "Transform Tool",
            Self::Text => "Text Tool",
            Self::Brush => "Brush Tool",
            Self::Eraser => "Eraser Tool",
            Self::Hand => "Hand Tool",
            Self::Zoom => "Zoom Tool",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellImportDiagnostic {
    pub severity_label: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellImportReport {
    pub id: u64,
    pub title: String,
    pub summary: String,
    pub diagnostics: Vec<ShellImportDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAlertTone {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellAlert {
    pub id: u64,
    pub tone: ShellAlertTone,
    pub title: String,
    pub body: String,
    pub secondary_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSnapshot {
    pub document_title: String,
    pub project_path: Option<PathBuf>,
    pub dirty: bool,
    pub recovery_offer_pending: bool,
    pub recovery_path: Option<PathBuf>,
    pub status_message: String,
    pub latest_alert: Option<ShellAlert>,
    pub latest_import_report: Option<ShellImportReport>,
    pub file_job_active: bool,
    pub autosave_job_active: bool,
    pub canvas_size: CanvasSize,
    pub canvas_revision: u64,
    pub active_tool_name: String,
    pub active_tool: ShellToolKind,
    pub layers: Vec<LayerPanelItem>,
    pub active_layer_name: String,
    pub active_layer_opacity_percent: u8,
    pub active_layer_visible: bool,
    pub active_layer_blend_mode: String,
    pub active_layer_has_mask: bool,
    pub active_layer_mask_enabled: bool,
    pub active_edit_target_name: String,
    pub selected_structure_name: String,
    pub selected_structure_is_group: bool,
    pub can_create_group_from_active_layer: bool,
    pub can_ungroup_selected_group: bool,
    pub can_move_active_layer_into_selected_group: bool,
    pub can_move_active_layer_out_of_group: bool,
    pub active_layer_bounds: Option<CanvasRect>,
    pub can_begin_transform: bool,
    pub transform_preview_rect: Option<CanvasRect>,
    pub transform_active: bool,
    pub transform_scale_percent: u32,
    pub transform_scale_x_percent: u32,
    pub transform_scale_y_percent: u32,
    pub transform_rotation_degrees: i32,
    pub can_apply_destructive_filters: bool,
    pub filter_job_active: bool,
    pub brush_preset_name: String,
    pub brush_radius: u32,
    pub brush_hardness_percent: u32,
    pub brush_spacing: u32,
    pub brush_flow_percent: u32,
    pub pressure_size_enabled: bool,
    pub pressure_opacity_enabled: bool,
    pub snapping_enabled: bool,
    pub snapping_temporarily_bypassed: bool,
    pub guides_visible: bool,
    pub guide_count: usize,
    pub guides: Vec<ShellGuide>,
    pub selection_rect: Option<CanvasRect>,
    pub selection_path: Option<Vec<(i32, i32)>>,
    pub selection_preview_path: Option<Vec<(i32, i32)>>,
    pub selection_inverted: bool,
    pub foreground_color: [u8; 4],
    pub background_color: [u8; 4],
    pub color_swatches: Vec<[u8; 4]>,
    pub selected_color_swatch: Option<usize>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub history_entries: Vec<String>,
    pub text: ShellTextSnapshot,
}

pub trait ShellController {
    fn snapshot(&self) -> ShellSnapshot;
    fn canvas_raster(&self) -> CanvasRaster;
    fn add_layer(&mut self);
    fn duplicate_active_layer(&mut self);
    fn delete_active_layer(&mut self);
    fn add_active_layer_mask(&mut self);
    fn remove_active_layer_mask(&mut self);
    fn toggle_active_layer_mask_enabled(&mut self);
    fn edit_active_layer_pixels(&mut self);
    fn edit_active_layer_mask(&mut self);
    fn select_layer(&mut self, layer_id: LayerId);
    fn select_group(&mut self, group_id: GroupId);
    fn toggle_layer_visibility(&mut self, layer_id: LayerId);
    fn toggle_group_visibility(&mut self, group_id: GroupId);
    fn create_group_from_active_layer(&mut self);
    fn ungroup_selected_group(&mut self);
    fn move_active_layer_into_selected_group(&mut self);
    fn move_active_layer_out_of_group(&mut self);
    fn increase_active_layer_opacity(&mut self);
    fn decrease_active_layer_opacity(&mut self);
    fn next_active_layer_blend_mode(&mut self);
    fn previous_active_layer_blend_mode(&mut self);
    fn move_active_layer_up(&mut self);
    fn move_active_layer_down(&mut self);
    fn swap_colors(&mut self);
    fn reset_colors(&mut self);
    fn set_foreground_color(&mut self, rgba: [u8; 4]);
    fn set_background_color(&mut self, rgba: [u8; 4]);
    fn add_color_swatch(&mut self);
    fn select_color_swatch(&mut self, index: usize);
    fn remove_selected_color_swatch(&mut self);
    fn clear_selection(&mut self);
    fn invert_selection(&mut self);
    fn add_horizontal_guide(&mut self);
    fn add_vertical_guide(&mut self);
    fn remove_last_guide(&mut self);
    fn toggle_guides_visible(&mut self);
    fn toggle_snapping_enabled(&mut self);
    fn toggle_pressure_size_enabled(&mut self);
    fn toggle_pressure_opacity_enabled(&mut self);
    fn increase_brush_radius(&mut self);
    fn decrease_brush_radius(&mut self);
    fn increase_brush_hardness(&mut self);
    fn decrease_brush_hardness(&mut self);
    fn increase_brush_spacing(&mut self);
    fn decrease_brush_spacing(&mut self);
    fn increase_brush_flow(&mut self);
    fn decrease_brush_flow(&mut self);
    fn next_brush_preset(&mut self);
    fn previous_brush_preset(&mut self);
    fn set_temporary_snap_bypass(&mut self, bypassed: bool);
    fn begin_transform(&mut self);
    fn scale_transform_up(&mut self);
    fn scale_transform_down(&mut self);
    fn scale_transform_x_up(&mut self);
    fn scale_transform_x_down(&mut self);
    fn scale_transform_y_up(&mut self);
    fn scale_transform_y_down(&mut self);
    fn rotate_transform_left(&mut self);
    fn rotate_transform_right(&mut self);
    fn commit_transform(&mut self);
    fn cancel_transform(&mut self);
    fn undo(&mut self);
    fn redo(&mut self);
    fn save_document(&mut self);
    fn save_document_as(&mut self, path: PathBuf);
    fn load_recovery_document(&mut self);
    fn discard_recovery_document(&mut self);
    fn open_document(&mut self, path: PathBuf);
    fn import_image(&mut self, path: PathBuf);
    fn export_document(&mut self, path: PathBuf);
    fn apply_destructive_filter(&mut self, filter: DestructiveFilterKind);
    fn poll_background_tasks(&mut self);
    fn select_tool(&mut self, tool: ShellToolKind);
    fn begin_text_edit(&mut self);
    fn update_text_session(&mut self, update: ShellTextUpdate);
    fn commit_text_session(&mut self);
    fn cancel_text_session(&mut self);
    fn begin_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32);
    fn begin_canvas_interaction_with_pressure(
        &mut self,
        canvas_x: i32,
        canvas_y: i32,
        pressure: f32,
    ) {
        let _ = pressure;
        self.begin_canvas_interaction(canvas_x, canvas_y);
    }
    fn update_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32);
    fn update_canvas_interaction_with_pressure(
        &mut self,
        canvas_x: i32,
        canvas_y: i32,
        pressure: f32,
    ) {
        let _ = pressure;
        self.update_canvas_interaction(canvas_x, canvas_y);
    }
    fn end_canvas_interaction(&mut self);
}

pub fn run(controller: Rc<RefCell<dyn ShellController>>) -> Result<()> {
    ensure_ui_resources_registered()?;
    let application = Application::builder()
        .application_id("com.phototux.app")
        .build();

    application.connect_activate(move |application| {
        gtk4::Window::set_default_icon_name(APP_WINDOW_ICON_NAME);
        startup::begin_startup(application, controller.clone())
    });
    let _exit_code = application.run();

    Ok(())
}

fn ensure_ui_resources_registered() -> Result<()> {
    static REGISTRATION: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    match REGISTRATION.get_or_init(|| {
        gio::resources_register_include!("phototux-ui.gresource").map_err(|error| error.to_string())
    }) {
        Ok(()) => Ok(()),
        Err(error) => anyhow::bail!("failed to register bundled PhotoTux UI resources: {error}"),
    }
}

fn build_color_chip(label_text: &str, css_class: &str) -> Button {
    let button = if label_text.is_empty() {
        Button::new()
    } else {
        Button::with_label(label_text)
    };
    button.add_css_class("color-chip");
    button.add_css_class(css_class);
    button
}

fn format_ruler_decimal(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn build_ruler_stops(max: u32) -> [u32; 5] {
    let quarter = (max / 4).max(1);
    [0, quarter, quarter * 2, quarter * 3, max.max(1)]
}
#[allow(dead_code)]
fn format_horizontal_ruler(max: u32) -> String {
    build_ruler_stops(max)
        .into_iter()
        .map(|value| format!("{value:>5}"))
        .collect::<Vec<_>>()
        .join("    ")
}

#[allow(dead_code)]
fn format_vertical_ruler(max: u32) -> String {
    build_ruler_stops(max)
        .into_iter()
        .map(|value| format!("{value:>4}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_ruler_stops_range(min_in: i32, max_in: i32) -> [i32; 5] {
    let (min, max) = if min_in <= max_in {
        (min_in, max_in)
    } else {
        (max_in, min_in)
    };
    let width = (max - min).max(1);
    let quarter = (width / 4).max(1);
    [
        min,
        min + quarter,
        min + quarter * 2,
        min + quarter * 3,
        max,
    ]
}

fn compute_visible_ruler_range(pan: f32, zoom: f32, viewport_extent: u32) -> Option<(i32, i32)> {
    if zoom <= 0.0 || viewport_extent == 0 {
        return None;
    }

    let mut min_value = ((0.0 - pan) / zoom).floor() as i32;
    let mut max_value = ((viewport_extent as f32 - pan) / zoom).ceil() as i32;

    if min_value > max_value {
        std::mem::swap(&mut min_value, &mut max_value);
    }

    Some((min_value, max_value))
}

fn normalize_ruler_step(raw_step: f64) -> f64 {
    let raw_step = raw_step.max(f64::EPSILON);
    let magnitude = 10_f64.powf(raw_step.log10().floor());
    let normalized = raw_step / magnitude;
    let base = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    base * magnitude
}

fn pick_ruler_major_step(unit: RulerUnit, zoom: f32) -> f64 {
    let target_screen_spacing = 90.0;
    let screen_pixels_per_unit = (zoom as f64 * unit.pixels_per_unit()).max(f64::EPSILON);
    normalize_ruler_step(target_screen_spacing / screen_pixels_per_unit)
}

fn draw_ruler_background(ctx: &gtk4::cairo::Context, width: f64, height: f64, horizontal: bool) {
    ctx.set_source_rgb(0.17, 0.17, 0.17);
    ctx.rectangle(0.0, 0.0, width, height);
    let _ = ctx.fill();

    ctx.set_line_width(1.0);
    ctx.set_source_rgb(0.28, 0.28, 0.28);
    if horizontal {
        ctx.move_to(0.0, height - 0.5);
        ctx.line_to(width, height - 0.5);
    } else {
        ctx.move_to(width - 0.5, 0.0);
        ctx.line_to(width - 0.5, height);
    }
    let _ = ctx.stroke();
}

fn draw_horizontal_ruler(
    ctx: &gtk4::cairo::Context,
    width: f64,
    height: f64,
    pan_x: f32,
    zoom: f32,
    viewport_width: u32,
    unit: RulerUnit,
) {
    draw_ruler_background(ctx, width, height, true);

    let Some((visible_min, visible_max)) = compute_visible_ruler_range(pan_x, zoom, viewport_width)
    else {
        return;
    };

    let major_step = pick_ruler_major_step(unit, zoom);
    let minor_step = major_step / 10.0;
    let pixels_per_unit = unit.pixels_per_unit();
    let min_units = visible_min as f64 / pixels_per_unit;
    let max_units = visible_max as f64 / pixels_per_unit;
    let start_index = (min_units / minor_step).floor() as i64 - 1;
    let end_index = (max_units / minor_step).ceil() as i64 + 1;

    ctx.set_source_rgb(0.72, 0.72, 0.72);
    ctx.set_font_size(9.0);
    for index in start_index..=end_index {
        let unit_value = index as f64 * minor_step;
        let canvas_value = unit_value * pixels_per_unit;
        let screen_x = canvas_value * zoom as f64 + pan_x as f64;
        if screen_x < -2.0 || screen_x > width + 2.0 {
            continue;
        }

        let tick_top = if index % 10 == 0 {
            height - 12.0
        } else if index % 5 == 0 {
            height - 8.0
        } else {
            height - 5.0
        };
        ctx.move_to(screen_x + 0.5, tick_top);
        ctx.line_to(screen_x + 0.5, height);
        let _ = ctx.stroke();

        if index % 10 == 0 {
            let label = unit.format_pixels(canvas_value);
            let label_x = (screen_x + 3.0).clamp(2.0, (width - 24.0).max(2.0));
            ctx.move_to(label_x, 9.5);
            let _ = ctx.show_text(&label);
        }
    }
}

fn draw_vertical_ruler(
    ctx: &gtk4::cairo::Context,
    width: f64,
    height: f64,
    pan_y: f32,
    zoom: f32,
    viewport_height: u32,
    unit: RulerUnit,
) {
    draw_ruler_background(ctx, width, height, false);

    let Some((visible_min, visible_max)) =
        compute_visible_ruler_range(pan_y, zoom, viewport_height)
    else {
        return;
    };

    let major_step = pick_ruler_major_step(unit, zoom);
    let minor_step = major_step / 10.0;
    let pixels_per_unit = unit.pixels_per_unit();
    let min_units = visible_min as f64 / pixels_per_unit;
    let max_units = visible_max as f64 / pixels_per_unit;
    let start_index = (min_units / minor_step).floor() as i64 - 1;
    let end_index = (max_units / minor_step).ceil() as i64 + 1;

    ctx.set_source_rgb(0.72, 0.72, 0.72);
    ctx.set_font_size(9.0);
    for index in start_index..=end_index {
        let unit_value = index as f64 * minor_step;
        let canvas_value = unit_value * pixels_per_unit;
        let screen_y = canvas_value * zoom as f64 + pan_y as f64;
        if screen_y < -2.0 || screen_y > height + 2.0 {
            continue;
        }

        let tick_left = if index % 10 == 0 {
            width - 12.0
        } else if index % 5 == 0 {
            width - 8.0
        } else {
            width - 5.0
        };
        ctx.move_to(tick_left, screen_y + 0.5);
        ctx.line_to(width, screen_y + 0.5);
        let _ = ctx.stroke();

        if index % 10 == 0 && screen_y > 20.0 {
            let label = unit.format_pixels(canvas_value);
            let _ = ctx.save();
            ctx.translate(10.0, screen_y - 2.0);
            ctx.rotate(-std::f64::consts::FRAC_PI_2);
            let _ = ctx.show_text(&label);
            let _ = ctx.restore();
        }
    }
}

#[allow(dead_code)]
fn format_horizontal_ruler_range(min: i32, max: i32) -> String {
    build_ruler_stops_range(min, max)
        .into_iter()
        .map(|value| format!("{value:>5}"))
        .collect::<Vec<_>>()
        .join("    ")
}
#[allow(dead_code)]
fn format_vertical_ruler_range(min: i32, max: i32) -> String {
    build_ruler_stops_range(min, max)
        .into_iter()
        .map(|value| format!("{value:>4}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingDocumentAction {
    ChooseOpenProject,
    ChooseImportImage,
    OpenProject(PathBuf),
    ImportImage(PathBuf),
}

impl PendingDocumentAction {
    const fn is_open_project(&self) -> bool {
        matches!(self, Self::ChooseOpenProject | Self::OpenProject(_))
    }

    const fn prompt_title(&self) -> &'static str {
        if self.is_open_project() {
            "Save changes before opening another project?"
        } else {
            "Save changes before importing?"
        }
    }

    const fn prompt_action_phrase(&self) -> &'static str {
        if self.is_open_project() {
            "open another project"
        } else {
            "import a file that replaces the current document"
        }
    }

    fn prompt_detail(&self, document_title: &str) -> String {
        format!(
            "{} has unsaved changes. Save them before you {}, discard them, or cancel and keep editing.",
            document_title,
            self.prompt_action_phrase()
        )
    }
}

type ShellTextUpdateCallback = Rc<dyn Fn()>;

#[derive(Clone)]
struct ShellTextDialogControls {
    content_entry: Entry,
    font_family: ComboBoxText,
    font_size: SpinButton,
    line_height: SpinButton,
    letter_spacing: SpinButton,
    alignment: ComboBoxText,
    color_r: SpinButton,
    color_g: SpinButton,
    color_b: SpinButton,
    color_a: SpinButton,
}

impl ShellTextDialogControls {
    fn build_update(&self, fallback_font_family: &str) -> ShellTextUpdate {
        ShellTextUpdate {
            content: self.content_entry.text().to_string(),
            font_family: self
                .font_family
                .active_text()
                .map(|value| value.to_string())
                .unwrap_or_else(|| fallback_font_family.to_string()),
            font_size_px: self.font_size.value().round() as u32,
            line_height_percent: self.line_height.value().round() as u32,
            letter_spacing: self.letter_spacing.value().round() as i32,
            fill_rgba: [
                self.color_r.value().round() as u8,
                self.color_g.value().round() as u8,
                self.color_b.value().round() as u8,
                self.color_a.value().round() as u8,
            ],
            alignment: match self.alignment.active_id().as_ref().map(|id| id.as_str()) {
                Some("center") => ShellTextAlignment::Center,
                Some("right") => ShellTextAlignment::Right,
                _ => ShellTextAlignment::Left,
            },
        }
    }

    fn connect_update_signals(&self, sync: &ShellTextUpdateCallback) {
        {
            let sync = sync.clone();
            self.content_entry.connect_changed(move |_| sync());
        }
        for spin in [
            &self.font_size,
            &self.line_height,
            &self.letter_spacing,
            &self.color_r,
            &self.color_g,
            &self.color_b,
            &self.color_a,
        ] {
            let sync = sync.clone();
            spin.connect_value_changed(move |_| sync());
        }
        {
            let sync = sync.clone();
            self.font_family.connect_changed(move |_| sync());
        }
        {
            let sync = sync.clone();
            self.alignment.connect_changed(move |_| sync());
        }
    }
}

struct ShellUiState {
    controller: Rc<RefCell<dyn ShellController>>,
    window: RefCell<Option<ApplicationWindow>>,
    recovery_prompt_visible: Cell<bool>,
    close_prompt_visible: Cell<bool>,
    replace_prompt_visible: Cell<bool>,
    import_report_visible: Cell<bool>,
    alert_dialog_visible: Cell<bool>,
    pending_close_after_save: Cell<bool>,
    pending_document_action_after_save: RefCell<Option<PendingDocumentAction>>,
    allow_close_once: Cell<bool>,
    prompted_recovery_path: RefCell<Option<PathBuf>>,
    presented_alert_id: Cell<Option<u64>>,
    presented_import_report_id: Cell<Option<u64>>,
    presented_text_request_id: Cell<Option<u64>>,
    text_dialog_visible: Cell<bool>,
    canvas_state: Rc<RefCell<CanvasHostState>>,
    automation_shortcuts_enabled: bool,
    tool_options_bar: GtkBox,
    tool_options_icon: Image,
    tool_options_label: Label,
    tool_options_content: GtkBox,
    canvas_picture: Picture,
    tool_rail: GtkBox,
    tool_slot_buttons: Vec<shell_chrome::ToolRailSlotButton>,
    document_tabs: GtkBox,
    document_tab_label: Label,
    document_tab_meta_label: Label,
    layers_group: GtkBox,
    layers_body: GtkBox,
    layers_tab_buttons: Vec<Button>,
    properties_group: GtkBox,
    properties_body: GtkBox,
    color_group: GtkBox,
    color_body: GtkBox,
    brush_group: GtkBox,
    brush_body: GtkBox,
    text_group: GtkBox,
    text_body: GtkBox,
    history_group: GtkBox,
    history_body: GtkBox,
    history_tab_buttons: Vec<Button>,
    active_top_dock_tab: Cell<RightSidebarTopTab>,
    active_bottom_dock_tab: Cell<RightSidebarBottomTab>,
    active_context_panel: Cell<Option<ContextDockPanel>>,
    context_toolbar_buttons: RefCell<Vec<(ContextDockPanel, Button)>>,
    context_panel_host: RefCell<Option<GtkBox>>,
    status_bar: GtkBox,
    menu_zoom_label: Label,
    status_doc: Label,
    status_zoom: Label,
    status_cursor: Label,
    status_notice: Label,
    status_mode: Label,
    contextual_fit_button: Button,
    contextual_zoom_out_button: Button,
    contextual_zoom_in_button: Button,
    contextual_clear_selection_button: Button,
    contextual_invert_selection_button: Button,
    contextual_edit_pixels_button: Button,
    contextual_edit_mask_button: Button,
    canvas_info_label: Label,
    horizontal_ruler_label: gtk4::DrawingArea,
    vertical_ruler_label: gtk4::DrawingArea,
    ruler_unit: Cell<RulerUnit>,
    ruler_units_menu_button: RefCell<Option<MenuButton>>,
    layers_filter_text: RefCell<String>,
    ui_revision: Cell<u64>,
    last_ui_revision: Cell<u64>,
    last_snapshot: RefCell<Option<ShellSnapshot>>,
    last_zoom_percent: RefCell<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightSidebarTopTab {
    History,
    Swatches,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightSidebarBottomTab {
    Layers,
    Channels,
    Paths,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextDockPanel {
    Color,
    Properties,
    Brush,
    Text,
}

impl ShellUiState {
    fn new(controller: Rc<RefCell<dyn ShellController>>) -> Rc<Self> {
        let (tool_options_bar, tool_options_icon, tool_options_label, tool_options_content) =
            shell_chrome::build_tool_options_bar(controller.clone());
        let (tool_rail, tool_slot_buttons) = shell_chrome::build_left_tool_rail(controller.clone());
        let (document_tabs, document_tab_label, document_tab_meta_label) =
            shell_chrome::build_document_tabs();
        let (canvas_picture, canvas_state) = build_canvas_host(controller.clone());
        let automation_shortcuts_enabled = env::var_os("PHOTOTUX_ENABLE_TEST_SHORTCUTS").is_some();

        let (color_group, color_body) =
            shell_chrome::build_panel_group("color", &["Color", "Swatches", "Gradients"], 6, false);
        color_group.set_vexpand(false);
        color_group.set_hexpand(true);

        let (properties_group, properties_body) = shell_chrome::build_panel_group(
            "properties",
            &["Properties", "Adjustments", "Libraries"],
            4,
            false,
        );
        properties_group.set_vexpand(false);
        properties_group.set_hexpand(true);

        let (brush_group, brush_body) =
            shell_chrome::build_panel_group("brush", &["Brush"], 4, false);
        brush_group.set_vexpand(false);
        brush_group.set_hexpand(false);
        let (text_group, text_body) = shell_chrome::build_panel_group("text", &["Text"], 4, false);
        text_group.set_vexpand(false);
        text_group.set_hexpand(false);

        let (layers_group, layers_body, layers_tab_buttons) =
            shell_chrome::build_interactive_panel_group(
                "layers",
                &["Layers", "Channels", "Paths"],
                4,
                true,
            );

        let (history_group, history_body, history_tab_buttons) =
            shell_chrome::build_interactive_panel_group(
                "history",
                &["History", "Swatches"],
                4,
                true,
            );

        let (status_bar, status_doc, status_zoom, status_cursor, status_notice, status_mode) =
            shell_chrome::build_status_bar();
        let menu_zoom_label = Label::new(Some("100%"));
        menu_zoom_label.add_css_class("menu-zoom-display");
        let contextual_fit_button =
            ui_support::build_contextual_icon_label_button("focus-3-line.svg", "Fit View");
        {
            let canvas_state = canvas_state.clone();
            contextual_fit_button.connect_clicked(move |_| canvas_state.borrow_mut().fit_to_view());
        }
        let contextual_zoom_out_button =
            ui_support::build_contextual_icon_label_button("zoom-out-line.svg", "Zoom Out");
        {
            let canvas_state = canvas_state.clone();
            contextual_zoom_out_button
                .connect_clicked(move |_| canvas_state.borrow_mut().zoom_out());
        }
        let contextual_zoom_in_button =
            ui_support::build_contextual_icon_label_button("zoom-in-line.svg", "Zoom In");
        {
            let canvas_state = canvas_state.clone();
            contextual_zoom_in_button.connect_clicked(move |_| canvas_state.borrow_mut().zoom_in());
        }
        let contextual_clear_selection_button =
            ui_support::build_contextual_icon_label_button("close-line.svg", "Clear Selection");
        {
            let controller = controller.clone();
            contextual_clear_selection_button
                .connect_clicked(move |_| controller.borrow_mut().clear_selection());
        }
        let contextual_invert_selection_button =
            ui_support::build_contextual_icon_label_button("swap-line.svg", "Invert Selection");
        {
            let controller = controller.clone();
            contextual_invert_selection_button
                .connect_clicked(move |_| controller.borrow_mut().invert_selection());
        }
        let contextual_edit_pixels_button =
            ui_support::build_contextual_icon_label_button("edit-line.svg", "Layer Pixels");
        contextual_edit_pixels_button.add_css_class("contextual-task-button-primary");
        {
            let controller = controller.clone();
            contextual_edit_pixels_button
                .connect_clicked(move |_| controller.borrow_mut().edit_active_layer_pixels());
        }
        let contextual_edit_mask_button =
            ui_support::build_contextual_icon_label_button("layout-column-line.svg", "Layer Mask");
        {
            let controller = controller.clone();
            contextual_edit_mask_button
                .connect_clicked(move |_| controller.borrow_mut().edit_active_layer_mask());
        }
        let canvas_info_label = Label::new(Some("untitled.ptx @ 100% (RGB/8)"));
        canvas_info_label.add_css_class("canvas-info");
        let horizontal_ruler_label = gtk4::DrawingArea::new();
        horizontal_ruler_label.add_css_class("ruler-horizontal");
        horizontal_ruler_label.set_content_height(20);
        horizontal_ruler_label.set_hexpand(true);

        let vertical_ruler_label = gtk4::DrawingArea::new();
        vertical_ruler_label.add_css_class("ruler-vertical");
        vertical_ruler_label.set_content_width(28);
        vertical_ruler_label.set_vexpand(true);

        let shell_state = Rc::new(Self {
            controller: controller.clone(),
            window: RefCell::new(None),
            recovery_prompt_visible: Cell::new(false),
            close_prompt_visible: Cell::new(false),
            replace_prompt_visible: Cell::new(false),
            import_report_visible: Cell::new(false),
            alert_dialog_visible: Cell::new(false),
            pending_close_after_save: Cell::new(false),
            pending_document_action_after_save: RefCell::new(None),
            allow_close_once: Cell::new(false),
            prompted_recovery_path: RefCell::new(None),
            presented_alert_id: Cell::new(None),
            presented_import_report_id: Cell::new(None),
            presented_text_request_id: Cell::new(None),
            text_dialog_visible: Cell::new(false),
            canvas_state: canvas_state.clone(),
            automation_shortcuts_enabled,
            tool_options_bar,
            tool_options_icon,
            tool_options_label,
            tool_options_content,
            canvas_picture,
            tool_rail,
            tool_slot_buttons,
            document_tabs,
            document_tab_label,
            document_tab_meta_label,
            layers_group,
            layers_body,
            layers_tab_buttons,
            properties_group,
            properties_body,
            color_group,
            color_body,
            brush_group,
            brush_body,
            text_group,
            text_body,
            history_group,
            history_body,
            history_tab_buttons,
            active_top_dock_tab: Cell::new(RightSidebarTopTab::History),
            active_bottom_dock_tab: Cell::new(RightSidebarBottomTab::Layers),
            active_context_panel: Cell::new(None),
            context_toolbar_buttons: RefCell::new(Vec::new()),
            context_panel_host: RefCell::new(None),
            status_bar,
            menu_zoom_label,
            status_doc,
            status_zoom,
            status_cursor,
            status_notice,
            status_mode,
            contextual_fit_button,
            contextual_zoom_out_button,
            contextual_zoom_in_button,
            contextual_clear_selection_button,
            contextual_invert_selection_button,
            contextual_edit_pixels_button,
            contextual_edit_mask_button,
            canvas_info_label,
            horizontal_ruler_label,
            vertical_ruler_label,
            ruler_unit: Cell::new(RulerUnit::Pixels),
            ruler_units_menu_button: RefCell::new(None),
            layers_filter_text: RefCell::new(String::new()),
            ui_revision: Cell::new(0),
            last_ui_revision: Cell::new(0),
            last_snapshot: RefCell::new(None),
            last_zoom_percent: RefCell::new(0),
        });

        // Setup ruler draw callbacks and unit selection menu
        {
            let canvas_state_h = canvas_state.clone();
            let shell_state_h = shell_state.clone();
            let canvas_state_v = canvas_state.clone();
            let shell_state_v = shell_state.clone();
            let horizontal = shell_state.horizontal_ruler_label.clone();
            let vertical = shell_state.vertical_ruler_label.clone();

            // Draw horizontal ruler ticks and labels
            horizontal.set_draw_func(move |_, ctx, width, height| {
                let width = width as f64;
                let height = height as f64;
                let (pan_x, _pan_y, zoom, pic_w, _pic_h) = canvas_state_h.borrow().viewport_info();
                draw_horizontal_ruler(
                    ctx,
                    width,
                    height,
                    pan_x,
                    zoom,
                    pic_w,
                    shell_state_h.ruler_unit.get(),
                );
            });

            // Draw vertical ruler ticks and labels
            vertical.set_draw_func(move |_, ctx, width, height| {
                let width = width as f64;
                let height = height as f64;
                let (_pan_x, pan_y, zoom, _pic_w, pic_h) = canvas_state_v.borrow().viewport_info();
                draw_vertical_ruler(
                    ctx,
                    width,
                    height,
                    pan_y,
                    zoom,
                    pic_h,
                    shell_state_v.ruler_unit.get(),
                );
            });

            // Build units popover/menu and attach secondary (right-click) gesture
            let units_button = MenuButton::new();
            units_button.set_has_frame(false);
            units_button.add_css_class("ruler-units-menu-button");

            let popover = Popover::new();
            popover.set_has_arrow(false);
            popover.add_css_class("menu-dropdown");
            let menu_box = GtkBox::new(Orientation::Vertical, 0);
            menu_box.add_css_class("menu-dropdown-body");

            let add_unit_item = |label: &str, unit: RulerUnit, shell_state: Rc<ShellUiState>| {
                let item = Button::with_label(label);
                item.add_css_class("menu-button");
                let pop = popover.clone();
                item.connect_clicked(move |_| {
                    shell_state.ruler_unit.set(unit);
                    shell_state.horizontal_ruler_label.queue_draw();
                    shell_state.vertical_ruler_label.queue_draw();
                    pop.popdown();
                });
                menu_box.append(&item);
            };

            // Note: we capture shell_state by cloning after it's created
            // We'll set items after shell_state is available below.

            popover.set_child(Some(&menu_box));
            units_button.set_popover(Some(&popover));
            shell_state
                .ruler_units_menu_button
                .replace(Some(units_button.clone()));

            // Attach secondary click gestures to show the units menu
            let h_click = GestureClick::new();
            h_click.set_button(gdk::BUTTON_SECONDARY);
            {
                let menu_button = units_button.clone();
                h_click.connect_pressed(move |gesture, _, _, _| {
                    menu_button.popup();
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                });
            }
            horizontal.add_controller(h_click.clone());

            let v_click = GestureClick::new();
            v_click.set_button(gdk::BUTTON_SECONDARY);
            {
                let menu_button = units_button.clone();
                v_click.connect_pressed(move |gesture, _, _, _| {
                    menu_button.popup();
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                });
            }
            vertical.add_controller(v_click.clone());

            // Now populate the unit items with access to shell_state
            {
                let ss = shell_state.clone();
                add_unit_item("Pixels", RulerUnit::Pixels, ss.clone());
                add_unit_item("Inches", RulerUnit::Inches, ss.clone());
                add_unit_item("Centimeters", RulerUnit::Centimeters, ss.clone());
            }
        }

        shell_state.connect_sidebar_tabs();
        let initial_snapshot = shell_state.controller.borrow().snapshot();
        shell_chrome::refresh_tool_options_bar(&shell_state, &initial_snapshot);
        shell_state
    }

    fn bump_ui_revision(&self) {
        self.ui_revision.set(self.ui_revision.get().wrapping_add(1));
    }

    fn connect_sidebar_tabs(self: &Rc<Self>) {
        for (index, button) in self.history_tab_buttons.iter().enumerate() {
            let shell_state = self.clone();
            button.connect_clicked(move |_| {
                let tab = match index {
                    0 => RightSidebarTopTab::History,
                    _ => RightSidebarTopTab::Swatches,
                };
                shell_state.set_top_dock_tab(tab);
            });
        }

        for (index, button) in self.layers_tab_buttons.iter().enumerate() {
            let shell_state = self.clone();
            button.connect_clicked(move |_| {
                let tab = match index {
                    0 => RightSidebarBottomTab::Layers,
                    1 => RightSidebarBottomTab::Channels,
                    _ => RightSidebarBottomTab::Paths,
                };
                shell_state.set_bottom_dock_tab(tab);
            });
        }
    }

    pub(crate) fn set_top_dock_tab(&self, tab: RightSidebarTopTab) {
        if self.active_top_dock_tab.get() != tab {
            self.active_top_dock_tab.set(tab);
            self.bump_ui_revision();
        }
    }

    pub(crate) fn active_top_dock_tab(&self) -> RightSidebarTopTab {
        self.active_top_dock_tab.get()
    }

    pub(crate) fn set_bottom_dock_tab(&self, tab: RightSidebarBottomTab) {
        if self.active_bottom_dock_tab.get() != tab {
            self.active_bottom_dock_tab.set(tab);
            self.bump_ui_revision();
        }
    }

    pub(crate) fn active_bottom_dock_tab(&self) -> RightSidebarBottomTab {
        self.active_bottom_dock_tab.get()
    }

    pub(crate) fn toggle_context_panel(&self, panel: ContextDockPanel) {
        let next = if self.active_context_panel.get() == Some(panel) {
            None
        } else {
            Some(panel)
        };
        if self.active_context_panel.get() != next {
            self.active_context_panel.set(next);
            self.bump_ui_revision();
        }
    }

    pub(crate) fn active_context_panel(&self) -> Option<ContextDockPanel> {
        self.active_context_panel.get()
    }

    pub(crate) fn restore_right_sidebar_defaults(&self) {
        self.active_top_dock_tab.set(RightSidebarTopTab::History);
        self.active_bottom_dock_tab
            .set(RightSidebarBottomTab::Layers);
        self.active_context_panel.set(None);
        self.bump_ui_revision();
    }

    fn handle_shortcut(self: &Rc<Self>, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
        let is_control = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        let is_shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
        let has_menu_navigation_modifier = modifiers.intersects(
            gdk::ModifierType::ALT_MASK
                | gdk::ModifierType::META_MASK
                | gdk::ModifierType::SUPER_MASK
                | gdk::ModifierType::HYPER_MASK,
        );
        let key_char = key
            .to_unicode()
            .map(|character| character.to_ascii_lowercase());

        if self.automation_shortcuts_enabled
            && is_control
            && is_shift
            && self.handle_automation_shortcut(key, key_char)
        {
            return true;
        }
        if is_control && self.handle_ctrl_shortcut(key, key_char, is_shift) {
            return true;
        }
        if self.handle_mode_exit_key(key) {
            return true;
        }
        if has_menu_navigation_modifier {
            return false;
        }
        self.handle_tool_select_shortcut(key_char)
    }

    fn handle_automation_shortcut(self: &Rc<Self>, key: gdk::Key, key_char: Option<char>) -> bool {
        if matches!(key, gdk::Key::F1) {
            self.controller.borrow_mut().add_layer();
            return true;
        }
        if matches!(key, gdk::Key::F2) {
            self.controller.borrow_mut().duplicate_active_layer();
            return true;
        }
        if matches!(key, gdk::Key::F3) {
            self.controller.borrow_mut().delete_active_layer();
            return true;
        }
        if matches!(key, gdk::Key::F4)
            && let Some(layer_id) = self.selected_layer_id()
        {
            self.controller
                .borrow_mut()
                .toggle_layer_visibility(layer_id);
            return true;
        }
        if matches!(key, gdk::Key::F5 | gdk::Key::bracketleft) {
            self.controller
                .borrow_mut()
                .previous_active_layer_blend_mode();
            return true;
        }
        if matches!(key, gdk::Key::F6 | gdk::Key::bracketright) {
            self.controller.borrow_mut().next_active_layer_blend_mode();
            return true;
        }
        if matches!(key, gdk::Key::F7 | gdk::Key::minus | gdk::Key::KP_Subtract) {
            self.controller.borrow_mut().decrease_active_layer_opacity();
            return true;
        }
        if matches!(
            key,
            gdk::Key::F8 | gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add
        ) {
            self.controller.borrow_mut().increase_active_layer_opacity();
            return true;
        }
        if matches!(key, gdk::Key::F9 | gdk::Key::Up) {
            self.controller.borrow_mut().move_active_layer_up();
            return true;
        }
        if matches!(key, gdk::Key::F10 | gdk::Key::Down) {
            self.controller.borrow_mut().move_active_layer_down();
            return true;
        }
        if matches!(key, gdk::Key::F11) {
            self.controller.borrow_mut().begin_transform();
            return true;
        }
        if matches!(key, gdk::Key::Page_Up | gdk::Key::KP_Page_Up) {
            self.controller.borrow_mut().scale_transform_up();
            return true;
        }
        if matches!(key, gdk::Key::Page_Down | gdk::Key::KP_Page_Down) {
            self.controller.borrow_mut().scale_transform_down();
            return true;
        }
        if matches!(key, gdk::Key::bracketleft) {
            self.controller.borrow_mut().rotate_transform_left();
            return true;
        }
        if matches!(key, gdk::Key::bracketright) {
            self.controller.borrow_mut().rotate_transform_right();
            return true;
        }
        if matches!(key, gdk::Key::F12) {
            self.controller.borrow_mut().scale_transform_x_up();
            return true;
        }
        if matches!(key, gdk::Key::F13) {
            self.controller.borrow_mut().scale_transform_y_up();
            return true;
        }
        match key_char {
            Some('x') => {
                self.controller.borrow_mut().swap_colors();
                true
            }
            Some('r') => {
                self.controller.borrow_mut().reset_colors();
                true
            }
            Some(digit @ '1'..='9') => {
                let layer_index = (digit as u8 - b'1') as usize;
                if let Some(layer_id) = self.nth_layer_id(layer_index) {
                    self.controller.borrow_mut().select_layer(layer_id);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn handle_ctrl_shortcut(
        self: &Rc<Self>,
        key: gdk::Key,
        key_char: Option<char>,
        is_shift: bool,
    ) -> bool {
        if matches!(key, gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add) {
            self.canvas_state.borrow_mut().zoom_in();
            return true;
        }
        if matches!(key, gdk::Key::minus | gdk::Key::KP_Subtract) {
            self.canvas_state.borrow_mut().zoom_out();
            return true;
        }
        match key_char {
            Some('z') if is_shift => {
                self.controller.borrow_mut().redo();
                true
            }
            Some('s') if is_shift => {
                self.request_project_save_as();
                true
            }
            Some('z') => {
                self.controller.borrow_mut().undo();
                true
            }
            Some('y') => {
                self.controller.borrow_mut().redo();
                true
            }
            Some('o') => {
                self.request_open_project();
                true
            }
            Some('s') => {
                self.request_project_save();
                true
            }
            Some('d') => {
                self.controller.borrow_mut().clear_selection();
                true
            }
            Some('i') => {
                self.controller.borrow_mut().invert_selection();
                true
            }
            Some('=') | Some('+') => {
                self.canvas_state.borrow_mut().zoom_in();
                true
            }
            Some('-') => {
                self.canvas_state.borrow_mut().zoom_out();
                true
            }
            Some('0') => {
                self.canvas_state.borrow_mut().fit_to_view();
                true
            }
            _ => false,
        }
    }

    fn handle_mode_exit_key(&self, key: gdk::Key) -> bool {
        match key {
            gdk::Key::Return | gdk::Key::KP_Enter => {
                let snapshot = self.controller.borrow().snapshot();
                if snapshot.text.editing {
                    self.controller.borrow_mut().commit_text_session();
                    return true;
                }
                if snapshot.transform_active {
                    self.controller.borrow_mut().commit_transform();
                    return true;
                }
                false
            }
            gdk::Key::Escape => {
                let snapshot = self.controller.borrow().snapshot();
                if snapshot.text.editing {
                    self.controller.borrow_mut().cancel_text_session();
                    return true;
                }
                if snapshot.transform_active {
                    self.controller.borrow_mut().cancel_transform();
                    return true;
                }
                if snapshot.selection_rect.is_some() {
                    self.controller.borrow_mut().clear_selection();
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn handle_tool_select_shortcut(&self, key_char: Option<char>) -> bool {
        let tool = match key_char {
            Some('v') => ShellToolKind::Move,
            Some('m') => ShellToolKind::RectangularMarquee,
            Some('l') => ShellToolKind::Lasso,
            Some('i') => ShellToolKind::Text,
            Some('t') => ShellToolKind::Transform,
            Some('b') => ShellToolKind::Brush,
            Some('e') => ShellToolKind::Eraser,
            Some('h') => ShellToolKind::Hand,
            Some('z') => ShellToolKind::Zoom,
            _ => return false,
        };
        self.controller.borrow_mut().select_tool(tool);
        true
    }

    fn selected_layer_id(&self) -> Option<LayerId> {
        self.controller
            .borrow()
            .snapshot()
            .layers
            .iter()
            .find(|layer| layer.is_selected || layer.is_active)
            .and_then(|layer| layer.layer_id)
    }

    fn nth_layer_id(&self, index: usize) -> Option<LayerId> {
        self.controller
            .borrow()
            .snapshot()
            .layers
            .iter()
            .filter_map(|layer| layer.layer_id)
            .nth(index)
    }

    fn attach_window(&self, window: ApplicationWindow) {
        self.window.replace(Some(window));
    }

    fn focus_canvas(&self) {
        self.canvas_picture.grab_focus();
    }

    fn present_recovery_prompt(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        let Some(window) = self.window.borrow().as_ref().cloned() else {
            return;
        };
        if self.recovery_prompt_visible.replace(true) {
            return;
        }

        let recovery_path = snapshot
            .recovery_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "the autosave file".to_string());

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::None)
            .text("Recovered state is available")
            .secondary_text(format!(
                "PhotoTux found autosaved work in {}.\n\nRecover it to restore the latest autosaved state, or discard it to keep the current document.",
                recovery_path
            ))
            .build();
        dialog.add_button("Discard Recovery", ResponseType::Reject);
        dialog.add_button("Recover", ResponseType::Accept);

        let shell_state = self.clone();
        dialog.connect_response(move |dialog, response| {
            shell_state.recovery_prompt_visible.set(false);
            match response {
                ResponseType::Accept => {
                    shell_state.controller.borrow_mut().load_recovery_document()
                }
                ResponseType::Reject => shell_state
                    .controller
                    .borrow_mut()
                    .discard_recovery_document(),
                _ => {}
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn present_close_prompt(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        let Some(window) = self.window.borrow().as_ref().cloned() else {
            return;
        };
        if self.close_prompt_visible.replace(true) {
            return;
        }

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::None)
            .text("Save changes before closing?")
            .secondary_text(format!(
                "{} has unsaved changes. Save them before closing, discard them, or cancel and keep editing.",
                snapshot.document_title
            ))
            .build();
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Discard Changes", ResponseType::Reject);
        dialog.add_button("Save", ResponseType::Accept);

        let shell_state = self.clone();
        dialog.connect_response(move |dialog, response| {
            shell_state.close_prompt_visible.set(false);
            match response {
                ResponseType::Accept => shell_state.request_project_save_for_close(),
                ResponseType::Reject => {
                    shell_state
                        .controller
                        .borrow_mut()
                        .discard_recovery_document();
                    shell_state.allow_close_once.set(true);
                    if let Some(window) = shell_state.window.borrow().as_ref() {
                        window.close();
                    }
                }
                _ => {}
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn present_document_replacement_prompt(
        self: &Rc<Self>,
        snapshot: &ShellSnapshot,
        action: PendingDocumentAction,
    ) {
        let Some(window) = self.window.borrow().as_ref().cloned() else {
            self.perform_document_replacement(action);
            return;
        };
        if self.replace_prompt_visible.replace(true) {
            return;
        }

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::None)
            .text(action.prompt_title())
            .secondary_text(action.prompt_detail(&snapshot.document_title))
            .build();
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Discard Changes", ResponseType::Reject);
        dialog.add_button("Save", ResponseType::Accept);

        let shell_state = self.clone();
        dialog.connect_response(move |dialog, response| {
            shell_state.replace_prompt_visible.set(false);
            match response {
                ResponseType::Accept => shell_state.request_project_save_for_action(action.clone()),
                ResponseType::Reject => {
                    shell_state
                        .controller
                        .borrow_mut()
                        .discard_recovery_document();
                    shell_state.perform_document_replacement(action.clone());
                }
                _ => {}
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn present_import_report(self: &Rc<Self>, report: &ShellImportReport) {
        let Some(window) = self.window.borrow().as_ref().cloned() else {
            return;
        };
        if self.import_report_visible.replace(true) {
            return;
        }

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Info)
            .buttons(ButtonsType::Close)
            .text(&report.title)
            .secondary_text(format_import_report_details(report))
            .build();

        let shell_state = self.clone();
        dialog.connect_response(move |dialog, _| {
            shell_state.import_report_visible.set(false);
            dialog.destroy();
        });

        dialog.show();
    }

    fn present_shell_alert(self: &Rc<Self>, alert: &ShellAlert) {
        let Some(window) = self.window.borrow().as_ref().cloned() else {
            return;
        };
        if self.alert_dialog_visible.replace(true) {
            return;
        }

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(match alert.tone {
                ShellAlertTone::Info => MessageType::Info,
                ShellAlertTone::Warning => MessageType::Warning,
                ShellAlertTone::Error => MessageType::Error,
            })
            .buttons(ButtonsType::Close)
            .text(&alert.title)
            .secondary_text({
                let mut text = alert.body.clone();
                if let Some(secondary) = format_shell_alert_secondary_text(alert) {
                    text.push_str("\n\n");
                    text.push_str(&secondary);
                }
                text
            })
            .build();

        let shell_state = self.clone();
        dialog.connect_response(move |dialog, _| {
            shell_state.alert_dialog_visible.set(false);
            dialog.destroy();
        });

        dialog.show();
    }

    fn present_text_dialog(self: &Rc<Self>, text: &ShellTextSnapshot) {
        let Some(window) = self.window.borrow().as_ref().cloned() else {
            return;
        };
        if self.text_dialog_visible.replace(true) {
            return;
        }

        let dialog = Dialog::builder()
            .transient_for(&window)
            .modal(true)
            .resizable(false)
            .title(if text.is_new_layer {
                "Create Text Layer"
            } else {
                "Edit Text Layer"
            })
            .build();
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button(
            if text.is_new_layer { "Create" } else { "Apply" },
            ResponseType::Accept,
        );
        dialog.set_default_response(ResponseType::Accept);

        let body = GtkBox::new(Orientation::Vertical, 8);
        body.set_margin_top(12);
        body.set_margin_bottom(12);
        body.set_margin_start(12);
        body.set_margin_end(12);

        let title = Label::new(Some(&format!(
            "{} at {}, {}",
            text.layer_name, text.origin_x, text.origin_y
        )));
        title.set_xalign(0.0);
        title.add_css_class("panel-row");
        body.append(&title);

        let content_entry = Entry::new();
        content_entry.set_hexpand(true);
        content_entry.set_placeholder_text(Some("Text content"));
        content_entry.set_text(&text.content);
        {
            let dialog = dialog.clone();
            content_entry.connect_activate(move |_| dialog.response(ResponseType::Accept));
        }
        body.append(&content_entry);

        let font_row = GtkBox::new(Orientation::Horizontal, 6);
        let font_family = ComboBoxText::new();
        for family in [text.font_family.as_str(), "Bitmap Sans"] {
            if font_family
                .active_text()
                .as_ref()
                .map(|value| value.as_str())
                != Some(family)
            {
                font_family.append_text(family);
            }
        }
        font_family.set_active(Some(0));
        font_row.append(&font_family);

        let font_size = SpinButton::with_range(8.0, 256.0, 1.0);
        font_size.set_value(text.font_size_px as f64);
        font_row.append(&font_size);
        body.append(&font_row);

        let metrics_row = GtkBox::new(Orientation::Horizontal, 6);
        let line_height = SpinButton::with_range(80.0, 300.0, 5.0);
        line_height.set_value(text.line_height_percent as f64);
        metrics_row.append(&line_height);

        let letter_spacing = SpinButton::with_range(-8.0, 32.0, 1.0);
        letter_spacing.set_value(text.letter_spacing as f64);
        metrics_row.append(&letter_spacing);

        let alignment = ComboBoxText::new();
        alignment.append(Some("left"), "Left");
        alignment.append(Some("center"), "Center");
        alignment.append(Some("right"), "Right");
        alignment.set_active_id(Some(match text.alignment {
            ShellTextAlignment::Left => "left",
            ShellTextAlignment::Center => "center",
            ShellTextAlignment::Right => "right",
        }));
        metrics_row.append(&alignment);
        body.append(&metrics_row);

        let color_row = GtkBox::new(Orientation::Horizontal, 6);
        let color_r = SpinButton::with_range(0.0, 255.0, 1.0);
        color_r.set_value(text.fill_rgba[0] as f64);
        color_row.append(&color_r);
        let color_g = SpinButton::with_range(0.0, 255.0, 1.0);
        color_g.set_value(text.fill_rgba[1] as f64);
        color_row.append(&color_g);
        let color_b = SpinButton::with_range(0.0, 255.0, 1.0);
        color_b.set_value(text.fill_rgba[2] as f64);
        color_row.append(&color_b);
        let color_a = SpinButton::with_range(0.0, 255.0, 1.0);
        color_a.set_value(text.fill_rgba[3] as f64);
        color_row.append(&color_a);
        body.append(&color_row);

        dialog.content_area().append(&body);

        let controller = self.controller.clone();
        let fallback_font_family = text.font_family.clone();
        let controls = ShellTextDialogControls {
            content_entry: content_entry.clone(),
            font_family: font_family.clone(),
            font_size: font_size.clone(),
            line_height: line_height.clone(),
            letter_spacing: letter_spacing.clone(),
            alignment: alignment.clone(),
            color_r: color_r.clone(),
            color_g: color_g.clone(),
            color_b: color_b.clone(),
            color_a: color_a.clone(),
        };
        let sync: ShellTextUpdateCallback = Rc::new({
            let controls = controls.clone();
            move || {
                controller
                    .borrow_mut()
                    .update_text_session(controls.build_update(&fallback_font_family));
            }
        });
        controls.connect_update_signals(&sync);

        let escape_controller = EventControllerKey::new();
        {
            let dialog = dialog.clone();
            escape_controller.connect_key_pressed(move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    dialog.response(ResponseType::Cancel);
                    return glib::Propagation::Stop;
                }
                if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
                    dialog.response(ResponseType::Accept);
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }
        dialog.add_controller(escape_controller);

        let shell_state = self.clone();
        dialog.connect_response(move |dialog, response| {
            shell_state.text_dialog_visible.set(false);
            match response {
                ResponseType::Accept => shell_state.controller.borrow_mut().commit_text_session(),
                _ => shell_state.controller.borrow_mut().cancel_text_session(),
            }
            dialog.destroy();
        });

        dialog.present();
    }

    fn handle_close_request(self: &Rc<Self>) -> bool {
        if self.allow_close_once.replace(false) {
            return false;
        }

        let snapshot = self.controller.borrow().snapshot();
        if !snapshot.dirty {
            return false;
        }

        self.present_close_prompt(&snapshot);
        true
    }

    fn request_project_save_for_close(self: &Rc<Self>) {
        let snapshot = self.controller.borrow().snapshot();
        if snapshot.project_path.is_some() {
            self.pending_close_after_save.set(true);
            self.controller.borrow_mut().save_document();
            return;
        }

        let Some(window) = self.window.borrow().as_ref().cloned() else {
            self.pending_close_after_save.set(true);
            self.controller.borrow_mut().save_document();
            return;
        };

        let shell_state = self.clone();
        let on_requested: Rc<dyn Fn()> = Rc::new(move || {
            shell_state.pending_close_after_save.set(true);
        });
        file_workflow::choose_save_project_path_with_callback(
            &window,
            self.controller.clone(),
            Some(on_requested),
        );
    }

    fn request_project_save_for_action(self: &Rc<Self>, action: PendingDocumentAction) {
        let snapshot = self.controller.borrow().snapshot();
        if snapshot.project_path.is_some() {
            self.pending_document_action_after_save
                .replace(Some(action));
            self.controller.borrow_mut().save_document();
            return;
        }

        let Some(window) = self.window.borrow().as_ref().cloned() else {
            self.pending_document_action_after_save
                .replace(Some(action));
            self.controller.borrow_mut().save_document();
            return;
        };

        let shell_state = self.clone();
        let action_to_store = action.clone();
        let on_requested: Rc<dyn Fn()> = Rc::new(move || {
            shell_state
                .pending_document_action_after_save
                .replace(Some(action_to_store.clone()));
        });
        file_workflow::choose_save_project_path_with_callback(
            &window,
            self.controller.clone(),
            Some(on_requested),
        );
    }

    fn request_document_replacement(self: &Rc<Self>, action: PendingDocumentAction) {
        let snapshot = self.controller.borrow().snapshot();
        if !snapshot.dirty {
            self.perform_document_replacement(action);
            return;
        }

        self.present_document_replacement_prompt(&snapshot, action);
    }

    fn request_open_project(self: &Rc<Self>) {
        self.request_document_replacement(PendingDocumentAction::ChooseOpenProject);
    }

    fn request_import_image(self: &Rc<Self>) {
        self.request_document_replacement(PendingDocumentAction::ChooseImportImage);
    }

    fn perform_document_replacement_choice(
        self: &Rc<Self>,
        action: &PendingDocumentAction,
    ) -> bool {
        let chooser: fn(&ApplicationWindow, Rc<ShellUiState>) = match action {
            PendingDocumentAction::ChooseOpenProject => file_workflow::choose_open_project,
            PendingDocumentAction::ChooseImportImage => file_workflow::choose_import_image,
            _ => return false,
        };

        let Some(window) = self.window.borrow().as_ref().cloned() else {
            return true;
        };

        chooser(&window, self.clone());
        true
    }

    fn perform_document_replacement(self: &Rc<Self>, action: PendingDocumentAction) {
        if self.perform_document_replacement_choice(&action) {
            return;
        }

        match action {
            PendingDocumentAction::OpenProject(path) => {
                self.controller.borrow_mut().open_document(path);
            }
            PendingDocumentAction::ImportImage(path) => {
                self.controller.borrow_mut().import_image(path);
            }
            PendingDocumentAction::ChooseOpenProject | PendingDocumentAction::ChooseImportImage => {
            }
        }
    }

    fn request_project_save(&self) {
        let snapshot = self.controller.borrow().snapshot();
        if snapshot.project_path.is_some() {
            self.controller.borrow_mut().save_document();
            return;
        }

        if let Some(window) = self.window.borrow().as_ref() {
            file_workflow::choose_save_project_path(window, self.controller.clone());
        } else {
            self.controller.borrow_mut().save_document();
        }
    }

    fn request_project_save_as(&self) {
        if let Some(window) = self.window.borrow().as_ref() {
            file_workflow::choose_save_project_path(window, self.controller.clone());
        } else {
            let snapshot = self.controller.borrow().snapshot();
            if let Some(path) = snapshot.project_path {
                self.controller.borrow_mut().save_document_as(path);
            } else {
                self.controller.borrow_mut().save_document();
            }
        }
    }

    fn refresh_snapshot_views(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        shell_chrome::refresh_tool_options_bar(self, snapshot);
        self.refresh_tool_buttons(snapshot);
        self.refresh_right_sidebar_chrome();
        self.refresh_color_panel(snapshot);
        self.refresh_properties_panel(snapshot);
        self.refresh_brush_panel(snapshot);
        self.refresh_text_panel(snapshot);
        self.refresh_layers_panel(snapshot);
        self.refresh_history_panel(snapshot);
        self.refresh_contextual_task_bar(snapshot);
    }

    fn refresh_right_sidebar_chrome(&self) {
        for (index, button) in self.history_tab_buttons.iter().enumerate() {
            let is_active = match index {
                0 => self.active_top_dock_tab.get() == RightSidebarTopTab::History,
                _ => self.active_top_dock_tab.get() == RightSidebarTopTab::Swatches,
            };
            set_sidebar_tab_active(button, is_active);
        }

        for (index, button) in self.layers_tab_buttons.iter().enumerate() {
            let is_active = match index {
                0 => self.active_bottom_dock_tab.get() == RightSidebarBottomTab::Layers,
                1 => self.active_bottom_dock_tab.get() == RightSidebarBottomTab::Channels,
                _ => self.active_bottom_dock_tab.get() == RightSidebarBottomTab::Paths,
            };
            set_sidebar_tab_active(button, is_active);
        }

        let active_context = self.active_context_panel.get();
        self.color_group
            .set_visible(active_context == Some(ContextDockPanel::Color));
        self.properties_group
            .set_visible(active_context == Some(ContextDockPanel::Properties));
        self.history_group.set_visible(true);
        self.brush_group
            .set_visible(active_context == Some(ContextDockPanel::Brush));
        self.text_group
            .set_visible(active_context == Some(ContextDockPanel::Text));
        if let Some(host) = self.context_panel_host.borrow().as_ref() {
            host.set_visible(matches!(
                active_context,
                Some(
                    ContextDockPanel::Color
                        | ContextDockPanel::Properties
                        | ContextDockPanel::Brush
                        | ContextDockPanel::Text
                )
            ));
        }

        for (panel, button) in self.context_toolbar_buttons.borrow().iter() {
            if active_context == Some(*panel) {
                button.add_css_class("dock-icon-button-active");
            } else {
                button.remove_css_class("dock-icon-button-active");
            }
        }
    }

    fn refresh_contextual_task_bar(&self, snapshot: &ShellSnapshot) {
        let has_selection = snapshot.selection_rect.is_some();
        let can_edit_pixels = !snapshot.text.selected;
        let can_edit_mask = !snapshot.text.selected && snapshot.active_layer_has_mask;
        let editing_mask = snapshot.active_edit_target_name == "Layer Mask";

        self.contextual_fit_button.set_sensitive(true);
        self.contextual_zoom_out_button.set_sensitive(true);
        self.contextual_zoom_in_button.set_sensitive(true);
        self.contextual_clear_selection_button
            .set_sensitive(has_selection);
        self.contextual_invert_selection_button
            .set_sensitive(has_selection);
        self.contextual_edit_pixels_button
            .set_sensitive(can_edit_pixels);
        self.contextual_edit_mask_button
            .set_sensitive(can_edit_mask);
        if editing_mask {
            self.contextual_edit_pixels_button
                .remove_css_class("contextual-task-button-primary");
            self.contextual_edit_mask_button
                .add_css_class("contextual-task-button-primary");
        } else {
            self.contextual_edit_mask_button
                .remove_css_class("contextual-task-button-primary");
            self.contextual_edit_pixels_button
                .add_css_class("contextual-task-button-primary");
        }
    }

    fn current_refresh_snapshot(&self) -> ShellSnapshot {
        self.last_snapshot
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.controller.borrow().snapshot())
    }

    fn present_pending_alert(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        if let Some(alert) = snapshot.latest_alert.as_ref() {
            let already_presented = self.presented_alert_id.get() == Some(alert.id);
            if !already_presented && !self.alert_dialog_visible.get() {
                self.presented_alert_id.set(Some(alert.id));
                self.present_shell_alert(alert);
            }
        }
    }

    fn finish_pending_close_after_save(&self, snapshot: &ShellSnapshot) {
        if self.pending_close_after_save.get() && !snapshot.dirty {
            self.pending_close_after_save.set(false);
            self.allow_close_once.set(true);
            if let Some(window) = self.window.borrow().as_ref() {
                window.close();
            }
        }
    }

    fn perform_pending_document_action_after_save(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        if !snapshot.dirty
            && let Some(action) = self.pending_document_action_after_save.borrow_mut().take()
        {
            self.perform_document_replacement(action);
        }
    }

    fn present_pending_recovery_prompt(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        let should_prompt_recovery = snapshot.recovery_offer_pending
            && self.prompted_recovery_path.borrow().as_ref() != snapshot.recovery_path.as_ref();
        if should_prompt_recovery && !self.recovery_prompt_visible.get() {
            self.prompted_recovery_path
                .replace(snapshot.recovery_path.clone());
            self.present_recovery_prompt(snapshot);
        }
    }

    fn present_pending_text_dialog(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        if snapshot.text.editing
            && snapshot.text.request_id.is_some()
            && self.presented_text_request_id.get() != snapshot.text.request_id
            && !self.text_dialog_visible.get()
        {
            self.presented_text_request_id.set(snapshot.text.request_id);
            self.present_text_dialog(&snapshot.text);
        }
    }

    fn present_pending_import_report(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        if let Some(report) = snapshot.latest_import_report.as_ref() {
            let already_presented = self.presented_import_report_id.get() == Some(report.id);
            if !already_presented && !self.import_report_visible.get() {
                self.presented_import_report_id.set(Some(report.id));
                self.present_import_report(report);
            }
        }
    }

    fn process_post_refresh_actions(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        self.present_pending_alert(snapshot);
        self.finish_pending_close_after_save(snapshot);
        self.perform_pending_document_action_after_save(snapshot);
        self.present_pending_recovery_prompt(snapshot);
        self.present_pending_text_dialog(snapshot);
        self.present_pending_import_report(snapshot);
    }

    fn refresh(self: &Rc<Self>) {
        self.controller.borrow_mut().poll_background_tasks();
        let snapshot = self.controller.borrow().snapshot();
        let zoom_percent = self.canvas_state.borrow().zoom_percent();
        let snapshot_changed = self.last_snapshot.borrow().as_ref() != Some(&snapshot);
        let zoom_changed = *self.last_zoom_percent.borrow() != zoom_percent;
        let ui_changed = self.ui_revision.get() != self.last_ui_revision.get();

        if !snapshot_changed && !zoom_changed && !ui_changed {
            return;
        }

        let tab_title = if snapshot.dirty {
            format!("{} *", snapshot.document_title)
        } else {
            snapshot.document_title.clone()
        };
        self.document_tab_label.set_label(&tab_title);
        self.document_tab_label.set_tooltip_text(Some(
            &snapshot
                .project_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "Unsaved project".to_string()),
        ));
        self.document_tab_meta_label
            .set_label(&format!("@ {}% ({})", zoom_percent, "RGB/8"));
        self.status_doc.set_label(&format!(
            "Doc: {} x {}",
            snapshot.canvas_size.width, snapshot.canvas_size.height
        ));
        self.menu_zoom_label
            .set_label(&format!("{}%", zoom_percent));
        self.status_zoom
            .set_label(&format!("Zoom: {}%", zoom_percent));
        self.status_cursor.set_label(&shell_status_hint(&snapshot));
        self.status_notice.set_label(&shell_notice_text(&snapshot));
        apply_status_notice_style(&self.status_notice, &snapshot);
        self.status_mode.set_label("RGB/8");
        self.canvas_info_label.set_label(&format!(
            "{} @ {}% ({})",
            snapshot.document_title, zoom_percent, "RGB/8"
        ));
        self.horizontal_ruler_label
            .set_content_width(self.canvas_picture.width().max(1));
        self.vertical_ruler_label
            .set_content_height(self.canvas_picture.height().max(1));
        // Trigger ruler redraws (draw callbacks read viewport info directly)
        self.horizontal_ruler_label.queue_draw();
        self.vertical_ruler_label.queue_draw();

        if snapshot_changed || ui_changed {
            self.refresh_snapshot_views(&snapshot);
            self.last_snapshot.replace(Some(snapshot));
        }

        self.last_ui_revision.set(self.ui_revision.get());

        let current_snapshot = self.current_refresh_snapshot();
        self.process_post_refresh_actions(&current_snapshot);

        self.last_zoom_percent.replace(zoom_percent);
    }
}

fn install_theme() {
    let provider = CssProvider::new();
    provider.load_from_data(THEME_CSS);

    if let Some(display) = gdk::Display::default() {
        IconTheme::for_display(&display).add_resource_path("/com/phototux/icons");
        // Also expose the logo assets so `APP_WINDOW_ICON_NAME` can resolve
        // the bundled logo resource as an icon name.
        IconTheme::for_display(&display).add_resource_path("/com/phototux/assets/logo");
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn set_sidebar_tab_active(button: &Button, active: bool) {
    if active {
        button.add_css_class("panel-tab-active");
    } else {
        button.remove_css_class("panel-tab-active");
    }
}

fn wire_window_shortcuts(window: &ApplicationWindow, shell_state: Rc<ShellUiState>) {
    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        if shell_state.handle_shortcut(key, modifiers) {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);
}

fn wire_window_close_request(window: &ApplicationWindow, shell_state: Rc<ShellUiState>) {
    window.connect_close_request(move |_| {
        if shell_state.handle_close_request() {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
}

const THEME_CSS: &str = r#"
.app-root {
    background: #1a1a1a;
    color: #e0e0e0;
    font-family: "Inter", "IBM Plex Sans", "Noto Sans", system-ui, sans-serif;
    font-size: 11px;
}

.titlebar {
    min-height: 26px;
    background: #202020;
    color: #e0e0e0;
    border-bottom: 1px solid #3a3a3a;
}

.app-brand {
    padding: 0 10px;
    min-height: 26px;
    border-right: 1px solid #3a3a3a;
}

.titlebar-actions {
    padding-right: 8px;
}

.titlebar-app-name {
    font-family: "Exo Bold", "Inter", "IBM Plex Sans", "Noto Sans", system-ui, sans-serif;
    font-weight: 700;
    font-size: 11px;
    color: #e0e0e0;
    letter-spacing: 0.02em;
}

.titlebar-icon,
.remix-icon {
    min-width: 12px;
    min-height: 12px;
    -gtk-icon-style: symbolic;
    -gtk-icon-shadow: none;
}

.titlebar-icon {
    color: #dfe5ee;
}

.remix-icon {
    color: currentColor;
}

.chrome-button,
.menu-button,
.tool-chip,
.tool-button,
.panel-tab,
.dock-icon-button,
.color-chip {
    background: transparent;
    color: #e0e0e0;
    border-radius: 3px;
    border: none;
    padding: 2px 6px;
    transition: all 100ms ease-in-out;
}

.document-tab-active {
    background: #383838;
    color: #e0e0e0;
    border-radius: 3px 3px 0 0;
    border: 1px solid #4a4a4a;
    border-bottom-color: #383838;
    padding: 2px 8px;
    font-weight: 600;
    transition: all 100ms ease-in-out;
}

.chrome-button:hover,
.tool-chip:hover,
.tool-button:hover,
.panel-tab:hover,
.dock-icon-button:hover {
    background: #383838;
    border: 1px solid #4a4a4a;
}

.chrome-button image,
.menu-button image,
menubutton.menu-button > button.toggle image,
.tool-chip image,
.tool-button image,
.dock-icon-button image,
.chrome-icon-button image,
.layer-visibility-button image,
.swatch-stack-action image {
    color: currentColor;
    -gtk-icon-style: symbolic;
    -gtk-icon-shadow: none;
}

.tool-chip {
    background: transparent;
    color: #bfc6d0;
    padding: 2px 7px;
}

.tool-chip:hover {
    color: #e0e0e0;
}

.tool-chip:disabled,
.tool-chip-icon-only:disabled {
    background: transparent;
    border-color: transparent;
    color: #6f7680;
    opacity: 1;
}

.tool-chip:disabled .icon-label-text,
.tool-chip:disabled image,
.tool-chip-icon-only:disabled image {
    color: #6f7680;
    opacity: 0.55;
}

.workspace-chip {
    min-height: 18px;
    padding: 0 8px;
    background: #303030;
    border-color: #3a3a3a;
    color: #999999;
}

.menu-button:active,
.tool-chip:active,
.tool-button:active {
    background: #232323;
}

.menu-bar {
    min-height: 30px;
    padding: 0 8px;
    background: #232323;
    border-bottom: 1px solid #3a3a3a;
}

.menu-divider {
    min-height: 18px;
    margin: 0 6px;
    color: #444444;
}

.menu-zoom-display {
    color: #a0a0a0;
    font-size: 12px;
    padding: 2px 8px;
}

.menu-button {
    background: transparent;
    border: none;
    border-radius: 3px;
    min-width: 0;
    padding: 2px 6px;
    color: #e0e0e0;
    box-shadow: none;
    outline: none;
}

menubutton.menu-button {
    padding: 0;
    margin: 0;
}

menubutton.menu-button > button.toggle {
    background: transparent;
    background-image: none;
    border: none;
    border-radius: 3px;
    min-width: 0;
    min-height: 0;
    padding: 2px 6px;
    color: #e0e0e0;
    box-shadow: none;
    outline: none;
    -gtk-icon-shadow: none;
}

.menu-button:hover,
.menu-button:active,
.menu-button:checked,
.menu-button:focus,
.menu-button:focus-visible {
    background: #383838;
    border: none;
    box-shadow: none;
    outline: none;
    color: #e0e0e0;
}

menubutton.menu-button > button.toggle:hover,
menubutton.menu-button > button.toggle:active,
menubutton.menu-button > button.toggle:checked,
menubutton.menu-button > button.toggle:focus,
menubutton.menu-button > button.toggle:focus-visible {
    background: #383838;
    background-image: none;
    border: none;
    box-shadow: none;
    outline: none;
    color: #e0e0e0;
    -gtk-icon-shadow: none;
}

.tool-options-bar {
    min-height: 38px;
    padding: 2px 10px;
    background: #2c2c2c;
    border-top: 1px solid #353535;
    border-bottom: 1px solid #171717;
}

.tool-options-identity {
    min-height: 32px;
    padding: 1px 6px 1px 0;
}

.tool-options-content {
    min-height: 32px;
    padding: 1px 0;
}

.tool-options-label {
    margin: 0 2px 0 2px;
    font-weight: 600;
    color: #e8e8e8;
}

.tool-options-identity .tool-options-label {
    font-size: 12px;
}

.tool-options-group {
    margin: 0 8px;
    min-height: 32px;
    padding: 0;
}

.tool-options-divider {
    margin: 9px 8px;
    color: #4a4a4a;
    min-height: 16px;
}

.tool-option-key {
    color: #b1b1b1;
    font-size: 11px;
    margin-top: 1px;
}

.tool-option-cluster {
    min-height: 26px;
}

.tool-option-box {
    background: #3b3b3b;
    border: 1px solid #474747;
    border-radius: 3px;
    padding: 0 8px;
    min-width: 64px;
    min-height: 24px;
}

.tool-option-value {
    color: #e4e4e4;
    font-size: 11px;
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.tool-options-icon {
    margin-left: 1px;
    margin-right: 1px;
}

.tool-option-button,
.tool-option-toggle-button,
.tool-option-icon-button {
    min-height: 24px;
    padding: 0 8px;
    margin: 1px 0;
    border-radius: 3px;
    border: 1px solid #474747;
    background: #3b3b3b;
    color: #e4e4e4;
}

.tool-option-button:hover,
.tool-option-toggle-button:hover,
.tool-option-icon-button:hover {
    background: #444444;
    border-color: #545454;
    color: #f0f0f0;
}

.tool-option-button:active,
.tool-option-toggle-button:active,
.tool-option-icon-button:active {
    background: #2f2f2f;
}

.tool-option-toggle-button-active {
    background: #4d4d4d;
    border-color: #656565;
    color: #ffffff;
}

.tool-option-icon-button {
    min-width: 24px;
    padding: 0;
}

.tool-option-icon-button image {
    color: currentColor;
    -gtk-icon-style: symbolic;
}

.template-dialog-content {
    min-width: 420px;
}

.template-dialog-title {
    font-weight: 600;
    font-size: 13px;
    color: #f2f4f7;
}

.template-dialog-body,
.template-dialog-secondary {
    color: #d0d4da;
    line-height: 1.35;
}

.template-dialog-secondary {
    color: #b7bec7;
}

.template-dialog-actions {
    margin-top: 6px;
}

.workspace-body {
    background: #1a1a1a;
}

.tool-rail {
    padding: 8px 0 10px 0;
    background: #383838;
    border-right: 1px solid #202020;
}

.tool-button {
    min-width: 32px;
    min-height: 32px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 2px;
    color: #d8d8d8;
}

.tool-button:hover {
    background: #464646;
    border-color: #5a5a5a;
    color: #e0e0e0;
}

.tool-button-corner-indicator {
    color: #9ca3ad;
    opacity: 0.92;
    font-size: 7px;
    line-height: 1;
}

.tool-button:hover .tool-button-corner-indicator,
.tool-button-active .tool-button-corner-indicator {
    color: #dfe8f8;
}

.tool-button-active {
    background: #2d6cb8;
    border: 1px solid #2d6cb8;
    color: #e0e0e0;
}

.tool-button-placeholder {
    color: #c9c9c9;
}

.tool-button-placeholder:hover {
    background: #434343;
    border-color: #565656;
}

.tool-separator {
    margin: 4px 8px;
    min-width: 24px;
    opacity: 1;
}

.tool-separator.horizontal {
    color: #4a4a4a;
}

.swatch-stack {
    margin-top: 8px;
    margin-bottom: 2px;
}

.color-chip {
    min-width: 18px;
    min-height: 18px;
    padding: 0;
    border-radius: 2px;
    border: 1px solid #666666;
    box-shadow: 0 1px 2px rgba(0,0,0,0.45);
}

.color-chip:hover {
    border-color: #3b8beb;
}

.swatch-fg {
    background: #d8dce3;
    color: #0E1116;
}

.swatch-bg {
    background: #111317;
    color: #E8ECF3;
}

.swatch-stack-actions {
    margin-bottom: 4px;
}

.swatch-stack-action {
    min-width: 14px;
    min-height: 14px;
    padding: 0;
    color: #e0e0e0;
}

.swatch-stack-action:hover {
    color: #eef2f8;
}

.document-region {
    background: #1a1a1a;
}

.document-tabs {
    min-height: 30px;
    padding: 4px 8px 0 8px;
    background: #2b2b2b;
    border-bottom: 1px solid #1e1e1e;
}

.document-tab-content {
    min-height: 24px;
    padding: 0 2px;
}

.document-tab-title {
    color: #e0e0e0;
    font-weight: 600;
}

.document-tab-meta {
    color: #98a0ab;
    font-size: 10px;
}

.document-tab-close {
    color: #7f8792;
    margin-left: 2px;
}

.document-tabs-spacer {
    min-width: 12px;
}

.document-workspace {
    background: #1a1a1a;
    padding: 0;
}

.canvas-cluster {
    padding: 0;
}

.document-tab-add {
    color: #7f8792;
    padding: 3px 8px;
}

.ruler-corner,
.ruler-horizontal,
.ruler-vertical {
    background: #2b2b2b;
    color: #b8b8b8;
    border: none;
    font-size: 9px; /* font.size.xs */
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.ruler-horizontal {
    min-height: 20px;
    padding: 0;
}

.ruler-vertical {
    min-width: 20px;
    padding: 0;
}

.canvas-frame {
    background: #0a0a0a;
    border: none;
    margin: 0;
    box-shadow: none;
    padding: 0;
}

.canvas-info {
    background: rgba(0,0,0,0.65);
    color: #cccccc;
    padding: 3px 12px;
    border-radius: 4px;
    font-size: 11px;
}

.contextual-task-bar {
    background: rgba(40,40,40,0.95);
    border: 1px solid #444444;
    border-radius: 8px;
    padding: 6px 8px;
}

.contextual-task-button {
    background: #3c3c3c;
    color: #e0e0e0;
    border: 1px solid #444444;
    border-radius: 4px;
    padding: 4px 10px;
    font-size: 11px;
}

.contextual-task-button:hover {
    background: #4a4a4a;
    border-color: #555555;
}

.contextual-task-button:disabled,
.contextual-task-button-primary:disabled {
    background: #2b2b2b;
    color: #6f7680;
    border-color: #353535;
}

.contextual-task-button-primary {
    background: #2680eb;
    border-color: #2680eb;
    color: #ffffff;
}

.contextual-task-button-primary:hover {
    background: #3a9eff;
    border-color: #3a9eff;
}

.contextual-task-separator {
    min-height: 20px;
    margin: 0 4px;
    color: #444444;
}

.right-sidebar {
    background: #383838;
    border-left: 1px solid #1e1e1e;
    min-width: 336px;
    padding-left: 0;
}

.right-sidebar-base {
    min-width: 336px;
    background: #383838;
}

.panel-icon-strip {
    min-width: 36px;
    padding: 8px 0;
    background: #383838;
    border-right: 1px solid #1f1f1f;
}

.dock-icon-button {
    min-width: 30px;
    min-height: 30px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 2px;
    color: #cfd4dc;
}

.dock-icon-button:hover {
    background: #474747;
    border-color: #5d5d5d;
    color: #e0e0e0;
}

.dock-icon-button-active {
    background: #2d6cb8;
    border-color: #2d6cb8;
    color: #ffffff;
}

.dock-icon-button-placeholder {
    color: #b3b9c2;
}

.chrome-icon-button,
.layer-visibility-button {
    min-width: 18px;
    min-height: 18px;
    padding: 0;
    background: transparent;
    border: none;
    color: #e0e0e0;
}

.chrome-icon-button:hover,
.layer-visibility-button:hover {
    color: #e0e0e0;
    background: rgba(255,255,255,0.05);
    border-radius: 4px;
}

.panel-dock {
    padding: 0;
    background: #383838;
}

.context-dock-host {
    background: #242424;
    border: 1px solid #3a3a3a;
    border-right: 1px solid #515151;
    border-radius: 6px 0 0 6px;
    box-shadow: 0 10px 28px rgba(0,0,0,0.34);
}

.workspace-context-dock {
    min-height: 0;
}

.panel-group {
    border: 0;
    border-bottom: 1px solid #1e1e1e;
    background: #383838;
    border-radius: 0;
    margin-bottom: 0;
}

.panel-group-header {
    padding: 0 4px;
    min-height: 26px;
    background: #2e2e2e;
    border-bottom: 1px solid #202020;
    border-radius: 0;
}

.panel-tab {
    background: transparent;
    border: none;
    border-right: 1px solid #1f1f1f;
    padding: 5px 10px;
    font-size: 11px;
    font-weight: 500;
    color: #9097a3;
    border-bottom: 2px solid transparent;
    border-radius: 0;
}

.panel-tab:hover {
    color: #d8dde6;
}

.panel-tab-active {
    background: transparent;
    border: none;
    border-right: 1px solid #1f1f1f;
    border-bottom: 2px solid #3b8beb;
    color: #e0e0e0;
    font-weight: 600;
    border-radius: 0;
}

.panel-tab-placeholder {
    color: #7a818c;
}

.panel-group-body {
    padding: 6px;
}

#layers-panel-body {
    padding: 0;
}

#color-panel-body {
    padding: 6px 8px 10px 8px;
}

.color-summary-row {
    margin-bottom: 6px;
}

.color-summary-chip {
    padding: 0;
}

.color-summary-label {
    color: #aeb6c1;
    font-size: 10px;
    font-weight: 600;
}

.color-gradient-frame,
.color-spectrum-frame {
    background: #1f1f1f;
    border: 1px solid #444444;
    border-radius: 4px;
    padding: 1px;
}

.color-picker-cursor {
    min-width: 10px;
    min-height: 10px;
    border-radius: 999px;
    border: 2px solid #ffffff;
    box-shadow: 0 0 0 1px rgba(0,0,0,0.45);
}

.color-spectrum-cursor {
    min-width: 18px;
    min-height: 4px;
    margin-left: -2px;
    border-radius: 3px;
    border: 1px solid rgba(0,0,0,0.85);
    background: rgba(255,255,255,0.7);
}

.color-value-row {
    margin-top: 3px;
}

.color-value-field {
    background: #303030;
    border: 1px solid #444444;
    border-radius: 3px;
    padding: 2px 5px;
}

.color-value-key {
    color: #7f8792;
    font-size: 9px;
    font-weight: 700;
}

.color-value-text {
    color: #dfe5ee;
    font-size: 11px;
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.color-panel-actions {
    margin-top: 6px;
    margin-bottom: 8px;
}

.color-action-button {
    min-height: 24px;
    padding: 2px 10px;
    border-radius: 4px;
    border: 1px solid #494949;
    background: #303030;
    color: #dfe5ee;
}

.color-action-button:hover {
    background: #383838;
    border-color: #5d5d5d;
}

.color-action-button:disabled {
    color: #7c848e;
    background: #262626;
    border-color: #373737;
}

.color-swatches-header {
    margin: 0 0 5px 0;
}

.color-swatches-title {
    color: #aeb6c1;
    font-size: 11px;
    font-weight: 600;
}

.panel-inline-menu {
    min-width: 18px;
    min-height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    color: #8c949f;
}

.panel-inline-menu:disabled {
    opacity: 1;
}

.color-swatches-grid {
    margin-top: 2px;
}

.panel-swatch-button {
    padding: 0;
    min-width: 14px;
    min-height: 14px;
    border-radius: 3px;
    border: 1px solid transparent;
    background: transparent;
}

.panel-swatch-button:hover {
    border-color: #ffffff;
    background: transparent;
}

.panel-swatch-button-active {
    border-color: #3b8beb;
    background: rgba(59,139,235,0.14);
}

.color-empty-state {
    color: #8f98a4;
    font-size: 11px;
}

popover.menu-dropdown contents {
    background: #2c2c2c;
    border: 1px solid #4a4a4a;
    border-radius: 4px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    padding: 4px 0;
}

.menu-dropdown-body {
    min-width: 220px;
}

.menu-dropdown-item {
    min-height: 28px;
    padding: 5px 12px;
    border: none;
    border-radius: 0;
    background: transparent;
    color: #999999;
}

.menu-dropdown-item .icon-label-text,
.menu-dropdown-item image {
    color: #d7d7d7;
    opacity: 1;
}

.menu-dropdown-item .icon-label-shortcut {
    color: #8d8d8d;
}

.menu-dropdown-item:hover {
    background: #3b8beb;
    color: #ffffff;
}

.menu-dropdown-item:hover .icon-label-text,
.menu-dropdown-item:hover .icon-label-shortcut,
.menu-dropdown-item:hover image {
    color: #ffffff;
}

.menu-dropdown-item:disabled {
    color: #666666;
}

.menu-dropdown-item:disabled .icon-label-text,
.menu-dropdown-item:disabled .icon-label-shortcut,
.menu-dropdown-item:disabled image {
    color: #666666;
}

.icon-label-text {
    font-size: 11px;
    font-weight: 400;
}

.icon-label-shortcut {
    color: #777777;
    font-size: 10px;
    margin-left: 12px;
}

.menu-dropdown-item:hover .icon-label-shortcut {
    color: #ffffff;
}

.panel-row {
    color: #999999;
    padding: 3px 0;
    border-radius: 0;
}

.panel-row:hover {
    background: transparent;
    color: #e0e0e0;
}

.panel-hint-row {
    color: #7b7b7b;
    font-size: 10px;
}

.layers-toolbar,
.history-toolbar {
    padding: 8px;
}

.panel-scroller {
    background: transparent;
    border: none;
}

.panel-scroller > viewport {
    background: transparent;
}

.panel-scroller scrollbar {
    background: transparent;
    padding: 2px;
}

.panel-scroller scrollbar.vertical slider {
    min-width: 8px;
    min-height: 28px;
    border-radius: 999px;
    background: rgba(255,255,255,0.14);
}

.panel-scroller scrollbar.horizontal slider {
    min-width: 28px;
    min-height: 8px;
    border-radius: 999px;
    background: rgba(255,255,255,0.14);
}

.panel-scroller scrollbar.vertical slider:hover,
.panel-scroller scrollbar.horizontal slider:hover {
    background: rgba(255,255,255,0.24);
}

.panel-scroller-content {
    padding: 4px 8px 8px 8px;
}

.dock-footer {
    padding: 6px 8px 8px 8px;
    border-top: 1px solid rgba(255,255,255,0.06);
    background: #2b2b2b;
}

.dock-footer-button {
    min-width: 22px;
    min-height: 22px;
    padding: 0;
}

.brush-adjust-row {
    padding: 0;
    min-height: 24px;
}

.compact-swatches-header {
    margin-bottom: 2px;
}

.compact-swatches-current {
    margin-bottom: 2px;
}

.compact-swatches-grid {
    margin-top: 2px;
}

.compact-swatch-action-button {
    min-width: 18px;
    min-height: 18px;
    padding: 0;
}

.compact-swatch-button {
    min-width: 14px;
    min-height: 14px;
    padding: 0;
    border-radius: 3px;
    border: 1px solid rgba(255,255,255,0.08);
    background: #252525;
}

.compact-swatch-button:hover {
    border-color: rgba(255,255,255,0.18);
    background: #2c2c2c;
}

.layer-action-chip {
    min-height: 22px;
    padding: 0 8px;
}

.tool-chip-icon-only {
    min-width: 24px;
    padding: 0 6px;
}

.layer-filter-box {
    background: #2f2f2f;
    border: 1px solid #444444;
    border-radius: 3px;
    padding: 2px 6px;
}

.layers-filter-entry {
    background: transparent;
    border: none;
    color: #d8dde6;
    padding: 0;
    min-height: 20px;
    font-size: 11px;
}

.layers-filter-entry:focus {
    box-shadow: none;
    outline: none;
}

.layer-filter-clear {
    min-width: 14px;
    min-height: 14px;
    padding: 0;
    color: #8f98a5;
}

.layers-blend-row {
    padding: 0 8px 8px 8px;
}

.layer-control-group {
    margin-right: 8px;
}

.layer-control-label {
    color: #9ea7b3;
    font-size: 10px;
    font-weight: 600;
}

.layer-value-box {
    background: #303030;
    border: 1px solid #444444;
    border-radius: 3px;
    padding: 2px 7px;
    min-width: 64px;
}

.layer-value-label {
    color: #dfe5ee;
    font-size: 11px;
}

.layers-info-row {
    padding: 0 8px 8px 8px;
}

.layers-info-chip {
    background: #2f2f2f;
    border: 1px solid #3f454d;
    border-radius: 3px;
    padding: 2px 6px;
    color: #9ea7b3;
    font-size: 10px;
    font-weight: 600;
}

.layers-list {
    padding: 0 0 6px 0;
}

.layer-row,
.layer-row-active {
    padding: 6px 8px;
    border-radius: 0;
    margin-bottom: 0;
    border-left: 3px solid transparent;
}

.layer-item-shell {
    border-bottom: 1px solid rgba(255,255,255,0.03);
}

.layer-row:hover {
    background: #2c2c2c;
}

.layer-row-active {
    background: #383838;
    border-left-color: #3b8beb;
    color: #e0e0e0;
}

.layer-row-mask-target {
    background: linear-gradient(90deg, rgba(59,139,235,0.18), rgba(56,56,56,0.92));
    border-left-color: #79b5ff;
}

.layer-row-mask-disabled {
    background: linear-gradient(90deg, rgba(152,128,66,0.16), rgba(44,44,44,0.92));
}

.layer-row-group {
    background: linear-gradient(90deg, rgba(80,92,110,0.18), rgba(44,44,44,0.92));
}

.layer-select-button {
    background: transparent;
    border: 1px solid transparent;
    color: #e0e0e0;
    padding: 0;
}

.layer-select-button:hover {
    background: transparent;
    border-color: transparent;
}

.layer-select-button-active {
    color: #e0e0e0;
}

.layer-target-strip {
    margin-right: 4px;
    margin-top: 3px;
}

.mask-target-chip {
    min-width: 24px;
    min-height: 18px;
    padding: 0 4px;
    border-radius: 3px;
    border: 1px solid #4a4a4a;
    background: #2a2a2a;
    color: #9d9d9d;
    font-size: 10px;
    font-weight: 700;
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.layer-preview {
    min-width: 28px;
    min-height: 28px;
    border-radius: 3px;
    border: 1px solid #505050;
    background: linear-gradient(180deg, #4b4b4b, #2f2f2f);
    margin-right: 6px;
}

.layer-preview-group {
    background: linear-gradient(180deg, #51657d, #354250);
}

.layer-preview-text {
    background: linear-gradient(180deg, #72595b, #4a393b);
}

.layer-preview-masked {
    border-color: #79b5ff;
}

.layer-preview-glyph {
    color: #eef2f8;
    font-size: 11px;
    font-weight: 700;
}

.layer-preview-image {
    min-width: 28px;
    min-height: 28px;
}

.layer-preview-button {
    padding: 0;
    border: none;
    background: transparent;
}

.layer-preview-button:hover {
    background: transparent;
    border: none;
}

.layer-content-button {
    background: transparent;
    border: none;
    padding: 0;
    color: inherit;
}

.layer-content-button:hover {
    background: transparent;
    border: none;
}

.layer-content-button-active {
    color: #eaf1fb;
}

.layer-name-title {
    color: #dfe5ee;
    font-size: 11px;
    font-weight: 600;
}

.layer-meta-label {
    color: #8f98a5;
    font-size: 10px;
}

.layer-state-badge {
    background: rgba(59,139,235,0.2);
    border: 1px solid rgba(121,181,255,0.35);
    border-radius: 3px;
    padding: 1px 4px;
    color: #dcecff;
    font-size: 9px;
    font-weight: 700;
}

.layers-bottom {
    padding: 8px;
    border-top: 1px solid #3a3a3a;
    background: #2b2b2b;
}

.mask-target-chip:hover {
    background: #343434;
    color: #e8ecf3;
}

.mask-target-chip-active {
    background: #173b64;
    border-color: #79b5ff;
    color: #f0f6ff;
}

.mask-target-chip-disabled {
    background: #232323;
    border-color: #383838;
    color: #666666;
}

.layer-row-context-popover {
    margin-left: 8px;
}

.layer-row-context-menu {
    min-width: 188px;
}

.layer-row-context-separator {
    margin: 4px 0;
    opacity: 0.28;
}

.mask-state-banner {
    padding: 8px;
    margin: 4px 0 8px 0;
    border-radius: 6px;
    border: 1px solid #3d4652;
    background: linear-gradient(180deg, rgba(29,34,40,0.96), rgba(24,28,33,0.96));
}

.mask-state-banner-active {
    border-color: #79b5ff;
    box-shadow: inset 0 0 0 1px rgba(121,181,255,0.2);
}

.mask-state-banner-disabled {
    border-color: #8a6a2a;
}

.mask-state-title {
    color: #e8ecf3;
    font-size: 11px;
    font-weight: 700;
}

.mask-state-hint {
    color: #a8b3c0;
    font-size: 10px;
}

.history-item,
.history-item-active {
    padding: 6px 0;
    border-radius: 0;
}

.history-item:hover {
    background: #2c2c2c;
}

.history-item-active {
    background: #383838;
}

.history-icon {
    color: #9aa3af;
    min-width: 12px;
}

.history-name {
    color: #999999;
    font-size: 11px;
}

.history-item-active .history-name {
    color: #e0e0e0;
}

.startup-splash {
    background: linear-gradient(180deg, #111111 0%, #0b0b0b 100%);
}

.startup-splash-content {
    min-width: 320px;
}

.startup-splash-logo {
    margin-bottom: 10px;
}

.startup-splash-phase {
    color: #d8d8d8;
    font-size: 13px;
    font-weight: 600;
    margin-top: 10px;
}

.startup-splash-detail {
    color: #969696;
    font-size: 11px;
}

.startup-splash-spinner {
    color: #cdb678;
    margin-top: 6px;
}

.status-bar {
    min-height: 22px;
    padding: 0 12px;
    background: #1a1a1a;
    border-top: 1px solid #3a3a3a;
}

.status-left,
.status-center,
.status-right {
    min-height: 22px;
}

.status-label {
    color: #666666;
    font-size: 10px;
    font-weight: 400;
}

.status-notice {
    color: #9a9a9a;
    font-size: 10px;
    font-weight: 500;
}

.status-notice-busy {
    color: #d9c37b;
}

.status-notice-success {
    color: #8ec07c;
}

.status-notice-error {
    color: #e07a7a;
}

.status-notice-warning {
    color: #d3ad67;
}

paned > separator {
    background-color: transparent;
    min-width: 4px;
    min-height: 4px;
}

paned > separator:hover {
    background-color: #4a4a4a;
}
"#;

#[cfg(test)]
mod tests {
    use super::{
        LayerPanelItem, PendingDocumentAction, RulerUnit, ShellGuide, ShellImportDiagnostic,
        ShellImportReport, ShellSnapshot, ShellTextAlignment, ShellTextSnapshot, ShellToolKind,
        canvas_host, compute_visible_ruler_range, format_import_report_details,
        pick_ruler_major_step, shell_status_hint, status_presenter,
    };
    use common::CanvasSize;
    use std::path::PathBuf;

    fn snapshot_for_tool(tool: ShellToolKind) -> ShellSnapshot {
        ShellSnapshot {
            document_title: "untitled.ptx".to_string(),
            project_path: None,
            dirty: false,
            recovery_offer_pending: false,
            recovery_path: None,
            status_message: String::new(),
            latest_alert: None,
            latest_import_report: None,
            file_job_active: false,
            autosave_job_active: false,
            canvas_size: CanvasSize::new(640, 480),
            canvas_revision: 1,
            active_tool_name: tool.label().to_string(),
            active_tool: tool,
            layers: Vec::<LayerPanelItem>::new(),
            active_layer_name: "Layer 1".to_string(),
            active_layer_opacity_percent: 100,
            active_layer_visible: true,
            active_layer_blend_mode: "Normal".to_string(),
            active_layer_has_mask: false,
            active_layer_mask_enabled: false,
            active_edit_target_name: "Layer Pixels".to_string(),
            selected_structure_name: "Layer 1".to_string(),
            selected_structure_is_group: false,
            can_create_group_from_active_layer: false,
            can_ungroup_selected_group: false,
            can_move_active_layer_into_selected_group: false,
            can_move_active_layer_out_of_group: false,
            active_layer_bounds: None,
            can_begin_transform: false,
            transform_preview_rect: None,
            transform_active: false,
            transform_scale_percent: 100,
            transform_scale_x_percent: 100,
            transform_scale_y_percent: 100,
            transform_rotation_degrees: 0,
            can_apply_destructive_filters: false,
            filter_job_active: false,
            brush_preset_name: "Balanced Round".to_string(),
            brush_radius: 12,
            brush_hardness_percent: 72,
            brush_spacing: 5,
            brush_flow_percent: 82,
            pressure_size_enabled: false,
            pressure_opacity_enabled: false,
            snapping_enabled: true,
            snapping_temporarily_bypassed: false,
            guides_visible: false,
            guide_count: 0,
            guides: Vec::<ShellGuide>::new(),
            selection_rect: None,
            selection_path: None,
            selection_preview_path: None,
            selection_inverted: false,
            foreground_color: [255, 255, 255, 255],
            background_color: [0, 0, 0, 255],
            color_swatches: vec![[255, 255, 255, 255], [0, 0, 0, 255]],
            selected_color_swatch: Some(0),
            can_undo: false,
            can_redo: false,
            history_entries: Vec::new(),
            text: ShellTextSnapshot {
                selected: false,
                editing: false,
                request_id: None,
                is_new_layer: false,
                layer_name: "Text 1".to_string(),
                content: String::new(),
                font_family: "Bitmap Sans".to_string(),
                font_size_px: 16,
                line_height_percent: 120,
                letter_spacing: 0,
                fill_rgba: [255, 255, 255, 255],
                alignment: ShellTextAlignment::Left,
                origin_x: 0,
                origin_y: 0,
            },
        }
    }

    #[test]
    fn brush_preview_radius_matches_pressure_mapping() {
        assert_eq!(canvas_host::brush_preview_radius(12, false, 0.2), 12.0);
        assert!((canvas_host::brush_preview_radius(12, true, 0.25) - 6.15).abs() < 0.001);
        assert!((canvas_host::brush_preview_radius(12, true, 1.0) - 12.0).abs() < 0.001);
    }

    #[test]
    fn ruler_units_format_pixels_and_metric_values() {
        assert_eq!(RulerUnit::Pixels.format_pixels(192.0), "192");
        assert_eq!(RulerUnit::Inches.format_pixels(192.0), "2");
        assert_eq!(RulerUnit::Centimeters.format_pixels(96.0), "2.54");
    }

    #[test]
    fn visible_ruler_range_tracks_viewport_bounds_without_rounding_jitter() {
        assert_eq!(compute_visible_ruler_range(0.0, 2.0, 400), Some((0, 200)));
        assert_eq!(
            compute_visible_ruler_range(25.0, 1.5, 450),
            Some((-17, 284))
        );
        assert_eq!(
            compute_visible_ruler_range(-48.0, 2.0, 320),
            Some((24, 184))
        );
    }

    #[test]
    fn ruler_major_step_scales_to_readable_screen_spacing() {
        assert_eq!(pick_ruler_major_step(RulerUnit::Pixels, 1.0), 100.0);
        assert_eq!(pick_ruler_major_step(RulerUnit::Pixels, 4.0), 50.0);
        assert_eq!(pick_ruler_major_step(RulerUnit::Inches, 1.0), 1.0);
    }

    #[test]
    fn brush_preview_paths_include_softness_and_spacing_markers() {
        let paths = canvas_host::build_brush_preview_paths(
            ShellToolKind::Brush,
            (120, 80),
            12.0,
            50,
            5.0,
            1.0,
        );

        assert_eq!(paths.len(), 5);
        assert!(paths[0].closed);
        assert!(paths[1].closed);
        assert!(!paths[2].closed);
        assert!(!paths[3].closed);
        assert!(paths[4].closed);
    }

    #[test]
    fn brush_status_hint_reports_spacing() {
        let mut snapshot = snapshot_for_tool(ShellToolKind::Brush);
        snapshot.pressure_size_enabled = true;

        let hint = shell_status_hint(&snapshot);
        assert!(hint.contains("Spacing 5"));
        assert!(hint.contains("Pressure size"));
    }

    #[test]
    fn import_report_details_include_summary_and_diagnostics() {
        let details = format_import_report_details(&ShellImportReport {
            id: 1,
            title: "PSD Imported With Warnings".to_string(),
            summary: "PhotoTux imported a flattened composite because the PSD exceeded the current layered subset.".to_string(),
            diagnostics: vec![ShellImportDiagnostic {
                severity_label: "Warning".to_string(),
                code: "unsupported_layer_kind".to_string(),
                message: "Source layer 1: Layer 'Title' uses unsupported kind Text for the current layered PSD subset.".to_string(),
            }],
        });

        assert!(details.contains("flattened composite"));
        assert!(details.contains("Details:"));
        assert!(details.contains("Warning: Source layer 1"));
    }

    #[test]
    fn pending_document_action_prompt_copy_matches_action_kind() {
        let open_action = PendingDocumentAction::OpenProject(PathBuf::from("test.ptx"));
        let import_action = PendingDocumentAction::ImportImage(PathBuf::from("image.png"));

        assert_eq!(
            open_action.prompt_title(),
            "Save changes before opening another project?"
        );
        assert!(
            open_action
                .prompt_detail("Scene")
                .contains("open another project")
        );

        assert_eq!(
            import_action.prompt_title(),
            "Save changes before importing?"
        );
        assert!(
            import_action
                .prompt_detail("Scene")
                .contains("import a file that replaces the current document")
        );
    }

    #[test]
    fn status_notice_class_prefers_explicit_activity_flags() {
        let mut snapshot = snapshot_for_tool(ShellToolKind::Brush);
        assert_eq!(
            status_presenter::status_notice_class(&snapshot),
            "status-notice-success"
        );

        snapshot.dirty = true;
        assert_eq!(
            status_presenter::status_notice_class(&snapshot),
            "status-notice-warning"
        );

        snapshot.file_job_active = true;
        assert_eq!(
            status_presenter::status_notice_class(&snapshot),
            "status-notice-busy"
        );
    }
}
