use anyhow::Result;
use common::{CanvasRaster, CanvasRect, CanvasSize, GroupId, APP_NAME};
use glib::ControlFlow;
use gtk4::prelude::*;
use gtk4::{
    gdk, Align, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType,
    CssProvider, EventControllerKey, EventControllerScroll, EventControllerScrollFlags,
    FileChooserAction, FileChooserNative, FileFilter, GestureDrag, GestureStylus, HeaderBar, Image, Label,
    MenuButton, MessageDialog, MessageType, Orientation, Paned, Picture, Popover,
    ResponseType, Separator,
};
use render_wgpu::{CanvasOverlayPath, CanvasOverlayRect, OffscreenCanvasRenderer, ViewportSize, ViewportState};
use std::cell::{Cell, RefCell};
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPanelItem {
    pub index: Option<usize>,
    pub group_id: Option<GroupId>,
    pub name: String,
    pub depth: usize,
    pub is_group: bool,
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
    Brush,
    Eraser,
    Hand,
    Zoom,
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
            Self::Brush => "Brush Tool",
            Self::Eraser => "Eraser Tool",
            Self::Hand => "Hand Tool",
            Self::Zoom => "Zoom Tool",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSnapshot {
    pub document_title: String,
    pub project_path: Option<PathBuf>,
    pub dirty: bool,
    pub recovery_offer_pending: bool,
    pub recovery_path: Option<PathBuf>,
    pub status_message: String,
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
    pub transform_preview_rect: Option<CanvasRect>,
    pub transform_active: bool,
    pub transform_scale_percent: u32,
    pub transform_scale_x_percent: u32,
    pub transform_scale_y_percent: u32,
    pub transform_rotation_degrees: i32,
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
    fn select_layer(&mut self, index: usize);
    fn select_group(&mut self, group_id: GroupId);
    fn toggle_layer_visibility(&mut self, index: usize);
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
    fn poll_background_tasks(&mut self);
    fn select_tool(&mut self, tool: ShellToolKind);
    fn begin_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32);
    fn begin_canvas_interaction_with_pressure(&mut self, canvas_x: i32, canvas_y: i32, pressure: f32) {
        let _ = pressure;
        self.begin_canvas_interaction(canvas_x, canvas_y);
    }
    fn update_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32);
    fn update_canvas_interaction_with_pressure(&mut self, canvas_x: i32, canvas_y: i32, pressure: f32) {
        let _ = pressure;
        self.update_canvas_interaction(canvas_x, canvas_y);
    }
    fn end_canvas_interaction(&mut self);
}

pub fn run(controller: Rc<RefCell<dyn ShellController>>) -> Result<()> {
    let application = Application::builder()
        .application_id("com.phototux.app")
        .build();

    application.connect_activate(move |application| build_ui(application, controller.clone()));
    let _exit_code = application.run();

    Ok(())
}

fn build_ui(application: &Application, controller: Rc<RefCell<dyn ShellController>>) {
    install_theme();

    let shell_state = ShellUiState::new(controller.clone());

    let window = ApplicationWindow::builder()
        .application(application)
        .title(APP_NAME)
        .default_width(1600)
        .default_height(900)
        .build();
    window.set_decorated(false);
    window.add_css_class("app-window");

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.add_css_class("app-root");
    root.append(&build_header_bar());
    root.append(&build_menu_bar(&window, shell_state.clone()));
    root.append(&shell_state.tool_options_bar);
    let workspace = build_workspace_body(&shell_state);
    root.append(&workspace);
    root.append(&shell_state.status_bar);

    window.set_child(Some(&root));
    shell_state.attach_window(window.clone());
    wire_window_shortcuts(&window, shell_state.clone());
    wire_window_close_request(&window, shell_state.clone());
    window.present();

    shell_state.refresh();
    glib::timeout_add_local(Duration::from_millis(33), move || {
        shell_state.refresh();
        ControlFlow::Continue
    });
}

fn build_header_bar() -> HeaderBar {
    let header = HeaderBar::new();
    header.add_css_class("titlebar");

    let title_row = GtkBox::new(Orientation::Horizontal, 6);
    title_row.add_css_class("app-brand");
    let title_icon = build_logo_icon(APP_NAME, 16);
    title_icon.add_css_class("titlebar-icon");
    title_row.append(&title_icon);

    let title = Label::new(Some(APP_NAME));
    title.add_css_class("titlebar-app-name");
    title_row.append(&title);
    header.pack_start(&title_row);

    let actions = GtkBox::new(Orientation::Horizontal, 6);
    actions.add_css_class("titlebar-actions");
    let preset = Button::with_label("Essentials");
    preset.add_css_class("chrome-button");
    preset.add_css_class("workspace-chip");
    actions.append(&preset);

    let search = build_icon_only_button("search-line.svg", "Search", "chrome-button", 12);
    search.add_css_class("chrome-icon-button");
    actions.append(&search);
    header.pack_end(&actions);
    header
}

fn build_menu_bar(window: &ApplicationWindow, shell_state: Rc<ShellUiState>) -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 2);
    bar.add_css_class("menu-bar");

    let file_button = build_file_menu_button(window, shell_state.clone());
    bar.append(&file_button);

    bar.append(&build_edit_menu_button(shell_state.clone()));
    bar.append(&build_image_menu_button(shell_state.clone()));
    bar.append(&build_layer_menu_button(shell_state.clone()));
    bar.append(&build_select_menu_button(shell_state.clone()));
    bar.append(&build_filter_menu_button(shell_state.clone()));
    bar.append(&build_view_menu_button(shell_state.clone()));
    bar.append(&build_window_menu_button(shell_state.clone()));
    bar.append(&build_help_menu_button(window));

    bar
}

fn build_edit_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("Edit").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let undo = build_icon_label_button("arrow-go-back-line.svg", "Undo");
    undo.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        undo.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().undo();
        });
    }
    menu.append(&undo);

    let redo = build_icon_label_button("arrow-go-forward-line.svg", "Redo");
    redo.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        redo.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().redo();
        });
    }
    menu.append(&redo);

    {
        let shell_state = shell_state.clone();
        let undo = undo.clone();
        let redo = redo.clone();
        popover.connect_show(move |_| {
            let snapshot = shell_state.controller.borrow().snapshot();
            undo.set_sensitive(snapshot.can_undo);
            redo.set_sensitive(snapshot.can_redo);
        });
    }

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_image_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("Image").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let start_transform = build_icon_label_shortcut_button(
        "expand-diagonal-2-line.svg",
        "Start Transform",
        Some("T"),
    );
    start_transform.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        start_transform.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().begin_transform();
        });
    }
    menu.append(&start_transform);

    let scale_up = build_icon_label_button("add-line.svg", "Scale Transform Up");
    scale_up.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        scale_up.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().scale_transform_up();
        });
    }
    menu.append(&scale_up);

    let scale_down = build_icon_label_button("subtract-line.svg", "Scale Transform Down");
    scale_down.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        scale_down.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().scale_transform_down();
        });
    }
    menu.append(&scale_down);

    let scale_x_up = build_icon_label_button("add-line.svg", "Scale X Up");
    scale_x_up.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        scale_x_up.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().scale_transform_x_up();
        });
    }
    menu.append(&scale_x_up);

    let scale_x_down = build_icon_label_button("subtract-line.svg", "Scale X Down");
    scale_x_down.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        scale_x_down.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().scale_transform_x_down();
        });
    }
    menu.append(&scale_x_down);

    let scale_y_up = build_icon_label_button("add-line.svg", "Scale Y Up");
    scale_y_up.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        scale_y_up.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().scale_transform_y_up();
        });
    }
    menu.append(&scale_y_up);

    let scale_y_down = build_icon_label_button("subtract-line.svg", "Scale Y Down");
    scale_y_down.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        scale_y_down.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().scale_transform_y_down();
        });
    }
    menu.append(&scale_y_down);

    let rotate_left = build_icon_label_button("history-line.svg", "Rotate Left");
    rotate_left.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        rotate_left.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().rotate_transform_left();
        });
    }
    menu.append(&rotate_left);

    let rotate_right = build_icon_label_button("history-line.svg", "Rotate Right");
    rotate_right.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        rotate_right.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().rotate_transform_right();
        });
    }
    menu.append(&rotate_right);

    let commit_transform = build_icon_label_shortcut_button(
        "check-line.svg",
        "Commit Transform",
        Some("Enter"),
    );
    commit_transform.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        commit_transform.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().commit_transform();
        });
    }
    menu.append(&commit_transform);

    let cancel_transform = build_icon_label_shortcut_button(
        "close-line.svg",
        "Cancel Transform",
        Some("Esc"),
    );
    cancel_transform.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        cancel_transform.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().cancel_transform();
        });
    }
    menu.append(&cancel_transform);

    {
        let shell_state = shell_state.clone();
        let start_transform = start_transform.clone();
        let scale_up = scale_up.clone();
        let scale_down = scale_down.clone();
        let commit_transform = commit_transform.clone();
        let cancel_transform = cancel_transform.clone();
        popover.connect_show(move |_| {
            let snapshot = shell_state.controller.borrow().snapshot();
            let has_active_bounds = snapshot.active_layer_bounds.is_some();
            let transform_active = snapshot.transform_active;

            start_transform.set_sensitive(has_active_bounds && !transform_active);
            scale_up.set_sensitive(transform_active);
            scale_down.set_sensitive(transform_active);
            commit_transform.set_sensitive(transform_active);
            cancel_transform.set_sensitive(transform_active);
        });
    }

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_layer_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("Layer").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let add = build_icon_label_button("add-line.svg", "New Layer");
    add.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        add.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().add_layer();
        });
    }
    menu.append(&add);

    let duplicate = build_icon_label_button("file-copy-line.svg", "Duplicate Layer");
    duplicate.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        duplicate.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().duplicate_active_layer();
        });
    }
    menu.append(&duplicate);

    let delete = build_icon_label_button("delete-bin-line.svg", "Delete Layer");
    delete.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        delete.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().delete_active_layer();
        });
    }
    menu.append(&delete);

    let add_mask = build_icon_label_button("add-line.svg", "Add Layer Mask");
    add_mask.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        add_mask.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().add_active_layer_mask();
        });
    }
    menu.append(&add_mask);

    let remove_mask = build_icon_label_button("delete-bin-line.svg", "Delete Layer Mask");
    remove_mask.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        remove_mask.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().remove_active_layer_mask();
        });
    }
    menu.append(&remove_mask);

    let toggle_mask = build_icon_label_button("contrast-2-line.svg", "Enable or Disable Layer Mask");
    toggle_mask.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        toggle_mask.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().toggle_active_layer_mask_enabled();
        });
    }
    menu.append(&toggle_mask);

    let edit_pixels = build_icon_label_button("brush-2-line.svg", "Edit Layer Pixels");
    edit_pixels.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        edit_pixels.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().edit_active_layer_pixels();
        });
    }
    menu.append(&edit_pixels);

    let edit_mask = build_icon_label_button("eraser-line.svg", "Edit Layer Mask");
    edit_mask.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        edit_mask.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().edit_active_layer_mask();
        });
    }
    menu.append(&edit_mask);

    let move_up = build_icon_label_button("arrow-up-line.svg", "Move Layer Up");
    move_up.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        move_up.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().move_active_layer_up();
        });
    }
    menu.append(&move_up);

    let move_down = build_icon_label_button("arrow-down-line.svg", "Move Layer Down");
    move_down.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        move_down.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().move_active_layer_down();
        });
    }
    menu.append(&move_down);

    {
        let shell_state = shell_state.clone();
        let duplicate = duplicate.clone();
        let delete = delete.clone();
        let add_mask = add_mask.clone();
        let remove_mask = remove_mask.clone();
        let toggle_mask = toggle_mask.clone();
        let edit_pixels = edit_pixels.clone();
        let edit_mask = edit_mask.clone();
        let move_up = move_up.clone();
        let move_down = move_down.clone();
        popover.connect_show(move |_| {
            let snapshot = shell_state.controller.borrow().snapshot();
            let layer_count = snapshot.layers.len();
            let active_index = snapshot.layers.iter().position(|layer| layer.is_active).unwrap_or(0);
            let has_multiple_layers = layer_count > 1;
            let has_mask = snapshot.active_layer_has_mask;

            duplicate.set_sensitive(layer_count > 0);
            delete.set_sensitive(has_multiple_layers);
            add_mask.set_sensitive(layer_count > 0 && !has_mask);
            remove_mask.set_sensitive(has_mask);
            toggle_mask.set_sensitive(has_mask);
            edit_pixels.set_sensitive(layer_count > 0);
            edit_mask.set_sensitive(has_mask);
            move_up.set_sensitive(has_multiple_layers && active_index + 1 < layer_count);
            move_down.set_sensitive(has_multiple_layers && active_index > 0);
        });
    }

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_select_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("Select").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let clear = build_icon_label_button("close-circle-line.svg", "Clear Selection");
    clear.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        clear.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().clear_selection();
        });
    }
    menu.append(&clear);

    let invert = build_icon_label_button("contrast-2-line.svg", "Invert Selection");
    invert.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        invert.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().invert_selection();
        });
    }
    menu.append(&invert);

    {
        let shell_state = shell_state.clone();
        let clear = clear.clone();
        let invert = invert.clone();
        popover.connect_show(move |_| {
            let has_selection = shell_state.controller.borrow().snapshot().selection_rect.is_some();
            clear.set_sensitive(has_selection);
            invert.set_sensitive(has_selection);
        });
    }

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_filter_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("Filter").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let opacity_up = build_icon_label_button("add-circle-line.svg", "Increase Layer Opacity");
    opacity_up.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        opacity_up.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().increase_active_layer_opacity();
        });
    }
    menu.append(&opacity_up);

    let opacity_down = build_icon_label_button("indeterminate-circle-line.svg", "Decrease Layer Opacity");
    opacity_down.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        opacity_down.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().decrease_active_layer_opacity();
        });
    }
    menu.append(&opacity_down);

    let next_blend = build_icon_label_button("arrow-right-s-line.svg", "Next Blend Mode");
    next_blend.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        next_blend.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().next_active_layer_blend_mode();
        });
    }
    menu.append(&next_blend);

    let previous_blend = build_icon_label_button("arrow-left-s-line.svg", "Previous Blend Mode");
    previous_blend.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        previous_blend.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().previous_active_layer_blend_mode();
        });
    }
    menu.append(&previous_blend);

    {
        let shell_state = shell_state.clone();
        let opacity_up = opacity_up.clone();
        let opacity_down = opacity_down.clone();
        let next_blend = next_blend.clone();
        let previous_blend = previous_blend.clone();
        popover.connect_show(move |_| {
            let snapshot = shell_state.controller.borrow().snapshot();
            let has_layer = !snapshot.layers.is_empty();

            opacity_up.set_sensitive(has_layer && snapshot.active_layer_opacity_percent < 100);
            opacity_down.set_sensitive(has_layer && snapshot.active_layer_opacity_percent > 0);
            next_blend.set_sensitive(has_layer);
            previous_blend.set_sensitive(has_layer);
        });
    }

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_view_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("View").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let zoom_in = build_icon_label_shortcut_button("zoom-in-line.svg", "Zoom In", Some("Ctrl++"));
    zoom_in.add_css_class("menu-dropdown-item");
    {
        let shell_state = shell_state.clone();
        let popover = popover.clone();
        zoom_in.connect_clicked(move |_| {
            popover.popdown();
            shell_state.canvas_state.borrow_mut().zoom_in();
        });
    }
    menu.append(&zoom_in);

    let zoom_out = build_icon_label_shortcut_button("zoom-out-line.svg", "Zoom Out", Some("Ctrl+-"));
    zoom_out.add_css_class("menu-dropdown-item");
    {
        let shell_state = shell_state.clone();
        let popover = popover.clone();
        zoom_out.connect_clicked(move |_| {
            popover.popdown();
            shell_state.canvas_state.borrow_mut().zoom_out();
        });
    }
    menu.append(&zoom_out);

    let fit = build_icon_label_shortcut_button("fullscreen-line.svg", "Fit To View", Some("Ctrl+0"));
    fit.add_css_class("menu-dropdown-item");
    {
        let shell_state = shell_state.clone();
        let popover = popover.clone();
        fit.connect_clicked(move |_| {
            popover.popdown();
            shell_state.canvas_state.borrow_mut().fit_to_view();
        });
    }
    menu.append(&fit);

    let add_horizontal_guide = build_icon_label_button("layout-column-line.svg", "Add Horizontal Guide");
    add_horizontal_guide.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        add_horizontal_guide.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().add_horizontal_guide();
        });
    }
    menu.append(&add_horizontal_guide);

    let add_vertical_guide = build_icon_label_button("layout-column-line.svg", "Add Vertical Guide");
    add_vertical_guide.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        add_vertical_guide.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().add_vertical_guide();
        });
    }
    menu.append(&add_vertical_guide);

    let toggle_guides = build_icon_label_button("eye-line.svg", "Show/Hide Guides");
    toggle_guides.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        toggle_guides.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().toggle_guides_visible();
        });
    }
    menu.append(&toggle_guides);

    let toggle_snapping = build_icon_label_button("settings-4-line.svg", "Toggle Snapping");
    toggle_snapping.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        toggle_snapping.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().toggle_snapping_enabled();
        });
    }
    menu.append(&toggle_snapping);

    let remove_guide = build_icon_label_button("eye-off-line.svg", "Remove Last Guide");
    remove_guide.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        remove_guide.connect_clicked(move |_| {
            popover.popdown();
            controller.borrow_mut().remove_last_guide();
        });
    }
    menu.append(&remove_guide);

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_file_menu_button(
    window: &ApplicationWindow,
    shell_state: Rc<ShellUiState>,
) -> MenuButton {
    let button = MenuButton::builder().label("File").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let open_project = build_icon_label_shortcut_button(
        "folder-open-line.svg",
        "Open Project...",
        Some("Ctrl+O"),
    );
    open_project.add_css_class("menu-dropdown-item");
    {
        let parent = window.clone();
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        open_project.connect_clicked(move |_| {
            popover.popdown();
            choose_open_project(&parent, controller.clone());
        });
    }
    menu.append(&open_project);

    let import_image = build_icon_label_button("image-add-line.svg", "Import Image...");
    import_image.add_css_class("menu-dropdown-item");
    {
        let parent = window.clone();
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        import_image.connect_clicked(move |_| {
            popover.popdown();
            choose_import_image(&parent, controller.clone());
        });
    }
    menu.append(&import_image);

    let save = build_icon_label_shortcut_button("save-3-line.svg", "Save", Some("Ctrl+S"));
    save.add_css_class("menu-dropdown-item");
    {
        let shell_state = shell_state.clone();
        let popover = popover.clone();
        save.connect_clicked(move |_| {
            popover.popdown();
            shell_state.request_project_save();
        });
    }
    menu.append(&save);

    let save_as = build_icon_label_shortcut_button(
        "save-3-line.svg",
        "Save As...",
        Some("Ctrl+Shift+S"),
    );
    save_as.add_css_class("menu-dropdown-item");
    {
        let shell_state = shell_state.clone();
        let popover = popover.clone();
        save_as.connect_clicked(move |_| {
            popover.popdown();
            shell_state.request_project_save_as();
        });
    }
    menu.append(&save_as);

    for (label, extension) in [
        ("Export PNG...", "png"),
        ("Export JPEG...", "jpg"),
        ("Export WebP...", "webp"),
    ] {
        let export = build_icon_label_button("export-line.svg", label);
        export.add_css_class("menu-dropdown-item");
        let parent = window.clone();
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        export.connect_clicked(move |_| {
            popover.popdown();
            choose_export_path(&parent, controller.clone(), extension);
        });
        menu.append(&export);
    }

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_window_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("Window").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let color_toggle = build_icon_label_button("palette-line.svg", "Toggle Color Panel");
    color_toggle.add_css_class("menu-dropdown-item");
    {
        let panel = shell_state.color_group.clone();
        let popover = popover.clone();
        color_toggle.connect_clicked(move |_| {
            popover.popdown();
            panel.set_visible(!panel.is_visible());
        });
    }
    menu.append(&color_toggle);

    let properties_toggle = build_icon_label_button("equalizer-line.svg", "Toggle Properties Panel");
    properties_toggle.add_css_class("menu-dropdown-item");
    {
        let panel = shell_state.properties_group.clone();
        let popover = popover.clone();
        properties_toggle.connect_clicked(move |_| {
            popover.popdown();
            panel.set_visible(!panel.is_visible());
        });
    }
    menu.append(&properties_toggle);

    let layers_toggle = build_icon_label_button("layout-column-line.svg", "Toggle Layers Panel");
    layers_toggle.add_css_class("menu-dropdown-item");
    {
        let panel = shell_state.layers_group.clone();
        let popover = popover.clone();
        layers_toggle.connect_clicked(move |_| {
            popover.popdown();
            panel.set_visible(!panel.is_visible());
        });
    }
    menu.append(&layers_toggle);

    let history_toggle = build_icon_label_button("history-line.svg", "Toggle History Panel");
    history_toggle.add_css_class("menu-dropdown-item");
    {
        let panel = shell_state.history_group.clone();
        let popover = popover.clone();
        history_toggle.connect_clicked(move |_| {
            popover.popdown();
            panel.set_visible(!panel.is_visible());
        });
    }
    menu.append(&history_toggle);

    let show_all = build_icon_label_button("layout-grid-line.svg", "Show All Panels");
    show_all.add_css_class("menu-dropdown-item");
    {
        let color_group = shell_state.color_group.clone();
        let properties_group = shell_state.properties_group.clone();
        let layers_group = shell_state.layers_group.clone();
        let history_group = shell_state.history_group.clone();
        let popover = popover.clone();
        show_all.connect_clicked(move |_| {
            popover.popdown();
            color_group.set_visible(true);
            properties_group.set_visible(true);
            layers_group.set_visible(true);
            history_group.set_visible(true);
        });
    }
    menu.append(&show_all);

    {
        let color_group = shell_state.color_group.clone();
        let properties_group = shell_state.properties_group.clone();
        let layers_group = shell_state.layers_group.clone();
        let history_group = shell_state.history_group.clone();
        let color_toggle = color_toggle.clone();
        let properties_toggle = properties_toggle.clone();
        let layers_toggle = layers_toggle.clone();
        let history_toggle = history_toggle.clone();
        let show_all = show_all.clone();
        popover.connect_show(move |_| {
            let color_visible = color_group.is_visible();
            let properties_visible = properties_group.is_visible();
            let layers_visible = layers_group.is_visible();
            let history_visible = history_group.is_visible();

            set_menu_button_label(&color_toggle, if color_visible { "Hide Color Panel" } else { "Show Color Panel" });
            set_menu_button_label(
                &properties_toggle,
                if properties_visible { "Hide Properties Panel" } else { "Show Properties Panel" },
            );
            set_menu_button_label(&layers_toggle, if layers_visible { "Hide Layers Panel" } else { "Show Layers Panel" });
            set_menu_button_label(&history_toggle, if history_visible { "Hide History Panel" } else { "Show History Panel" });
            show_all.set_sensitive(!(color_visible && properties_visible && layers_visible && history_visible));
        });
    }

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_help_menu_button(window: &ApplicationWindow) -> MenuButton {
    let button = MenuButton::builder().label("Help").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let shortcuts = build_icon_label_button("keyboard-line.svg", "Keyboard Shortcuts");
    shortcuts.add_css_class("menu-dropdown-item");
    {
        let parent = window.clone();
        let popover = popover.clone();
        shortcuts.connect_clicked(move |_| {
            popover.popdown();
            show_info_dialog(
                &parent,
                "Keyboard Shortcuts",
                "Core keyboard shortcuts",
                Some(
                    "Ctrl+O Open Project\nCtrl+S Save\nCtrl+Shift+S Save As\nCtrl+Z Undo\nCtrl+Shift+Z or Ctrl+Y Redo\nCtrl+D Clear Selection\nCtrl+I Invert Selection\nCtrl++ Zoom In\nCtrl+- Zoom Out\nCtrl+0 Fit To View\nV Move Tool\nM Marquee Tool\nT Transform Tool\nB Brush Tool\nE Eraser Tool\nH Hand Tool\nZ Zoom Tool\nEnter Commit Transform\nEsc Cancel Transform or Clear Selection",
                ),
            );
        });
    }
    menu.append(&shortcuts);

    let about = build_icon_label_button("information-line.svg", "About PhotoTux");
    about.add_css_class("menu-dropdown-item");
    {
        let parent = window.clone();
        let popover = popover.clone();
        about.connect_clicked(move |_| {
            popover.popdown();
            show_info_dialog(
                &parent,
                "About PhotoTux",
                "PhotoTux",
                Some(
                    "Linux-first raster editor built with Rust, GTK4, and wgpu.\n\nThe GTK shell owns menus, panels, and status surfaces while the document model remains the source of truth.",
                ),
            );
        });
    }
    menu.append(&about);

    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn build_extension_filter(name: &str, patterns: &[&str]) -> FileFilter {
    let filter = FileFilter::new();
    filter.set_name(Some(name));
    for pattern in patterns {
        filter.add_pattern(pattern);
    }
    filter
}

fn ensure_extension(path: &Path, extension: &str) -> PathBuf {
    match path.extension().and_then(|value| value.to_str()) {
        Some(existing) if existing.eq_ignore_ascii_case(extension) => path.to_path_buf(),
        _ => path.with_extension(extension),
    }
}

fn suggested_export_name(document_title: &str, extension: &str) -> String {
    let stem = document_title
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(document_title);
    format!("{}.{}", stem, extension)
}

fn choose_path<F>(
    parent: &ApplicationWindow,
    title: &str,
    action: FileChooserAction,
    accept_label: &str,
    suggested_name: Option<&str>,
    filters: &[FileFilter],
    on_accept: F,
) where
    F: Fn(PathBuf) + 'static,
{
    let dialog = FileChooserNative::new(
        Some(title),
        Some(parent),
        action,
        Some(accept_label),
        Some("Cancel"),
    );
    dialog.set_modal(true);
    if let Some(name) = suggested_name {
        dialog.set_current_name(name);
    }
    for filter in filters {
        dialog.add_filter(filter);
    }

    let on_accept: Rc<dyn Fn(PathBuf)> = Rc::new(on_accept);
    let parent = parent.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(path) = dialog.file().and_then(|file| file.path()) {
                if action == FileChooserAction::Save && path.exists() {
                    confirm_overwrite(&parent, path, on_accept.clone());
                } else {
                    on_accept(path);
                }
            }
        }
        dialog.destroy();
    });

    dialog.show();
}

fn confirm_overwrite(parent: &ApplicationWindow, path: PathBuf, on_accept: Rc<dyn Fn(PathBuf)>) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(MessageType::Question)
        .buttons(ButtonsType::None)
        .text("Replace existing file?")
        .secondary_text(format!("{} already exists. Do you want to replace it?", path.display()))
        .build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Replace", ResponseType::Accept);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            on_accept(path.clone());
        }
        dialog.destroy();
    });

    dialog.show();
}

fn show_info_dialog(
    parent: &ApplicationWindow,
    title: &str,
    text: &str,
    secondary_text: Option<&str>,
) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(MessageType::Info)
        .buttons(ButtonsType::Close)
        .text(text)
        .secondary_text(secondary_text.unwrap_or(""))
        .build();
    dialog.set_title(Some(title));
    dialog.connect_response(|dialog, _| dialog.destroy());
    dialog.show();
}

fn choose_open_project(parent: &ApplicationWindow, controller: Rc<RefCell<dyn ShellController>>) {
    choose_path(
        parent,
        "Open Project",
        FileChooserAction::Open,
        "Open",
        None,
        &[build_extension_filter("PhotoTux Project", &["*.ptx"])],
        move |path| controller.borrow_mut().open_document(path),
    );
}

fn choose_import_image(parent: &ApplicationWindow, controller: Rc<RefCell<dyn ShellController>>) {
    choose_path(
        parent,
        "Import Image",
        FileChooserAction::Open,
        "Import",
        None,
        &[build_extension_filter(
            "Supported Images",
            &["*.png", "*.jpg", "*.jpeg", "*.webp"],
        )],
        move |path| controller.borrow_mut().import_image(path),
    );
}

fn choose_export_path(
    parent: &ApplicationWindow,
    controller: Rc<RefCell<dyn ShellController>>,
    extension: &'static str,
) {
    let suggested_name = suggested_export_name(&controller.borrow().snapshot().document_title, extension);
    choose_path(
        parent,
        "Export Image",
        FileChooserAction::Save,
        "Export",
        Some(&suggested_name),
        &[build_extension_filter(
            &format!("{}.{}", extension.to_ascii_uppercase(), extension),
            &[&format!("*.{}", extension)],
        )],
        move |path| controller
            .borrow_mut()
            .export_document(ensure_extension(&path, extension)),
    );
}

fn choose_save_project_path(parent: &ApplicationWindow, controller: Rc<RefCell<dyn ShellController>>) {
    choose_save_project_path_with_callback(parent, controller, None);
}

fn choose_save_project_path_with_callback(
    parent: &ApplicationWindow,
    controller: Rc<RefCell<dyn ShellController>>,
    on_requested: Option<Rc<dyn Fn()>>,
) {
    let snapshot = controller.borrow().snapshot();
    let suggested_name = snapshot
        .project_path
        .as_ref()
        .and_then(|path| path.file_name().and_then(|name| name.to_str()))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| suggested_export_name(&snapshot.document_title, "ptx"));

    choose_path(
        parent,
        "Save Project As",
        FileChooserAction::Save,
        "Save",
        Some(&suggested_name),
        &[build_extension_filter("PhotoTux Project", &["*.ptx"])],
        move |path| {
            controller
                .borrow_mut()
                .save_document_as(ensure_extension(&path, "ptx"));
            if let Some(callback) = on_requested.as_ref() {
                callback();
            }
        },
    );
}

fn build_tool_options_bar(
    controller: Rc<RefCell<dyn ShellController>>,
) -> (GtkBox, Image, Label) {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.add_css_class("tool-options-bar");

    let snapshot = controller.borrow().snapshot();
    let tool_icon = build_remix_icon(
        shell_tool_icon(snapshot.active_tool),
        &snapshot.active_tool_name,
        12,
    );
    tool_icon.add_css_class("tool-options-icon");
    bar.append(&tool_icon);

    let tool_name = snapshot.active_tool_name;
    let tool_label = Label::new(Some(&tool_name));
    tool_label.add_css_class("tool-options-label");
    bar.append(&tool_label);

    for title in [
        "Preset: Soft Round",
        "Size 24",
        "Hardness 80%",
        "Opacity 100%",
        "Flow 100%",
        "Mode Normal",
    ] {
        let chip = Button::with_label(title);
        chip.add_css_class("tool-chip");
        chip.add_css_class("tool-option-chip");
        chip.set_has_frame(false);
        bar.append(&chip);
    }

    (bar, tool_icon, tool_label)
}

fn build_workspace_body(shell_state: &ShellUiState) -> GtkBox {
    let outer = GtkBox::new(Orientation::Horizontal, 0);
    outer.add_css_class("workspace-body");
    outer.append(&shell_state.tool_rail);

    let inner = Paned::new(Orientation::Horizontal);
    inner.set_wide_handle(true);
    inner.set_start_child(Some(&build_document_region(shell_state)));
    inner.set_end_child(Some(&build_right_sidebar(shell_state)));
    inner.set_position(1120);
    inner.set_hexpand(true);
    inner.set_vexpand(true);

    outer.append(&inner);
    outer
}

fn build_left_tool_rail(
    controller: Rc<RefCell<dyn ShellController>>,
) -> (GtkBox, Vec<(ShellToolKind, Button)>) {
    let rail = GtkBox::new(Orientation::Vertical, 3);
    rail.add_css_class("tool-rail");
    rail.set_size_request(36, -1);

    let mut buttons = Vec::new();

    for (index, (tool, icon_name, tooltip)) in [
        (ShellToolKind::Move, "drag-move-line.svg", ShellToolKind::Move.label()),
        (
            ShellToolKind::RectangularMarquee,
            "focus-3-line.svg",
            ShellToolKind::RectangularMarquee.label(),
        ),
        (ShellToolKind::Lasso, "focus-3-line.svg", ShellToolKind::Lasso.label()),
        (
            ShellToolKind::Transform,
            "expand-diagonal-2-line.svg",
            ShellToolKind::Transform.label(),
        ),
        (ShellToolKind::Brush, "brush-2-line.svg", ShellToolKind::Brush.label()),
        (ShellToolKind::Eraser, "eraser-line.svg", ShellToolKind::Eraser.label()),
        (ShellToolKind::Hand, "hand.svg", ShellToolKind::Hand.label()),
        (ShellToolKind::Zoom, "zoom-in-line.svg", ShellToolKind::Zoom.label()),
    ]
    .into_iter()
    .enumerate()
    {
        if index == 4 || index == 6 {
            let separator = Separator::new(Orientation::Horizontal);
            separator.add_css_class("tool-separator");
            rail.append(&separator);
        }

        let tooltip = format!("{} ({})", tooltip, shell_tool_shortcut(tool));
        let button = build_icon_only_button(icon_name, &tooltip, "tool-button", 18);
        button.add_css_class("tool-button");
        button.set_size_request(24, 24);
        let tool_controller = controller.clone();
        button.connect_clicked(move |_| tool_controller.borrow_mut().select_tool(tool));
        rail.append(&button);
        buttons.push((tool, button));
    }

    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    rail.append(&spacer);

    let swatches = gtk4::Overlay::new();
    swatches.set_size_request(24, 24);
    swatches.add_css_class("swatch-stack");

    let bg = build_color_chip("", "swatch-bg");
    bg.set_tooltip_text(Some("Background Color"));
    bg.set_halign(gtk4::Align::End);
    bg.set_valign(gtk4::Align::End);
    swatches.set_child(Some(&bg));

    let fg = build_color_chip("", "swatch-fg");
    fg.set_tooltip_text(Some("Foreground Color"));
    fg.set_halign(gtk4::Align::Start);
    fg.set_valign(gtk4::Align::Start);
    swatches.add_overlay(&fg);

    let rail_spacer = GtkBox::new(Orientation::Vertical, 4);
    rail_spacer.set_halign(gtk4::Align::Center);
    rail_spacer.append(&swatches);
    rail.append(&rail_spacer);

    (rail, buttons)
}

fn build_document_region(shell_state: &ShellUiState) -> GtkBox {
    let region = GtkBox::new(Orientation::Vertical, 0);
    region.add_css_class("document-region");

    region.append(&shell_state.document_tabs);
    region.append(&build_document_workspace(shell_state));

    region
}

fn build_document_tabs() -> (GtkBox, Label) {
    let tabs = GtkBox::new(Orientation::Horizontal, 4);
    tabs.add_css_class("document-tabs");

    let active_tab = Button::with_label("");
    active_tab.add_css_class("document-tab-active");
    let active_tab_label = Label::new(None);

    let tab_content = GtkBox::new(Orientation::Horizontal, 6);
    tab_content.add_css_class("document-tab-content");
    active_tab_label.add_css_class("document-tab-title");
    tab_content.append(&active_tab_label);

    active_tab.set_child(Some(&tab_content));
    tabs.append(&active_tab);

    let plus_tab = Button::with_label("+");
    plus_tab.add_css_class("document-tab-add");
    plus_tab.set_sensitive(false);
    plus_tab.set_tooltip_text(Some("Multiple document tabs are not active yet"));
    tabs.append(&plus_tab);

    (tabs, active_tab_label)
}

fn build_document_workspace(shell_state: &ShellUiState) -> GtkBox {
    let workspace = GtkBox::new(Orientation::Vertical, 0);
    workspace.add_css_class("document-workspace");

    let top_strip = GtkBox::new(Orientation::Horizontal, 0);
    let top_left_corner = Label::new(Some(""));
    top_left_corner.add_css_class("ruler-corner");
    top_left_corner.set_size_request(24, 24);
    top_strip.append(&top_left_corner);

    let horizontal_ruler = Label::new(Some("0    250    500    750    1000    1250    1500"));
    horizontal_ruler.add_css_class("ruler-horizontal");
    horizontal_ruler.set_hexpand(true);
    top_strip.append(&horizontal_ruler);

    workspace.append(&top_strip);

    let content = GtkBox::new(Orientation::Horizontal, 0);
    let vertical_ruler = Label::new(Some("0\n250\n500\n750\n1000"));
    vertical_ruler.add_css_class("ruler-vertical");
    vertical_ruler.set_size_request(24, -1);
    content.append(&vertical_ruler);

    let canvas_frame = GtkBox::new(Orientation::Vertical, 0);
    canvas_frame.add_css_class("canvas-frame");
    canvas_frame.set_hexpand(true);
    canvas_frame.set_vexpand(true);

    canvas_frame.append(&shell_state.canvas_picture);

    content.append(&canvas_frame);
    workspace.append(&content);

    workspace
}

fn build_right_sidebar(shell_state: &ShellUiState) -> GtkBox {
    let sidebar = GtkBox::new(Orientation::Horizontal, 0);
    sidebar.add_css_class("right-sidebar");
    sidebar.set_size_request(280, -1);

    let dock_icons = GtkBox::new(Orientation::Vertical, 3);
    dock_icons.add_css_class("panel-icon-strip");
    for (icon_name, tooltip) in [
        ("palette-line.svg", "Color"),
        ("settings-4-line.svg", "Properties"),
        ("layout-column-line.svg", "Layers"),
        ("history-line.svg", "History"),
    ] {
        let button = build_icon_only_button(icon_name, tooltip, "dock-icon-button", 18);
        button.add_css_class("dock-icon-button");
        button.set_size_request(24, 24);
        dock_icons.append(&button);
    }

    let dock = GtkBox::new(Orientation::Vertical, 8);
    dock.add_css_class("panel-dock");
    dock.set_hexpand(true);
    dock.set_vexpand(true);

    let paned_bottom = Paned::new(Orientation::Vertical);
    paned_bottom.set_start_child(Some(&shell_state.layers_group));
    paned_bottom.set_end_child(Some(&shell_state.history_group));
    paned_bottom.set_position(200);
    paned_bottom.set_wide_handle(true);

    let paned_middle = Paned::new(Orientation::Vertical);
    paned_middle.set_start_child(Some(&shell_state.properties_group));
    paned_middle.set_end_child(Some(&paned_bottom));
    paned_middle.set_position(150);
    paned_middle.set_wide_handle(true);

    let paned_top = Paned::new(Orientation::Vertical);
    paned_top.set_start_child(Some(&shell_state.color_group));
    paned_top.set_end_child(Some(&paned_middle));
    paned_top.set_position(150);
    paned_top.set_wide_handle(true);
    paned_top.set_vexpand(true);

    dock.append(&paned_top);

    sidebar.append(&dock_icons);
    sidebar.append(&dock);
    sidebar
}

fn build_status_bar() -> (GtkBox, Label, Label, Label, Label, Label) {
    let bar = GtkBox::new(Orientation::Horizontal, 0);
    bar.add_css_class("status-bar");

    let left = GtkBox::new(Orientation::Horizontal, 16);
    left.add_css_class("status-left");

    let center = GtkBox::new(Orientation::Horizontal, 12);
    center.add_css_class("status-center");

    let right = GtkBox::new(Orientation::Horizontal, 16);
    right.add_css_class("status-right");
    right.set_halign(Align::End);
    right.set_hexpand(true);

    let doc = build_status_label("");
    let zoom = build_status_label("Zoom: 100%");
    let cursor = build_status_label("Tool: Brush (B)");
    let notice = build_status_notice_label("Ready");
    let mode = build_status_label("RGB/8");

    left.append(&doc);
    left.append(&cursor);
    center.append(&notice);
    right.append(&zoom);
    right.append(&mode);

    bar.append(&left);
    bar.append(&center);
    bar.append(&right);
    (bar, doc, zoom, cursor, notice, mode)
}

fn build_panel_group(tabs: &[&str], body: &impl gtk4::prelude::IsA<gtk4::Widget>) -> GtkBox {
    let group = GtkBox::new(Orientation::Vertical, 0);
    group.add_css_class("panel-group");
    group.set_vexpand(true);

    let header = GtkBox::new(Orientation::Horizontal, 2);
    header.add_css_class("panel-group-header");
    for (index, tab) in tabs.iter().enumerate() {
        let button = Button::with_label(tab);
        button.add_css_class("panel-tab");
        if index == 0 {
            button.add_css_class("panel-tab-active");
        }
        header.append(&button);
    }

    group.append(&header);
    group.append(body);
    group
}

fn build_color_chip(label_text: &str, css_class: &str) -> Button {
    let button = Button::with_label(label_text);
    button.add_css_class("color-chip");
    button.add_css_class(css_class);
    button
}

fn build_status_label(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.add_css_class("status-label");
    label
}

fn build_status_notice_label(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.add_css_class("status-notice");
    label.set_hexpand(true);
    label.set_xalign(0.5);
    label
}

struct ShellUiState {
    controller: Rc<RefCell<dyn ShellController>>,
    window: RefCell<Option<ApplicationWindow>>,
    recovery_prompt_visible: Cell<bool>,
    close_prompt_visible: Cell<bool>,
    pending_close_after_save: Cell<bool>,
    allow_close_once: Cell<bool>,
    prompted_recovery_path: RefCell<Option<PathBuf>>,
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
    status_doc: Label,
    status_zoom: Label,
    status_cursor: Label,
    status_notice: Label,
    status_mode: Label,
    last_snapshot: RefCell<Option<ShellSnapshot>>,
    last_zoom_percent: RefCell<u32>,
}

impl ShellUiState {
    fn new(controller: Rc<RefCell<dyn ShellController>>) -> Rc<Self> {
        let (tool_options_bar, tool_options_icon, tool_options_label) =
            build_tool_options_bar(controller.clone());
        let (tool_rail, tool_buttons) = build_left_tool_rail(controller.clone());
        let (document_tabs, document_tab_label) = build_document_tabs();
        let (canvas_picture, canvas_state) = build_canvas_host(controller.clone());
        let automation_shortcuts_enabled = env::var_os("PHOTOTUX_ENABLE_TEST_SHORTCUTS").is_some();

        let color_body = GtkBox::new(Orientation::Vertical, 6);
        color_body.add_css_class("panel-group-body");
        let color_group = build_panel_group(&["Color", "Swatches"], &color_body);

        let properties_body = GtkBox::new(Orientation::Vertical, 4);
        properties_body.add_css_class("panel-group-body");
        let properties_group = build_panel_group(&["Properties", "Adjust"], &properties_body);

        let layers_body = GtkBox::new(Orientation::Vertical, 4);
        layers_body.add_css_class("panel-group-body");
        let layers_group = build_panel_group(&["Layers", "Channels", "Paths"], &layers_body);

        let history_body = GtkBox::new(Orientation::Vertical, 4);
        history_body.add_css_class("panel-group-body");
        history_body.set_vexpand(true);
        let history_group = build_panel_group(&["History"], &history_body);
        history_group.set_vexpand(true);

        let (status_bar, status_doc, status_zoom, status_cursor, status_notice, status_mode) =
            build_status_bar();

        Rc::new(Self {
            controller,
            window: RefCell::new(None),
            recovery_prompt_visible: Cell::new(false),
            close_prompt_visible: Cell::new(false),
            pending_close_after_save: Cell::new(false),
            allow_close_once: Cell::new(false),
            prompted_recovery_path: RefCell::new(None),
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
            status_doc,
            status_zoom,
            status_cursor,
            status_notice,
            status_mode,
            last_snapshot: RefCell::new(None),
            last_zoom_percent: RefCell::new(0),
        })
    }

    fn handle_shortcut(&self, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
        let is_control = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        let is_shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
        let key_char = key.to_unicode().map(|character| character.to_ascii_lowercase());

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
            if matches!(key, gdk::Key::F4) {
                if let Some(index) = self.active_layer_index() {
                    self.controller.borrow_mut().toggle_layer_visibility(index);
                    return true;
                }
            }
            if matches!(key, gdk::Key::F5 | gdk::Key::bracketleft) {
                self.controller.borrow_mut().previous_active_layer_blend_mode();
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
            if matches!(key, gdk::Key::F8 | gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add) {
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
                    let layer_count = self.controller.borrow().snapshot().layers.len();
                    if layer_index < layer_count {
                        self.controller.borrow_mut().select_layer(layer_index);
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
                    if let Some(window) = self.window.borrow().as_ref() {
                        choose_open_project(window, self.controller.clone());
                        return true;
                    }
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
                if self.controller.borrow().snapshot().transform_active {
                    self.controller.borrow_mut().commit_transform();
                    return true;
                }
            }
            gdk::Key::Escape => {
                let snapshot = self.controller.borrow().snapshot();
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
            Some('v') => self.controller.borrow_mut().select_tool(ShellToolKind::Move),
            Some('m') => self.controller.borrow_mut().select_tool(ShellToolKind::RectangularMarquee),
            Some('l') => self.controller.borrow_mut().select_tool(ShellToolKind::Lasso),
            Some('t') => self.controller.borrow_mut().select_tool(ShellToolKind::Transform),
            Some('b') => self.controller.borrow_mut().select_tool(ShellToolKind::Brush),
            Some('e') => self.controller.borrow_mut().select_tool(ShellToolKind::Eraser),
            Some('h') => self.controller.borrow_mut().select_tool(ShellToolKind::Hand),
            Some('z') => self.controller.borrow_mut().select_tool(ShellToolKind::Zoom),
            _ => return false,
        }

        true
    }

    fn active_layer_index(&self) -> Option<usize> {
        self.controller
            .borrow()
            .snapshot()
            .layers
            .iter()
            .position(|layer| layer.is_active)
    }

    fn attach_window(&self, window: ApplicationWindow) {
        self.window.replace(Some(window));
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
                ResponseType::Accept => shell_state.controller.borrow_mut().load_recovery_document(),
                ResponseType::Reject => shell_state.controller.borrow_mut().discard_recovery_document(),
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
                    shell_state.controller.borrow_mut().discard_recovery_document();
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
        choose_save_project_path_with_callback(&window, self.controller.clone(), Some(on_requested));
    }

    fn request_project_save(&self) {
        let snapshot = self.controller.borrow().snapshot();
        if snapshot.project_path.is_some() {
            self.controller.borrow_mut().save_document();
            return;
        }

        if let Some(window) = self.window.borrow().as_ref() {
            choose_save_project_path(window, self.controller.clone());
        } else {
            self.controller.borrow_mut().save_document();
        }
    }

    fn request_project_save_as(&self) {
        if let Some(window) = self.window.borrow().as_ref() {
            choose_save_project_path(window, self.controller.clone());
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
        self.status_zoom.set_label(&format!("Zoom: {}%", zoom_percent));
        self.status_cursor.set_label(&shell_status_hint(&snapshot));
        self.status_notice.set_label(&shell_notice_text(&snapshot));
        apply_status_notice_style(&self.status_notice, &shell_notice_text(&snapshot));
        self.status_mode.set_label("RGB/8");

        if snapshot_changed {
            self.tool_options_label.set_label(&snapshot.active_tool_name);
            self.tool_options_icon
                .set_from_file(Some(remix_icon_path(shell_tool_icon(snapshot.active_tool))));
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

        if self.pending_close_after_save.get() && !current_snapshot.dirty {
            self.pending_close_after_save.set(false);
            self.allow_close_once.set(true);
            if let Some(window) = self.window.borrow().as_ref() {
                window.close();
            }
        }

        let should_prompt_recovery = current_snapshot.recovery_offer_pending
            && self.prompted_recovery_path.borrow().as_ref() != current_snapshot.recovery_path.as_ref();
        if should_prompt_recovery && !self.recovery_prompt_visible.get() {
            self.prompted_recovery_path
                .replace(current_snapshot.recovery_path.clone());
            self.present_recovery_prompt(&current_snapshot);
        }

        self.last_zoom_percent.replace(zoom_percent);
    }

    fn refresh_tool_buttons(&self, snapshot: &ShellSnapshot) {
        for (tool, button) in &self.tool_buttons {
            if *tool == snapshot.active_tool {
                button.add_css_class("tool-button-active");
            } else {
                button.remove_css_class("tool-button-active");
            }
        }
    }

    fn refresh_color_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.color_body);

        let chip_row = GtkBox::new(Orientation::Horizontal, 6);
        chip_row.append(&build_color_swatch_label("FG", snapshot.foreground_color));
        chip_row.append(&build_color_swatch_label("BG", snapshot.background_color));
        self.color_body.append(&chip_row);

        let buttons = GtkBox::new(Orientation::Horizontal, 6);
        let swap = Button::with_label("Swap");
        swap.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            swap.connect_clicked(move |_| controller.borrow_mut().swap_colors());
        }
        buttons.append(&swap);

        let reset = Button::with_label("Reset");
        reset.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            reset.connect_clicked(move |_| controller.borrow_mut().reset_colors());
        }
        buttons.append(&reset);

        self.color_body.append(&buttons);
    }

    fn refresh_properties_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.properties_body);
        let editing_mask = snapshot.active_edit_target_name == "Layer Mask";
        for row in [
            format!("Tool: {}", snapshot.active_tool_name),
            format!("Tool Shortcut: {}", shell_tool_shortcut(snapshot.active_tool)),
            format!("Layer: {}", snapshot.active_layer_name),
            format!("Selection: {}", snapshot.selected_structure_name),
            format!("Edit Target: {}", snapshot.active_edit_target_name),
            format!("Blend: {}", snapshot.active_layer_blend_mode),
            format!("Opacity: {}%", snapshot.active_layer_opacity_percent),
            format!(
                "Visible: {}",
                if snapshot.active_layer_visible { "Yes" } else { "No" }
            ),
            format!(
                "Mask: {}",
                if !snapshot.active_layer_has_mask {
                    "None"
                } else if snapshot.active_layer_mask_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            ),
            format!("Brush Radius: {} px", snapshot.brush_radius),
            format!("Brush Hardness: {}%", snapshot.brush_hardness_percent),
            format!("Brush Spacing: {} px", snapshot.brush_spacing),
            format!("Brush Flow: {}%", snapshot.brush_flow_percent),
        ] {
            let label = Label::new(Some(&row));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.properties_body.append(&label);
        }

        let mask_banner = GtkBox::new(Orientation::Vertical, 6);
        mask_banner.add_css_class("mask-state-banner");
        if editing_mask {
            mask_banner.add_css_class("mask-state-banner-active");
        }
        if snapshot.active_layer_has_mask && !snapshot.active_layer_mask_enabled {
            mask_banner.add_css_class("mask-state-banner-disabled");
        }

        let banner_title = Label::new(Some(if !snapshot.active_layer_has_mask {
            "Mask Target"
        } else if editing_mask {
            "Mask Target: Editing Layer Mask"
        } else {
            "Mask Target: Editing Layer Pixels"
        }));
        banner_title.set_xalign(0.0);
        banner_title.add_css_class("mask-state-title");
        mask_banner.append(&banner_title);

        let target_strip = GtkBox::new(Orientation::Horizontal, 6);
        let layer_chip = build_target_chip(
            "Layer",
            "Edit this layer's pixel content",
            !editing_mask,
            true,
        );
        {
            let controller = self.controller.clone();
            layer_chip.connect_clicked(move |_| controller.borrow_mut().edit_active_layer_pixels());
        }
        target_strip.append(&layer_chip);

        let mask_chip = build_target_chip(
            if snapshot.active_layer_mask_enabled { "Mask" } else { "Mask Off" },
            "Edit this layer's mask",
            editing_mask,
            snapshot.active_layer_has_mask,
        );
        if snapshot.active_layer_has_mask {
            let controller = self.controller.clone();
            mask_chip.connect_clicked(move |_| controller.borrow_mut().edit_active_layer_mask());
        }
        target_strip.append(&mask_chip);
        mask_banner.append(&target_strip);

        let banner_hint = Label::new(Some(if !snapshot.active_layer_has_mask {
            "No layer mask is attached to the active layer."
        } else if editing_mask && snapshot.active_layer_mask_enabled {
            "Brush hides content through the mask. Eraser reveals it again."
        } else if editing_mask {
            "Mask edits are live, but the mask is disabled in composite output until re-enabled."
        } else if snapshot.active_layer_mask_enabled {
            "The layer mask affects composite output, but the layer pixels are the current edit target."
        } else {
            "The layer has a disabled mask. Re-enable it to make the mask affect composite output."
        }));
        banner_hint.set_xalign(0.0);
        banner_hint.set_wrap(true);
        banner_hint.add_css_class("mask-state-hint");
        mask_banner.append(&banner_hint);
        self.properties_body.append(&mask_banner);

        if let Some(selection) = snapshot.selection_rect {
            for row in [
                format!(
                    "Selection: {},{}  {}x{}",
                    selection.x, selection.y, selection.width, selection.height
                ),
                format!(
                    "Selection Mode: {}",
                    if snapshot.selection_inverted { "Inverted" } else { "Normal" }
                ),
            ] {
                let label = Label::new(Some(&row));
                label.set_xalign(0.0);
                label.add_css_class("panel-row");
                self.properties_body.append(&label);
            }

            if snapshot.transform_active {
                let label = Label::new(Some(&format!(
                    "Transform: {}% | X {}% | Y {}% | {}deg",
                    snapshot.transform_scale_percent,
                    snapshot.transform_scale_x_percent,
                    snapshot.transform_scale_y_percent,
                    snapshot.transform_rotation_degrees
                )));
                label.set_xalign(0.0);
                label.add_css_class("panel-row");
                self.properties_body.append(&label);
            }

            let guides_label = Label::new(Some(&format!(
                "Guides: {} ({}) | Snapping {}",
                snapshot.guide_count,
                if snapshot.guides_visible { "Visible" } else { "Hidden" },
                if snapshot.snapping_enabled { "On" } else { "Off" }
            )));
            guides_label.set_xalign(0.0);
            guides_label.add_css_class("panel-row");
            self.properties_body.append(&guides_label);
        }

        let controls = GtkBox::new(Orientation::Horizontal, 6);
        let opacity_down = Button::with_label("Opacity -");
        opacity_down.add_css_class("tool-chip");
        opacity_down.set_tooltip_text(Some("Decrease active layer opacity"));
        {
            let controller = self.controller.clone();
            opacity_down.connect_clicked(move |_| controller.borrow_mut().decrease_active_layer_opacity());
        }
        controls.append(&opacity_down);

        let opacity_up = Button::with_label("Opacity +");
        opacity_up.add_css_class("tool-chip");
        opacity_up.set_tooltip_text(Some("Increase active layer opacity"));
        {
            let controller = self.controller.clone();
            opacity_up.connect_clicked(move |_| controller.borrow_mut().increase_active_layer_opacity());
        }
        controls.append(&opacity_up);
        self.properties_body.append(&controls);

        let blend_controls = GtkBox::new(Orientation::Horizontal, 6);
        let blend_prev = Button::with_label("Blend -");
        blend_prev.add_css_class("tool-chip");
        blend_prev.set_tooltip_text(Some("Previous blend mode"));
        {
            let controller = self.controller.clone();
            blend_prev.connect_clicked(move |_| controller.borrow_mut().previous_active_layer_blend_mode());
        }
        blend_controls.append(&blend_prev);

        let blend_next = Button::with_label("Blend +");
        blend_next.add_css_class("tool-chip");
        blend_next.set_tooltip_text(Some("Next blend mode"));
        {
            let controller = self.controller.clone();
            blend_next.connect_clicked(move |_| controller.borrow_mut().next_active_layer_blend_mode());
        }
        blend_controls.append(&blend_next);
        self.properties_body.append(&blend_controls);

        let mask_controls = GtkBox::new(Orientation::Horizontal, 6);

        let add_mask = Button::with_label("Add Mask");
        add_mask.add_css_class("tool-chip");
        add_mask.set_sensitive(!snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            add_mask.connect_clicked(move |_| controller.borrow_mut().add_active_layer_mask());
        }
        mask_controls.append(&add_mask);

        let toggle_mask = Button::with_label(if snapshot.active_layer_mask_enabled { "Mask Off" } else { "Mask On" });
        toggle_mask.add_css_class("tool-chip");
        toggle_mask.set_sensitive(snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            toggle_mask.connect_clicked(move |_| controller.borrow_mut().toggle_active_layer_mask_enabled());
        }
        mask_controls.append(&toggle_mask);

        let remove_mask = Button::with_label("Delete Mask");
        remove_mask.add_css_class("tool-chip");
        remove_mask.set_sensitive(snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            remove_mask.connect_clicked(move |_| controller.borrow_mut().remove_active_layer_mask());
        }
        mask_controls.append(&remove_mask);
        self.properties_body.append(&mask_controls);

        let target_controls = GtkBox::new(Orientation::Horizontal, 6);
        let edit_pixels = Button::with_label("Edit Layer");
        edit_pixels.add_css_class("tool-chip");
        edit_pixels.set_sensitive(snapshot.active_edit_target_name != "Layer Pixels");
        {
            let controller = self.controller.clone();
            edit_pixels.connect_clicked(move |_| controller.borrow_mut().edit_active_layer_pixels());
        }
        target_controls.append(&edit_pixels);

        let edit_mask = Button::with_label("Edit Mask");
        edit_mask.add_css_class("tool-chip");
        edit_mask.set_sensitive(
            snapshot.active_layer_has_mask && snapshot.active_edit_target_name != "Layer Mask",
        );
        {
            let controller = self.controller.clone();
            edit_mask.connect_clicked(move |_| controller.borrow_mut().edit_active_layer_mask());
        }
        target_controls.append(&edit_mask);
        self.properties_body.append(&target_controls);

        let selection_controls = GtkBox::new(Orientation::Horizontal, 6);
        let clear_selection = Button::with_label("Clear Sel");
        clear_selection.add_css_class("tool-chip");
        clear_selection.set_tooltip_text(Some("Clear selection (Ctrl+D)"));
        clear_selection.set_sensitive(snapshot.selection_rect.is_some());
        {
            let controller = self.controller.clone();
            clear_selection.connect_clicked(move |_| controller.borrow_mut().clear_selection());
        }
        selection_controls.append(&clear_selection);

        let invert_selection = Button::with_label("Invert Sel");
        invert_selection.add_css_class("tool-chip");
        invert_selection.set_tooltip_text(Some("Invert selection (Ctrl+I)"));
        invert_selection.set_sensitive(snapshot.selection_rect.is_some());
        {
            let controller = self.controller.clone();
            invert_selection.connect_clicked(move |_| controller.borrow_mut().invert_selection());
        }
        selection_controls.append(&invert_selection);
        self.properties_body.append(&selection_controls);

        let brush_controls_row_one = GtkBox::new(Orientation::Horizontal, 6);

        let radius_down = Button::with_label("Radius -");
        radius_down.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            radius_down.connect_clicked(move |_| controller.borrow_mut().decrease_brush_radius());
        }
        brush_controls_row_one.append(&radius_down);

        let radius_up = Button::with_label("Radius +");
        radius_up.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            radius_up.connect_clicked(move |_| controller.borrow_mut().increase_brush_radius());
        }
        brush_controls_row_one.append(&radius_up);

        let hardness_down = Button::with_label("Hardness -");
        hardness_down.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            hardness_down.connect_clicked(move |_| controller.borrow_mut().decrease_brush_hardness());
        }
        brush_controls_row_one.append(&hardness_down);

        let hardness_up = Button::with_label("Hardness +");
        hardness_up.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            hardness_up.connect_clicked(move |_| controller.borrow_mut().increase_brush_hardness());
        }
        brush_controls_row_one.append(&hardness_up);

        self.properties_body.append(&brush_controls_row_one);

        let brush_controls_row_two = GtkBox::new(Orientation::Horizontal, 6);

        let spacing_down = Button::with_label("Spacing -");
        spacing_down.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            spacing_down.connect_clicked(move |_| controller.borrow_mut().decrease_brush_spacing());
        }
        brush_controls_row_two.append(&spacing_down);

        let spacing_up = Button::with_label("Spacing +");
        spacing_up.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            spacing_up.connect_clicked(move |_| controller.borrow_mut().increase_brush_spacing());
        }
        brush_controls_row_two.append(&spacing_up);

        let flow_down = Button::with_label("Flow -");
        flow_down.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            flow_down.connect_clicked(move |_| controller.borrow_mut().decrease_brush_flow());
        }
        brush_controls_row_two.append(&flow_down);

        let flow_up = Button::with_label("Flow +");
        flow_up.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            flow_up.connect_clicked(move |_| controller.borrow_mut().increase_brush_flow());
        }
        brush_controls_row_two.append(&flow_up);

        self.properties_body.append(&brush_controls_row_two);

        let pressure_controls = GtkBox::new(Orientation::Horizontal, 6);

        let pressure_size = Button::with_label(if snapshot.pressure_size_enabled {
            "Pressure Size On"
        } else {
            "Pressure Size Off"
        });
        pressure_size.add_css_class("tool-chip");
        pressure_size.set_tooltip_text(Some("Toggle pressure-to-size mapping"));
        {
            let controller = self.controller.clone();
            pressure_size.connect_clicked(move |_| controller.borrow_mut().toggle_pressure_size_enabled());
        }
        pressure_controls.append(&pressure_size);

        let pressure_opacity = Button::with_label(if snapshot.pressure_opacity_enabled {
            "Pressure Opacity On"
        } else {
            "Pressure Opacity Off"
        });
        pressure_opacity.add_css_class("tool-chip");
        pressure_opacity.set_tooltip_text(Some("Toggle pressure-to-opacity mapping"));
        {
            let controller = self.controller.clone();
            pressure_opacity.connect_clicked(move |_| controller.borrow_mut().toggle_pressure_opacity_enabled());
        }
        pressure_controls.append(&pressure_opacity);

        self.properties_body.append(&pressure_controls);

        let guide_controls = GtkBox::new(Orientation::Horizontal, 6);

        let add_h_guide = Button::with_label("Guide H");
        add_h_guide.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            add_h_guide.connect_clicked(move |_| controller.borrow_mut().add_horizontal_guide());
        }
        guide_controls.append(&add_h_guide);

        let add_v_guide = Button::with_label("Guide V");
        add_v_guide.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            add_v_guide.connect_clicked(move |_| controller.borrow_mut().add_vertical_guide());
        }
        guide_controls.append(&add_v_guide);

        let toggle_guides = Button::with_label(if snapshot.guides_visible { "Hide Guides" } else { "Show Guides" });
        toggle_guides.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            toggle_guides.connect_clicked(move |_| controller.borrow_mut().toggle_guides_visible());
        }
        guide_controls.append(&toggle_guides);

        let remove_guide = Button::with_label("Remove Guide");
        remove_guide.add_css_class("tool-chip");
        remove_guide.set_sensitive(snapshot.guide_count > 0);
        {
            let controller = self.controller.clone();
            remove_guide.connect_clicked(move |_| controller.borrow_mut().remove_last_guide());
        }
        guide_controls.append(&remove_guide);

        let toggle_snapping = Button::with_label(if snapshot.snapping_enabled { "Snap On" } else { "Snap Off" });
        toggle_snapping.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            toggle_snapping.connect_clicked(move |_| controller.borrow_mut().toggle_snapping_enabled());
        }
        guide_controls.append(&toggle_snapping);
        self.properties_body.append(&guide_controls);

        let transform_controls = GtkBox::new(Orientation::Horizontal, 6);

        let begin_transform = Button::with_label("Start Xform");
        begin_transform.add_css_class("tool-chip");
        begin_transform.set_tooltip_text(Some("Start transform (T)"));
        begin_transform.set_sensitive(snapshot.active_layer_bounds.is_some() && !snapshot.transform_active);
        {
            let controller = self.controller.clone();
            begin_transform.connect_clicked(move |_| controller.borrow_mut().begin_transform());
        }
        transform_controls.append(&begin_transform);

        let scale_down = Button::with_label("Scale -");
        scale_down.add_css_class("tool-chip");
        scale_down.set_tooltip_text(Some("Scale transform down"));
        scale_down.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_down.connect_clicked(move |_| controller.borrow_mut().scale_transform_down());
        }
        transform_controls.append(&scale_down);

        let scale_up = Button::with_label("Scale +");
        scale_up.add_css_class("tool-chip");
        scale_up.set_tooltip_text(Some("Scale transform up"));
        scale_up.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_up.connect_clicked(move |_| controller.borrow_mut().scale_transform_up());
        }
        transform_controls.append(&scale_up);
        self.properties_body.append(&transform_controls);

        let transform_axis_controls = GtkBox::new(Orientation::Horizontal, 6);

        let scale_x_down = Button::with_label("Scale X-");
        scale_x_down.add_css_class("tool-chip");
        scale_x_down.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_x_down.connect_clicked(move |_| controller.borrow_mut().scale_transform_x_down());
        }
        transform_axis_controls.append(&scale_x_down);

        let scale_x_up = Button::with_label("Scale X+");
        scale_x_up.add_css_class("tool-chip");
        scale_x_up.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_x_up.connect_clicked(move |_| controller.borrow_mut().scale_transform_x_up());
        }
        transform_axis_controls.append(&scale_x_up);

        let scale_y_down = Button::with_label("Scale Y-");
        scale_y_down.add_css_class("tool-chip");
        scale_y_down.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_y_down.connect_clicked(move |_| controller.borrow_mut().scale_transform_y_down());
        }
        transform_axis_controls.append(&scale_y_down);

        let scale_y_up = Button::with_label("Scale Y+");
        scale_y_up.add_css_class("tool-chip");
        scale_y_up.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_y_up.connect_clicked(move |_| controller.borrow_mut().scale_transform_y_up());
        }
        transform_axis_controls.append(&scale_y_up);

        self.properties_body.append(&transform_axis_controls);

        let transform_rotate_controls = GtkBox::new(Orientation::Horizontal, 6);
        let rotate_left = Button::with_label("Rotate L");
        rotate_left.add_css_class("tool-chip");
        rotate_left.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            rotate_left.connect_clicked(move |_| controller.borrow_mut().rotate_transform_left());
        }
        transform_rotate_controls.append(&rotate_left);

        let rotate_right = Button::with_label("Rotate R");
        rotate_right.add_css_class("tool-chip");
        rotate_right.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            rotate_right.connect_clicked(move |_| controller.borrow_mut().rotate_transform_right());
        }
        transform_rotate_controls.append(&rotate_right);
        self.properties_body.append(&transform_rotate_controls);

        let transform_commit_row = GtkBox::new(Orientation::Horizontal, 6);
        let commit_transform = Button::with_label("Commit Xform");
        commit_transform.add_css_class("tool-chip");
        commit_transform.set_tooltip_text(Some("Commit transform (Enter)"));
        commit_transform.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            commit_transform.connect_clicked(move |_| controller.borrow_mut().commit_transform());
        }
        transform_commit_row.append(&commit_transform);

        let cancel_transform = Button::with_label("Cancel Xform");
        cancel_transform.add_css_class("tool-chip");
        cancel_transform.set_tooltip_text(Some("Cancel transform (Esc)"));
        cancel_transform.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            cancel_transform.connect_clicked(move |_| controller.borrow_mut().cancel_transform());
        }
        transform_commit_row.append(&cancel_transform);
        self.properties_body.append(&transform_commit_row);

        let hints = [
            shell_status_hint(snapshot),
            "Save: Ctrl+S | Save As: Ctrl+Shift+S | Open: Ctrl+O".to_string(),
            "Zoom: Ctrl++ / Ctrl+- / Ctrl+0".to_string(),
        ];
        for hint in hints {
            let label = Label::new(Some(&hint));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            label.add_css_class("panel-hint-row");
            self.properties_body.append(&label);
        }
    }

    fn refresh_layers_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.layers_body);

        let actions = GtkBox::new(Orientation::Horizontal, 4);
        actions.add_css_class("layers-toolbar");
        for (label, action) in [
            ("+ Layer", LayerAction::Add),
            ("+ Group", LayerAction::AddGroup),
            ("Ungroup", LayerAction::Ungroup),
            ("Duplicate", LayerAction::Duplicate),
            ("Delete", LayerAction::Delete),
            ("Into Group", LayerAction::MoveIntoGroup),
            ("Out Group", LayerAction::MoveOutOfGroup),
            ("+ Mask", LayerAction::AddMask),
            (
                if snapshot.active_layer_mask_enabled { "Mask Off" } else { "Mask On" },
                LayerAction::ToggleMask,
            ),
            (
                if snapshot.active_edit_target_name == "Layer Mask" { "Edit Layer" } else { "Edit Mask" },
                LayerAction::ToggleMaskTarget,
            ),
            ("Up", LayerAction::MoveUp),
            ("Down", LayerAction::MoveDown),
        ] {
            let button = Button::with_label(label);
            button.add_css_class("tool-chip");
            button.add_css_class("layer-action-chip");
            match action {
                LayerAction::AddGroup => button.set_sensitive(snapshot.can_create_group_from_active_layer),
                LayerAction::Ungroup => button.set_sensitive(snapshot.can_ungroup_selected_group),
                LayerAction::MoveIntoGroup => button.set_sensitive(snapshot.can_move_active_layer_into_selected_group),
                LayerAction::MoveOutOfGroup => button.set_sensitive(snapshot.can_move_active_layer_out_of_group),
                LayerAction::AddMask => button.set_sensitive(!snapshot.active_layer_has_mask),
                LayerAction::ToggleMask => button.set_sensitive(snapshot.active_layer_has_mask),
                LayerAction::ToggleMaskTarget => button.set_sensitive(snapshot.active_layer_has_mask),
                _ => {}
            }
            let controller = self.controller.clone();
            button.connect_clicked(move |_| match action {
                LayerAction::Add => controller.borrow_mut().add_layer(),
                LayerAction::AddGroup => controller.borrow_mut().create_group_from_active_layer(),
                LayerAction::Ungroup => controller.borrow_mut().ungroup_selected_group(),
                LayerAction::Duplicate => controller.borrow_mut().duplicate_active_layer(),
                LayerAction::Delete => controller.borrow_mut().delete_active_layer(),
                LayerAction::MoveIntoGroup => controller.borrow_mut().move_active_layer_into_selected_group(),
                LayerAction::MoveOutOfGroup => controller.borrow_mut().move_active_layer_out_of_group(),
                LayerAction::AddMask => controller.borrow_mut().add_active_layer_mask(),
                LayerAction::ToggleMask => controller.borrow_mut().toggle_active_layer_mask_enabled(),
                LayerAction::ToggleMaskTarget => {
                    let mut controller = controller.borrow_mut();
                    let snapshot = controller.snapshot();
                    if snapshot.active_edit_target_name == "Layer Mask" {
                        controller.edit_active_layer_pixels();
                    } else {
                        controller.edit_active_layer_mask();
                    }
                }
                LayerAction::MoveUp => controller.borrow_mut().move_active_layer_up(),
                LayerAction::MoveDown => controller.borrow_mut().move_active_layer_down(),
            });
            actions.append(&button);
        }
        self.layers_body.append(&actions);

        for layer in &snapshot.layers {
            let row = GtkBox::new(Orientation::Horizontal, 4);
            row.add_css_class(if layer.is_selected { "layer-row-active" } else { "layer-row" });
            row.set_margin_start((layer.depth as i32) * 14);
            if layer.mask_target_active {
                row.add_css_class("layer-row-mask-target");
            }
            if layer.has_mask && !layer.mask_enabled {
                row.add_css_class("layer-row-mask-disabled");
            }
            if layer.is_group {
                row.add_css_class("layer-row-group");
            }

            let visibility_icon = if layer.visible {
                "eye-line.svg"
            } else {
                "eye-off-line.svg"
            };
            let visibility =
                build_icon_only_button(visibility_icon, "Toggle Visibility", "menu-button", 12);
            visibility.add_css_class("layer-visibility-button");
            if let Some(index) = layer.index {
                let controller = self.controller.clone();
                visibility.connect_clicked(move |_| controller.borrow_mut().toggle_layer_visibility(index));
            } else if let Some(group_id) = layer.group_id {
                let controller = self.controller.clone();
                visibility.connect_clicked(move |_| controller.borrow_mut().toggle_group_visibility(group_id));
            }
            row.append(&visibility);

            if layer.is_group {
                let target_strip = GtkBox::new(Orientation::Horizontal, 3);
                target_strip.add_css_class("layer-target-strip");
                let group_chip = build_target_chip("G", "Select this group", layer.is_selected, true);
                if let Some(group_id) = layer.group_id {
                    let controller = self.controller.clone();
                    group_chip.connect_clicked(move |_| controller.borrow_mut().select_group(group_id));
                }
                target_strip.append(&group_chip);
                row.append(&target_strip);

                let select = Button::with_label(&format!("{}  ({}%) [Group]", layer.name, layer.opacity_percent));
                select.add_css_class("layer-select-button");
                if layer.is_selected {
                    select.add_css_class("layer-select-button-active");
                }
                if let Some(group_id) = layer.group_id {
                    let controller = self.controller.clone();
                    select.connect_clicked(move |_| controller.borrow_mut().select_group(group_id));
                }
                row.append(&select);
            } else {
                let target_strip = GtkBox::new(Orientation::Horizontal, 3);
                target_strip.add_css_class("layer-target-strip");

                let layer_target = build_target_chip(
                    "L",
                    "Select the layer and edit its pixels",
                    layer.is_active && !layer.mask_target_active,
                    true,
                );
                if let Some(index) = layer.index {
                    let controller = self.controller.clone();
                    layer_target.connect_clicked(move |_| {
                        let mut controller = controller.borrow_mut();
                        controller.select_layer(index);
                        controller.edit_active_layer_pixels();
                    });
                }
                target_strip.append(&layer_target);

                let mask_target = build_target_chip(
                    if layer.mask_enabled { "M" } else { "M!" },
                    "Select the layer and edit its mask",
                    layer.mask_target_active,
                    layer.has_mask,
                );
                if layer.has_mask {
                    if let Some(index) = layer.index {
                        let controller = self.controller.clone();
                        mask_target.connect_clicked(move |_| {
                            let mut controller = controller.borrow_mut();
                            controller.select_layer(index);
                            controller.edit_active_layer_mask();
                        });
                    }
                }
                target_strip.append(&mask_target);
                row.append(&target_strip);

                let mask_suffix = if !layer.has_mask {
                    String::new()
                } else if layer.mask_target_active {
                    if layer.mask_enabled {
                        "  [Mask Editing]".to_string()
                    } else {
                        "  [Mask Editing Off]".to_string()
                    }
                } else {
                    if layer.mask_enabled {
                        "  [Mask]".to_string()
                    } else {
                        "  [Mask Off]".to_string()
                    }
                };
                let select = Button::with_label(&format!("{}  ({}%){}", layer.name, layer.opacity_percent, mask_suffix));
                select.add_css_class("layer-select-button");
                if layer.is_selected {
                    select.add_css_class("layer-select-button-active");
                }
                if let Some(index) = layer.index {
                    let controller = self.controller.clone();
                    select.connect_clicked(move |_| controller.borrow_mut().select_layer(index));
                }
                row.append(&select);
            }

            self.layers_body.append(&row);
        }
    }

    fn refresh_history_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.history_body);

        let actions = GtkBox::new(Orientation::Horizontal, 6);
        actions.add_css_class("history-toolbar");
        let undo = Button::with_label("Undo");
        undo.add_css_class("tool-chip");
        undo.set_sensitive(snapshot.can_undo);
        {
            let controller = self.controller.clone();
            undo.connect_clicked(move |_| controller.borrow_mut().undo());
        }
        actions.append(&undo);

        let redo = Button::with_label("Redo");
        redo.add_css_class("tool-chip");
        redo.set_sensitive(snapshot.can_redo);
        {
            let controller = self.controller.clone();
            redo.connect_clicked(move |_| controller.borrow_mut().redo());
        }
        actions.append(&redo);
        self.history_body.append(&actions);

        let active_index = snapshot.history_entries.len().saturating_sub(1);
        for (index, entry) in snapshot.history_entries.iter().enumerate() {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.add_css_class(if index == active_index {
                "history-item-active"
            } else {
                "history-item"
            });

            let icon = Label::new(Some("•"));
            icon.add_css_class("history-icon");
            row.append(&icon);

            let label = Label::new(Some(entry));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            label.add_css_class("history-name");
            row.append(&label);

            self.history_body.append(&row);
        }
    }
}

fn remix_icon_path(icon_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/remixicon")
        .join(icon_name)
}

fn logo_icon_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/logo/Logo_01.png")
}

fn build_logo_icon(alt_text: &str, size: i32) -> Image {
    let image = Image::from_file(logo_icon_path());
    image.set_pixel_size(size);
    image.set_halign(Align::Center);
    image.set_valign(Align::Center);
    image.set_tooltip_text(Some(alt_text));
    image
}

fn build_remix_icon(icon_name: &str, alt_text: &str, size: i32) -> Image {
    let image = Image::from_file(remix_icon_path(icon_name));
    image.set_pixel_size(size);
    image.set_halign(Align::Center);
    image.set_valign(Align::Center);
    image.set_tooltip_text(Some(alt_text));
    image.add_css_class("remix-icon");
    image
}

fn build_icon_only_button(icon_name: &str, tooltip: &str, css_class: &str, size: i32) -> Button {
    let button = Button::new();
    button.add_css_class(css_class);
    button.set_has_frame(false);
    button.set_tooltip_text(Some(tooltip));

    let icon = build_remix_icon(icon_name, tooltip, size);
    button.set_child(Some(&icon));
    button
}

fn build_icon_label_button(icon_name: &str, label: &str) -> Button {
    build_icon_label_shortcut_button(icon_name, label, None)
}

fn build_icon_label_shortcut_button(icon_name: &str, label: &str, shortcut: Option<&str>) -> Button {
    let button = Button::new();
    button.set_has_frame(false);
    button.set_hexpand(true);
    button.set_halign(Align::Fill);
    button.set_tooltip_text(Some(label));

    let content = GtkBox::new(Orientation::Horizontal, 5);
    content.append(&build_remix_icon(icon_name, label, 12));
    content.set_hexpand(true);

    let text = Label::new(Some(label));
    text.set_xalign(0.0);
    text.set_hexpand(true);
    text.add_css_class("icon-label-text");
    content.append(&text);

    if let Some(shortcut) = shortcut {
        let shortcut_label = Label::new(Some(shortcut));
        shortcut_label.set_xalign(1.0);
        shortcut_label.add_css_class("icon-label-shortcut");
        content.append(&shortcut_label);
    }

    button.set_child(Some(&content));
    button
}

fn set_menu_button_label(button: &Button, label: &str) {
    if let Some(content) = button.child().and_downcast::<GtkBox>() {
        if let Some(widget) = content.first_child() {
            let mut current = widget.next_sibling();
            while let Some(child) = current {
                if let Ok(text) = child.clone().downcast::<Label>() {
                    if text.css_classes().iter().any(|class_name| class_name == "icon-label-text") {
                        text.set_label(label);
                        button.set_tooltip_text(Some(label));
                        break;
                    }
                }
                current = child.next_sibling();
            }
        }
    }
}

fn create_menu_popover(button: &MenuButton) -> (Popover, GtkBox) {
    let popover = Popover::new();
    popover.set_has_arrow(false);
    popover.add_css_class("menu-dropdown");
    popover.set_position(gtk4::PositionType::Bottom);

    let anchor = button.clone();
    popover.connect_show(move |popover| {
        let button_width = anchor.width().max(1);
        let visible_width = popover.child().map(|child| child.width()).unwrap_or(220).max(220);
        let offset_x = ((visible_width - button_width) / 2).max(0);
        popover.set_offset(offset_x, 0);
    });

    let menu = GtkBox::new(Orientation::Vertical, 0);
    menu.add_css_class("menu-dropdown-body");

    (popover, menu)
}

fn shell_tool_icon(tool: ShellToolKind) -> &'static str {
    match tool {
        ShellToolKind::Move => "drag-move-line.svg",
        ShellToolKind::RectangularMarquee => "focus-3-line.svg",
        ShellToolKind::Lasso => "focus-3-line.svg",
        ShellToolKind::Transform => "expand-diagonal-2-line.svg",
        ShellToolKind::Brush => "brush-2-line.svg",
        ShellToolKind::Eraser => "eraser-line.svg",
        ShellToolKind::Hand => "hand.svg",
        ShellToolKind::Zoom => "zoom-in-line.svg",
    }
}

fn shell_tool_shortcut(tool: ShellToolKind) -> &'static str {
    match tool {
        ShellToolKind::Move => "V",
        ShellToolKind::RectangularMarquee => "M",
        ShellToolKind::Lasso => "L",
        ShellToolKind::Transform => "T",
        ShellToolKind::Brush => "B",
        ShellToolKind::Eraser => "E",
        ShellToolKind::Hand => "H",
        ShellToolKind::Zoom => "Z",
    }
}

fn shell_status_hint(snapshot: &ShellSnapshot) -> String {
    let tool_hint = format!("Tool: {} ({})", snapshot.active_tool_name, shell_tool_shortcut(snapshot.active_tool));
    if matches!(snapshot.active_tool, ShellToolKind::Brush | ShellToolKind::Eraser)
        && (snapshot.pressure_size_enabled || snapshot.pressure_opacity_enabled)
    {
        return format!(
            "{} | Radius {} | Hardness {}% | Flow {}% | Pressure {}{}",
            tool_hint,
            snapshot.brush_radius,
            snapshot.brush_hardness_percent,
            snapshot.brush_flow_percent,
            if snapshot.pressure_size_enabled { "size" } else { "" },
            if snapshot.pressure_opacity_enabled {
                if snapshot.pressure_size_enabled { " + opacity" } else { "opacity" }
            } else {
                ""
            }
        );
    }
    if matches!(snapshot.active_tool, ShellToolKind::Brush | ShellToolKind::Eraser) {
        return format!(
            "{} | Radius {} | Hardness {}% | Flow {}%",
            tool_hint,
            snapshot.brush_radius,
            snapshot.brush_hardness_percent,
            snapshot.brush_flow_percent,
        );
    }
    if snapshot.active_edit_target_name == "Layer Mask" {
        return format!("{} | Editing mask | Brush hides | Eraser reveals", tool_hint);
    }
    if snapshot.transform_active {
        if snapshot.snapping_enabled {
            return format!(
                "{} | Enter commit | Esc cancel | Snap {} | Hold Shift bypass",
                tool_hint,
                if snapshot.snapping_temporarily_bypassed { "bypassed" } else { "on" }
            );
        }
        return format!("{} | Enter commit | Esc cancel", tool_hint);
    }
    if matches!(snapshot.active_tool, ShellToolKind::Move) && snapshot.snapping_enabled {
        return format!(
            "{} | Snap {} | Hold Shift bypass",
            tool_hint,
            if snapshot.snapping_temporarily_bypassed { "bypassed" } else { "on" }
        );
    }
    if snapshot.selection_rect.is_some() {
        return format!("{} | Ctrl+D clear | Ctrl+I invert", tool_hint);
    }
    tool_hint
}

fn shell_notice_text(snapshot: &ShellSnapshot) -> String {
    if snapshot.status_message.is_empty() {
        if snapshot.recovery_offer_pending {
            "Recovery available: choose Recover or Discard".to_string()
        } else {
            "Ready".to_string()
        }
    } else {
        snapshot.status_message.clone()
    }
}

fn apply_status_notice_style(label: &Label, message: &str) {
    for class_name in [
        "status-notice-busy",
        "status-notice-success",
        "status-notice-error",
        "status-notice-warning",
    ] {
        label.remove_css_class(class_name);
    }

    let lowered = message.to_ascii_lowercase();
    let class_name = if lowered.contains("failed") || lowered.contains("error") {
        "status-notice-error"
    } else if lowered.contains("saving")
        || lowered.contains("opening")
        || lowered.contains("importing")
        || lowered.contains("exporting")
        || lowered.contains("loading")
        || lowered.contains("autosaving")
    {
        "status-notice-busy"
    } else if lowered.contains("recovery available") || lowered.contains("modified") {
        "status-notice-warning"
    } else {
        "status-notice-success"
    };

    label.add_css_class(class_name);
}

#[derive(Clone, Copy)]
enum LayerAction {
    Add,
    AddGroup,
    Ungroup,
    Duplicate,
    Delete,
    MoveIntoGroup,
    MoveOutOfGroup,
    AddMask,
    ToggleMask,
    ToggleMaskTarget,
    MoveUp,
    MoveDown,
}

fn clear_box_children(container: &GtkBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn build_color_swatch_label(prefix: &str, rgba: [u8; 4]) -> Label {
    let label = Label::new(Some(&format!(
        "{}: #{:02X}{:02X}{:02X}",
        prefix, rgba[0], rgba[1], rgba[2]
    )));
    label.set_xalign(0.0);
    label.add_css_class("panel-row");
    label
}

fn build_target_chip(label: &str, tooltip: &str, active: bool, enabled: bool) -> Button {
    let chip = Button::with_label(label);
    chip.add_css_class("mask-target-chip");
    if active {
        chip.add_css_class("mask-target-chip-active");
    }
    if !enabled {
        chip.add_css_class("mask-target-chip-disabled");
        chip.set_sensitive(false);
    }
    chip.set_tooltip_text(Some(tooltip));
    chip
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

fn build_canvas_host(controller: Rc<RefCell<dyn ShellController>>) -> (Picture, Rc<RefCell<CanvasHostState>>) {
    let picture = Picture::new();
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture.set_can_shrink(true);
    picture.add_css_class("frame");

    let state = Rc::new(RefCell::new(CanvasHostState::new(picture.clone(), controller)));
    wire_canvas_drag(&picture, state.clone());
    wire_canvas_stylus(&picture, state.clone());
    wire_canvas_scroll(&picture, state.clone());

    let tick_state = state.clone();
    glib::timeout_add_local(Duration::from_millis(16), move || {
        tick_state.borrow_mut().tick();
        ControlFlow::Continue
    });

    (picture, state)
}

fn wire_canvas_drag(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let drag = GestureDrag::new();

    {
        let state = state.clone();
        drag.connect_drag_begin(move |gesture, start_x, start_y| {
            let bypass = gesture.current_event_state().contains(gdk::ModifierType::SHIFT_MASK);
            state.borrow_mut().begin_drag(start_x as f32, start_y as f32, bypass);
        });
    }

    {
        let state = state.clone();
        drag.connect_drag_update(move |gesture, offset_x, offset_y| {
            let bypass = gesture.current_event_state().contains(gdk::ModifierType::SHIFT_MASK);
            state.borrow_mut().drag_to(offset_x as f32, offset_y as f32, bypass);
        });
    }

    drag.connect_drag_end(move |_, _, _| {
        state.borrow_mut().end_drag();
    });

    picture.add_controller(drag);
}

fn wire_canvas_scroll(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let picture = picture.clone();
    let picture_for_scroll = picture.clone();
    let scroll = EventControllerScroll::new(
        EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::DISCRETE,
    );

    scroll.connect_scroll(move |_, _, delta_y| {
        let width = picture_for_scroll.width().max(1) as f32;
        let height = picture_for_scroll.height().max(1) as f32;
        state.borrow_mut().zoom(delta_y, width * 0.5, height * 0.5);
        glib::Propagation::Stop
    });

    picture.add_controller(scroll);
}

fn wire_canvas_stylus(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let stylus = GestureStylus::new();

    {
        let state = state.clone();
        stylus.connect_down(move |gesture, _, _| {
            state.borrow_mut().set_pointer_pressure(stylus_pressure(gesture));
        });
    }

    {
        let state = state.clone();
        stylus.connect_motion(move |gesture, _, _| {
            state.borrow_mut().set_pointer_pressure(stylus_pressure(gesture));
        });
    }

    {
        let state = state.clone();
        stylus.connect_up(move |_, _, _| {
            state.borrow_mut().clear_pointer_pressure();
        });
    }

    picture.add_controller(stylus);
}

fn stylus_pressure(gesture: &GestureStylus) -> f32 {
    gesture
        .axis(gdk::AxisUse::Pressure)
        .map(|pressure| pressure.clamp(0.0, 1.0) as f32)
        .unwrap_or(1.0)
}

struct CanvasHostState {
    picture: Picture,
    controller: Rc<RefCell<dyn ShellController>>,
    renderer: Option<OffscreenCanvasRenderer>,
    viewport_state: ViewportState,
    canvas_size: CanvasSize,
    canvas_raster: Option<CanvasRaster>,
    drag_origin_pan: Option<(f32, f32)>,
    drag_start_screen: Option<(f32, f32)>,
    pointer_pressure: f32,
    viewport_fitted: bool,
    last_logical_size: (u32, u32),
    last_canvas_revision: Option<u64>,
    last_active_layer_bounds: Option<CanvasRect>,
    last_selection_rect: Option<CanvasRect>,
    last_selection_path: Option<Vec<(i32, i32)>>,
    last_selection_preview_path: Option<Vec<(i32, i32)>>,
    last_guides_visible: bool,
    last_guides: Vec<ShellGuide>,
    last_selection_inverted: bool,
    dirty: bool,
}

impl CanvasHostState {
    fn new(picture: Picture, controller: Rc<RefCell<dyn ShellController>>) -> Self {
        let renderer = match OffscreenCanvasRenderer::new_blocking() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                tracing::error!(%error, "failed to initialize offscreen canvas renderer");
                None
            }
        };

        Self {
            picture,
            controller,
            renderer,
            viewport_state: ViewportState::default(),
            canvas_size: CanvasSize::new(1920, 1080),
            canvas_raster: None,
            drag_origin_pan: None,
            drag_start_screen: None,
            pointer_pressure: 1.0,
            viewport_fitted: false,
            last_logical_size: (0, 0),
            last_canvas_revision: None,
            last_active_layer_bounds: None,
            last_selection_rect: None,
            last_selection_path: None,
            last_selection_preview_path: None,
            last_guides_visible: true,
            last_guides: Vec::new(),
            last_selection_inverted: false,
            dirty: true,
        }
    }

    fn tick(&mut self) {
        let snapshot = self.controller.borrow().snapshot();
        if self.canvas_size != snapshot.canvas_size {
            self.canvas_size = snapshot.canvas_size;
            self.viewport_fitted = false;
            self.dirty = true;
        }

        if self.last_canvas_revision != Some(snapshot.canvas_revision) {
            self.canvas_raster = Some(self.controller.borrow().canvas_raster());
            self.last_canvas_revision = Some(snapshot.canvas_revision);
            self.dirty = true;
        }

        if self.last_active_layer_bounds != snapshot.active_layer_bounds
            || self.last_selection_rect != snapshot.selection_rect
            || self.last_selection_path != snapshot.selection_path
            || self.last_selection_preview_path != snapshot.selection_preview_path
            || self.last_guides_visible != snapshot.guides_visible
            || self.last_guides != snapshot.guides
            || self.last_selection_inverted != snapshot.selection_inverted
        {
            self.last_active_layer_bounds = snapshot.active_layer_bounds;
            self.last_selection_rect = snapshot.selection_rect;
            self.last_selection_path = snapshot.selection_path.clone();
            self.last_selection_preview_path = snapshot.selection_preview_path.clone();
            self.last_guides_visible = snapshot.guides_visible;
            self.last_guides = snapshot.guides.clone();
            self.last_selection_inverted = snapshot.selection_inverted;
            self.dirty = true;
        }

        let logical_width = self.picture.width().max(0) as u32;
        let logical_height = self.picture.height().max(0) as u32;

        // If the layout isn't fully calculated by GTK yet (often sizes like 0 or 1 on boot), 
        // wait before trying to fit the canvas view, otherwise it starts massively zoomed out.
        if logical_width <= 1 || logical_height <= 1 {
            return;
        }

        if !self.viewport_fitted {
            self.viewport_state = ViewportState::fit_canvas(
                self.canvas_size,
                ViewportSize::new(logical_width as f32, logical_height as f32),
            );
            self.viewport_fitted = true;
            self.last_logical_size = (logical_width, logical_height);
            self.dirty = true;
        } else if self.last_logical_size != (logical_width, logical_height) {
            self.adjust_viewport_for_resize(logical_width, logical_height);
            self.last_logical_size = (logical_width, logical_height);
            self.dirty = true;
        }

        if !self.dirty {
            return;
        }

        let Some(renderer) = &self.renderer else {
            return;
        };

        let scale_factor = self.picture.scale_factor() as f64;
        let mut overlays = Vec::new();
        let mut overlay_paths = Vec::new();
        if let Some(bounds) = snapshot.active_layer_bounds {
            overlays.push(CanvasOverlayRect {
                rect: bounds,
                stroke_rgba: [79, 140, 255, 255],
                fill_rgba: None,
            });
        }
        if let Some(bounds) = snapshot.transform_preview_rect {
            overlays.push(CanvasOverlayRect {
                rect: bounds,
                stroke_rgba: [255, 170, 61, 255],
                fill_rgba: Some([255, 170, 61, 28]),
            });
        }
        if snapshot.guides_visible {
            for guide in &snapshot.guides {
                match *guide {
                    ShellGuide::Horizontal { y } => overlay_paths.push(CanvasOverlayPath {
                        points: vec![(0, y), (self.canvas_size.width as i32 - 1, y)],
                        stroke_rgba: [255, 72, 72, 220],
                        closed: false,
                    }),
                    ShellGuide::Vertical { x } => overlay_paths.push(CanvasOverlayPath {
                        points: vec![(x, 0), (x, self.canvas_size.height as i32 - 1)],
                        stroke_rgba: [255, 72, 72, 220],
                        closed: false,
                    }),
                }
            }
        }
        if let Some(points) = snapshot.selection_preview_path.clone() {
            overlay_paths.push(CanvasOverlayPath {
                points,
                stroke_rgba: [116, 167, 255, 255],
                closed: false,
            });
        } else if let Some(points) = snapshot.selection_path.clone() {
            overlay_paths.push(CanvasOverlayPath {
                points,
                stroke_rgba: [116, 167, 255, 255],
                closed: true,
            });
        } else if let Some(selection) = snapshot.selection_rect {
            overlays.push(CanvasOverlayRect {
                rect: selection,
                stroke_rgba: [116, 167, 255, 255],
                fill_rgba: Some([79, 140, 255, 36]),
            });
        }
        match renderer.render(
            self.canvas_size,
            self.viewport_state,
            logical_width,
            logical_height,
            scale_factor,
            self.canvas_raster.as_ref(),
            &overlays,
            &overlay_paths,
        ) {
            Ok(frame) => {
                let bytes = glib::Bytes::from_owned(frame.pixels);
                let texture = gdk::MemoryTexture::new(
                    frame.width as i32,
                    frame.height as i32,
                    gdk::MemoryFormat::R8g8b8a8,
                    &bytes,
                    frame.stride,
                );
                self.picture.set_paintable(Some(&texture));
                self.dirty = false;
            }
            Err(error) => {
                tracing::error!(%error, "failed to render offscreen canvas frame");
            }
        }
    }

    fn begin_drag(&mut self, start_x: f32, start_y: f32, snap_bypass: bool) {
        self.drag_start_screen = Some((start_x, start_y));
        let snapshot = self.controller.borrow().snapshot();
        match snapshot.active_tool {
            ShellToolKind::Hand => {
                self.drag_origin_pan = Some((self.viewport_state.pan_x, self.viewport_state.pan_y));
            }
            ShellToolKind::Move
            | ShellToolKind::RectangularMarquee
            | ShellToolKind::Lasso
            | ShellToolKind::Transform
            | ShellToolKind::Brush
            | ShellToolKind::Eraser => {
                let (canvas_x, canvas_y) = self.screen_to_canvas(start_x, start_y);
                self.controller.borrow_mut().set_temporary_snap_bypass(snap_bypass);
                self.controller
                    .borrow_mut()
                    .begin_canvas_interaction_with_pressure(canvas_x, canvas_y, self.pointer_pressure);
            }
            _ => {}
        }
    }

    fn drag_to(&mut self, offset_x: f32, offset_y: f32, snap_bypass: bool) {
        let snapshot = self.controller.borrow().snapshot();
        match snapshot.active_tool {
            ShellToolKind::Hand => {
                if let Some((origin_x, origin_y)) = self.drag_origin_pan {
                    self.viewport_state.pan_x = origin_x + offset_x;
                    self.viewport_state.pan_y = origin_y + offset_y;
                    self.dirty = true;
                }
            }
            ShellToolKind::Move
            | ShellToolKind::RectangularMarquee
            | ShellToolKind::Lasso
            | ShellToolKind::Transform
            | ShellToolKind::Brush
            | ShellToolKind::Eraser => {
                if let Some((start_x, start_y)) = self.drag_start_screen {
                    let (canvas_x, canvas_y) = self.screen_to_canvas(start_x + offset_x, start_y + offset_y);
                    self.controller.borrow_mut().set_temporary_snap_bypass(snap_bypass);
                    self.controller
                        .borrow_mut()
                        .update_canvas_interaction_with_pressure(canvas_x, canvas_y, self.pointer_pressure);
                    self.dirty = true;
                    self.tick();
                }
            }
            _ => {}
        }
    }

    fn end_drag(&mut self) {
        self.drag_origin_pan = None;
        self.drag_start_screen = None;
        self.controller.borrow_mut().set_temporary_snap_bypass(false);
        self.controller.borrow_mut().end_canvas_interaction();
        self.dirty = true;
        self.tick();
    }

    fn set_pointer_pressure(&mut self, pressure: f32) {
        self.pointer_pressure = pressure.clamp(0.0, 1.0);
    }

    fn clear_pointer_pressure(&mut self) {
        self.pointer_pressure = 1.0;
    }

    fn zoom(&mut self, delta_y: f64, focal_x: f32, focal_y: f32) {
        let zoom_factor = if delta_y < 0.0 { 1.1 } else { 1.0 / 1.1 };
        self.viewport_state.zoom_towards(zoom_factor, focal_x, focal_y);
        self.dirty = true;
        self.tick();
    }

    fn zoom_in(&mut self) {
        let width = self.picture.width().max(1) as f32;
        let height = self.picture.height().max(1) as f32;
        self.viewport_state.zoom_towards(1.1, width * 0.5, height * 0.5);
        self.dirty = true;
        self.tick();
    }

    fn zoom_out(&mut self) {
        let width = self.picture.width().max(1) as f32;
        let height = self.picture.height().max(1) as f32;
        self.viewport_state.zoom_towards(1.0 / 1.1, width * 0.5, height * 0.5);
        self.dirty = true;
        self.tick();
    }

    fn fit_to_view(&mut self) {
        let logical_width = self.picture.width().max(0) as u32;
        let logical_height = self.picture.height().max(0) as u32;
        if logical_width <= 1 || logical_height <= 1 {
            return;
        }
        self.viewport_state = ViewportState::fit_canvas(
            self.canvas_size,
            ViewportSize::new(logical_width as f32, logical_height as f32),
        );
        self.viewport_fitted = true;
        self.last_logical_size = (logical_width, logical_height);
        self.dirty = true;
        self.tick();
    }

    fn adjust_viewport_for_resize(&mut self, logical_width: u32, logical_height: u32) {
        let delta_x = logical_width as f32 - self.last_logical_size.0 as f32;
        let delta_y = logical_height as f32 - self.last_logical_size.1 as f32;
        self.viewport_state.pan_x += delta_x * 0.5;
        self.viewport_state.pan_y += delta_y * 0.5;
    }

    fn zoom_percent(&self) -> u32 {
        (self.viewport_state.zoom * 100.0).round().max(1.0) as u32
    }

    fn screen_to_canvas(&self, screen_x: f32, screen_y: f32) -> (i32, i32) {
        let canvas_x = ((screen_x - self.viewport_state.pan_x) / self.viewport_state.zoom).round() as i32;
        let canvas_y = ((screen_y - self.viewport_state.pan_y) / self.viewport_state.zoom).round() as i32;
        (canvas_x, canvas_y)
    }
}

const THEME_CSS: &str = r#"
.app-root {
    background: #1a1a1a;
    color: #e0e0e0;
    font-family: "Inter", "IBM Plex Sans", "Noto Sans", system-ui, sans-serif;
    font-size: 11px;
}

.titlebar {
    min-height: 30px;
    background: #1a1a1a;
    color: #e0e0e0;
    border-bottom: 1px solid #3a3a3a;
}

.app-brand {
    padding: 0 12px;
    min-height: 30px;
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
    min-height: 20px;
    padding: 0 8px;
    background: #232323;
    border-color: #3a3a3a;
    color: #999999;
}

.menu-button:active,
.tool-chip:active,
.tool-button:active {
    background: #232323;
}

.menu-bar {
    min-height: 24px;
    padding: 0 6px;
    background: #232323;
    border-bottom: 1px solid #3a3a3a;
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
    min-height: 34px;
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
    background: transparent;
    border: none;
    border-radius: 0;
    color: #8f8f8f;
    padding: 0 10px;
}

.tool-option-chip:hover {
    background: rgba(255,255,255,0.04);
    border: none;
    color: #e0e0e0;
}

.tool-options-icon {
    margin-left: 2px;
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
    min-width: 24px;
    min-height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 3px;
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
    margin: 7px 7px;
    min-width: 18px;
    opacity: 1;
}

.tool-separator.horizontal {
    color: #4a4a4a;
}

.swatch-stack {
    margin-top: 8px;
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
    background: #535353;
    border: 1px solid #3a3a3a;
    margin-left: 0;
    box-shadow: inset 0 0 0 1px rgba(0,0,0,0.35);
}

.right-sidebar {
    background: #232323;
    border-left: 1px solid #3a3a3a;
}

.panel-icon-strip {
    min-width: 36px;
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
    padding: 8px 12px;
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
    padding: 8px 12px 12px;
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

