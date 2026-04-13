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
) -> (GtkBox, Image, Label, [Label; 6], [Label; 6]) {
    let snapshot = controller.borrow().snapshot();
    build_tool_options_bar_fallback(snapshot)
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
    swatches.set_size_request(30, 30);
    swatches.add_css_class("swatch-stack");

    let bg = build_color_chip("", "swatch-bg");
    bg.set_tooltip_text(Some("Background Color"));
    bg.set_halign(gtk4::Align::End);
    bg.set_valign(gtk4::Align::End);
    bg.set_margin_bottom(1);
    bg.set_margin_end(1);
    swatches.set_child(Some(&bg));

    let fg = build_color_chip("", "swatch-fg");
    fg.set_tooltip_text(Some("Foreground Color"));
    fg.set_halign(gtk4::Align::Start);
    fg.set_valign(gtk4::Align::Start);
    fg.set_margin_top(1);
    fg.set_margin_start(1);
    swatches.add_overlay(&fg);

    let rail_spacer = GtkBox::new(Orientation::Vertical, 4);
    rail_spacer.set_halign(gtk4::Align::Center);
    rail_spacer.append(&swatches);

    let swatch_actions = GtkBox::new(Orientation::Horizontal, 2);
    swatch_actions.add_css_class("swatch-stack-actions");

    let reset_colors = build_icon_only_button("refresh-line.svg", "Default colors", "chrome-button", 10);
    reset_colors.add_css_class("swatch-stack-action");
    {
        let controller = controller.clone();
        reset_colors.connect_clicked(move |_| controller.borrow_mut().reset_colors());
    }
    swatch_actions.append(&reset_colors);

    let swap_colors = build_icon_only_button("swap-line.svg", "Swap foreground/background", "chrome-button", 10);
    swap_colors.add_css_class("swatch-stack-action");
    {
        let controller = controller.clone();
        swap_colors.connect_clicked(move |_| controller.borrow_mut().swap_colors());
    }
    swatch_actions.append(&swap_colors);

    rail_spacer.append(&swatch_actions);
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

    let canvas_cluster = GtkBox::new(Orientation::Vertical, 0);
    canvas_cluster.add_css_class("canvas-cluster");
    canvas_cluster.set_hexpand(true);
    canvas_cluster.set_vexpand(true);
    canvas_cluster.set_halign(Align::Center);
    canvas_cluster.set_valign(Align::Center);

    let top_strip = GtkBox::new(Orientation::Horizontal, 0);
    top_strip.set_halign(Align::Center);
    let top_left_corner = Label::new(Some(""));
    top_left_corner.add_css_class("ruler-corner");
    top_left_corner.set_size_request(24, 24);
    top_strip.append(&top_left_corner);

    top_strip.append(&shell_state.horizontal_ruler_label);
    canvas_cluster.append(&top_strip);

    let content = GtkBox::new(Orientation::Horizontal, 0);
    content.set_halign(Align::Center);
    content.set_valign(Align::Center);
    content.append(&shell_state.vertical_ruler_label);

    let canvas_frame = GtkBox::new(Orientation::Vertical, 0);
    canvas_frame.add_css_class("canvas-frame");
    canvas_frame.set_halign(Align::Start);
    canvas_frame.set_valign(Align::Start);

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
    canvas_cluster.append(&content);
    workspace.append(&canvas_cluster);

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

pub(super) fn tool_option_groups(snapshot: &ShellSnapshot) -> [(String, String); 6] {
    match snapshot.active_tool {
        ShellToolKind::Brush | ShellToolKind::Eraser => [
            ("Preset".to_string(), snapshot.brush_preset_name.clone()),
            ("Size".to_string(), format!("{} px", snapshot.brush_radius)),
            (
                "Hardness".to_string(),
                format!("{}%", snapshot.brush_hardness_percent),
            ),
            ("Spacing".to_string(), format!("{} px", snapshot.brush_spacing)),
            ("Flow".to_string(), format!("{}%", snapshot.brush_flow_percent)),
            (
                "Pressure".to_string(),
                match (
                    snapshot.pressure_size_enabled,
                    snapshot.pressure_opacity_enabled,
                ) {
                    (true, true) => "Size + Opacity".to_string(),
                    (true, false) => "Size".to_string(),
                    (false, true) => "Opacity".to_string(),
                    (false, false) => "Off".to_string(),
                },
            ),
        ],
        ShellToolKind::Text => [
            ("Font".to_string(), snapshot.text.font_family.clone()),
            ("Size".to_string(), format!("{} px", snapshot.text.font_size_px)),
            (
                "Leading".to_string(),
                format!("{}%", snapshot.text.line_height_percent),
            ),
            (
                "Tracking".to_string(),
                snapshot.text.letter_spacing.to_string(),
            ),
            (
                "Align".to_string(),
                match snapshot.text.alignment {
                    ShellTextAlignment::Left => "Left",
                    ShellTextAlignment::Center => "Center",
                    ShellTextAlignment::Right => "Right",
                }
                .to_string(),
            ),
            (
                "Fill".to_string(),
                format!(
                    "#{:02X}{:02X}{:02X}",
                    snapshot.text.fill_rgba[0],
                    snapshot.text.fill_rgba[1],
                    snapshot.text.fill_rgba[2]
                ),
            ),
        ],
        ShellToolKind::Transform => [
            (
                "Scale".to_string(),
                format!("{}%", snapshot.transform_scale_percent),
            ),
            (
                "Scale X".to_string(),
                format!("{}%", snapshot.transform_scale_x_percent),
            ),
            (
                "Scale Y".to_string(),
                format!("{}%", snapshot.transform_scale_y_percent),
            ),
            (
                "Rotate".to_string(),
                format!("{}°", snapshot.transform_rotation_degrees),
            ),
            ("Target".to_string(), snapshot.active_edit_target_name.clone()),
            (
                "Snap".to_string(),
                if snapshot.snapping_enabled {
                    "On"
                } else {
                    "Off"
                }
                .to_string(),
            ),
        ],
        _ => [
            ("Layer".to_string(), snapshot.active_layer_name.clone()),
            (
                "Canvas".to_string(),
                format!("{}×{}", snapshot.canvas_size.width, snapshot.canvas_size.height),
            ),
            ("Blend".to_string(), snapshot.active_layer_blend_mode.clone()),
            (
                "Opacity".to_string(),
                format!("{}%", snapshot.active_layer_opacity_percent),
            ),
            (
                "Selection".to_string(),
                if snapshot.selection_rect.is_some()
                    || snapshot.selection_path.is_some()
                    || snapshot.selection_preview_path.is_some()
                {
                    "Active"
                } else {
                    "None"
                }
                .to_string(),
            ),
            (
                "Snap".to_string(),
                if snapshot.snapping_enabled {
                    "On"
                } else {
                    "Off"
                }
                .to_string(),
            ),
        ],
    }
}

fn build_tool_options_bar_fallback(snapshot: ShellSnapshot) -> (GtkBox, Image, Label, [Label; 6], [Label; 6]) {
    let bar = GtkBox::new(Orientation::Horizontal, 0);
    bar.add_css_class("tool-options-bar");
    let tool_icon = build_remix_icon(
        shell_tool_icon(snapshot.active_tool),
        &snapshot.active_tool_name,
        12,
    );
    tool_icon.add_css_class("tool-options-icon");
    bar.append(&tool_icon);

    let tool_name = snapshot.active_tool_name.clone();
    let tool_label = Label::new(Some(&tool_name));
    tool_label.add_css_class("tool-options-label");
    bar.append(&tool_label);

    let groups = tool_option_groups(&snapshot);
    let option_keys = std::array::from_fn(|index| {
        let label = Label::new(Some(&groups[index].0));
        label.add_css_class("tool-option-key");
        label
    });
    let option_values = std::array::from_fn(|index| {
        let label = Label::new(Some(&groups[index].1));
        label.add_css_class("tool-option-value");
        label.set_tooltip_text(Some(&format!("{}: {}", groups[index].0, groups[index].1)));
        label
    });

    for index in 0..groups.len() {
        let divider = Separator::new(Orientation::Vertical);
        divider.add_css_class("tool-options-divider");
        bar.append(&divider);

        let group = GtkBox::new(Orientation::Horizontal, 4);
        group.add_css_class("tool-options-group");
        group.append(&option_keys[index]);

        let value_box = GtkBox::new(Orientation::Horizontal, 0);
        value_box.add_css_class("tool-option-box");
        value_box.append(&option_values[index]);
        group.append(&value_box);
        bar.append(&group);
    }

    (bar, tool_icon, tool_label, option_keys, option_values)
}
