use super::*;

pub(super) fn remix_icon_theme_name(icon_name: &str) -> String {
    icon_name
        .strip_suffix(".svg")
        .map(|name| format!("phototux-{name}-symbolic"))
        .unwrap_or_else(|| icon_name.to_string())
}

pub(super) fn remix_icon_resource_path(icon_name: &str) -> String {
    format!(
        "{UI_RESOURCE_PREFIX}/icons/scalable/actions/{}.svg",
        remix_icon_theme_name(icon_name)
    )
}

pub(super) const APP_WINDOW_ICON_NAME: &str = "phototux-logo-white";

pub(super) fn logo_icon_resource_path(dark_background: bool) -> &'static str {
    if dark_background {
        "/com/phototux/assets/logo/Logo_White.png"
    } else {
        "/com/phototux/assets/logo/Logo_Dark.png"
    }
}

pub(super) fn build_logo_icon(dark_background: bool, alt_text: &str, size: i32) -> Image {
    build_optional_resource_image(logo_icon_resource_path(dark_background), alt_text, size)
}

pub(super) fn build_remix_icon(icon_name: &str, alt_text: &str, size: i32) -> Image {
    let image = build_optional_themed_icon(icon_name, alt_text, size);
    image.add_css_class("remix-icon");
    image
}

pub(super) fn set_remix_icon_or_fallback(
    image: &Image,
    icon_name: &str,
    alt_text: &str,
    size: i32,
) {
    if bundled_ui_resource_exists(&remix_icon_resource_path(icon_name)) {
        image.set_icon_name(Some(&remix_icon_theme_name(icon_name)));
    } else {
        warn_missing_optional_ui_resource(&remix_icon_resource_path(icon_name));
        image.set_icon_name(Some(OPTIONAL_ICON_FALLBACK_NAME));
    }
    image.set_pixel_size(size);
    image.set_tooltip_text(Some(alt_text));
}

pub(super) fn set_image_resource_or_fallback(
    image: &Image,
    resource_path: &str,
    alt_text: &str,
    size: i32,
) {
    if bundled_ui_resource_exists(resource_path) {
        image.set_resource(Some(resource_path));
    } else {
        warn_missing_optional_ui_resource(resource_path);
        image.set_icon_name(Some(OPTIONAL_ICON_FALLBACK_NAME));
    }
    image.set_pixel_size(size);
    image.set_tooltip_text(Some(alt_text));
}

pub(super) fn build_icon_only_button(
    icon_name: &str,
    tooltip: &str,
    css_class: &str,
    size: i32,
) -> Button {
    let button = Button::new();
    button.add_css_class(css_class);
    button.set_has_frame(false);
    button.set_tooltip_text(Some(tooltip));
    button.update_property(&[gtk4::accessible::Property::Label(tooltip)]);

    let icon = build_remix_icon(icon_name, tooltip, size);
    icon.set_accessible_role(gtk4::AccessibleRole::Presentation);
    button.set_child(Some(&icon));
    button
}

pub(super) fn build_icon_label_button(icon_name: &str, label: &str) -> Button {
    build_icon_label_shortcut_button(icon_name, label, None)
}

pub(super) fn build_compact_icon_label_button(icon_name: &str, label: &str, size: i32) -> Button {
    let button = Button::new();
    button.set_has_frame(false);
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk4::accessible::Property::Label(label)]);

    let content = GtkBox::new(Orientation::Horizontal, 4);
    let icon = build_remix_icon(icon_name, label, size);
    icon.set_accessible_role(gtk4::AccessibleRole::Presentation);
    content.append(&icon);

    let text = Label::new(Some(label));
    text.add_css_class("icon-label-text");
    content.append(&text);

    button.set_child(Some(&content));
    button
}

pub(super) fn build_tool_chip_icon_button(icon_name: &str, label: &str) -> Button {
    let button = build_icon_only_button(icon_name, label, "tool-chip", 12);
    button.add_css_class("tool-chip-icon-only");
    button
}

pub(super) fn build_tool_chip_icon_label_button(icon_name: &str, label: &str) -> Button {
    let button = build_compact_icon_label_button(icon_name, label, 12);
    button.add_css_class("tool-chip");
    button
}

pub(super) fn build_contextual_icon_label_button(icon_name: &str, label: &str) -> Button {
    let button = build_compact_icon_label_button(icon_name, label, 14);
    button.add_css_class("contextual-task-button");
    button
}

pub(super) fn build_icon_label_shortcut_button(
    icon_name: &str,
    label: &str,
    shortcut: Option<&str>,
) -> Button {
    let button = Button::new();
    button.set_has_frame(false);
    button.set_hexpand(true);
    button.set_halign(Align::Fill);
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk4::accessible::Property::Label(label)]);

    let content = GtkBox::new(Orientation::Horizontal, 5);
    let icon = build_remix_icon(icon_name, label, 12);
    icon.set_accessible_role(gtk4::AccessibleRole::Presentation);
    content.append(&icon);
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

pub(super) fn set_menu_button_label(button: &Button, label: &str) {
    if let Some(content) = button.child().and_downcast::<GtkBox>()
        && let Some(widget) = content.first_child()
    {
        let mut current = widget.next_sibling();
        while let Some(child) = current {
            if let Ok(text) = child.clone().downcast::<Label>()
                && text
                    .css_classes()
                    .iter()
                    .any(|class_name| class_name == "icon-label-text")
            {
                text.set_label(label);
                button.set_tooltip_text(Some(label));
                button.update_property(&[gtk4::accessible::Property::Label(label)]);
                break;
            }
            current = child.next_sibling();
        }
    }
}

pub(super) fn create_menu_popover(button: &MenuButton) -> (Popover, GtkBox) {
    let popover = Popover::new();
    popover.set_has_arrow(false);
    popover.add_css_class("menu-dropdown");
    popover.set_position(gtk4::PositionType::Bottom);

    let anchor = button.clone();
    popover.connect_show(move |popover| {
        let button_width = anchor.width().max(1);
        let visible_width = popover
            .child()
            .map(|child| child.width())
            .unwrap_or(220)
            .max(220);
        let offset_x = ((visible_width - button_width) / 2).max(0);
        popover.set_offset(offset_x, 0);
    });

    let menu = GtkBox::new(Orientation::Vertical, 0);
    menu.add_css_class("menu-dropdown-body");

    (popover, menu)
}

pub(super) fn shell_tool_icon(tool: ShellToolKind) -> &'static str {
    match tool {
        ShellToolKind::Move => "drag-move-line.svg",
        ShellToolKind::RectangularMarquee => "focus-3-line.svg",
        ShellToolKind::Lasso => "edit-line.svg",
        ShellToolKind::Transform => "expand-diagonal-2-line.svg",
        ShellToolKind::Text => "text.svg",
        ShellToolKind::Brush => "brush-2-line.svg",
        ShellToolKind::Eraser => "eraser-line.svg",
        ShellToolKind::Hand => "hand.svg",
        ShellToolKind::Zoom => "zoom-in-line.svg",
    }
}

pub(super) fn shell_tool_shortcut(tool: ShellToolKind) -> &'static str {
    match tool {
        ShellToolKind::Move => "V",
        ShellToolKind::RectangularMarquee => "M",
        ShellToolKind::Lasso => "L",
        ShellToolKind::Text => "I",
        ShellToolKind::Transform => "T",
        ShellToolKind::Brush => "B",
        ShellToolKind::Eraser => "E",
        ShellToolKind::Hand => "H",
        ShellToolKind::Zoom => "Z",
    }
}

fn build_optional_resource_image(resource_path: &str, alt_text: &str, size: i32) -> Image {
    let image = if bundled_ui_resource_exists(resource_path) {
        Image::from_resource(resource_path)
    } else {
        warn_missing_optional_ui_resource(resource_path);
        Image::from_icon_name(OPTIONAL_ICON_FALLBACK_NAME)
    };
    image.set_pixel_size(size);
    image.set_halign(Align::Center);
    image.set_valign(Align::Center);
    image.set_tooltip_text(Some(alt_text));
    image
}

fn build_optional_themed_icon(icon_name: &str, alt_text: &str, size: i32) -> Image {
    let image = if bundled_ui_resource_exists(&remix_icon_resource_path(icon_name)) {
        Image::from_icon_name(&remix_icon_theme_name(icon_name))
    } else {
        warn_missing_optional_ui_resource(&remix_icon_resource_path(icon_name));
        Image::from_icon_name(OPTIONAL_ICON_FALLBACK_NAME)
    };
    image.set_pixel_size(size);
    image.set_halign(Align::Center);
    image.set_valign(Align::Center);
    image.set_tooltip_text(Some(alt_text));
    image
}

fn bundled_ui_resource_exists(resource_path: &str) -> bool {
    gio::resources_get_info(resource_path, gio::ResourceLookupFlags::NONE).is_ok()
}

fn warn_missing_optional_ui_resource(resource_path: &str) {
    static MISSING_RESOURCES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen_resources = MISSING_RESOURCES.get_or_init(|| Mutex::new(HashSet::new()));
    match seen_resources.lock() {
        Ok(mut seen) => {
            if !seen.insert(resource_path.to_string()) {
                return;
            }
        }
        Err(_) => return,
    }
    tracing::warn!(path = resource_path, "missing optional bundled UI resource");
}
