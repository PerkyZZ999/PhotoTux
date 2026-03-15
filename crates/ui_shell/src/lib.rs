use anyhow::Result;
use common::{CanvasRaster, CanvasRect, CanvasSize, APP_NAME};
use glib::ControlFlow;
use gtk4::prelude::*;
use gtk4::{
    gdk, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, EventControllerKey,
    EventControllerScroll, EventControllerScrollFlags, GestureDrag, HeaderBar, Label, Orientation,
    Paned, Picture, Separator,
};
use render_wgpu::{CanvasOverlayRect, OffscreenCanvasRenderer, ViewportSize, ViewportState};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPanelItem {
    pub index: usize,
    pub name: String,
    pub visible: bool,
    pub opacity_percent: u8,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellToolKind {
    Move,
    RectangularMarquee,
    Transform,
    Brush,
    Eraser,
    Hand,
    Zoom,
}

impl ShellToolKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Move => "Move Tool",
            Self::RectangularMarquee => "Rectangular Marquee",
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
    pub active_layer_bounds: Option<CanvasRect>,
    pub transform_preview_rect: Option<CanvasRect>,
    pub transform_active: bool,
    pub transform_scale_percent: u32,
    pub selection_rect: Option<CanvasRect>,
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
    fn select_layer(&mut self, index: usize);
    fn toggle_layer_visibility(&mut self, index: usize);
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
    fn begin_transform(&mut self);
    fn scale_transform_up(&mut self);
    fn scale_transform_down(&mut self);
    fn commit_transform(&mut self);
    fn cancel_transform(&mut self);
    fn undo(&mut self);
    fn redo(&mut self);
    fn save_document(&mut self);
    fn poll_background_tasks(&mut self);
    fn select_tool(&mut self, tool: ShellToolKind);
    fn begin_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32);
    fn update_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32);
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

    let window = ApplicationWindow::builder()
        .application(application)
        .title(APP_NAME)
        .default_width(1600)
        .default_height(900)
        .build();
    window.add_css_class("app-window");

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.add_css_class("app-root");
    root.append(&build_header_bar());
    root.append(&build_menu_bar());
    root.append(&build_tool_options_bar(controller.clone()));

    let shell_state = ShellUiState::new(controller.clone());
    let workspace = build_workspace_body(&shell_state);
    root.append(&workspace);
    root.append(&shell_state.status_bar);

    window.set_child(Some(&root));
    wire_window_shortcuts(&window, shell_state.clone());
    window.present();

    shell_state.refresh();
    glib::timeout_add_local(Duration::from_millis(150), move || {
        shell_state.refresh();
        ControlFlow::Continue
    });
}

fn build_header_bar() -> HeaderBar {
    let header = HeaderBar::new();
    header.add_css_class("titlebar");

    let title = Label::new(Some(APP_NAME));
    title.add_css_class("titlebar-app-name");
    header.set_title_widget(Some(&title));

    let preset = Button::with_label("Essentials");
    preset.add_css_class("chrome-button");
    header.pack_start(&preset);

    let search = Button::with_label("Search");
    search.add_css_class("chrome-button");
    header.pack_end(&search);
    header
}

fn build_menu_bar() -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 2);
    bar.add_css_class("menu-bar");

    for title in ["File", "Edit", "Image", "Layer", "Select", "Filter", "View", "Window", "Help"] {
        let button = Button::with_label(title);
        button.add_css_class("menu-button");
        bar.append(&button);
    }

    bar
}

fn build_tool_options_bar(controller: Rc<RefCell<dyn ShellController>>) -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.add_css_class("tool-options-bar");

    let tool_name = controller.borrow().snapshot().active_tool_name;
    let tool_label = Label::new(Some(&tool_name));
    tool_label.add_css_class("tool-options-label");
    bar.append(&tool_label);

    for title in ["Preset: Soft Round", "Size 24", "Hardness 80%", "Opacity 100%", "Flow 100%", "Mode Normal"] {
        let chip = Button::with_label(title);
        chip.add_css_class("tool-chip");
        bar.append(&chip);
    }

    bar
}

fn build_workspace_body(shell_state: &ShellUiState) -> Paned {
    let outer = Paned::new(Orientation::Horizontal);
    outer.set_wide_handle(true);
    outer.add_css_class("workspace-body");
    outer.set_start_child(Some(&shell_state.tool_rail));

    let inner = Paned::new(Orientation::Horizontal);
    inner.set_wide_handle(true);
    inner.set_start_child(Some(&build_document_region(shell_state)));
    inner.set_end_child(Some(&build_right_sidebar(shell_state)));
    inner.set_position(1120);

    outer.set_end_child(Some(&inner));
    outer.set_position(60);
    outer
}

fn build_left_tool_rail(controller: Rc<RefCell<dyn ShellController>>) -> (GtkBox, Vec<(ShellToolKind, Button)>) {
    let rail = GtkBox::new(Orientation::Vertical, 4);
    rail.add_css_class("tool-rail");
    rail.set_size_request(44, -1);

    let mut buttons = Vec::new();

    for (index, (tool, label)) in [
        (ShellToolKind::Move, "Move"),
        (ShellToolKind::RectangularMarquee, "Marquee"),
        (ShellToolKind::Transform, "Transform"),
        (ShellToolKind::Brush, "Brush"),
        (ShellToolKind::Eraser, "Eraser"),
        (ShellToolKind::Hand, "Hand"),
        (ShellToolKind::Zoom, "Zoom"),
    ]
    .into_iter()
    .enumerate()
    {
        if index == 4 || index == 6 {
            rail.append(&Separator::new(Orientation::Horizontal));
        }

        let button = Button::with_label(label);
        button.add_css_class("tool-button");
        let tool_controller = controller.clone();
        button.connect_clicked(move |_| tool_controller.borrow_mut().select_tool(tool));
        rail.append(&button);
        buttons.push((tool, button));
    }

    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    rail.append(&spacer);

    let swatches = GtkBox::new(Orientation::Vertical, 4);
    swatches.add_css_class("swatch-stack");
    swatches.append(&build_color_chip("FG", "swatch-fg"));
    swatches.append(&build_color_chip("BG", "swatch-bg"));
    rail.append(&swatches);

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
    let tabs = GtkBox::new(Orientation::Horizontal, 6);
    tabs.add_css_class("document-tabs");

    let active_tab = Button::with_label("");
    active_tab.add_css_class("document-tab-active");
    let active_tab_label = Label::new(None);
    active_tab.set_child(Some(&active_tab_label));
    tabs.append(&active_tab);

    let plus_tab = Button::with_label("+");
    plus_tab.add_css_class("document-tab-add");
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
    sidebar.set_size_request(312, -1);

    let dock_icons = GtkBox::new(Orientation::Vertical, 6);
    dock_icons.add_css_class("panel-icon-strip");
    for icon in ["Clr", "Prop", "Lyr", "His"] {
        let button = Button::with_label(icon);
        button.add_css_class("dock-icon-button");
        dock_icons.append(&button);
    }

    let dock = GtkBox::new(Orientation::Vertical, 8);
    dock.add_css_class("panel-dock");
    dock.set_hexpand(true);
    dock.append(&shell_state.color_group);
    dock.append(&shell_state.properties_group);
    dock.append(&shell_state.layers_group);
    dock.append(&shell_state.history_group);

    sidebar.append(&dock_icons);
    sidebar.append(&dock);
    sidebar
}

fn build_status_bar() -> (GtkBox, Label, Label, Label, Label) {
    let bar = GtkBox::new(Orientation::Horizontal, 12);
    bar.add_css_class("status-bar");
    let doc = build_status_label("");
    let zoom = build_status_label("Zoom: 100%");
    let cursor = build_status_label("Cursor: 0,0");
    let mode = build_status_label("RGB/8");
    bar.append(&doc);
    bar.append(&zoom);
    bar.append(&cursor);
    bar.append(&mode);
    (bar, doc, zoom, cursor, mode)
}

fn build_panel_group(tabs: &[&str], body: &GtkBox) -> GtkBox {
    let group = GtkBox::new(Orientation::Vertical, 0);
    group.add_css_class("panel-group");

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

struct ShellUiState {
    controller: Rc<RefCell<dyn ShellController>>,
    canvas_state: Rc<RefCell<CanvasHostState>>,
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
    status_mode: Label,
}

impl ShellUiState {
    fn new(controller: Rc<RefCell<dyn ShellController>>) -> Rc<Self> {
        let (tool_rail, tool_buttons) = build_left_tool_rail(controller.clone());
        let (document_tabs, document_tab_label) = build_document_tabs();
        let (canvas_picture, canvas_state) = build_canvas_host(controller.clone());

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
        let history_group = build_panel_group(&["History"], &history_body);

        let (status_bar, status_doc, status_zoom, status_cursor, status_mode) = build_status_bar();

        Rc::new(Self {
            controller,
            canvas_state,
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
            status_mode,
        })
    }

    fn handle_shortcut(&self, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
        let is_control = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        let is_shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
        let key_char = key.to_unicode().map(|character| character.to_ascii_lowercase());

        if is_control {
            match key_char {
                Some('z') if is_shift => {
                    self.controller.borrow_mut().redo();
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
                Some('s') => {
                    self.controller.borrow_mut().save_document();
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
            Some('t') => self.controller.borrow_mut().select_tool(ShellToolKind::Transform),
            Some('b') => self.controller.borrow_mut().select_tool(ShellToolKind::Brush),
            Some('e') => self.controller.borrow_mut().select_tool(ShellToolKind::Eraser),
            Some('h') => self.controller.borrow_mut().select_tool(ShellToolKind::Hand),
            Some('z') => self.controller.borrow_mut().select_tool(ShellToolKind::Zoom),
            _ => return false,
        }

        true
    }

    fn refresh(&self) {
        self.controller.borrow_mut().poll_background_tasks();
        let snapshot = self.controller.borrow().snapshot();
        self.document_tab_label
            .set_label(&format!("{}   100%   RGB/8", snapshot.document_title));
        self.status_doc.set_label(&format!(
            "Doc: {} x {}",
            snapshot.canvas_size.width, snapshot.canvas_size.height
        ));
        self.status_zoom.set_label("Zoom: 100%");
        self.status_cursor.set_label("Cursor: 0,0");
        if snapshot.status_message.is_empty() {
            self.status_mode.set_label("RGB/8");
        } else {
            self.status_mode
                .set_label(&format!("RGB/8  {}", snapshot.status_message));
        }

        self.refresh_tool_buttons(&snapshot);
        self.refresh_color_panel(&snapshot);
        self.refresh_properties_panel(&snapshot);
        self.refresh_layers_panel(&snapshot);
        self.refresh_history_panel(&snapshot);
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
        for row in [
            format!("Tool: {}", snapshot.active_tool_name),
            format!("Layer: {}", snapshot.active_layer_name),
            format!("Blend: {}", snapshot.active_layer_blend_mode),
            format!("Opacity: {}%", snapshot.active_layer_opacity_percent),
            format!(
                "Visible: {}",
                if snapshot.active_layer_visible { "Yes" } else { "No" }
            ),
        ] {
            let label = Label::new(Some(&row));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.properties_body.append(&label);
        }

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
                    "Transform: {}%",
                    snapshot.transform_scale_percent
                )));
                label.set_xalign(0.0);
                label.add_css_class("panel-row");
                self.properties_body.append(&label);
            }
        }

        let controls = GtkBox::new(Orientation::Horizontal, 6);
        let opacity_down = Button::with_label("Opacity -");
        opacity_down.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            opacity_down.connect_clicked(move |_| controller.borrow_mut().decrease_active_layer_opacity());
        }
        controls.append(&opacity_down);

        let opacity_up = Button::with_label("Opacity +");
        opacity_up.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            opacity_up.connect_clicked(move |_| controller.borrow_mut().increase_active_layer_opacity());
        }
        controls.append(&opacity_up);
        self.properties_body.append(&controls);

        let blend_controls = GtkBox::new(Orientation::Horizontal, 6);
        let blend_prev = Button::with_label("Blend -");
        blend_prev.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            blend_prev.connect_clicked(move |_| controller.borrow_mut().previous_active_layer_blend_mode());
        }
        blend_controls.append(&blend_prev);

        let blend_next = Button::with_label("Blend +");
        blend_next.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            blend_next.connect_clicked(move |_| controller.borrow_mut().next_active_layer_blend_mode());
        }
        blend_controls.append(&blend_next);
        self.properties_body.append(&blend_controls);

        let selection_controls = GtkBox::new(Orientation::Horizontal, 6);
        let clear_selection = Button::with_label("Clear Sel");
        clear_selection.add_css_class("tool-chip");
        clear_selection.set_sensitive(snapshot.selection_rect.is_some());
        {
            let controller = self.controller.clone();
            clear_selection.connect_clicked(move |_| controller.borrow_mut().clear_selection());
        }
        selection_controls.append(&clear_selection);

        let invert_selection = Button::with_label("Invert Sel");
        invert_selection.add_css_class("tool-chip");
        invert_selection.set_sensitive(snapshot.selection_rect.is_some());
        {
            let controller = self.controller.clone();
            invert_selection.connect_clicked(move |_| controller.borrow_mut().invert_selection());
        }
        selection_controls.append(&invert_selection);
        self.properties_body.append(&selection_controls);

        let transform_controls = GtkBox::new(Orientation::Horizontal, 6);

        let begin_transform = Button::with_label("Start Xform");
        begin_transform.add_css_class("tool-chip");
        begin_transform.set_sensitive(snapshot.active_layer_bounds.is_some() && !snapshot.transform_active);
        {
            let controller = self.controller.clone();
            begin_transform.connect_clicked(move |_| controller.borrow_mut().begin_transform());
        }
        transform_controls.append(&begin_transform);

        let scale_down = Button::with_label("Scale -");
        scale_down.add_css_class("tool-chip");
        scale_down.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_down.connect_clicked(move |_| controller.borrow_mut().scale_transform_down());
        }
        transform_controls.append(&scale_down);

        let scale_up = Button::with_label("Scale +");
        scale_up.add_css_class("tool-chip");
        scale_up.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            scale_up.connect_clicked(move |_| controller.borrow_mut().scale_transform_up());
        }
        transform_controls.append(&scale_up);
        self.properties_body.append(&transform_controls);

        let transform_commit_row = GtkBox::new(Orientation::Horizontal, 6);
        let commit_transform = Button::with_label("Commit Xform");
        commit_transform.add_css_class("tool-chip");
        commit_transform.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            commit_transform.connect_clicked(move |_| controller.borrow_mut().commit_transform());
        }
        transform_commit_row.append(&commit_transform);

        let cancel_transform = Button::with_label("Cancel Xform");
        cancel_transform.add_css_class("tool-chip");
        cancel_transform.set_sensitive(snapshot.transform_active);
        {
            let controller = self.controller.clone();
            cancel_transform.connect_clicked(move |_| controller.borrow_mut().cancel_transform());
        }
        transform_commit_row.append(&cancel_transform);
        self.properties_body.append(&transform_commit_row);
    }

    fn refresh_layers_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.layers_body);

        let actions = GtkBox::new(Orientation::Horizontal, 4);
        for (label, action) in [
            ("+ Layer", LayerAction::Add),
            ("Duplicate", LayerAction::Duplicate),
            ("Delete", LayerAction::Delete),
            ("Up", LayerAction::MoveUp),
            ("Down", LayerAction::MoveDown),
        ] {
            let button = Button::with_label(label);
            button.add_css_class("tool-chip");
            let controller = self.controller.clone();
            button.connect_clicked(move |_| match action {
                LayerAction::Add => controller.borrow_mut().add_layer(),
                LayerAction::Duplicate => controller.borrow_mut().duplicate_active_layer(),
                LayerAction::Delete => controller.borrow_mut().delete_active_layer(),
                LayerAction::MoveUp => controller.borrow_mut().move_active_layer_up(),
                LayerAction::MoveDown => controller.borrow_mut().move_active_layer_down(),
            });
            actions.append(&button);
        }
        self.layers_body.append(&actions);

        for layer in &snapshot.layers {
            let row = GtkBox::new(Orientation::Horizontal, 4);
            row.add_css_class(if layer.is_active { "layer-row-active" } else { "layer-row" });

            let visibility = Button::with_label(if layer.visible { "Eye" } else { "Off" });
            visibility.add_css_class("menu-button");
            {
                let controller = self.controller.clone();
                let index = layer.index;
                visibility.connect_clicked(move |_| controller.borrow_mut().toggle_layer_visibility(index));
            }
            row.append(&visibility);

            let select = Button::with_label(&format!("{}  ({}%)", layer.name, layer.opacity_percent));
            select.add_css_class(if layer.is_active {
                "document-tab-active"
            } else {
                "tool-chip"
            });
            {
                let controller = self.controller.clone();
                let index = layer.index;
                select.connect_clicked(move |_| controller.borrow_mut().select_layer(index));
            }
            row.append(&select);

            self.layers_body.append(&row);
        }
    }

    fn refresh_history_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.history_body);

        let actions = GtkBox::new(Orientation::Horizontal, 6);
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

        for entry in &snapshot.history_entries {
            let label = Label::new(Some(entry));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.history_body.append(&label);
        }
    }
}

#[derive(Clone, Copy)]
enum LayerAction {
    Add,
    Duplicate,
    Delete,
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
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        if shell_state.handle_shortcut(key, modifiers) {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);
}

fn build_canvas_host(controller: Rc<RefCell<dyn ShellController>>) -> (Picture, Rc<RefCell<CanvasHostState>>) {
    let picture = Picture::new();
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture.set_can_shrink(true);
    picture.add_css_class("frame");

    let state = Rc::new(RefCell::new(CanvasHostState::new(picture.clone(), controller)));
    wire_canvas_drag(&picture, state.clone());
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
        drag.connect_drag_begin(move |_, start_x, start_y| {
            state.borrow_mut().begin_drag(start_x as f32, start_y as f32);
        });
    }

    {
        let state = state.clone();
        drag.connect_drag_update(move |_, offset_x, offset_y| {
            state.borrow_mut().drag_to(offset_x as f32, offset_y as f32);
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

struct CanvasHostState {
    picture: Picture,
    controller: Rc<RefCell<dyn ShellController>>,
    renderer: Option<OffscreenCanvasRenderer>,
    viewport_state: ViewportState,
    canvas_size: CanvasSize,
    canvas_raster: Option<CanvasRaster>,
    drag_origin_pan: Option<(f32, f32)>,
    drag_start_screen: Option<(f32, f32)>,
    viewport_fitted: bool,
    last_logical_size: (u32, u32),
    last_canvas_revision: Option<u64>,
    last_active_layer_bounds: Option<CanvasRect>,
    last_selection_rect: Option<CanvasRect>,
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
            viewport_fitted: false,
            last_logical_size: (0, 0),
            last_canvas_revision: None,
            last_active_layer_bounds: None,
            last_selection_rect: None,
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
            || self.last_selection_inverted != snapshot.selection_inverted
        {
            self.last_active_layer_bounds = snapshot.active_layer_bounds;
            self.last_selection_rect = snapshot.selection_rect;
            self.last_selection_inverted = snapshot.selection_inverted;
            self.dirty = true;
        }

        let logical_width = self.picture.width().max(1) as u32;
        let logical_height = self.picture.height().max(1) as u32;

        if logical_width == 0 || logical_height == 0 {
            return;
        }

        if !self.viewport_fitted || self.last_logical_size != (logical_width, logical_height) {
            self.viewport_state = ViewportState::fit_canvas(
                self.canvas_size,
                ViewportSize::new(logical_width as f32, logical_height as f32),
            );
            self.viewport_fitted = true;
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
        if let Some(selection) = snapshot.selection_rect {
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

    fn begin_drag(&mut self, start_x: f32, start_y: f32) {
        self.drag_start_screen = Some((start_x, start_y));
        let snapshot = self.controller.borrow().snapshot();
        match snapshot.active_tool {
            ShellToolKind::Hand => {
                self.drag_origin_pan = Some((self.viewport_state.pan_x, self.viewport_state.pan_y));
            }
            ShellToolKind::Move
            | ShellToolKind::RectangularMarquee
            | ShellToolKind::Transform
            | ShellToolKind::Brush
            | ShellToolKind::Eraser => {
                let (canvas_x, canvas_y) = self.screen_to_canvas(start_x, start_y);
                self.controller.borrow_mut().begin_canvas_interaction(canvas_x, canvas_y);
            }
            _ => {}
        }
    }

    fn drag_to(&mut self, offset_x: f32, offset_y: f32) {
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
            | ShellToolKind::Transform
            | ShellToolKind::Brush
            | ShellToolKind::Eraser => {
                if let Some((start_x, start_y)) = self.drag_start_screen {
                    let (canvas_x, canvas_y) = self.screen_to_canvas(start_x + offset_x, start_y + offset_y);
                    self.controller.borrow_mut().update_canvas_interaction(canvas_x, canvas_y);
                    self.dirty = true;
                }
            }
            _ => {}
        }
    }

    fn end_drag(&mut self) {
        self.drag_origin_pan = None;
        self.drag_start_screen = None;
        self.controller.borrow_mut().end_canvas_interaction();
        self.dirty = true;
    }

    fn zoom(&mut self, delta_y: f64, focal_x: f32, focal_y: f32) {
        let zoom_factor = if delta_y < 0.0 { 1.1 } else { 1.0 / 1.1 };
        self.viewport_state.zoom_towards(zoom_factor, focal_x, focal_y);
        self.dirty = true;
    }

    fn zoom_in(&mut self) {
        let width = self.picture.width().max(1) as f32;
        let height = self.picture.height().max(1) as f32;
        self.viewport_state.zoom_towards(1.1, width * 0.5, height * 0.5);
        self.dirty = true;
    }

    fn zoom_out(&mut self) {
        let width = self.picture.width().max(1) as f32;
        let height = self.picture.height().max(1) as f32;
        self.viewport_state.zoom_towards(1.0 / 1.1, width * 0.5, height * 0.5);
        self.dirty = true;
    }

    fn fit_to_view(&mut self) {
        let logical_width = self.picture.width().max(1) as u32;
        let logical_height = self.picture.height().max(1) as u32;
        self.viewport_state = ViewportState::fit_canvas(
            self.canvas_size,
            ViewportSize::new(logical_width as f32, logical_height as f32),
        );
        self.viewport_fitted = true;
        self.last_logical_size = (logical_width, logical_height);
        self.dirty = true;
    }

    fn screen_to_canvas(&self, screen_x: f32, screen_y: f32) -> (i32, i32) {
        let canvas_x = ((screen_x - self.viewport_state.pan_x) / self.viewport_state.zoom).round() as i32;
        let canvas_y = ((screen_y - self.viewport_state.pan_y) / self.viewport_state.zoom).round() as i32;
        (canvas_x, canvas_y)
    }
}

const THEME_CSS: &str = r#"
.app-root {
    background: #1B1D21;
    color: #E8ECF3;
    font-family: "IBM Plex Sans", "Noto Sans", system-ui, sans-serif;
    font-size: 11px;
}

.titlebar {
    min-height: 28px;
    background: #202329;
    color: #E8ECF3;
    border-bottom: 1px solid #313741;
}

.titlebar-app-name {
    font-weight: 600;
}

.chrome-button,
.menu-button,
.tool-chip,
.tool-button,
.document-tab-active,
.document-tab-add,
.panel-tab,
.dock-icon-button,
.color-chip {
    background: #252930;
    color: #E8ECF3;
    border-radius: 4px;
    border: 1px solid #3A414D;
    padding: 4px 8px;
}

.chrome-button:hover,
.menu-button:hover,
.tool-chip:hover,
.tool-button:hover,
.panel-tab:hover,
.document-tab-active:hover,
.document-tab-add:hover,
.dock-icon-button:hover {
    background: #2A2F37;
}

.menu-bar {
    min-height: 24px;
    padding: 2px 6px;
    background: #202329;
    border-bottom: 1px solid #313741;
}

.menu-button {
    background: transparent;
    border: none;
    border-radius: 3px;
    padding: 3px 8px;
}

.tool-options-bar {
    min-height: 36px;
    padding: 4px 6px;
    background: #202329;
    border-bottom: 1px solid #313741;
}

.tool-options-label {
    margin: 0 8px 0 2px;
    font-weight: 600;
}

.tool-chip {
    padding: 4px 10px;
}

.workspace-body {
    background: #14161A;
}

.tool-rail {
    padding: 8px 6px;
    background: #202329;
    border-right: 1px solid #313741;
}

.tool-button {
    min-height: 30px;
    padding: 0;
}

.tool-button-active {
    background: #3B79F1;
    border-color: #4F8CFF;
}

.swatch-stack {
    margin-top: 8px;
}

.color-chip {
    min-width: 30px;
    min-height: 30px;
    padding: 0;
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
    background: #14161A;
}

.document-tabs {
    min-height: 28px;
    padding: 4px 8px 0 8px;
    background: #202329;
    border-bottom: 1px solid #313741;
}

.document-tab-active,
.document-tab-add {
    background: #2A2F37;
}

.document-workspace {
    background: #14161A;
    padding: 0 8px 8px 8px;
}

.ruler-corner,
.ruler-horizontal,
.ruler-vertical {
    background: #202329;
    color: #8A94A3;
    border: 1px solid #313741;
    font-size: 10px;
}

.ruler-horizontal {
    min-height: 24px;
    padding: 4px 8px;
}

.ruler-vertical {
    padding: 8px 4px;
}

.canvas-frame {
    background: #101216;
    border: 1px solid #3A414D;
    margin-left: 0;
}

.right-sidebar {
    background: #202329;
    border-left: 1px solid #313741;
}

.panel-icon-strip {
    min-width: 36px;
    padding: 8px 4px;
    background: #1D2026;
    border-right: 1px solid #313741;
}

.dock-icon-button {
    min-height: 28px;
    padding: 0;
}

.panel-dock {
    padding: 8px;
    background: #252930;
}

.panel-group {
    border: 1px solid #3A414D;
    background: #252930;
}

.panel-group-header {
    padding: 4px;
    background: #2C3139;
    border-bottom: 1px solid #313741;
}

.panel-tab {
    background: transparent;
    border: none;
    padding: 3px 8px;
    font-size: 10px;
}

.panel-tab-active {
    background: #252930;
    border: 1px solid #3A414D;
}

.panel-group-body {
    padding: 6px;
}

.panel-row {
    color: #B3BCC8;
    padding: 3px 2px;
}

.layer-row,
.layer-row-active {
    padding: 2px 0;
}

.layer-row-active {
    background: rgba(79, 140, 255, 0.10);
}

.status-bar {
    min-height: 20px;
    padding: 3px 8px;
    background: #202329;
    border-top: 1px solid #313741;
}

.status-label {
    color: #B3BCC8;
    font-size: 10px;
}
"#;
