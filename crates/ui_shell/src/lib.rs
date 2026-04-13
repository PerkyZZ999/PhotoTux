use anyhow::Result;
use common::{
    APP_NAME, CanvasRaster, CanvasRect, CanvasSize, DestructiveFilterKind, GroupId, LayerId,
};
use glib::ControlFlow;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, ComboBoxText,
    CssProvider, Dialog, Entry, EventControllerKey, EventControllerMotion, EventControllerScroll,
    EventControllerScrollFlags, GestureDrag, GestureStylus, HeaderBar, Image, Label, MenuButton,
    MessageDialog, MessageType, Orientation, Paned, Picture, Popover, ResponseType, Separator,
    SpinButton, gdk,
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
    load_status_bar_template, load_titlebar_template, load_tool_options_bar_template,
};

mod layout;
mod menus;
mod file_workflow;
mod shell_chrome;
mod startup;
mod canvas_host;
mod panels;
mod status_presenter;
mod ui_support;
mod ui_templates;

use canvas_host::{CanvasHostState, build_canvas_host};
use ui_support::{
    build_icon_label_button, build_icon_label_shortcut_button, build_icon_only_button,
    build_logo_icon, build_remix_icon, create_menu_popover, logo_icon_resource_path,
    remix_icon_resource_path, set_image_resource_or_fallback, set_menu_button_label,
    shell_tool_icon, shell_tool_shortcut,
};
use status_presenter::{
    apply_status_notice_style, format_import_report_details, format_shell_alert_secondary_text,
    shell_notice_text, shell_status_hint,
};

const UI_RESOURCE_PREFIX: &str = "/com/phototux";
const OPTIONAL_ICON_FALLBACK_NAME: &str = "image-missing";
const MAIN_WINDOW_DEFAULT_WIDTH: i32 = 1600;
const MAIN_WINDOW_DEFAULT_HEIGHT: i32 = 900;
const STARTUP_WARMUP_WIDTH: u32 = 1280;
const STARTUP_WARMUP_HEIGHT: u32 = 720;

type StartupWindowHook = Box<dyn FnOnce(&ApplicationWindow)>;

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
    let button = Button::with_label(label_text);
    button.add_css_class("color-chip");
    button.add_css_class(css_class);
    button
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingDocumentAction {
    ChooseOpenProject,
    ChooseImportImage,
    OpenProject(PathBuf),
    ImportImage(PathBuf),
}

impl PendingDocumentAction {
    const fn prompt_title(&self) -> &'static str {
        match self {
            Self::ChooseOpenProject | Self::OpenProject(_) => {
                "Save changes before opening another project?"
            }
            Self::ChooseImportImage | Self::ImportImage(_) => "Save changes before importing?",
        }
    }

    const fn prompt_action_phrase(&self) -> &'static str {
        match self {
            Self::ChooseOpenProject | Self::OpenProject(_) => "open another project",
            Self::ChooseImportImage | Self::ImportImage(_) => {
                "import a file that replaces the current document"
            }
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
    canvas_picture: Picture,
    tool_rail: GtkBox,
    tool_buttons: Vec<(ShellToolKind, Button)>,
    document_tabs: GtkBox,
    document_tab_label: Label,
    layers_group: GtkBox,
    layers_body: GtkBox,
    properties_group: GtkBox,
    properties_body: GtkBox,
    color_group: GtkBox,
    color_body: GtkBox,
    history_group: GtkBox,
    history_body: GtkBox,
    status_bar: GtkBox,
    menu_zoom_label: Label,
    status_doc: Label,
    status_zoom: Label,
    status_cursor: Label,
    status_notice: Label,
    status_mode: Label,
    canvas_info_label: Label,
    last_snapshot: RefCell<Option<ShellSnapshot>>,
    last_zoom_percent: RefCell<u32>,
}

impl ShellUiState {
    fn new(controller: Rc<RefCell<dyn ShellController>>) -> Rc<Self> {
        let (tool_options_bar, tool_options_icon, tool_options_label) =
            shell_chrome::build_tool_options_bar(controller.clone());
        let (tool_rail, tool_buttons) = shell_chrome::build_left_tool_rail(controller.clone());
        let (document_tabs, document_tab_label) = shell_chrome::build_document_tabs();
        let (canvas_picture, canvas_state) = build_canvas_host(controller.clone());
        let automation_shortcuts_enabled = env::var_os("PHOTOTUX_ENABLE_TEST_SHORTCUTS").is_some();

        let (color_group, color_body) =
            shell_chrome::build_panel_group("color", &["Color", "Swatches"], 6, false);

        let (properties_group, properties_body) =
            shell_chrome::build_panel_group("properties", &["Properties", "Adjust"], 4, false);

        let (layers_group, layers_body) =
            shell_chrome::build_panel_group("layers", &["Layers", "Channels", "Paths"], 4, false);

        let (history_group, history_body) =
            shell_chrome::build_panel_group("history", &["History"], 4, true);

        let (status_bar, status_doc, status_zoom, status_cursor, status_notice, status_mode) =
            shell_chrome::build_status_bar();
        let menu_zoom_label = Label::new(Some("100%"));
        menu_zoom_label.add_css_class("menu-zoom-display");
        let canvas_info_label = Label::new(Some("untitled.ptx @ 100% (RGB/8)"));
        canvas_info_label.add_css_class("canvas-info");

        Rc::new(Self {
            controller,
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
            canvas_state,
            automation_shortcuts_enabled,
            tool_options_bar,
            tool_options_icon,
            tool_options_label,
            canvas_picture,
            tool_rail,
            tool_buttons,
            document_tabs,
            document_tab_label,
            layers_group,
            layers_body,
            properties_group,
            properties_body,
            color_group,
            color_body,
            history_group,
            history_body,
            status_bar,
            menu_zoom_label,
            status_doc,
            status_zoom,
            status_cursor,
            status_notice,
            status_mode,
            canvas_info_label,
            last_snapshot: RefCell::new(None),
            last_zoom_percent: RefCell::new(0),
        })
    }

    fn handle_shortcut(self: &Rc<Self>, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
        let is_control = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        let is_shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
        let key_char = key
            .to_unicode()
            .map(|character| character.to_ascii_lowercase());

        if self.automation_shortcuts_enabled && is_control && is_shift {
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
                    return true;
                }
                Some('r') => {
                    self.controller.borrow_mut().reset_colors();
                    return true;
                }
                Some(digit @ '1'..='9') => {
                    let layer_index = (digit as u8 - b'1') as usize;
                    if let Some(layer_id) = self.nth_layer_id(layer_index) {
                        self.controller.borrow_mut().select_layer(layer_id);
                        return true;
                    }
                }
                _ => {}
            }
        }

        if is_control {
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
                    return true;
                }
                Some('s') if is_shift => {
                    self.request_project_save_as();
                    return true;
                }
                Some('z') => {
                    self.controller.borrow_mut().undo();
                    return true;
                }
                Some('y') => {
                    self.controller.borrow_mut().redo();
                    return true;
                }
                Some('o') => {
                    self.request_open_project();
                    return true;
                }
                Some('s') => {
                    self.request_project_save();
                    return true;
                }
                Some('d') => {
                    self.controller.borrow_mut().clear_selection();
                    return true;
                }
                Some('i') => {
                    self.controller.borrow_mut().invert_selection();
                    return true;
                }
                Some('=') | Some('+') => {
                    self.canvas_state.borrow_mut().zoom_in();
                    return true;
                }
                Some('-') => {
                    self.canvas_state.borrow_mut().zoom_out();
                    return true;
                }
                Some('0') => {
                    self.canvas_state.borrow_mut().fit_to_view();
                    return true;
                }
                _ => {}
            }
        }

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
            }
            _ => {}
        }

        match key_char {
            Some('v') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Move),
            Some('m') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::RectangularMarquee),
            Some('l') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Lasso),
            Some('i') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Text),
            Some('t') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Transform),
            Some('b') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Brush),
            Some('e') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Eraser),
            Some('h') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Hand),
            Some('z') => self
                .controller
                .borrow_mut()
                .select_tool(ShellToolKind::Zoom),
            _ => return false,
        }

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
        let sync: Rc<dyn Fn()> = Rc::new({
            let content_entry = content_entry.clone();
            let font_family = font_family.clone();
            let font_size = font_size.clone();
            let line_height = line_height.clone();
            let letter_spacing = letter_spacing.clone();
            let alignment = alignment.clone();
            let color_r = color_r.clone();
            let color_g = color_g.clone();
            let color_b = color_b.clone();
            let color_a = color_a.clone();
            move || {
                controller
                    .borrow_mut()
                    .update_text_session(ShellTextUpdate {
                        content: content_entry.text().to_string(),
                        font_family: font_family
                            .active_text()
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| fallback_font_family.clone()),
                        font_size_px: font_size.value().round() as u32,
                        line_height_percent: line_height.value().round() as u32,
                        letter_spacing: letter_spacing.value().round() as i32,
                        fill_rgba: [
                            color_r.value().round() as u8,
                            color_g.value().round() as u8,
                            color_b.value().round() as u8,
                            color_a.value().round() as u8,
                        ],
                        alignment: match alignment.active_id().as_ref().map(|id| id.as_str()) {
                            Some("center") => ShellTextAlignment::Center,
                            Some("right") => ShellTextAlignment::Right,
                            _ => ShellTextAlignment::Left,
                        },
                    });
            }
        });

        {
            let sync = sync.clone();
            content_entry.connect_changed(move |_| sync());
        }
        for spin in [
            &font_size,
            &line_height,
            &letter_spacing,
            &color_r,
            &color_g,
            &color_b,
            &color_a,
        ] {
            let sync = sync.clone();
            spin.connect_value_changed(move |_| sync());
        }
        {
            let sync = sync.clone();
            font_family.connect_changed(move |_| sync());
        }
        {
            let sync = sync.clone();
            alignment.connect_changed(move |_| sync());
        }

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
        let snapshot = self.controller.borrow().snapshot();
        if snapshot.dirty {
            self.present_document_replacement_prompt(&snapshot, PendingDocumentAction::ChooseOpenProject);
        } else if let Some(window) = self.window.borrow().as_ref() {
            file_workflow::choose_open_project(window, self.clone());
        }
    }

    fn request_import_image(self: &Rc<Self>) {
        let snapshot = self.controller.borrow().snapshot();
        if snapshot.dirty {
            self.present_document_replacement_prompt(&snapshot, PendingDocumentAction::ChooseImportImage);
        } else if let Some(window) = self.window.borrow().as_ref() {
            file_workflow::choose_import_image(window, self.clone());
        }
    }

    fn perform_document_replacement(self: &Rc<Self>, action: PendingDocumentAction) {
        match action {
            PendingDocumentAction::ChooseOpenProject => {
                if let Some(window) = self.window.borrow().as_ref() {
                    file_workflow::choose_open_project(window, self.clone());
                }
            }
            PendingDocumentAction::ChooseImportImage => {
                if let Some(window) = self.window.borrow().as_ref() {
                    file_workflow::choose_import_image(window, self.clone());
                }
            }
            PendingDocumentAction::OpenProject(path) => {
                self.controller.borrow_mut().open_document(path);
            }
            PendingDocumentAction::ImportImage(path) => {
                self.controller.borrow_mut().import_image(path);
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

    fn refresh(self: &Rc<Self>) {
        self.controller.borrow_mut().poll_background_tasks();
        let snapshot = self.controller.borrow().snapshot();
        let zoom_percent = self.canvas_state.borrow().zoom_percent();
        let snapshot_changed = self.last_snapshot.borrow().as_ref() != Some(&snapshot);
        let zoom_changed = *self.last_zoom_percent.borrow() != zoom_percent;

        if !snapshot_changed && !zoom_changed {
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

        if snapshot_changed {
            self.tool_options_label
                .set_label(&snapshot.active_tool_name);
            set_image_resource_or_fallback(
                &self.tool_options_icon,
                &remix_icon_resource_path(shell_tool_icon(snapshot.active_tool)),
                &snapshot.active_tool_name,
                18,
            );
            self.refresh_tool_buttons(&snapshot);
            self.refresh_color_panel(&snapshot);
            self.refresh_properties_panel(&snapshot);
            self.refresh_layers_panel(&snapshot);
            self.refresh_history_panel(&snapshot);
            self.last_snapshot.replace(Some(snapshot));
        }

        let current_snapshot = self
            .last_snapshot
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.controller.borrow().snapshot());

        if let Some(alert) = current_snapshot.latest_alert.as_ref() {
            let already_presented = self.presented_alert_id.get() == Some(alert.id);
            if !already_presented && !self.alert_dialog_visible.get() {
                self.presented_alert_id.set(Some(alert.id));
                self.present_shell_alert(alert);
            }
        }

        if self.pending_close_after_save.get() && !current_snapshot.dirty {
            self.pending_close_after_save.set(false);
            self.allow_close_once.set(true);
            if let Some(window) = self.window.borrow().as_ref() {
                window.close();
            }
        }

        if !current_snapshot.dirty
            && let Some(action) = self.pending_document_action_after_save.borrow_mut().take()
        {
            self.perform_document_replacement(action);
        }

        let should_prompt_recovery = current_snapshot.recovery_offer_pending
            && self.prompted_recovery_path.borrow().as_ref()
                != current_snapshot.recovery_path.as_ref();
        if should_prompt_recovery && !self.recovery_prompt_visible.get() {
            self.prompted_recovery_path
                .replace(current_snapshot.recovery_path.clone());
            self.present_recovery_prompt(&current_snapshot);
        }

        if current_snapshot.text.editing
            && current_snapshot.text.request_id.is_some()
            && self.presented_text_request_id.get() != current_snapshot.text.request_id
            && !self.text_dialog_visible.get()
        {
            self.presented_text_request_id
                .set(current_snapshot.text.request_id);
            self.present_text_dialog(&current_snapshot.text);
        }

        if let Some(report) = current_snapshot.latest_import_report.as_ref() {
            let already_presented = self.presented_import_report_id.get() == Some(report.id);
            if !already_presented && !self.import_report_visible.get() {
                self.presented_import_report_id.set(Some(report.id));
                self.present_import_report(report);
            }
        }

        self.last_zoom_percent.replace(zoom_percent);
    }

}

fn install_theme() {
    let provider = CssProvider::new();
    provider.load_from_data(THEME_CSS);

    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
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
    font-weight: 600;
    font-size: 11px;
    color: #e0e0e0;
}

.titlebar-icon,
.remix-icon {
    min-width: 12px;
    min-height: 12px;
}

.titlebar-icon {
    color: #3b8beb;
}

.chrome-button,
.menu-button,
.tool-chip,
.tool-button,
.document-tab-add,
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
.document-tab-add:hover,
.dock-icon-button:hover {
    background: #383838;
    border: 1px solid #4a4a4a;
}

.tool-chip {
    background: transparent;
    color: #999999;
    padding: 2px 7px;
}

.tool-chip:hover {
    color: #e0e0e0;
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
    color: #999999;
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
    color: #999999;
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
    min-height: 32px;
    padding: 0 12px;
    background: #232323;
    border-bottom: 1px solid #3a3a3a;
}

.tool-options-label {
    margin: 0 6px 0 1px;
    font-weight: 600;
    color: #e0e0e0;
}

.tool-option-chip {
    background: #3c3c3c;
    border: 1px solid #444444;
    border-radius: 3px;
    color: #d0d0d0;
    padding: 2px 8px;
    font-size: 11px;
}

.tool-option-chip:hover {
    background: #464646;
    border: 1px solid #555555;
    color: #e0e0e0;
}

.tool-options-icon {
    margin-left: 2px;
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
    padding: 6px 0;
    background: #2a2a2a;
    border-right: 1px solid #4a4a4a;
}

.tool-button {
    min-width: 34px;
    min-height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: #a8a8a8;
}

.tool-button:hover {
    background: #343434;
    border-color: #4a4a4a;
    color: #e0e0e0;
}

.tool-button-active {
    background: #3f3f3f;
    border: 1px solid #565656;
    border-left: 2px solid #3b8beb;
    color: #e0e0e0;
}

.tool-separator {
    margin: 2px 8px;
    min-width: 24px;
    opacity: 1;
}

.tool-separator.horizontal {
    color: #4a4a4a;
}

.swatch-stack {
    margin-top: 10px;
    margin-bottom: 6px;
}

.color-chip {
    min-width: 14px;
    min-height: 14px;
    padding: 0;
    border-radius: 2px;
    border: 2px solid #3f3f3f;
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

.document-region {
    background: #1a1a1a;
}

.document-tabs {
    min-height: 26px;
    padding: 4px 6px 0 6px;
    background: #232323;
    border-bottom: 1px solid #3a3a3a;
}

.document-tab-content {
}

.document-tab-title {
    color: #e0e0e0;
}

.document-workspace {
    background: #1a1a1a;
    padding: 0;
}

.document-tab-add:disabled {
    color: #5a5a5a;
    opacity: 1;
}

.ruler-corner,
.ruler-horizontal,
.ruler-vertical {
    background: #232323;
    color: #666666;
    border: 1px solid #3a3a3a;
    font-size: 9px; /* font.size.xs */
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.ruler-horizontal {
    min-height: 20px;
    padding: 2px 8px;
}

.ruler-vertical {
    min-width: 20px;
    padding: 8px 2px;
}

.canvas-frame {
    background: #0a0a0a;
    border: 1px solid #2e2e2e;
    margin: 18px;
    box-shadow: 0 2px 20px rgba(0,0,0,0.5);
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
    background: #232323;
    border-left: 1px solid #3a3a3a;
    min-width: 300px;
}

.panel-icon-strip {
    min-width: 40px;
    padding: 6px 0;
    background: #2a2a2a;
    border-right: 1px solid #4a4a4a;
}

.dock-icon-button {
    min-width: 24px;
    min-height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 3px;
    color: #a8a8a8;
}

.dock-icon-button:hover {
    background: #343434;
    border-color: #4a4a4a;
    color: #e0e0e0;
}

.chrome-icon-button,
.layer-visibility-button {
    min-width: 18px;
    min-height: 18px;
    padding: 0;
    background: transparent;
    border: none;
    color: #999999;
}

.chrome-icon-button:hover,
.layer-visibility-button:hover {
    color: #e0e0e0;
    background: rgba(255,255,255,0.05);
    border-radius: 4px;
}

.panel-dock {
    padding: 0;
    background: #232323;
}

.panel-group {
    border: 0;
    border-bottom: 1px solid #3a3a3a;
    background: #232323;
    border-radius: 0;
    margin-bottom: 0;
}

.panel-group-header {
    padding: 6px 8px;
    background: #232323;
    border-bottom: 1px solid #3a3a3a;
    border-radius: 0;
}

.panel-tab {
    background: transparent;
    border: 1px solid transparent;
    padding: 0 0 6px 0;
    font-size: 10px;
    font-weight: 500;
    color: #666666;
    border-bottom: 2px solid transparent;
}

.panel-tab:hover {
    color: #999999;
}

.panel-tab-active {
    background: transparent;
    border: 1px solid transparent;
    border-bottom: 2px solid #3b8beb;
    color: #e0e0e0;
    font-weight: 600;
    border-radius: 0;
    margin-bottom: 0;
}

.panel-group-body {
    padding: 8px;
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

.menu-dropdown-item:hover {
    background: #3b8beb;
    color: #ffffff;
}

.menu-dropdown-item:disabled {
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
    padding: 6px 0 8px 0;
}

.layer-action-chip {
    min-height: 22px;
    padding: 0 8px;
}

.tool-chip-icon-only {
    min-width: 24px;
    padding: 0 6px;
}

.layer-row,
.layer-row-active {
    padding: 6px 8px;
    border-radius: 0;
    margin-bottom: 0;
    border-left: 3px solid transparent;
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
    color: #666666;
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
        LayerPanelItem, PendingDocumentAction, ShellGuide, ShellImportDiagnostic,
        ShellImportReport, ShellSnapshot, ShellTextAlignment, ShellTextSnapshot, ShellToolKind,
        canvas_host, format_import_report_details, shell_status_hint, status_presenter,
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
