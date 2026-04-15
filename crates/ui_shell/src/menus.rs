use super::*;

pub(super) fn build_menu_bar(window: &ApplicationWindow, shell_state: Rc<ShellUiState>) -> GtkBox {
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

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bar.append(&spacer);

    let separator = Separator::new(Orientation::Vertical);
    separator.add_css_class("menu-divider");
    bar.append(&separator);
    bar.append(&shell_state.menu_zoom_label);

    bar
}

fn build_top_level_menu(label: &str) -> (MenuButton, Popover, GtkBox) {
    let button = MenuButton::builder()
        .label(label)
        .use_underline(true)
        .build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");
    let (popover, menu) = create_menu_popover(&button);
    (button, popover, menu)
}

fn finish_top_level_menu(button: MenuButton, popover: Popover, menu: GtkBox) -> MenuButton {
    popover.set_child(Some(&menu));
    button.set_popover(Some(&popover));
    button
}

fn append_menu_item<F>(menu: &GtkBox, popover: &Popover, item: &Button, action: F)
where
    F: Fn() + 'static,
{
    item.add_css_class("menu-dropdown-item");
    let popover = popover.clone();
    item.connect_clicked(move |_| {
        popover.popdown();
        action();
    });
    menu.append(item);
}

fn append_icon_menu_item<F>(
    menu: &GtkBox,
    popover: &Popover,
    icon: &str,
    label: &str,
    action: F,
) -> Button
where
    F: Fn() + 'static,
{
    let item = build_icon_label_button(icon, label);
    append_menu_item(menu, popover, &item, action);
    item
}

fn append_icon_shortcut_menu_item<F>(
    menu: &GtkBox,
    popover: &Popover,
    icon: &str,
    label: &str,
    shortcut: Option<&str>,
    action: F,
) -> Button
where
    F: Fn() + 'static,
{
    let item = build_icon_label_shortcut_button(icon, label, shortcut);
    append_menu_item(menu, popover, &item, action);
    item
}

fn append_menu_separator(menu: &GtkBox) {
    menu.append(&Separator::new(Orientation::Horizontal));
}

fn build_edit_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_Edit");

    let undo = append_icon_menu_item(&menu, &popover, "arrow-go-back-line.svg", "Undo", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().undo()
    });

    let redo = append_icon_menu_item(&menu, &popover, "arrow-go-forward-line.svg", "Redo", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().redo()
    });

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

    finish_top_level_menu(button, popover, menu)
}

fn build_image_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_Image");

    let start_transform = append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "expand-diagonal-2-line.svg",
        "Start Transform",
        Some("T"),
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().begin_transform()
        },
    );

    let scale_up = append_icon_menu_item(&menu, &popover, "add-line.svg", "Scale Transform Up", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().scale_transform_up()
    });

    let scale_down = append_icon_menu_item(
        &menu,
        &popover,
        "subtract-line.svg",
        "Scale Transform Down",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().scale_transform_down()
        },
    );

    append_icon_menu_item(&menu, &popover, "add-line.svg", "Scale X Up", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().scale_transform_x_up()
    });

    append_icon_menu_item(&menu, &popover, "subtract-line.svg", "Scale X Down", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().scale_transform_x_down()
    });

    append_icon_menu_item(&menu, &popover, "add-line.svg", "Scale Y Up", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().scale_transform_y_up()
    });

    append_icon_menu_item(&menu, &popover, "subtract-line.svg", "Scale Y Down", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().scale_transform_y_down()
    });

    append_icon_menu_item(&menu, &popover, "history-line.svg", "Rotate Left", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().rotate_transform_left()
    });

    append_icon_menu_item(&menu, &popover, "history-line.svg", "Rotate Right", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().rotate_transform_right()
    });

    let commit_transform = append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "check-line.svg",
        "Commit Transform",
        Some("Enter"),
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().commit_transform()
        },
    );

    let cancel_transform = append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "close-line.svg",
        "Cancel Transform",
        Some("Esc"),
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().cancel_transform()
        },
    );

    {
        let shell_state = shell_state.clone();
        let start_transform = start_transform.clone();
        let scale_up = scale_up.clone();
        let scale_down = scale_down.clone();
        let commit_transform = commit_transform.clone();
        let cancel_transform = cancel_transform.clone();
        popover.connect_show(move |_| {
            let snapshot = shell_state.controller.borrow().snapshot();
            let transform_active = snapshot.transform_active;

            start_transform.set_sensitive(snapshot.can_begin_transform && !transform_active);
            scale_up.set_sensitive(transform_active);
            scale_down.set_sensitive(transform_active);
            commit_transform.set_sensitive(transform_active);
            cancel_transform.set_sensitive(transform_active);
        });
    }

    finish_top_level_menu(button, popover, menu)
}

fn build_layer_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_Layer");

    append_icon_menu_item(&menu, &popover, "add-line.svg", "New Layer", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().add_layer()
    });

    let duplicate =
        append_icon_menu_item(&menu, &popover, "file-copy-line.svg", "Duplicate Layer", {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().duplicate_active_layer()
        });

    let delete = append_icon_menu_item(&menu, &popover, "delete-bin-line.svg", "Delete Layer", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().delete_active_layer()
    });

    let add_mask = append_icon_menu_item(&menu, &popover, "add-line.svg", "Add Layer Mask", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().add_active_layer_mask()
    });

    let remove_mask = append_icon_menu_item(
        &menu,
        &popover,
        "delete-bin-line.svg",
        "Delete Layer Mask",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().remove_active_layer_mask()
        },
    );

    let toggle_mask = append_icon_menu_item(
        &menu,
        &popover,
        "eye-line.svg",
        "Enable or Disable Layer Mask",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().toggle_active_layer_mask_enabled()
        },
    );

    let edit_pixels =
        append_icon_menu_item(&menu, &popover, "brush-2-line.svg", "Edit Layer Pixels", {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().edit_active_layer_pixels()
        });

    let edit_mask = append_icon_menu_item(&menu, &popover, "eraser-line.svg", "Edit Layer Mask", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().edit_active_layer_mask()
    });

    let move_up = append_icon_menu_item(&menu, &popover, "arrow-up-line.svg", "Move Layer Up", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().move_active_layer_up()
    });

    let move_down =
        append_icon_menu_item(&menu, &popover, "arrow-down-line.svg", "Move Layer Down", {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().move_active_layer_down()
        });

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
            let active_index = snapshot
                .layers
                .iter()
                .position(|layer| layer.is_active)
                .unwrap_or(0);
            let has_multiple_layers = layer_count > 1;
            let has_mask = snapshot.active_layer_has_mask;
            let text_selected = snapshot.text.selected;

            duplicate.set_sensitive(layer_count > 0 && !text_selected);
            delete.set_sensitive(has_multiple_layers);
            add_mask.set_sensitive(layer_count > 0 && !text_selected && !has_mask);
            remove_mask.set_sensitive(!text_selected && has_mask);
            toggle_mask.set_sensitive(!text_selected && has_mask);
            edit_pixels.set_sensitive(layer_count > 0 && !text_selected);
            edit_mask.set_sensitive(!text_selected && has_mask);
            move_up.set_sensitive(
                !text_selected && has_multiple_layers && active_index + 1 < layer_count,
            );
            move_down.set_sensitive(!text_selected && has_multiple_layers && active_index > 0);
        });
    }

    finish_top_level_menu(button, popover, menu)
}

fn build_select_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_Select");

    let clear = append_icon_menu_item(&menu, &popover, "close-line.svg", "Clear Selection", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().clear_selection()
    });

    let invert = append_icon_menu_item(&menu, &popover, "swap-line.svg", "Invert Selection", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().invert_selection()
    });

    {
        let shell_state = shell_state.clone();
        let clear = clear.clone();
        let invert = invert.clone();
        popover.connect_show(move |_| {
            let has_selection = shell_state
                .controller
                .borrow()
                .snapshot()
                .selection_rect
                .is_some();
            clear.set_sensitive(has_selection);
            invert.set_sensitive(has_selection);
        });
    }

    finish_top_level_menu(button, popover, menu)
}

fn build_filter_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_Filter");

    let opacity_up =
        append_icon_menu_item(&menu, &popover, "add-line.svg", "Increase Layer Opacity", {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().increase_active_layer_opacity()
        });

    let opacity_down = append_icon_menu_item(
        &menu,
        &popover,
        "subtract-line.svg",
        "Decrease Layer Opacity",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().decrease_active_layer_opacity()
        },
    );

    let next_blend = append_icon_menu_item(
        &menu,
        &popover,
        "arrow-go-forward-line.svg",
        "Next Blend Mode",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().next_active_layer_blend_mode()
        },
    );

    let previous_blend = append_icon_menu_item(
        &menu,
        &popover,
        "arrow-go-back-line.svg",
        "Previous Blend Mode",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().previous_active_layer_blend_mode()
        },
    );

    append_menu_separator(&menu);

    let invert_colors =
        append_icon_menu_item(&menu, &popover, "refresh-line.svg", "Invert Colors", {
            let controller = shell_state.controller.clone();
            move || {
                controller
                    .borrow_mut()
                    .apply_destructive_filter(DestructiveFilterKind::InvertColors)
            }
        });

    let desaturate = append_icon_menu_item(&menu, &popover, "palette-line.svg", "Desaturate", {
        let controller = shell_state.controller.clone();
        move || {
            controller
                .borrow_mut()
                .apply_destructive_filter(DestructiveFilterKind::Desaturate)
        }
    });

    {
        let shell_state = shell_state.clone();
        let opacity_up = opacity_up.clone();
        let opacity_down = opacity_down.clone();
        let next_blend = next_blend.clone();
        let previous_blend = previous_blend.clone();
        let invert_colors = invert_colors.clone();
        let desaturate = desaturate.clone();
        popover.connect_show(move |_| {
            let snapshot = shell_state.controller.borrow().snapshot();
            let has_layer = !snapshot.layers.is_empty();

            opacity_up.set_sensitive(has_layer && snapshot.active_layer_opacity_percent < 100);
            opacity_down.set_sensitive(has_layer && snapshot.active_layer_opacity_percent > 0);
            next_blend.set_sensitive(has_layer);
            previous_blend.set_sensitive(has_layer);
            invert_colors.set_sensitive(snapshot.can_apply_destructive_filters);
            desaturate.set_sensitive(snapshot.can_apply_destructive_filters);
        });
    }

    finish_top_level_menu(button, popover, menu)
}

fn build_view_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_View");

    append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "zoom-in-line.svg",
        "Zoom In",
        Some("Ctrl++"),
        {
            let shell_state = shell_state.clone();
            move || shell_state.canvas_state.borrow_mut().zoom_in()
        },
    );

    append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "zoom-out-line.svg",
        "Zoom Out",
        Some("Ctrl+-"),
        {
            let shell_state = shell_state.clone();
            move || shell_state.canvas_state.borrow_mut().zoom_out()
        },
    );

    append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "focus-3-line.svg",
        "Fit To View",
        Some("Ctrl+0"),
        {
            let shell_state = shell_state.clone();
            move || shell_state.canvas_state.borrow_mut().fit_to_view()
        },
    );

    append_icon_menu_item(
        &menu,
        &popover,
        "layout-column-line.svg",
        "Add Horizontal Guide",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().add_horizontal_guide()
        },
    );

    append_icon_menu_item(
        &menu,
        &popover,
        "layout-column-line.svg",
        "Add Vertical Guide",
        {
            let controller = shell_state.controller.clone();
            move || controller.borrow_mut().add_vertical_guide()
        },
    );

    append_icon_menu_item(&menu, &popover, "eye-line.svg", "Show/Hide Guides", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().toggle_guides_visible()
    });

    append_icon_menu_item(&menu, &popover, "settings-4-line.svg", "Toggle Snapping", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().toggle_snapping_enabled()
    });

    append_icon_menu_item(&menu, &popover, "eye-off-line.svg", "Remove Last Guide", {
        let controller = shell_state.controller.clone();
        move || controller.borrow_mut().remove_last_guide()
    });

    finish_top_level_menu(button, popover, menu)
}

fn build_file_menu_button(window: &ApplicationWindow, shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_File");

    append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "folder-open-line.svg",
        "Open Project...",
        Some("Ctrl+O"),
        {
            let shell_state = shell_state.clone();
            move || shell_state.request_open_project()
        },
    );

    append_icon_menu_item(
        &menu,
        &popover,
        "image-add-line.svg",
        "Import Image Or PSD...",
        {
            let shell_state = shell_state.clone();
            move || shell_state.request_import_image()
        },
    );

    append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "save-3-line.svg",
        "Save",
        Some("Ctrl+S"),
        {
            let shell_state = shell_state.clone();
            move || shell_state.request_project_save()
        },
    );

    append_icon_shortcut_menu_item(
        &menu,
        &popover,
        "save-3-line.svg",
        "Save As...",
        Some("Ctrl+Shift+S"),
        {
            let shell_state = shell_state.clone();
            move || shell_state.request_project_save_as()
        },
    );

    for (label, extension) in [
        ("Export PNG...", "png"),
        ("Export JPEG...", "jpg"),
        ("Export WebP...", "webp"),
    ] {
        let parent = window.clone();
        let controller = shell_state.controller.clone();
        append_icon_menu_item(&menu, &popover, "export-line.svg", label, move || {
            file_workflow::choose_export_path(&parent, controller.clone(), extension);
        });
    }

    finish_top_level_menu(button, popover, menu)
}

fn build_window_menu_button(shell_state: Rc<ShellUiState>) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_Window");

    let color_toggle =
        append_icon_menu_item(&menu, &popover, "palette-line.svg", "Toggle Color Panel", {
            let shell_state = shell_state.clone();
            move || shell_state.toggle_context_panel(ContextDockPanel::Color)
        });

    let properties_toggle = append_icon_menu_item(
        &menu,
        &popover,
        "settings-4-line.svg",
        "Toggle Properties Panel",
        {
            let shell_state = shell_state.clone();
            move || shell_state.toggle_context_panel(ContextDockPanel::Properties)
        },
    );

    let history_toggle =
        append_icon_menu_item(&menu, &popover, "history-line.svg", "Show History Tab", {
            let shell_state = shell_state.clone();
            move || shell_state.set_top_dock_tab(RightSidebarTopTab::History)
        });

    let swatches_toggle =
        append_icon_menu_item(&menu, &popover, "palette-line.svg", "Show Swatches Tab", {
            let shell_state = shell_state.clone();
            move || shell_state.set_top_dock_tab(RightSidebarTopTab::Swatches)
        });

    let layers_toggle = append_icon_menu_item(
        &menu,
        &popover,
        "layout-column-line.svg",
        "Show Layers Tab",
        {
            let shell_state = shell_state.clone();
            move || shell_state.set_bottom_dock_tab(RightSidebarBottomTab::Layers)
        },
    );

    let channels_toggle = append_icon_menu_item(
        &menu,
        &popover,
        "layout-column-line.svg",
        "Show Channels Tab",
        {
            let shell_state = shell_state.clone();
            move || shell_state.set_bottom_dock_tab(RightSidebarBottomTab::Channels)
        },
    );

    let paths_toggle =
        append_icon_menu_item(&menu, &popover, "focus-3-line.svg", "Show Paths Tab", {
            let shell_state = shell_state.clone();
            move || shell_state.set_bottom_dock_tab(RightSidebarBottomTab::Paths)
        });

    let show_all = append_icon_menu_item(
        &menu,
        &popover,
        "layout-column-line.svg",
        "Restore Default Sidebar",
        {
            let shell_state = shell_state.clone();
            move || shell_state.restore_right_sidebar_defaults()
        },
    );

    {
        let shell_state = shell_state.clone();
        let color_toggle = color_toggle.clone();
        let properties_toggle = properties_toggle.clone();
        let swatches_toggle = swatches_toggle.clone();
        let layers_toggle = layers_toggle.clone();
        let channels_toggle = channels_toggle.clone();
        let paths_toggle = paths_toggle.clone();
        let history_toggle = history_toggle.clone();
        let show_all = show_all.clone();
        popover.connect_show(move |_| {
            let color_open = shell_state.active_context_panel() == Some(ContextDockPanel::Color);
            let properties_open =
                shell_state.active_context_panel() == Some(ContextDockPanel::Properties);
            let top_tab = shell_state.active_top_dock_tab();
            let bottom_tab = shell_state.active_bottom_dock_tab();

            set_menu_button_label(
                &color_toggle,
                if color_open {
                    "Hide Color Panel"
                } else {
                    "Show Color Panel"
                },
            );
            set_menu_button_label(
                &properties_toggle,
                if properties_open {
                    "Hide Properties Panel"
                } else {
                    "Show Properties Panel"
                },
            );
            set_menu_button_label(
                &history_toggle,
                if top_tab == RightSidebarTopTab::History {
                    "History Tab Active"
                } else {
                    "Show History Tab"
                },
            );
            set_menu_button_label(
                &swatches_toggle,
                if top_tab == RightSidebarTopTab::Swatches {
                    "Swatches Tab Active"
                } else {
                    "Show Swatches Tab"
                },
            );
            set_menu_button_label(
                &layers_toggle,
                if bottom_tab == RightSidebarBottomTab::Layers {
                    "Layers Tab Active"
                } else {
                    "Show Layers Tab"
                },
            );
            set_menu_button_label(
                &channels_toggle,
                if bottom_tab == RightSidebarBottomTab::Channels {
                    "Channels Tab Active"
                } else {
                    "Show Channels Tab"
                },
            );
            set_menu_button_label(
                &paths_toggle,
                if bottom_tab == RightSidebarBottomTab::Paths {
                    "Paths Tab Active"
                } else {
                    "Show Paths Tab"
                },
            );
            show_all.set_sensitive(
                top_tab != RightSidebarTopTab::History
                    || bottom_tab != RightSidebarBottomTab::Layers
                    || shell_state.active_context_panel() != Some(ContextDockPanel::Color),
            );
        });
    }

    finish_top_level_menu(button, popover, menu)
}

fn build_help_menu_button(window: &ApplicationWindow) -> MenuButton {
    let (button, popover, menu) = build_top_level_menu("_Help");

    append_icon_menu_item(&menu, &popover, "text.svg", "Keyboard Shortcuts", {
        let parent = window.clone();
        move || {
            file_workflow::show_info_dialog(
                &parent,
                "Keyboard Shortcuts",
                "Core keyboard shortcuts",
                Some(
                    "Ctrl+O Open Project\nCtrl+S Save\nCtrl+Shift+S Save As\nCtrl+Z Undo\nCtrl+Shift+Z or Ctrl+Y Redo\nCtrl+D Clear Selection\nCtrl+I Invert Selection\nCtrl++ Zoom In\nCtrl+- Zoom Out\nCtrl+0 Fit To View\nV Move Tool\nM Marquee Tool\nI Text Tool\nT Transform Tool\nB Brush Tool\nE Eraser Tool\nH Hand Tool\nZ Zoom Tool\nEnter Commit Transform Or Text\nEsc Cancel Transform, Text, Or Clear Selection",
                ),
            );
        }
    });

    append_icon_menu_item(&menu, &popover, "node-tree.svg", "About PhotoTux", {
        let parent = window.clone();
        move || {
            file_workflow::show_info_dialog(
                &parent,
                "About PhotoTux",
                "PhotoTux",
                Some(
                    "Linux-first raster editor built with Rust, GTK4, and wgpu.\n\nThe GTK shell owns menus, panels, and status surfaces while the document model remains the source of truth.",
                ),
            );
        }
    });

    finish_top_level_menu(button, popover, menu)
}
