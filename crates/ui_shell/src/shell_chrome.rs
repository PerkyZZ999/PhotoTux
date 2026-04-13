use super::*;
use crate::ui_support::build_contextual_icon_label_button;

pub(super) fn build_document_region(shell_state: &ShellUiState) -> GtkBox {
    let region = GtkBox::new(Orientation::Vertical, 0);
    region.add_css_class("document-region");

    region.append(&shell_state.document_tabs);
    region.append(&build_document_workspace(shell_state));

    region
}

pub(super) fn build_tool_options_bar(
    controller: Rc<RefCell<dyn ShellController>>,
) -> (GtkBox, Image, Label) {
    let snapshot = controller.borrow().snapshot();
    let chip_titles = [
        "Preset: Soft Round",
        "Size 24",
        "Hardness 80%",
        "Opacity 100%",
        "Flow 100%",
        "Mode Normal",
    ];

    match load_tool_options_bar_template() {
        Ok(template) => {
            set_image_resource_or_fallback(
                &template.tool_icon,
                &remix_icon_resource_path(shell_tool_icon(snapshot.active_tool)),
                &snapshot.active_tool_name,
                12,
            );
            template.tool_label.set_label(&snapshot.active_tool_name);
            for (button, title) in template.option_chips.iter().zip(chip_titles) {
                button.set_label(title);
                button.set_has_frame(false);
            }
            (template.root, template.tool_icon, template.tool_label)
        }
        Err(error) => {
            tracing::error!(%error, "failed to load tool options template");
            build_tool_options_bar_fallback(snapshot)
        }
    }
}

pub(super) fn build_left_tool_rail(
    controller: Rc<RefCell<dyn ShellController>>,
) -> (GtkBox, Vec<(ShellToolKind, Button)>) {
    let rail = GtkBox::new(Orientation::Vertical, 3);
    rail.add_css_class("tool-rail");
    rail.set_size_request(36, -1);

    let mut buttons = Vec::new();

    for (index, (tool, icon_name, tooltip)) in [
        (
            ShellToolKind::Move,
            "drag-move-line.svg",
            ShellToolKind::Move.label(),
        ),
        (
            ShellToolKind::RectangularMarquee,
            "focus-3-line.svg",
            ShellToolKind::RectangularMarquee.label(),
        ),
        (
            ShellToolKind::Lasso,
            "focus-3-line.svg",
            ShellToolKind::Lasso.label(),
        ),
        (
            ShellToolKind::Transform,
            "expand-diagonal-2-line.svg",
            ShellToolKind::Transform.label(),
        ),
        (
            ShellToolKind::Text,
            "layout-column-line.svg",
            ShellToolKind::Text.label(),
        ),
        (
            ShellToolKind::Brush,
            "brush-2-line.svg",
            ShellToolKind::Brush.label(),
        ),
        (
            ShellToolKind::Eraser,
            "eraser-line.svg",
            ShellToolKind::Eraser.label(),
        ),
        (ShellToolKind::Hand, "hand.svg", ShellToolKind::Hand.label()),
        (
            ShellToolKind::Zoom,
            "zoom-in-line.svg",
            ShellToolKind::Zoom.label(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        if index == 5 || index == 7 {
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

pub(super) fn build_document_tabs() -> (GtkBox, Label) {
    match load_document_tabs_template() {
        Ok(template) => {
            template.active_tab_button.set_can_focus(false);
            template.add_tab_button.set_sensitive(false);
            template
                .add_tab_button
                .set_tooltip_text(Some("Multiple document tabs are not active yet"));
            (template.root, template.active_tab_label)
        }
        Err(error) => {
            tracing::error!(%error, "failed to load document tabs template");
            build_document_tabs_fallback()
        }
    }
}

pub(super) fn build_right_sidebar(shell_state: &ShellUiState) -> GtkBox {
    let sidebar = GtkBox::new(Orientation::Horizontal, 0);
    sidebar.add_css_class("right-sidebar");
    sidebar.set_size_request(300, -1);

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

pub(super) fn build_status_bar() -> (GtkBox, Label, Label, Label, Label, Label) {
    match load_status_bar_template() {
        Ok(template) => (
            template.root,
            template.doc_label,
            template.zoom_label,
            template.cursor_label,
            template.notice_label,
            template.mode_label,
        ),
        Err(error) => {
            tracing::error!(%error, "failed to load status bar template");
            build_status_bar_fallback()
        }
    }
}

pub(super) fn build_panel_group(
    shell_name: &str,
    tabs: &[&str],
    body_spacing: i32,
    body_vexpand: bool,
) -> (GtkBox, GtkBox) {
    match build_panel_group_shell(shell_name, tabs, body_spacing, body_vexpand) {
        Ok(shell) => shell,
        Err(error) => {
            tracing::error!(%error, panel = shell_name, "failed to load panel group template");

            let group = GtkBox::new(Orientation::Vertical, 0);
            group.set_widget_name(&format!("{shell_name}-panel"));
            group.add_css_class("panel-group");
            group.set_vexpand(true);

            let header = GtkBox::new(Orientation::Horizontal, 2);
            header.set_widget_name(&format!("{shell_name}-panel-header"));
            header.add_css_class("panel-group-header");
            for (index, tab) in tabs.iter().enumerate() {
                let button = Button::with_label(tab);
                button.set_widget_name(&format!("{shell_name}-panel-tab-{}", index + 1));
                button.add_css_class("panel-tab");
                if index == 0 {
                    button.add_css_class("panel-tab-active");
                }
                header.append(&button);
            }

            let body = GtkBox::new(Orientation::Vertical, body_spacing);
            body.set_widget_name(&format!("{shell_name}-panel-body"));
            body.add_css_class("panel-group-body");
            body.set_vexpand(body_vexpand);

            group.append(&header);
            group.append(&body);
            (group, body)
        }
    }
}

fn build_document_tabs_fallback() -> (GtkBox, Label) {
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
    canvas_frame.set_halign(Align::Center);
    canvas_frame.set_valign(Align::Center);

    let canvas_overlay = gtk4::Overlay::new();
    canvas_overlay.set_hexpand(true);
    canvas_overlay.set_vexpand(true);
    canvas_overlay.set_child(Some(&shell_state.canvas_picture));
    shell_state.canvas_info_label.set_halign(Align::Center);
    shell_state.canvas_info_label.set_valign(Align::Start);
    shell_state.canvas_info_label.set_margin_top(8);
    canvas_overlay.add_overlay(&shell_state.canvas_info_label);

    let task_bar = GtkBox::new(Orientation::Horizontal, 4);
    task_bar.add_css_class("contextual-task-bar");
    task_bar.set_halign(Align::Center);
    task_bar.set_valign(Align::End);
    task_bar.set_margin_bottom(16);

    let fit_button = build_contextual_icon_label_button("focus-3-line.svg", "Fit View");
    {
        let canvas_state = shell_state.canvas_state.clone();
        fit_button.connect_clicked(move |_| canvas_state.borrow_mut().fit_to_view());
    }
    task_bar.append(&fit_button);

    let zoom_out_button = build_contextual_icon_label_button("zoom-out-line.svg", "Zoom Out");
    {
        let canvas_state = shell_state.canvas_state.clone();
        zoom_out_button.connect_clicked(move |_| canvas_state.borrow_mut().zoom_out());
    }
    task_bar.append(&zoom_out_button);

    let zoom_in_button = build_contextual_icon_label_button("zoom-in-line.svg", "Zoom In");
    {
        let canvas_state = shell_state.canvas_state.clone();
        zoom_in_button.connect_clicked(move |_| canvas_state.borrow_mut().zoom_in());
    }
    task_bar.append(&zoom_in_button);

    let separator = Separator::new(Orientation::Vertical);
    separator.add_css_class("contextual-task-separator");
    task_bar.append(&separator);

    let clear_selection = build_contextual_icon_label_button("close-line.svg", "Clear Selection");
    {
        let controller = shell_state.controller.clone();
        clear_selection.connect_clicked(move |_| controller.borrow_mut().clear_selection());
    }
    task_bar.append(&clear_selection);

    let invert_selection = build_contextual_icon_label_button("swap-line.svg", "Invert Selection");
    {
        let controller = shell_state.controller.clone();
        invert_selection.connect_clicked(move |_| controller.borrow_mut().invert_selection());
    }
    task_bar.append(&invert_selection);

    let edit_pixels = build_contextual_icon_label_button("edit-line.svg", "Layer Pixels");
    edit_pixels.add_css_class("contextual-task-button-primary");
    {
        let controller = shell_state.controller.clone();
        edit_pixels.connect_clicked(move |_| controller.borrow_mut().edit_active_layer_pixels());
    }
    task_bar.append(&edit_pixels);

    let edit_mask = build_contextual_icon_label_button("layout-column-line.svg", "Layer Mask");
    {
        let controller = shell_state.controller.clone();
        edit_mask.connect_clicked(move |_| controller.borrow_mut().edit_active_layer_mask());
    }
    task_bar.append(&edit_mask);

    canvas_overlay.add_overlay(&task_bar);
    canvas_frame.append(&canvas_overlay);

    content.append(&canvas_frame);
    workspace.append(&content);

    workspace
}

fn build_status_bar_fallback() -> (GtkBox, Label, Label, Label, Label, Label) {
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

fn build_tool_options_bar_fallback(snapshot: ShellSnapshot) -> (GtkBox, Image, Label) {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.add_css_class("tool-options-bar");
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
