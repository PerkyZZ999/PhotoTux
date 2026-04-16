use super::*;
use crate::ui_templates::{
    DocumentTabsTemplate, PanelGroupTemplate, StatusBarTemplate, load_panel_group_template,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolRailSlot {
    Move,
    Selection,
    Transform,
    Text,
    Paint,
    Navigation,
}

pub(super) struct ToolRailSlotButton {
    pub slot: ToolRailSlot,
    pub root: gtk4::Overlay,
    pub button: Button,
    icon: Image,
    visible_tool: Rc<Cell<ShellToolKind>>,
}

#[derive(Clone, Copy)]
struct ToolRailSlotSpec {
    slot: ToolRailSlot,
    default_tool: ShellToolKind,
    separator_before: bool,
}

const TOOL_RAIL_SLOT_SPECS: [ToolRailSlotSpec; 6] = [
    ToolRailSlotSpec {
        slot: ToolRailSlot::Move,
        default_tool: ShellToolKind::Move,
        separator_before: false,
    },
    ToolRailSlotSpec {
        slot: ToolRailSlot::Selection,
        default_tool: ShellToolKind::RectangularMarquee,
        separator_before: false,
    },
    ToolRailSlotSpec {
        slot: ToolRailSlot::Transform,
        default_tool: ShellToolKind::Transform,
        separator_before: true,
    },
    ToolRailSlotSpec {
        slot: ToolRailSlot::Text,
        default_tool: ShellToolKind::Text,
        separator_before: false,
    },
    ToolRailSlotSpec {
        slot: ToolRailSlot::Paint,
        default_tool: ShellToolKind::Brush,
        separator_before: true,
    },
    ToolRailSlotSpec {
        slot: ToolRailSlot::Navigation,
        default_tool: ShellToolKind::Hand,
        separator_before: true,
    },
];

pub(super) fn tool_rail_slot_for_tool(tool: ShellToolKind) -> ToolRailSlot {
    match tool {
        ShellToolKind::Move => ToolRailSlot::Move,
        ShellToolKind::RectangularMarquee | ShellToolKind::Lasso => ToolRailSlot::Selection,
        ShellToolKind::Transform => ToolRailSlot::Transform,
        ShellToolKind::Text => ToolRailSlot::Text,
        ShellToolKind::Brush | ShellToolKind::Eraser => ToolRailSlot::Paint,
        ShellToolKind::Hand | ShellToolKind::Zoom => ToolRailSlot::Navigation,
    }
}

pub(super) fn sync_tool_rail_slot_button(
    slot_button: &ToolRailSlotButton,
    active_tool: ShellToolKind,
) {
    if tool_rail_slot_for_tool(active_tool) == slot_button.slot
        && slot_button.visible_tool.get() != active_tool
    {
        slot_button.visible_tool.set(active_tool);
        update_tool_rail_slot_visuals(
            &slot_button.button,
            &slot_button.icon,
            active_tool,
            slot_button.slot,
        );
    }
}

fn tool_rail_slot_tools(slot: ToolRailSlot) -> &'static [ShellToolKind] {
    match slot {
        ToolRailSlot::Move => &[ShellToolKind::Move],
        ToolRailSlot::Selection => &[ShellToolKind::RectangularMarquee, ShellToolKind::Lasso],
        ToolRailSlot::Transform => &[ShellToolKind::Transform],
        ToolRailSlot::Text => &[ShellToolKind::Text],
        ToolRailSlot::Paint => &[ShellToolKind::Brush, ShellToolKind::Eraser],
        ToolRailSlot::Navigation => &[ShellToolKind::Hand, ShellToolKind::Zoom],
    }
}

fn tool_rail_slot_label(slot: ToolRailSlot) -> &'static str {
    match slot {
        ToolRailSlot::Move => "Move",
        ToolRailSlot::Selection => "Selection",
        ToolRailSlot::Transform => "Transform",
        ToolRailSlot::Text => "Type",
        ToolRailSlot::Paint => "Paint",
        ToolRailSlot::Navigation => "Navigation",
    }
}

pub(super) fn build_document_region(shell_state: &ShellUiState) -> GtkBox {
    let region = GtkBox::new(Orientation::Vertical, 0);
    region.add_css_class("document-region");

    region.append(&shell_state.document_tabs);
    region.append(&build_document_workspace(shell_state));

    region
}

pub(super) fn build_tool_options_bar(
    controller: Rc<RefCell<dyn ShellController>>,
) -> (GtkBox, Image, Label, GtkBox) {
    let snapshot = controller.borrow().snapshot();
    build_tool_options_bar_fallback(snapshot)
}

pub(super) fn build_left_tool_rail(
    controller: Rc<RefCell<dyn ShellController>>,
) -> (GtkBox, Vec<ToolRailSlotButton>) {
    let rail = GtkBox::new(Orientation::Vertical, 3);
    rail.add_css_class("tool-rail");
    rail.set_size_request(36, -1);

    let mut buttons = Vec::new();

    for spec in TOOL_RAIL_SLOT_SPECS {
        if spec.separator_before {
            rail.append(&build_tool_rail_separator());
        }

        let slot_button = build_tool_rail_button(controller.clone(), spec);
        rail.append(&slot_button.root);
        buttons.push(slot_button);
    }

    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    rail.append(&spacer);

    let swatches = gtk4::Overlay::new();
    swatches.set_size_request(30, 30);
    swatches.add_css_class("swatch-stack");

    let bg = build_color_chip("", "swatch-bg");
    bg.set_tooltip_text(Some("Background Color"));
    bg.update_property(&[gtk4::accessible::Property::Label("Background Color")]);
    bg.set_halign(gtk4::Align::End);
    bg.set_valign(gtk4::Align::End);
    bg.set_margin_bottom(1);
    bg.set_margin_end(1);
    swatches.set_child(Some(&bg));

    let fg = build_color_chip("", "swatch-fg");
    fg.set_tooltip_text(Some("Foreground Color"));
    fg.update_property(&[gtk4::accessible::Property::Label("Foreground Color")]);
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

    let reset_colors = build_tool_rail_action_button(
        controller.clone(),
        "refresh-line.svg",
        "Default colors",
        |controller| controller.reset_colors(),
    );
    swatch_actions.append(&reset_colors);

    let swap_colors = build_tool_rail_action_button(
        controller,
        "swap-line.svg",
        "Swap foreground/background",
        |controller| controller.swap_colors(),
    );
    swatch_actions.append(&swap_colors);

    rail_spacer.append(&swatch_actions);
    rail.append(&rail_spacer);

    (rail, buttons)
}

fn build_tool_rail_button(
    controller: Rc<RefCell<dyn ShellController>>,
    spec: ToolRailSlotSpec,
) -> ToolRailSlotButton {
    let visible_tool = Rc::new(Cell::new(spec.default_tool));
    let root = gtk4::Overlay::new();
    root.set_halign(Align::Center);
    root.set_valign(Align::Start);

    let button = Button::new();
    button.add_css_class("tool-button");
    button.set_size_request(24, 24);
    button.set_has_frame(false);

    let overlay = gtk4::Overlay::new();
    overlay.set_halign(Align::Fill);
    overlay.set_valign(Align::Fill);

    let icon = build_remix_icon(
        shell_tool_icon(spec.default_tool),
        spec.default_tool.label(),
        18,
    );
    icon.set_accessible_role(gtk4::AccessibleRole::Presentation);
    overlay.set_child(Some(&icon));

    button.set_child(Some(&overlay));
    update_tool_rail_slot_visuals(&button, &icon, spec.default_tool, spec.slot);
    root.set_child(Some(&button));

    {
        let controller = controller.clone();
        let visible_tool = visible_tool.clone();
        button.connect_clicked(move |_| controller.borrow_mut().select_tool(visible_tool.get()));
    }

    if tool_rail_slot_tools(spec.slot).len() > 1 {
        let menu_button = MenuButton::new();
        menu_button.set_has_frame(false);
        menu_button.add_css_class("tool-button-flyout-hotspot");
        menu_button.add_css_class("tool-button-flyout-indicator");
        menu_button.set_halign(Align::End);
        menu_button.set_valign(Align::End);
        menu_button.set_margin_end(1);
        menu_button.set_margin_bottom(1);
        menu_button.set_label("▾");

        let popover = build_tool_rail_slot_popover(
            controller.clone(),
            button.clone(),
            icon.clone(),
            visible_tool.clone(),
            spec.slot,
        );
        menu_button.set_popover(Some(&popover));
        root.add_overlay(&menu_button);
        attach_tool_rail_slot_flyout(&button, &menu_button);
    }

    ToolRailSlotButton {
        slot: spec.slot,
        root,
        button,
        icon,
        visible_tool,
    }
}

fn build_tool_rail_separator() -> Separator {
    let separator = Separator::new(Orientation::Horizontal);
    separator.add_css_class("tool-separator");
    separator
}

fn build_tool_rail_action_button<F>(
    controller: Rc<RefCell<dyn ShellController>>,
    icon_name: &'static str,
    tooltip: &'static str,
    action: F,
) -> Button
where
    F: Fn(&mut dyn ShellController) + 'static,
{
    let button = build_icon_only_button(icon_name, tooltip, "chrome-button", 10);
    button.add_css_class("swatch-stack-action");
    button.connect_clicked(move |_| {
        let mut controller = controller.borrow_mut();
        action(&mut *controller);
    });
    button
}

fn build_tool_rail_slot_popover(
    controller: Rc<RefCell<dyn ShellController>>,
    anchor: Button,
    icon: Image,
    visible_tool: Rc<Cell<ShellToolKind>>,
    slot: ToolRailSlot,
) -> Popover {
    let popover = Popover::new();
    popover.set_has_arrow(false);
    popover.add_css_class("menu-dropdown");
    popover.set_position(gtk4::PositionType::Right);
    popover.add_css_class("tool-flyout-popover");

    let menu = GtkBox::new(Orientation::Vertical, 0);
    menu.add_css_class("menu-dropdown-body");
    menu.add_css_class("tool-flyout-body");

    for &tool in tool_rail_slot_tools(slot) {
        let label = tool.label();
        let item = build_icon_label_shortcut_button(
            shell_tool_icon(tool),
            label,
            Some(shell_tool_shortcut(tool)),
        );
        if tool == visible_tool.get() {
            item.add_css_class("menu-button-active");
        }
        {
            let controller = controller.clone();
            let anchor = anchor.clone();
            let icon = icon.clone();
            let visible_tool = visible_tool.clone();
            let popover = popover.clone();
            item.connect_clicked(move |_| {
                visible_tool.set(tool);
                update_tool_rail_slot_visuals(&anchor, &icon, tool, slot);
                popover.popdown();
                controller.borrow_mut().select_tool(tool);
            });
        }
        menu.append(&item);
    }

    popover.set_child(Some(&menu));
    popover
}

fn attach_tool_rail_slot_flyout(button: &Button, menu_button: &MenuButton) {
    let secondary_click = GestureClick::new();
    secondary_click.set_button(gdk::BUTTON_SECONDARY);
    {
        let menu_button = menu_button.clone();
        secondary_click.connect_pressed(move |gesture, _, _, _| {
            menu_button.popup();
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });
    }
    button.add_controller(secondary_click);

    let long_press = GestureLongPress::new();
    long_press.set_touch_only(false);
    {
        let menu_button = menu_button.clone();
        long_press.connect_pressed(move |gesture, _, _| {
            menu_button.popup();
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });
    }
    button.add_controller(long_press);
}

fn update_tool_rail_slot_visuals(
    button: &Button,
    icon: &Image,
    tool: ShellToolKind,
    slot: ToolRailSlot,
) {
    set_remix_icon_or_fallback(icon, shell_tool_icon(tool), tool.label(), 18);
    let tooltip = if tool_rail_slot_tools(slot).len() > 1 {
        format!(
            "{} ({}) — open the flyout for {} tools",
            tool.label(),
            shell_tool_shortcut(tool),
            tool_rail_slot_label(slot).to_lowercase()
        )
    } else {
        format!("{} ({})", tool.label(), shell_tool_shortcut(tool))
    };
    button.set_tooltip_text(Some(&tooltip));
    button.update_property(&[gtk4::accessible::Property::Label(&tooltip)]);
}

pub(super) fn build_document_tabs() -> (GtkBox, Label) {
    load_document_tabs_template()
        .map(build_document_tabs_from_template)
        .unwrap_or_else(|error| {
            tracing::error!(%error, "failed to load document tabs template");
            build_document_tabs_fallback()
        })
}

pub(super) fn build_workspace_context_dock(shell_state: &Rc<ShellUiState>) -> GtkBox {
    let context_host = GtkBox::new(Orientation::Vertical, 0);
    context_host.add_css_class("context-dock-host");
    context_host.add_css_class("workspace-context-dock");
    context_host.set_size_request(248, -1);
    context_host.set_vexpand(false);
    context_host.set_hexpand(false);
    context_host.set_halign(Align::End);
    context_host.set_valign(Align::Start);
    context_host.set_margin_end(530);
    context_host.set_margin_top(4);
    context_host.append(&shell_state.color_group);
    context_host.append(&shell_state.properties_group);
    context_host.append(&shell_state.brush_group);
    context_host.append(&shell_state.text_group);
    context_host.set_visible(shell_state.active_context_panel.get().is_some());
    shell_state
        .context_panel_host
        .replace(Some(context_host.clone()));

    context_host
}

pub(super) fn build_right_sidebar(shell_state: &Rc<ShellUiState>) -> GtkBox {
    let sidebar = GtkBox::new(Orientation::Horizontal, 0);
    sidebar.add_css_class("right-sidebar");
    sidebar.set_size_request(360, -1);

    let dock_icons = GtkBox::new(Orientation::Vertical, 4);
    dock_icons.add_css_class("panel-icon-strip");
    for (panel, icon_name, tooltip) in [
        (ContextDockPanel::Color, "palette-line.svg", "Color"),
        (
            ContextDockPanel::Properties,
            "settings-4-line.svg",
            "Properties",
        ),
        (ContextDockPanel::Brush, "brush-2-line.svg", "Brush"),
        (ContextDockPanel::Text, "text.svg", "Text"),
    ] {
        let button = build_icon_only_button(icon_name, tooltip, "dock-icon-button", 18);
        button.add_css_class("dock-icon-button");
        button.set_size_request(28, 28);
        {
            let shell_state = shell_state.clone();
            button.connect_clicked(move |_| shell_state.toggle_context_panel(panel));
        }
        shell_state
            .context_toolbar_buttons
            .borrow_mut()
            .push((panel, button.clone()));
        dock_icons.append(&button);
    }

    let dock = GtkBox::new(Orientation::Vertical, 0);
    dock.add_css_class("panel-dock");
    dock.set_hexpand(true);
    dock.set_vexpand(true);

    let paned = Paned::new(Orientation::Vertical);
    paned.set_start_child(Some(&shell_state.history_group));
    paned.set_end_child(Some(&shell_state.layers_group));
    paned.set_position(210);
    paned.set_wide_handle(true);
    paned.set_vexpand(true);
    paned.set_focusable(false);
    dock.append(&paned);

    let base = GtkBox::new(Orientation::Horizontal, 0);
    base.add_css_class("right-sidebar-base");
    base.append(&dock_icons);
    base.append(&dock);

    sidebar.append(&base);
    sidebar
}

pub(super) fn build_interactive_panel_group(
    shell_name: &str,
    tabs: &[&str],
    body_spacing: i32,
    body_vexpand: bool,
) -> (GtkBox, GtkBox, Vec<Button>) {
    match build_interactive_panel_group_shell(shell_name, tabs, body_spacing, body_vexpand) {
        Ok(shell) => shell,
        Err(error) => {
            tracing::error!(%error, panel = shell_name, "failed to load interactive panel group");
            let (group, body) = build_panel_group(shell_name, tabs, body_spacing, body_vexpand);
            (group, body, Vec::new())
        }
    }
}

fn build_interactive_panel_group_shell(
    shell_name: &str,
    tabs: &[&str],
    body_spacing: i32,
    body_vexpand: bool,
) -> anyhow::Result<(GtkBox, GtkBox, Vec<Button>)> {
    let PanelGroupTemplate {
        root,
        header,
        body,
        tab_buttons,
    } = load_panel_group_template()?;
    root.set_focusable(false);
    header.set_focusable(false);
    body.set_focusable(false);
    root.set_widget_name(&format!("{shell_name}-panel"));
    header.set_widget_name(&format!("{shell_name}-panel-header"));
    body.set_widget_name(&format!("{shell_name}-panel-body"));
    body.set_spacing(body_spacing);
    body.set_vexpand(body_vexpand);

    while let Some(child) = header.first_child() {
        header.remove(&child);
    }

    let mut active_buttons = Vec::new();
    for (index, tab) in tabs.iter().enumerate() {
        let button = tab_buttons.get(index).cloned().unwrap_or_else(Button::new);
        button.set_label(tab);
        button.set_widget_name(&format!("{shell_name}-panel-tab-{}", index + 1));
        button.add_css_class("panel-tab");
        if index == 0 {
            button.add_css_class("panel-tab-active");
        } else {
            button.remove_css_class("panel-tab-active");
        }
        header.append(&button);
        active_buttons.push(button);
    }

    Ok((root, body, active_buttons))
}

pub(super) fn build_status_bar() -> (GtkBox, Label, Label, Label, Label, Label) {
    load_status_bar_template()
        .map(build_status_bar_from_template)
        .unwrap_or_else(|error| {
            tracing::error!(%error, "failed to load status bar template");
            build_status_bar_fallback()
        })
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
            group.set_focusable(false);

            let header = GtkBox::new(Orientation::Horizontal, 2);
            header.set_widget_name(&format!("{shell_name}-panel-header"));
            header.add_css_class("panel-group-header");
            header.set_focusable(false);
            for (index, tab) in tabs.iter().enumerate() {
                if index == 0 {
                    let button = Button::with_label(tab);
                    button.set_widget_name(&format!("{shell_name}-panel-tab-{}", index + 1));
                    button.add_css_class("panel-tab");
                    button.add_css_class("panel-tab-active");
                    header.append(&button);
                } else {
                    let label = Label::new(Some(tab));
                    label.set_widget_name(&format!("{shell_name}-panel-tab-{}", index + 1));
                    label.add_css_class("panel-tab");
                    label.add_css_class("panel-tab-placeholder");
                    header.append(&label);
                }
            }

            let body = GtkBox::new(Orientation::Vertical, body_spacing);
            body.set_widget_name(&format!("{shell_name}-panel-body"));
            body.add_css_class("panel-group-body");
            body.set_vexpand(body_vexpand);
            body.set_focusable(false);

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

    let plus_tab = Label::new(Some("+"));
    plus_tab.add_css_class("document-tab-add");
    plus_tab.set_tooltip_text(Some("Multiple document tabs are not active yet"));
    tabs.append(&plus_tab);

    (tabs, active_tab_label)
}

fn build_document_tabs_from_template(template: DocumentTabsTemplate) -> (GtkBox, Label) {
    template.active_tab_button.set_can_focus(false);
    template
        .add_tab_placeholder
        .set_tooltip_text(Some("Multiple document tabs are not active yet"));
    (template.root, template.active_tab_label)
}

fn build_document_workspace(shell_state: &ShellUiState) -> GtkBox {
    let workspace = GtkBox::new(Orientation::Vertical, 0);
    workspace.add_css_class("document-workspace");
    workspace.set_hexpand(true);
    workspace.set_vexpand(true);

    let canvas_cluster = GtkBox::new(Orientation::Vertical, 0);
    canvas_cluster.add_css_class("canvas-cluster");
    canvas_cluster.set_hexpand(true);
    canvas_cluster.set_vexpand(true);
    canvas_cluster.set_halign(Align::Fill);
    canvas_cluster.set_valign(Align::Fill);

    let chrome_grid = gtk4::Grid::new();
    chrome_grid.set_hexpand(true);
    chrome_grid.set_vexpand(true);
    chrome_grid.set_halign(Align::Fill);
    chrome_grid.set_valign(Align::Fill);
    let top_left_corner = Label::new(Some(""));
    top_left_corner.add_css_class("ruler-corner");
    top_left_corner.set_accessible_role(gtk4::AccessibleRole::Presentation);
    top_left_corner.set_size_request(20, 20);
    chrome_grid.attach(&top_left_corner, 0, 0, 1, 1);

    shell_state.horizontal_ruler_label.set_hexpand(true);
    shell_state.horizontal_ruler_label.set_halign(Align::Fill);
    chrome_grid.attach(&shell_state.horizontal_ruler_label, 1, 0, 1, 1);

    shell_state.vertical_ruler_label.set_vexpand(true);
    shell_state.vertical_ruler_label.set_valign(Align::Fill);
    chrome_grid.attach(&shell_state.vertical_ruler_label, 0, 1, 1, 1);

    let canvas_frame = GtkBox::new(Orientation::Vertical, 0);
    canvas_frame.add_css_class("canvas-frame");
    canvas_frame.set_halign(Align::Fill);
    canvas_frame.set_valign(Align::Fill);
    canvas_frame.set_hexpand(true);
    canvas_frame.set_vexpand(true);

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

    task_bar.append(&shell_state.contextual_fit_button);
    task_bar.append(&shell_state.contextual_zoom_out_button);
    task_bar.append(&shell_state.contextual_zoom_in_button);

    let separator = Separator::new(Orientation::Vertical);
    separator.add_css_class("contextual-task-separator");
    task_bar.append(&separator);

    task_bar.append(&shell_state.contextual_clear_selection_button);
    task_bar.append(&shell_state.contextual_invert_selection_button);
    task_bar.append(&shell_state.contextual_edit_pixels_button);
    task_bar.append(&shell_state.contextual_edit_mask_button);

    canvas_overlay.add_overlay(&task_bar);
    canvas_frame.append(&canvas_overlay);

    chrome_grid.attach(&canvas_frame, 1, 1, 1, 1);
    canvas_cluster.append(&chrome_grid);
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

fn build_status_bar_from_template(
    template: StatusBarTemplate,
) -> (GtkBox, Label, Label, Label, Label, Label) {
    (
        template.root,
        template.doc_label,
        template.zoom_label,
        template.cursor_label,
        template.notice_label,
        template.mode_label,
    )
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

fn selection_status_label(snapshot: &ShellSnapshot) -> &'static str {
    if snapshot.selection_rect.is_some()
        || snapshot.selection_path.is_some()
        || snapshot.selection_preview_path.is_some()
    {
        "Active"
    } else {
        "None"
    }
}

fn on_off_label(enabled: bool) -> &'static str {
    if enabled { "On" } else { "Off" }
}

pub(super) fn refresh_tool_options_bar(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) {
    shell_state
        .tool_options_label
        .set_label(&snapshot.active_tool_name);
    set_remix_icon_or_fallback(
        &shell_state.tool_options_icon,
        shell_tool_icon(snapshot.active_tool),
        &snapshot.active_tool_name,
        14,
    );

    clear_tool_option_children(&shell_state.tool_options_content);
    populate_tool_options(shell_state, snapshot, &shell_state.tool_options_content);
}

fn build_tool_options_bar_fallback(snapshot: ShellSnapshot) -> (GtkBox, Image, Label, GtkBox) {
    let bar = GtkBox::new(Orientation::Horizontal, 8);
    bar.add_css_class("tool-options-bar");
    bar.set_hexpand(true);

    let identity = GtkBox::new(Orientation::Horizontal, 6);
    identity.add_css_class("tool-options-identity");
    let tool_icon = build_remix_icon(
        shell_tool_icon(snapshot.active_tool),
        &snapshot.active_tool_name,
        14,
    );
    tool_icon.add_css_class("tool-options-icon");
    identity.append(&tool_icon);

    let tool_name = snapshot.active_tool_name.clone();
    let tool_label = Label::new(Some(&tool_name));
    tool_label.add_css_class("tool-options-label");
    identity.append(&tool_label);
    bar.append(&identity);
    bar.append(&build_tool_options_divider());

    let content = GtkBox::new(Orientation::Horizontal, 0);
    content.add_css_class("tool-options-content");
    content.set_hexpand(true);
    bar.append(&content);

    (bar, tool_icon, tool_label, content)
}

fn populate_tool_options(
    shell_state: &Rc<ShellUiState>,
    snapshot: &ShellSnapshot,
    content: &GtkBox,
) {
    let mut first = true;

    match snapshot.active_tool {
        ShellToolKind::Brush | ShellToolKind::Eraser => {
            append_tool_option_group(
                content,
                &mut first,
                &build_brush_preset_group(shell_state, snapshot),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "Size",
                    &format!("{} px", snapshot.brush_radius),
                    "Decrease brush size",
                    |controller| controller.decrease_brush_radius(),
                    "Increase brush size",
                    |controller| controller.increase_brush_radius(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "Hardness",
                    &format!("{}%", snapshot.brush_hardness_percent),
                    "Decrease brush hardness",
                    |controller| controller.decrease_brush_hardness(),
                    "Increase brush hardness",
                    |controller| controller.increase_brush_hardness(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "Spacing",
                    &format!("{} px", snapshot.brush_spacing),
                    "Decrease brush spacing",
                    |controller| controller.decrease_brush_spacing(),
                    "Increase brush spacing",
                    |controller| controller.increase_brush_spacing(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "Flow",
                    &format!("{}%", snapshot.brush_flow_percent),
                    "Decrease brush flow",
                    |controller| controller.decrease_brush_flow(),
                    "Increase brush flow",
                    |controller| controller.increase_brush_flow(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_pressure_group(shell_state, snapshot),
            );
        }
        ShellToolKind::Text => {
            append_tool_option_group(
                content,
                &mut first,
                &build_text_action_group(shell_state, snapshot),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_text_font_group(shell_state, snapshot),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_text_adjust_group(
                    shell_state,
                    snapshot,
                    "Size",
                    &format!("{} px", snapshot.text.font_size_px),
                    |update| {
                        update.font_size_px = update.font_size_px.saturating_sub(1).max(8);
                    },
                    |update| {
                        update.font_size_px = (update.font_size_px + 1).min(256);
                    },
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_text_adjust_group(
                    shell_state,
                    snapshot,
                    "Leading",
                    &format!("{}%", snapshot.text.line_height_percent),
                    |update| {
                        update.line_height_percent =
                            update.line_height_percent.saturating_sub(5).max(80);
                    },
                    |update| {
                        update.line_height_percent = (update.line_height_percent + 5).min(300);
                    },
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_text_adjust_group(
                    shell_state,
                    snapshot,
                    "Tracking",
                    &snapshot.text.letter_spacing.to_string(),
                    |update| {
                        update.letter_spacing = (update.letter_spacing - 1).max(-8);
                    },
                    |update| {
                        update.letter_spacing = (update.letter_spacing + 1).min(32);
                    },
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_text_alignment_group(shell_state, snapshot),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_text_fill_group(shell_state, snapshot),
            );
        }
        ShellToolKind::Transform => {
            append_tool_option_group(
                content,
                &mut first,
                &build_transform_action_group(shell_state, snapshot),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "Scale",
                    &format!("{}%", snapshot.transform_scale_percent),
                    "Scale transform down",
                    |controller| controller.scale_transform_down(),
                    "Scale transform up",
                    |controller| controller.scale_transform_up(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "W",
                    &format!("{}%", snapshot.transform_scale_x_percent),
                    "Scale width down",
                    |controller| controller.scale_transform_x_down(),
                    "Scale width up",
                    |controller| controller.scale_transform_x_up(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "H",
                    &format!("{}%", snapshot.transform_scale_y_percent),
                    "Scale height down",
                    |controller| controller.scale_transform_y_down(),
                    "Scale height up",
                    |controller| controller.scale_transform_y_up(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_adjust_group(
                    shell_state,
                    "Rotate",
                    &format!("{}°", snapshot.transform_rotation_degrees),
                    "Rotate transform left",
                    |controller| controller.rotate_transform_left(),
                    "Rotate transform right",
                    |controller| controller.rotate_transform_right(),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_readonly_group("Target", &snapshot.active_edit_target_name),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_snap_group(shell_state, snapshot),
            );
        }
        ShellToolKind::RectangularMarquee | ShellToolKind::Lasso => {
            append_tool_option_group(
                content,
                &mut first,
                &build_readonly_group(
                    "Mode",
                    if snapshot.active_tool == ShellToolKind::RectangularMarquee {
                        "Rectangular"
                    } else {
                        "Lasso"
                    },
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_readonly_group("Selection", selection_status_label(snapshot)),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_selection_action_group(shell_state, snapshot),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_snap_group(shell_state, snapshot),
            );
        }
        _ => {
            append_tool_option_group(
                content,
                &mut first,
                &build_readonly_group("Layer", &snapshot.active_layer_name),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_readonly_group("Selection", selection_status_label(snapshot)),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_move_snap_guides_group(shell_state, snapshot),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_readonly_group(
                    "Canvas",
                    &format!(
                        "{} × {}",
                        snapshot.canvas_size.width, snapshot.canvas_size.height
                    ),
                ),
            );
            append_tool_option_group(
                content,
                &mut first,
                &build_readonly_group("Blend", &snapshot.active_layer_blend_mode),
            );
        }
    }

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    content.append(&spacer);
}

fn clear_tool_option_children(container: &GtkBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn append_tool_option_group(container: &GtkBox, first: &mut bool, group: &GtkBox) {
    if !*first {
        container.append(&build_tool_options_divider());
    }
    container.append(group);
    *first = false;
}

fn build_tool_option_group(label: &str, control: &impl IsA<gtk4::Widget>) -> GtkBox {
    let group = GtkBox::new(Orientation::Horizontal, 6);
    group.add_css_class("tool-options-group");
    group.set_valign(Align::Center);
    let key_label = Label::new(Some(label));
    key_label.add_css_class("tool-option-key");
    group.append(&key_label);

    let value_box = GtkBox::new(Orientation::Horizontal, 4);
    value_box.add_css_class("tool-option-cluster");
    value_box.set_valign(Align::Center);
    value_box.append(control);
    group.append(&value_box);
    group
}

fn build_tool_options_divider() -> Separator {
    let divider = Separator::new(Orientation::Vertical);
    divider.add_css_class("tool-options-divider");
    divider
}

fn build_readonly_group(label: &str, value: &str) -> GtkBox {
    build_tool_option_group(label, &build_tool_option_value_box(value))
}

fn build_tool_option_value_box(value: &str) -> GtkBox {
    let value_box = GtkBox::new(Orientation::Horizontal, 0);
    value_box.add_css_class("tool-option-box");
    value_box.set_valign(Align::Center);
    let value_label = Label::new(Some(value));
    value_label.add_css_class("tool-option-value");
    value_box.append(&value_label);
    value_box
}

fn build_tool_option_text_button(label: &str, tooltip: Option<&str>) -> Button {
    let button = Button::with_label(label);
    button.set_has_frame(false);
    button.add_css_class("tool-option-button");
    if let Some(tooltip) = tooltip {
        button.set_tooltip_text(Some(tooltip));
    }
    button
}

fn build_tool_option_toggle_button(label: &str, active: bool, tooltip: Option<&str>) -> Button {
    let button = build_tool_option_text_button(label, tooltip);
    button.add_css_class("tool-option-toggle-button");
    if active {
        button.add_css_class("tool-option-toggle-button-active");
    }
    button
}

fn build_tool_option_icon_button<F>(
    shell_state: &Rc<ShellUiState>,
    icon_name: &str,
    tooltip: &str,
    action: F,
) -> Button
where
    F: Fn(&mut dyn ShellController) + 'static,
{
    let button = build_icon_only_button(icon_name, tooltip, "tool-option-icon-button", 10);
    button.set_sensitive(true);
    let controller = shell_state.controller.clone();
    button.connect_clicked(move |_| {
        let mut controller = controller.borrow_mut();
        action(&mut *controller);
    });
    button
}

fn build_adjust_group<F, G>(
    shell_state: &Rc<ShellUiState>,
    label: &str,
    value: &str,
    decrease_tooltip: &str,
    decrease_action: F,
    increase_tooltip: &str,
    increase_action: G,
) -> GtkBox
where
    F: Fn(&mut dyn ShellController) + 'static,
    G: Fn(&mut dyn ShellController) + 'static,
{
    let control = GtkBox::new(Orientation::Horizontal, 4);
    control.append(&build_tool_option_icon_button(
        shell_state,
        "subtract-line.svg",
        decrease_tooltip,
        decrease_action,
    ));
    control.append(&build_tool_option_value_box(value));
    control.append(&build_tool_option_icon_button(
        shell_state,
        "add-line.svg",
        increase_tooltip,
        increase_action,
    ));
    build_tool_option_group(label, &control)
}

fn build_brush_preset_group(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) -> GtkBox {
    let control = GtkBox::new(Orientation::Horizontal, 4);
    control.append(&build_tool_option_icon_button(
        shell_state,
        "arrow-go-back-line.svg",
        "Previous brush preset",
        |controller| controller.previous_brush_preset(),
    ));
    let preset =
        build_tool_option_text_button(&snapshot.brush_preset_name, Some("Next brush preset"));
    {
        let controller = shell_state.controller.clone();
        preset.connect_clicked(move |_| controller.borrow_mut().next_brush_preset());
    }
    control.append(&preset);
    control.append(&build_tool_option_icon_button(
        shell_state,
        "arrow-go-forward-line.svg",
        "Next brush preset",
        |controller| controller.next_brush_preset(),
    ));
    build_tool_option_group("Preset", &control)
}

fn build_pressure_group(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) -> GtkBox {
    let control = GtkBox::new(Orientation::Horizontal, 4);
    let size = build_tool_option_toggle_button(
        "Size",
        snapshot.pressure_size_enabled,
        Some("Toggle pressure to size"),
    );
    {
        let controller = shell_state.controller.clone();
        size.connect_clicked(move |_| controller.borrow_mut().toggle_pressure_size_enabled());
    }
    control.append(&size);

    let opacity = build_tool_option_toggle_button(
        "Opacity",
        snapshot.pressure_opacity_enabled,
        Some("Toggle pressure to opacity"),
    );
    {
        let controller = shell_state.controller.clone();
        opacity.connect_clicked(move |_| controller.borrow_mut().toggle_pressure_opacity_enabled());
    }
    control.append(&opacity);

    build_tool_option_group("Pressure", &control)
}

fn build_selection_action_group(
    shell_state: &Rc<ShellUiState>,
    snapshot: &ShellSnapshot,
) -> GtkBox {
    let has_selection = selection_status_label(snapshot) == "Active";
    let control = GtkBox::new(Orientation::Horizontal, 4);

    let clear = build_tool_option_text_button("Clear", Some("Clear selection"));
    clear.set_sensitive(has_selection);
    {
        let controller = shell_state.controller.clone();
        clear.connect_clicked(move |_| controller.borrow_mut().clear_selection());
    }
    control.append(&clear);

    let invert = build_tool_option_text_button(
        if snapshot.selection_inverted {
            "Uninvert"
        } else {
            "Invert"
        },
        Some("Invert selection"),
    );
    invert.set_sensitive(has_selection);
    {
        let controller = shell_state.controller.clone();
        invert.connect_clicked(move |_| controller.borrow_mut().invert_selection());
    }
    control.append(&invert);

    build_tool_option_group("Actions", &control)
}

fn build_snap_group(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) -> GtkBox {
    let toggle = build_tool_option_toggle_button(
        on_off_label(snapshot.snapping_enabled),
        snapshot.snapping_enabled,
        Some("Toggle guide snapping"),
    );
    {
        let controller = shell_state.controller.clone();
        toggle.connect_clicked(move |_| controller.borrow_mut().toggle_snapping_enabled());
    }
    build_tool_option_group("Snap", &toggle)
}

fn build_move_snap_guides_group(
    shell_state: &Rc<ShellUiState>,
    snapshot: &ShellSnapshot,
) -> GtkBox {
    let control = GtkBox::new(Orientation::Horizontal, 4);

    let guides = build_tool_option_toggle_button(
        if snapshot.guides_visible {
            "Guides On"
        } else {
            "Guides Off"
        },
        snapshot.guides_visible,
        Some("Toggle guides visibility"),
    );
    {
        let controller = shell_state.controller.clone();
        guides.connect_clicked(move |_| controller.borrow_mut().toggle_guides_visible());
    }
    control.append(&guides);

    let snap = build_tool_option_toggle_button(
        if snapshot.snapping_enabled {
            "Snap On"
        } else {
            "Snap Off"
        },
        snapshot.snapping_enabled,
        Some("Toggle guide snapping"),
    );
    {
        let controller = shell_state.controller.clone();
        snap.connect_clicked(move |_| controller.borrow_mut().toggle_snapping_enabled());
    }
    control.append(&snap);

    build_tool_option_group("Guides", &control)
}

fn build_transform_action_group(
    shell_state: &Rc<ShellUiState>,
    snapshot: &ShellSnapshot,
) -> GtkBox {
    let control = GtkBox::new(Orientation::Horizontal, 4);

    let start = build_tool_option_text_button("Start", Some("Start transform"));
    start.set_sensitive(snapshot.can_begin_transform && !snapshot.transform_active);
    {
        let controller = shell_state.controller.clone();
        start.connect_clicked(move |_| controller.borrow_mut().begin_transform());
    }
    control.append(&start);

    let apply = build_tool_option_text_button("Apply", Some("Commit transform"));
    apply.set_sensitive(snapshot.transform_active);
    {
        let controller = shell_state.controller.clone();
        apply.connect_clicked(move |_| controller.borrow_mut().commit_transform());
    }
    control.append(&apply);

    let cancel = build_tool_option_text_button("Cancel", Some("Cancel transform"));
    cancel.set_sensitive(snapshot.transform_active);
    {
        let controller = shell_state.controller.clone();
        cancel.connect_clicked(move |_| controller.borrow_mut().cancel_transform());
    }
    control.append(&cancel);

    build_tool_option_group("Action", &control)
}

fn build_text_action_group(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) -> GtkBox {
    let control = GtkBox::new(Orientation::Horizontal, 4);

    let primary = if snapshot.text.editing {
        let button = build_tool_option_text_button("Apply", Some("Commit text changes"));
        {
            let controller = shell_state.controller.clone();
            button.connect_clicked(move |_| controller.borrow_mut().commit_text_session());
        }
        button
    } else {
        let button = build_tool_option_text_button(
            if snapshot.text.selected {
                "Edit Text"
            } else {
                "Place Text"
            },
            Some("Begin text editing"),
        );
        button.set_sensitive(snapshot.text.selected || snapshot.active_tool == ShellToolKind::Text);
        {
            let controller = shell_state.controller.clone();
            button.connect_clicked(move |_| controller.borrow_mut().begin_text_edit());
        }
        button
    };
    control.append(&primary);

    let cancel = build_tool_option_text_button("Cancel", Some("Cancel text editing"));
    cancel.set_sensitive(snapshot.text.editing);
    {
        let controller = shell_state.controller.clone();
        cancel.connect_clicked(move |_| controller.borrow_mut().cancel_text_session());
    }
    control.append(&cancel);

    build_tool_option_group("Text", &control)
}

fn build_text_font_group(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) -> GtkBox {
    let can_edit = snapshot.text.editing || snapshot.text.selected;
    let fonts = available_text_fonts(snapshot);
    let control = GtkBox::new(Orientation::Horizontal, 4);

    let font_button =
        build_tool_option_text_button(&snapshot.text.font_family, Some("Cycle text font"));
    font_button.set_sensitive(can_edit && fonts.len() > 1);
    if let Some(base_update) = text_update_from_snapshot(snapshot) {
        let controller = shell_state.controller.clone();
        let selected = snapshot.text.selected;
        let editing = snapshot.text.editing;
        let current_font = snapshot.text.font_family.clone();
        let next_font = next_text_font(&current_font, &fonts).to_string();
        font_button.connect_clicked(move |_| {
            let mut update = base_update.clone();
            if !editing && selected {
                controller.borrow_mut().begin_text_edit();
            }
            update.font_family = next_font.clone();
            controller.borrow_mut().update_text_session(update);
        });
    }
    control.append(&font_button);

    build_tool_option_group("Font", &control)
}

fn build_text_adjust_group<F, G>(
    shell_state: &Rc<ShellUiState>,
    snapshot: &ShellSnapshot,
    label: &str,
    value: &str,
    decrease: F,
    increase: G,
) -> GtkBox
where
    F: Fn(&mut ShellTextUpdate) + 'static,
    G: Fn(&mut ShellTextUpdate) + 'static,
{
    let control = GtkBox::new(Orientation::Horizontal, 4);
    control.append(&build_text_update_icon_button(
        shell_state,
        snapshot,
        "subtract-line.svg",
        &format!("Decrease {label}"),
        decrease,
    ));
    control.append(&build_tool_option_value_box(value));
    control.append(&build_text_update_icon_button(
        shell_state,
        snapshot,
        "add-line.svg",
        &format!("Increase {label}"),
        increase,
    ));
    build_tool_option_group(label, &control)
}

fn build_text_alignment_group(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) -> GtkBox {
    let control = GtkBox::new(Orientation::Horizontal, 4);
    for (label, alignment) in [
        ("Left", ShellTextAlignment::Left),
        ("Center", ShellTextAlignment::Center),
        ("Right", ShellTextAlignment::Right),
    ] {
        let button = build_tool_option_toggle_button(
            label,
            snapshot.text.alignment == alignment,
            Some("Set text alignment"),
        );
        let controller = shell_state.controller.clone();
        let selected = snapshot.text.selected;
        let editing = snapshot.text.editing;
        let Some(base_update) = text_update_from_snapshot(snapshot) else {
            button.set_sensitive(false);
            control.append(&button);
            continue;
        };
        button.connect_clicked(move |_| {
            let mut update = base_update.clone();
            update.alignment = alignment;
            if !editing && selected {
                controller.borrow_mut().begin_text_edit();
            }
            controller.borrow_mut().update_text_session(update);
        });
        control.append(&button);
    }
    build_tool_option_group("Align", &control)
}

fn build_text_fill_group(shell_state: &Rc<ShellUiState>, snapshot: &ShellSnapshot) -> GtkBox {
    let control = GtkBox::new(Orientation::Horizontal, 4);
    control.append(&build_tool_option_value_box(&format!(
        "#{:02X}{:02X}{:02X}",
        snapshot.text.fill_rgba[0], snapshot.text.fill_rgba[1], snapshot.text.fill_rgba[2]
    )));

    let use_fg =
        build_tool_option_text_button("Use FG", Some("Apply the current foreground color"));
    let can_edit = snapshot.text.editing || snapshot.text.selected;
    use_fg.set_sensitive(can_edit);
    {
        let controller = shell_state.controller.clone();
        let selected = snapshot.text.selected;
        let editing = snapshot.text.editing;
        let foreground = snapshot.foreground_color;
        if let Some(base_update) = text_update_from_snapshot(snapshot) {
            use_fg.connect_clicked(move |_| {
                let mut update = base_update.clone();
                update.fill_rgba = foreground;
                if !editing && selected {
                    controller.borrow_mut().begin_text_edit();
                }
                controller.borrow_mut().update_text_session(update);
            });
        }
    }
    control.append(&use_fg);
    build_tool_option_group("Fill", &control)
}

fn build_text_update_icon_button<F>(
    shell_state: &Rc<ShellUiState>,
    snapshot: &ShellSnapshot,
    icon_name: &str,
    tooltip: &str,
    update_fn: F,
) -> Button
where
    F: Fn(&mut ShellTextUpdate) + 'static,
{
    let button = build_icon_only_button(icon_name, tooltip, "tool-option-icon-button", 10);
    let can_edit = snapshot.text.editing || snapshot.text.selected;
    button.set_sensitive(can_edit);
    let controller = shell_state.controller.clone();
    let selected = snapshot.text.selected;
    let editing = snapshot.text.editing;
    if let Some(base_update) = text_update_from_snapshot(snapshot) {
        button.connect_clicked(move |_| {
            let mut update = base_update.clone();
            update_fn(&mut update);
            if !editing && selected {
                controller.borrow_mut().begin_text_edit();
            }
            controller.borrow_mut().update_text_session(update);
        });
    }
    button
}

fn text_update_from_snapshot(snapshot: &ShellSnapshot) -> Option<ShellTextUpdate> {
    if !(snapshot.text.editing || snapshot.text.selected) {
        return None;
    }
    Some(ShellTextUpdate {
        content: snapshot.text.content.clone(),
        font_family: snapshot.text.font_family.clone(),
        font_size_px: snapshot.text.font_size_px,
        line_height_percent: snapshot.text.line_height_percent,
        letter_spacing: snapshot.text.letter_spacing,
        fill_rgba: snapshot.text.fill_rgba,
        alignment: snapshot.text.alignment,
    })
}

fn available_text_fonts(snapshot: &ShellSnapshot) -> Vec<String> {
    let mut fonts = vec![snapshot.text.font_family.clone()];
    if !fonts.iter().any(|font| font == "Bitmap Sans") {
        fonts.push("Bitmap Sans".to_string());
    }
    fonts
}

fn next_text_font<'a>(current: &'a str, fonts: &'a [String]) -> &'a str {
    let Some(index) = fonts.iter().position(|font| font == current) else {
        return fonts.first().map(String::as_str).unwrap_or(current);
    };
    fonts
        .get((index + 1) % fonts.len())
        .map(String::as_str)
        .unwrap_or(current)
}
