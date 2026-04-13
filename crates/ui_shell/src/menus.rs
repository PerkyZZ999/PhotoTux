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

    let commit_transform =
        build_icon_label_shortcut_button("check-line.svg", "Commit Transform", Some("Enter"));
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

    let cancel_transform =
        build_icon_label_shortcut_button("close-line.svg", "Cancel Transform", Some("Esc"));
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
            let transform_active = snapshot.transform_active;

            start_transform.set_sensitive(snapshot.can_begin_transform && !transform_active);
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

    let toggle_mask =
        build_icon_label_button("contrast-2-line.svg", "Enable or Disable Layer Mask");
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

    let opacity_down =
        build_icon_label_button("indeterminate-circle-line.svg", "Decrease Layer Opacity");
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

    let filter_separator = Separator::new(Orientation::Horizontal);
    menu.append(&filter_separator);

    let invert_colors = build_icon_label_button("refresh-line.svg", "Invert Colors");
    invert_colors.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        invert_colors.connect_clicked(move |_| {
            popover.popdown();
            controller
                .borrow_mut()
                .apply_destructive_filter(DestructiveFilterKind::InvertColors);
        });
    }
    menu.append(&invert_colors);

    let desaturate = build_icon_label_button("contrast-drop-2-line.svg", "Desaturate");
    desaturate.add_css_class("menu-dropdown-item");
    {
        let controller = shell_state.controller.clone();
        let popover = popover.clone();
        desaturate.connect_clicked(move |_| {
            popover.popdown();
            controller
                .borrow_mut()
                .apply_destructive_filter(DestructiveFilterKind::Desaturate);
        });
    }
    menu.append(&desaturate);

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

    let zoom_out =
        build_icon_label_shortcut_button("zoom-out-line.svg", "Zoom Out", Some("Ctrl+-"));
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

    let fit =
        build_icon_label_shortcut_button("fullscreen-line.svg", "Fit To View", Some("Ctrl+0"));
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

    let add_horizontal_guide =
        build_icon_label_button("layout-column-line.svg", "Add Horizontal Guide");
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

    let add_vertical_guide =
        build_icon_label_button("layout-column-line.svg", "Add Vertical Guide");
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

fn build_file_menu_button(window: &ApplicationWindow, shell_state: Rc<ShellUiState>) -> MenuButton {
    let button = MenuButton::builder().label("File").build();
    button.set_has_frame(false);
    button.add_css_class("menu-button");

    let (popover, menu) = create_menu_popover(&button);

    let open_project =
        build_icon_label_shortcut_button("folder-open-line.svg", "Open Project...", Some("Ctrl+O"));
    open_project.add_css_class("menu-dropdown-item");
    {
        let parent = window.clone();
        let shell_state = shell_state.clone();
        let popover = popover.clone();
        open_project.connect_clicked(move |_| {
            popover.popdown();
            file_workflow::choose_open_project(&parent, shell_state.clone());
        });
    }
    menu.append(&open_project);

    let import_image = build_icon_label_button("image-add-line.svg", "Import Image Or PSD...");
    import_image.add_css_class("menu-dropdown-item");
    {
        let parent = window.clone();
        let shell_state = shell_state.clone();
        let popover = popover.clone();
        import_image.connect_clicked(move |_| {
            popover.popdown();
            file_workflow::choose_import_image(&parent, shell_state.clone());
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

    let save_as =
        build_icon_label_shortcut_button("save-3-line.svg", "Save As...", Some("Ctrl+Shift+S"));
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
            file_workflow::choose_export_path(&parent, controller.clone(), extension);
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

    let properties_toggle =
        build_icon_label_button("equalizer-line.svg", "Toggle Properties Panel");
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

            set_menu_button_label(
                &color_toggle,
                if color_visible {
                    "Hide Color Panel"
                } else {
                    "Show Color Panel"
                },
            );
            set_menu_button_label(
                &properties_toggle,
                if properties_visible {
                    "Hide Properties Panel"
                } else {
                    "Show Properties Panel"
                },
            );
            set_menu_button_label(
                &layers_toggle,
                if layers_visible {
                    "Hide Layers Panel"
                } else {
                    "Show Layers Panel"
                },
            );
            set_menu_button_label(
                &history_toggle,
                if history_visible {
                    "Hide History Panel"
                } else {
                    "Show History Panel"
                },
            );
            show_all.set_sensitive(
                !(color_visible && properties_visible && layers_visible && history_visible),
            );
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
            file_workflow::show_info_dialog(
                &parent,
                "Keyboard Shortcuts",
                "Core keyboard shortcuts",
                Some(
                    "Ctrl+O Open Project\nCtrl+S Save\nCtrl+Shift+S Save As\nCtrl+Z Undo\nCtrl+Shift+Z or Ctrl+Y Redo\nCtrl+D Clear Selection\nCtrl+I Invert Selection\nCtrl++ Zoom In\nCtrl+- Zoom Out\nCtrl+0 Fit To View\nV Move Tool\nM Marquee Tool\nI Text Tool\nT Transform Tool\nB Brush Tool\nE Eraser Tool\nH Hand Tool\nZ Zoom Tool\nEnter Commit Transform Or Text\nEsc Cancel Transform, Text, Or Clear Selection",
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
            file_workflow::show_info_dialog(
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
