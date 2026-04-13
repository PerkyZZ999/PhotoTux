use super::*;
use crate::ui_support::{
    build_icon_only_button, build_remix_icon, build_tool_chip_icon_button,
    build_tool_chip_icon_label_button,
};

type ControllerAction = fn(&mut dyn ShellController);
type IconChipAction = (&'static str, &'static str, ControllerAction);
type LabelChipAction = (&'static str, ControllerAction);

impl ShellUiState {
    pub(super) fn refresh_tool_buttons(&self, snapshot: &ShellSnapshot) {
        for (tool, button) in &self.tool_buttons {
            if *tool == snapshot.active_tool {
                button.add_css_class("tool-button-active");
            } else {
                button.remove_css_class("tool-button-active");
            }
        }
    }

    pub(super) fn refresh_color_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.color_body);
        self.color_body.add_css_class("color-panel-body");

        let summary = GtkBox::new(Orientation::Horizontal, 8);
        summary.add_css_class("color-summary-row");
        summary.append(&build_color_summary_chip("FG", snapshot.foreground_color));
        summary.append(&build_color_summary_chip("BG", snapshot.background_color));
        self.color_body.append(&summary);

        let picker_row = GtkBox::new(Orientation::Horizontal, 6);
        picker_row.append(&build_color_gradient_preview(snapshot.foreground_color));
        picker_row.append(&build_color_spectrum_preview(snapshot.foreground_color));
        self.color_body.append(&picker_row);

        let fg_hex = rgba_hex(snapshot.foreground_color);
        self.color_body.append(&build_color_value_row(&[(
            "#",
            fg_hex.trim_start_matches('#').to_string(),
        )]));
        self.color_body.append(&build_color_value_row(&[
            ("R", snapshot.foreground_color[0].to_string()),
            ("G", snapshot.foreground_color[1].to_string()),
            ("B", snapshot.foreground_color[2].to_string()),
        ]));
        let [c, m, y, k] = rgba_to_cmyk(snapshot.foreground_color);
        self.color_body.append(&build_color_value_row(&[
            ("C", c.to_string()),
            ("M", m.to_string()),
            ("Y", y.to_string()),
            ("K", k.to_string()),
        ]));

        let buttons = GtkBox::new(Orientation::Horizontal, 6);
        buttons.add_css_class("color-panel-actions");
        for button in [
            wired_icon_chip(
                &self.controller,
                "swap-line.svg",
                "Swap foreground and background colors",
                |controller| controller.swap_colors(),
            ),
            wired_icon_chip(
                &self.controller,
                "refresh-line.svg",
                "Reset colors to black and white",
                |controller| controller.reset_colors(),
            ),
        ] {
            buttons.append(&button);
        }
        self.color_body.append(&buttons);

        let swatch_header = GtkBox::new(Orientation::Horizontal, 4);
        swatch_header.add_css_class("color-swatches-header");
        let swatch_title = Label::new(Some("Swatches"));
        swatch_title.set_xalign(0.0);
        swatch_title.add_css_class("color-swatches-title");
        swatch_header.append(&swatch_title);

        let swatch_menu = Button::with_label("☰");
        swatch_menu.add_css_class("panel-inline-menu");
        swatch_menu.set_hexpand(true);
        swatch_menu.set_halign(Align::End);
        swatch_menu.set_sensitive(false);
        swatch_menu.set_tooltip_text(Some("Swatch options"));
        swatch_header.append(&swatch_menu);
        self.color_body.append(&swatch_header);

        let swatch_grid = gtk4::Grid::new();
        swatch_grid.set_column_spacing(2);
        swatch_grid.set_row_spacing(2);
        swatch_grid.add_css_class("color-swatches-grid");
        for (index, color) in DEFAULT_COLOR_SWATCHES.iter().copied().enumerate() {
            let swatch = Button::new();
            swatch.set_has_frame(false);
            swatch.add_css_class("panel-swatch-button");
            if color
                == [
                    snapshot.foreground_color[0],
                    snapshot.foreground_color[1],
                    snapshot.foreground_color[2],
                ]
            {
                swatch.add_css_class("panel-swatch-button-active");
            }
            swatch.set_tooltip_text(Some(&format!(
                "Set foreground color to #{:02X}{:02X}{:02X}",
                color[0], color[1], color[2]
            )));
            swatch.set_child(Some(&build_color_patch(
                [color[0], color[1], color[2], 255],
                14,
            )));
            {
                let controller = self.controller.clone();
                swatch.connect_clicked(move |_| {
                    controller
                        .borrow_mut()
                        .set_foreground_color([color[0], color[1], color[2], 255]);
                });
            }
            swatch_grid.attach(&swatch, (index % 18) as i32, (index / 18) as i32, 1, 1);
        }
        self.color_body.append(&swatch_grid);
    }

    pub(super) fn refresh_properties_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.properties_body);
        self.append_props_overview_rows(snapshot);

        if snapshot.text.selected || snapshot.text.editing {
            self.append_props_text_section(snapshot);
        }

        self.properties_body
            .append(&self.build_props_mask_banner(snapshot));

        if snapshot.selection_rect.is_some() {
            self.append_props_selection_rows(snapshot);
        }

        self.properties_body
            .append(&self.build_props_opacity_controls());
        self.properties_body
            .append(&self.build_props_blend_controls());
        self.properties_body
            .append(&self.build_props_mask_controls(snapshot));
        self.properties_body
            .append(&self.build_props_target_controls(snapshot));
        self.properties_body
            .append(&self.build_props_selection_controls(snapshot));
        self.properties_body
            .append(&self.build_props_brush_preset_controls(snapshot));
        self.append_props_brush_parameter_controls();
        self.properties_body
            .append(&self.build_props_pressure_controls(snapshot));
        self.properties_body
            .append(&self.build_props_guide_controls(snapshot));
        self.append_props_transform_controls(snapshot);
        self.append_props_hint_rows(snapshot);
    }

    fn append_props_overview_rows(&self, snapshot: &ShellSnapshot) {
        for row in props_overview_rows(snapshot) {
            let label = Label::new(Some(&row));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.properties_body.append(&label);
        }
    }

    fn append_props_text_section(&self, snapshot: &ShellSnapshot) {
        for row in props_text_rows(snapshot) {
            let label = Label::new(Some(&row));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.properties_body.append(&label);
        }

        let text_controls = GtkBox::new(Orientation::Horizontal, 6);
        let edit_text = build_tool_chip_icon_label_button(
            "text.svg",
            if snapshot.text.editing {
                "Editing Text"
            } else {
                "Edit Text"
            },
        );
        edit_text.set_sensitive(snapshot.text.selected && !snapshot.text.editing);
        {
            let controller = self.controller.clone();
            edit_text.connect_clicked(move |_| controller.borrow_mut().begin_text_edit());
        }
        text_controls.append(&edit_text);
        self.properties_body.append(&text_controls);
    }

    fn build_props_mask_banner(&self, snapshot: &ShellSnapshot) -> GtkBox {
        let editing_mask = snapshot.active_edit_target_name == "Layer Mask";

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
            if snapshot.active_layer_mask_enabled {
                "Mask"
            } else {
                "Mask Off"
            },
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

        mask_banner
    }

    fn append_props_selection_rows(&self, snapshot: &ShellSnapshot) {
        let Some(rows) = props_selection_rows(snapshot) else {
            return;
        };
        for row in rows {
            let label = Label::new(Some(&row));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.properties_body.append(&label);
        }

        if snapshot.transform_active {
            let row = props_transform_row(snapshot);
            let label = Label::new(Some(&row));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.properties_body.append(&label);
        }

        let guides_row = props_guides_row(snapshot);
        let guides_label = Label::new(Some(&guides_row));
        guides_label.set_xalign(0.0);
        guides_label.add_css_class("panel-row");
        self.properties_body.append(&guides_label);
    }

    fn build_props_opacity_controls(&self) -> GtkBox {
        let controls = GtkBox::new(Orientation::Horizontal, 6);
        for button in [
            wired_icon_chip(
                &self.controller,
                "subtract-line.svg",
                "Decrease active layer opacity",
                |controller| controller.decrease_active_layer_opacity(),
            ),
            wired_icon_chip(
                &self.controller,
                "add-line.svg",
                "Increase active layer opacity",
                |controller| controller.increase_active_layer_opacity(),
            ),
        ] {
            controls.append(&button);
        }
        controls
    }

    fn build_props_blend_controls(&self) -> GtkBox {
        let blend_controls = GtkBox::new(Orientation::Horizontal, 6);
        for button in [
            wired_icon_chip(
                &self.controller,
                "arrow-go-back-line.svg",
                "Previous blend mode",
                |controller| controller.previous_active_layer_blend_mode(),
            ),
            wired_icon_chip(
                &self.controller,
                "arrow-go-forward-line.svg",
                "Next blend mode",
                |controller| controller.next_active_layer_blend_mode(),
            ),
        ] {
            blend_controls.append(&button);
        }
        blend_controls
    }

    fn build_props_mask_controls(&self, snapshot: &ShellSnapshot) -> GtkBox {
        let mask_controls = GtkBox::new(Orientation::Horizontal, 6);

        let add_mask = build_tool_chip_icon_label_button("add-line.svg", "Add Mask");
        add_mask.set_sensitive(!snapshot.text.selected && !snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            add_mask.connect_clicked(move |_| controller.borrow_mut().add_active_layer_mask());
        }
        mask_controls.append(&add_mask);

        let toggle_mask = build_tool_chip_icon_label_button(
            if snapshot.active_layer_mask_enabled {
                "eye-off-line.svg"
            } else {
                "eye-line.svg"
            },
            if snapshot.active_layer_mask_enabled {
                "Mask Off"
            } else {
                "Mask On"
            },
        );
        toggle_mask.set_sensitive(snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            toggle_mask.connect_clicked(move |_| {
                controller.borrow_mut().toggle_active_layer_mask_enabled()
            });
        }
        mask_controls.append(&toggle_mask);

        let remove_mask = build_tool_chip_icon_label_button("delete-bin-line.svg", "Delete Mask");
        remove_mask.set_sensitive(snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            remove_mask
                .connect_clicked(move |_| controller.borrow_mut().remove_active_layer_mask());
        }
        mask_controls.append(&remove_mask);

        mask_controls
    }

    fn build_props_target_controls(&self, snapshot: &ShellSnapshot) -> GtkBox {
        let target_controls = GtkBox::new(Orientation::Horizontal, 6);

        let edit_pixels = build_tool_chip_icon_label_button("edit-line.svg", "Edit Layer");
        edit_pixels.set_sensitive(
            !snapshot.text.selected && snapshot.active_edit_target_name != "Layer Pixels",
        );
        {
            let controller = self.controller.clone();
            edit_pixels
                .connect_clicked(move |_| controller.borrow_mut().edit_active_layer_pixels());
        }
        target_controls.append(&edit_pixels);

        let edit_mask = build_tool_chip_icon_label_button("layout-column-line.svg", "Edit Mask");
        edit_mask.set_sensitive(
            !snapshot.text.selected
                && snapshot.active_layer_has_mask
                && snapshot.active_edit_target_name != "Layer Mask",
        );
        {
            let controller = self.controller.clone();
            edit_mask.connect_clicked(move |_| controller.borrow_mut().edit_active_layer_mask());
        }
        target_controls.append(&edit_mask);

        target_controls
    }

    fn build_props_selection_controls(&self, snapshot: &ShellSnapshot) -> GtkBox {
        let selection_controls = GtkBox::new(Orientation::Horizontal, 6);

        let clear_selection =
            build_tool_chip_icon_label_button("close-line.svg", "Clear Selection");
        clear_selection.set_tooltip_text(Some("Clear selection (Ctrl+D)"));
        clear_selection.set_sensitive(snapshot.selection_rect.is_some());
        {
            let controller = self.controller.clone();
            clear_selection.connect_clicked(move |_| controller.borrow_mut().clear_selection());
        }
        selection_controls.append(&clear_selection);

        let invert_selection =
            build_tool_chip_icon_label_button("swap-line.svg", "Invert Selection");
        invert_selection.set_tooltip_text(Some("Invert selection (Ctrl+I)"));
        invert_selection.set_sensitive(snapshot.selection_rect.is_some());
        {
            let controller = self.controller.clone();
            invert_selection.connect_clicked(move |_| controller.borrow_mut().invert_selection());
        }
        selection_controls.append(&invert_selection);

        selection_controls
    }

    fn build_props_brush_preset_controls(&self, snapshot: &ShellSnapshot) -> GtkBox {
        let brush_preset_controls = GtkBox::new(Orientation::Horizontal, 6);

        brush_preset_controls.append(&wired_icon_chip(
            &self.controller,
            "arrow-go-back-line.svg",
            "Previous brush preset",
            |controller| controller.previous_brush_preset(),
        ));

        let preset_current = Label::new(Some(&format!("Preset: {}", snapshot.brush_preset_name)));
        preset_current.set_xalign(0.0);
        preset_current.add_css_class("panel-row");
        brush_preset_controls.append(&preset_current);

        brush_preset_controls.append(&wired_icon_chip(
            &self.controller,
            "arrow-go-forward-line.svg",
            "Next brush preset",
            |controller| controller.next_brush_preset(),
        ));

        brush_preset_controls
    }

    fn append_props_brush_parameter_controls(&self) {
        let row_one = GtkBox::new(Orientation::Horizontal, 6);
        let row_one_btns: [IconChipAction; 4] = [
            ("subtract-line.svg", "Decrease brush radius", |c| {
                c.decrease_brush_radius()
            }),
            ("add-line.svg", "Increase brush radius", |c| {
                c.increase_brush_radius()
            }),
            ("subtract-line.svg", "Decrease brush hardness", |c| {
                c.decrease_brush_hardness()
            }),
            ("add-line.svg", "Increase brush hardness", |c| {
                c.increase_brush_hardness()
            }),
        ];
        for (icon, tip, action) in row_one_btns {
            row_one.append(&wired_icon_chip(&self.controller, icon, tip, action));
        }
        self.properties_body.append(&row_one);

        let row_two = GtkBox::new(Orientation::Horizontal, 6);
        let row_two_btns: [IconChipAction; 4] = [
            ("subtract-line.svg", "Decrease brush spacing", |c| {
                c.decrease_brush_spacing()
            }),
            ("add-line.svg", "Increase brush spacing", |c| {
                c.increase_brush_spacing()
            }),
            ("subtract-line.svg", "Decrease brush flow", |c| {
                c.decrease_brush_flow()
            }),
            ("add-line.svg", "Increase brush flow", |c| {
                c.increase_brush_flow()
            }),
        ];
        for (icon, tip, action) in row_two_btns {
            row_two.append(&wired_icon_chip(&self.controller, icon, tip, action));
        }
        self.properties_body.append(&row_two);
    }

    fn build_props_pressure_controls(&self, snapshot: &ShellSnapshot) -> GtkBox {
        let pressure_controls = GtkBox::new(Orientation::Horizontal, 6);

        let pressure_size = wired_toggle_label_chip(
            "Pressure Size On",
            "Pressure Size Off",
            snapshot.pressure_size_enabled,
            Some("Toggle pressure-to-size mapping"),
            &self.controller,
            |controller| controller.toggle_pressure_size_enabled(),
        );
        pressure_controls.append(&pressure_size);

        let pressure_opacity = wired_toggle_label_chip(
            "Pressure Opacity On",
            "Pressure Opacity Off",
            snapshot.pressure_opacity_enabled,
            Some("Toggle pressure-to-opacity mapping"),
            &self.controller,
            |controller| controller.toggle_pressure_opacity_enabled(),
        );
        pressure_controls.append(&pressure_opacity);

        pressure_controls
    }

    fn build_props_guide_controls(&self, snapshot: &ShellSnapshot) -> GtkBox {
        let guide_controls = GtkBox::new(Orientation::Horizontal, 6);

        let add_h_guide = wired_label_chip("Guide H", true, None, &self.controller, |controller| {
            controller.add_horizontal_guide()
        });
        guide_controls.append(&add_h_guide);

        let add_v_guide = wired_label_chip("Guide V", true, None, &self.controller, |controller| {
            controller.add_vertical_guide()
        });
        guide_controls.append(&add_v_guide);

        let toggle_guides = wired_toggle_label_chip(
            "Hide Guides",
            "Show Guides",
            snapshot.guides_visible,
            None,
            &self.controller,
            |controller| controller.toggle_guides_visible(),
        );
        guide_controls.append(&toggle_guides);

        let remove_guide = wired_label_chip(
            "Remove Guide",
            snapshot.guide_count > 0,
            None,
            &self.controller,
            |controller| controller.remove_last_guide(),
        );
        guide_controls.append(&remove_guide);

        let toggle_snapping = wired_toggle_label_chip(
            "Snap On",
            "Snap Off",
            snapshot.snapping_enabled,
            None,
            &self.controller,
            |controller| controller.toggle_snapping_enabled(),
        );
        guide_controls.append(&toggle_snapping);

        guide_controls
    }

    fn append_props_transform_controls(&self, snapshot: &ShellSnapshot) {
        let transform_controls = GtkBox::new(Orientation::Horizontal, 6);

        let begin_transform = Button::with_label("Start Xform");
        begin_transform.add_css_class("tool-chip");
        begin_transform.set_tooltip_text(Some("Start transform (T)"));
        begin_transform.set_sensitive(snapshot.can_begin_transform && !snapshot.transform_active);
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
        let axis_btns: [LabelChipAction; 4] = [
            ("Scale X-", |c| c.scale_transform_x_down()),
            ("Scale X+", |c| c.scale_transform_x_up()),
            ("Scale Y-", |c| c.scale_transform_y_down()),
            ("Scale Y+", |c| c.scale_transform_y_up()),
        ];
        for (label, action) in axis_btns {
            transform_axis_controls.append(&wired_label_chip(
                label,
                snapshot.transform_active,
                None,
                &self.controller,
                action,
            ));
        }
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
    }

    fn append_props_hint_rows(&self, snapshot: &ShellSnapshot) {
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

    pub(super) fn refresh_layers_panel(self: &Rc<Self>, snapshot: &ShellSnapshot) {
        clear_box_children(&self.layers_body);
        self.layers_body.append(&self.build_layers_filter_row());
        self.layers_body
            .append(&self.build_layers_controls_row(snapshot));
        self.layers_body.append(&build_layers_info_row(snapshot));

        let filter_text = self.layers_filter_text.borrow().trim().to_ascii_lowercase();
        let list = self.build_layers_list(snapshot, &filter_text);
        let actions = self.build_layers_actions_row(snapshot);

        if list.first_child().is_none() {
            let empty = Label::new(Some("No layers match the current filter."));
            empty.set_xalign(0.0);
            empty.add_css_class("panel-row");
            empty.add_css_class("panel-hint-row");
            list.append(&empty);
        }

        self.layers_body.append(&list);
        self.layers_body.append(&actions);
    }

    fn build_layers_filter_row(self: &Rc<Self>) -> GtkBox {
        let filter_row = GtkBox::new(Orientation::Horizontal, 6);
        filter_row.add_css_class("layers-toolbar");

        let filter_box = GtkBox::new(Orientation::Horizontal, 4);
        filter_box.add_css_class("layer-filter-box");
        filter_box.append(&build_remix_icon("search-line.svg", "Find layers", 12));

        let filter_entry = Entry::new();
        filter_entry.set_hexpand(true);
        filter_entry.set_placeholder_text(Some("Find"));
        filter_entry.set_text(&self.layers_filter_text.borrow());
        filter_entry.add_css_class("layers-filter-entry");
        {
            let shell_state = self.clone();
            filter_entry.connect_changed(move |entry| {
                let next = entry.text().to_string();
                if *shell_state.layers_filter_text.borrow() != next {
                    shell_state.layers_filter_text.replace(next);
                    shell_state.bump_ui_revision();
                }
            });
        }
        filter_box.append(&filter_entry);

        let clear_filter =
            build_icon_only_button("close-line.svg", "Clear layer filter", "chrome-button", 10);
        clear_filter.add_css_class("layer-filter-clear");
        clear_filter.set_sensitive(!self.layers_filter_text.borrow().is_empty());
        {
            let shell_state = self.clone();
            clear_filter.connect_clicked(move |_| {
                if !shell_state.layers_filter_text.borrow().is_empty() {
                    shell_state.layers_filter_text.replace(String::new());
                    shell_state.bump_ui_revision();
                }
            });
        }
        filter_box.append(&clear_filter);
        filter_row.append(&filter_box);
        filter_row
    }

    fn build_layers_controls_row(self: &Rc<Self>, snapshot: &ShellSnapshot) -> GtkBox {
        let controls_row = GtkBox::new(Orientation::Horizontal, 8);
        controls_row.add_css_class("layers-blend-row");
        controls_row.append(&self.build_layer_blend_group(snapshot));
        controls_row.append(&self.build_layer_opacity_group(snapshot));
        controls_row
    }

    fn build_layer_blend_group(self: &Rc<Self>, snapshot: &ShellSnapshot) -> GtkBox {
        let blend_group = GtkBox::new(Orientation::Horizontal, 4);
        blend_group.add_css_class("layer-control-group");

        let blend_prev = build_icon_only_button(
            "arrow-go-back-line.svg",
            "Previous blend mode",
            "chrome-button",
            10,
        );
        {
            let controller = self.controller.clone();
            blend_prev.connect_clicked(move |_| {
                controller.borrow_mut().previous_active_layer_blend_mode()
            });
        }
        blend_group.append(&blend_prev);

        let blend_box = GtkBox::new(Orientation::Horizontal, 0);
        blend_box.add_css_class("layer-value-box");
        let blend_label = Label::new(Some(&snapshot.active_layer_blend_mode));
        blend_label.add_css_class("layer-value-label");
        blend_box.append(&blend_label);
        blend_group.append(&blend_box);

        let blend_next = build_icon_only_button(
            "arrow-go-forward-line.svg",
            "Next blend mode",
            "chrome-button",
            10,
        );
        {
            let controller = self.controller.clone();
            blend_next
                .connect_clicked(move |_| controller.borrow_mut().next_active_layer_blend_mode());
        }
        blend_group.append(&blend_next);
        blend_group
    }

    fn build_layer_opacity_group(self: &Rc<Self>, snapshot: &ShellSnapshot) -> GtkBox {
        let opacity_group = GtkBox::new(Orientation::Horizontal, 4);
        opacity_group.add_css_class("layer-control-group");

        let opacity_label = Label::new(Some("Opacity:"));
        opacity_label.add_css_class("layer-control-label");
        opacity_group.append(&opacity_label);

        let opacity_down = build_icon_only_button(
            "subtract-line.svg",
            "Decrease layer opacity",
            "chrome-button",
            10,
        );
        {
            let controller = self.controller.clone();
            opacity_down
                .connect_clicked(move |_| controller.borrow_mut().decrease_active_layer_opacity());
        }
        opacity_group.append(&opacity_down);

        let opacity_box = GtkBox::new(Orientation::Horizontal, 0);
        opacity_box.add_css_class("layer-value-box");
        let opacity_value =
            Label::new(Some(&format!("{}%", snapshot.active_layer_opacity_percent)));
        opacity_value.add_css_class("layer-value-label");
        opacity_box.append(&opacity_value);
        opacity_group.append(&opacity_box);

        let opacity_up = build_icon_only_button(
            "add-line.svg",
            "Increase layer opacity",
            "chrome-button",
            10,
        );
        {
            let controller = self.controller.clone();
            opacity_up
                .connect_clicked(move |_| controller.borrow_mut().increase_active_layer_opacity());
        }
        opacity_group.append(&opacity_up);
        opacity_group
    }

    fn build_layers_list(self: &Rc<Self>, snapshot: &ShellSnapshot, filter_text: &str) -> GtkBox {
        let list = GtkBox::new(Orientation::Vertical, 0);
        list.add_css_class("layers-list");

        for layer in &snapshot.layers {
            if layer_matches_filter(layer, filter_text) {
                list.append(&self.build_layer_row(layer));
            }
        }

        list
    }

    fn build_layer_row(self: &Rc<Self>, layer: &LayerPanelItem) -> GtkBox {
        let row = GtkBox::new(Orientation::Horizontal, 4);
        row.add_css_class("layer-item-shell");
        row.add_css_class(if layer.is_selected {
            "layer-row-active"
        } else {
            "layer-row"
        });
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

        row.append(&self.build_layer_visibility_button(layer));
        row.append(&build_layer_preview(layer));

        if layer.is_group {
            self.append_group_layer_content(&row, layer);
        } else if layer.is_text {
            self.append_text_layer_content(&row, layer);
        } else {
            self.append_raster_layer_content(&row, layer);
        }

        row
    }

    fn build_layer_visibility_button(self: &Rc<Self>, layer: &LayerPanelItem) -> Button {
        let visibility_icon = if layer.visible {
            "eye-line.svg"
        } else {
            "eye-off-line.svg"
        };
        let visibility =
            build_icon_only_button(visibility_icon, "Toggle Visibility", "menu-button", 12);
        visibility.add_css_class("layer-visibility-button");

        if let Some(layer_id) = layer.layer_id {
            let controller = self.controller.clone();
            visibility.connect_clicked(move |_| {
                controller.borrow_mut().toggle_layer_visibility(layer_id)
            });
        } else if let Some(group_id) = layer.group_id {
            let controller = self.controller.clone();
            visibility.connect_clicked(move |_| {
                controller.borrow_mut().toggle_group_visibility(group_id)
            });
        }

        visibility
    }

    fn append_group_layer_content(self: &Rc<Self>, row: &GtkBox, layer: &LayerPanelItem) {
        let target_strip = GtkBox::new(Orientation::Horizontal, 3);
        target_strip.add_css_class("layer-target-strip");

        let group_chip = build_target_chip("G", "Select this group", layer.is_selected, true);
        if let Some(group_id) = layer.group_id {
            let controller = self.controller.clone();
            group_chip.connect_clicked(move |_| controller.borrow_mut().select_group(group_id));
        }
        target_strip.append(&group_chip);
        row.append(&target_strip);

        let group_id = layer.group_id;
        row.append(&build_layer_content_button(
            layer,
            &format!("Group • {}%", layer.opacity_percent),
            None,
            {
                let controller = self.controller.clone();
                move || {
                    if let Some(group_id) = group_id {
                        controller.borrow_mut().select_group(group_id);
                    }
                }
            },
        ));
    }

    fn append_text_layer_content(self: &Rc<Self>, row: &GtkBox, layer: &LayerPanelItem) {
        let target_strip = GtkBox::new(Orientation::Horizontal, 3);
        target_strip.add_css_class("layer-target-strip");

        let text_target = build_target_chip("T", "Select this text layer", layer.is_selected, true);
        if let Some(layer_id) = layer.layer_id {
            let controller = self.controller.clone();
            text_target.connect_clicked(move |_| controller.borrow_mut().select_layer(layer_id));
        }
        target_strip.append(&text_target);

        let edit_target = build_target_chip("E", "Open text editing", false, true);
        if let Some(layer_id) = layer.layer_id {
            let controller = self.controller.clone();
            edit_target.connect_clicked(move |_| {
                let mut controller = controller.borrow_mut();
                controller.select_layer(layer_id);
                controller.begin_text_edit();
            });
        }
        target_strip.append(&edit_target);
        row.append(&target_strip);

        let layer_id = layer.layer_id;
        row.append(&build_layer_content_button(
            layer,
            &format!("Text • {}%", layer.opacity_percent),
            None,
            {
                let controller = self.controller.clone();
                move || {
                    if let Some(layer_id) = layer_id {
                        controller.borrow_mut().select_layer(layer_id);
                    }
                }
            },
        ));
    }

    fn append_raster_layer_content(self: &Rc<Self>, row: &GtkBox, layer: &LayerPanelItem) {
        let target_strip = GtkBox::new(Orientation::Horizontal, 3);
        target_strip.add_css_class("layer-target-strip");

        let layer_target = build_target_chip(
            "L",
            "Select the layer and edit its pixels",
            layer.is_active && !layer.mask_target_active,
            true,
        );
        if let Some(layer_id) = layer.layer_id {
            let controller = self.controller.clone();
            layer_target.connect_clicked(move |_| {
                let mut controller = controller.borrow_mut();
                controller.select_layer(layer_id);
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
        if layer.has_mask
            && let Some(layer_id) = layer.layer_id
        {
            let controller = self.controller.clone();
            mask_target.connect_clicked(move |_| {
                let mut controller = controller.borrow_mut();
                controller.select_layer(layer_id);
                controller.edit_active_layer_mask();
            });
        }
        target_strip.append(&mask_target);
        row.append(&target_strip);

        let layer_id = layer.layer_id;
        row.append(&build_layer_content_button(
            layer,
            &format!("{}%{}", layer.opacity_percent, layer_mask_suffix(layer)),
            Some(if layer.is_active { "Active" } else { "" }),
            {
                let controller = self.controller.clone();
                move || {
                    if let Some(layer_id) = layer_id {
                        controller.borrow_mut().select_layer(layer_id);
                    }
                }
            },
        ));
    }

    fn build_layers_actions_row(self: &Rc<Self>, snapshot: &ShellSnapshot) -> GtkBox {
        let actions = GtkBox::new(Orientation::Horizontal, 4);
        actions.add_css_class("layers-bottom");

        for (icon_name, label, action) in build_layer_action_specs(snapshot) {
            let button = build_tool_chip_icon_button(icon_name, &label);
            button.add_css_class("layer-action-chip");
            button.set_sensitive(layer_action_sensitive(action, snapshot));

            let controller = self.controller.clone();
            button.connect_clicked(move |_| run_layer_action(&controller, action));
            actions.append(&button);
        }

        actions
    }

    pub(super) fn refresh_history_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.history_body);

        let actions = GtkBox::new(Orientation::Horizontal, 6);
        actions.add_css_class("history-toolbar");
        let undo = build_tool_chip_icon_label_button("arrow-go-back-line.svg", "Undo");
        undo.set_sensitive(snapshot.can_undo);
        {
            let controller = self.controller.clone();
            undo.connect_clicked(move |_| controller.borrow_mut().undo());
        }
        actions.append(&undo);

        let redo = build_tool_chip_icon_label_button("arrow-go-forward-line.svg", "Redo");
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

fn props_overview_rows(snapshot: &ShellSnapshot) -> [String; 14] {
    [
        format!("Tool: {}", snapshot.active_tool_name),
        format!(
            "Tool Shortcut: {}",
            shell_tool_shortcut(snapshot.active_tool)
        ),
        format!("Layer: {}", snapshot.active_layer_name),
        format!("Selection: {}", snapshot.selected_structure_name),
        format!("Edit Target: {}", snapshot.active_edit_target_name),
        format!("Blend: {}", snapshot.active_layer_blend_mode),
        format!("Opacity: {}%", snapshot.active_layer_opacity_percent),
        format!(
            "Visible: {}",
            if snapshot.active_layer_visible {
                "Yes"
            } else {
                "No"
            }
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
        format!("Brush Preset: {}", snapshot.brush_preset_name),
        format!("Brush Radius: {} px", snapshot.brush_radius),
        format!("Brush Hardness: {}%", snapshot.brush_hardness_percent),
        format!("Brush Spacing: {} px", snapshot.brush_spacing),
        format!("Brush Flow: {}%", snapshot.brush_flow_percent),
    ]
}

fn props_text_rows(snapshot: &ShellSnapshot) -> [String; 4] {
    [
        format!(
            "Text Content: {}",
            if snapshot.text.content.is_empty() {
                "<empty>"
            } else {
                snapshot.text.content.as_str()
            }
        ),
        format!(
            "Text Style: {} {}px | Line {}% | Track {}",
            snapshot.text.font_family,
            snapshot.text.font_size_px,
            snapshot.text.line_height_percent,
            snapshot.text.letter_spacing
        ),
        format!(
            "Text Fill: #{:02X}{:02X}{:02X}{:02X} | Align {:?}",
            snapshot.text.fill_rgba[0],
            snapshot.text.fill_rgba[1],
            snapshot.text.fill_rgba[2],
            snapshot.text.fill_rgba[3],
            snapshot.text.alignment
        ),
        format!(
            "Text Origin: {}, {}",
            snapshot.text.origin_x, snapshot.text.origin_y
        ),
    ]
}

fn props_selection_rows(snapshot: &ShellSnapshot) -> Option<[String; 2]> {
    let selection = snapshot.selection_rect?;
    Some([
        format!(
            "Selection: {},{}  {}x{}",
            selection.x, selection.y, selection.width, selection.height
        ),
        format!(
            "Selection Mode: {}",
            if snapshot.selection_inverted {
                "Inverted"
            } else {
                "Normal"
            }
        ),
    ])
}

fn props_transform_row(snapshot: &ShellSnapshot) -> String {
    format!(
        "Transform: {}% | X {}% | Y {}% | {}deg",
        snapshot.transform_scale_percent,
        snapshot.transform_scale_x_percent,
        snapshot.transform_scale_y_percent,
        snapshot.transform_rotation_degrees
    )
}

fn props_guides_row(snapshot: &ShellSnapshot) -> String {
    format!(
        "Guides: {} ({}) | Snapping {}",
        snapshot.guide_count,
        if snapshot.guides_visible {
            "Visible"
        } else {
            "Hidden"
        },
        if snapshot.snapping_enabled {
            "On"
        } else {
            "Off"
        }
    )
}

#[derive(Clone, Copy)]
enum LayerAction {
    Add,
    AddGroup,
    Ungroup,
    Duplicate,
    Delete,
    EditText,
    MoveIntoGroup,
    MoveOutOfGroup,
    AddMask,
    ToggleMask,
    ToggleMaskTarget,
    MoveUp,
    MoveDown,
}

fn build_layers_info_row(snapshot: &ShellSnapshot) -> GtkBox {
    let info_row = GtkBox::new(Orientation::Horizontal, 6);
    info_row.add_css_class("layers-info-row");

    for text in [
        format!(
            "Visible: {}",
            if snapshot.active_layer_visible {
                "On"
            } else {
                "Off"
            }
        ),
        format!(
            "Mask: {}",
            if !snapshot.active_layer_has_mask {
                "None"
            } else if snapshot.active_layer_mask_enabled {
                "On"
            } else {
                "Off"
            }
        ),
        format!("Target: {}", snapshot.active_edit_target_name),
    ] {
        let chip = Label::new(Some(&text));
        chip.add_css_class("layers-info-chip");
        info_row.append(&chip);
    }

    info_row
}

fn build_layer_action_specs(snapshot: &ShellSnapshot) -> Vec<(&'static str, String, LayerAction)> {
    let toggle_mask_icon = if snapshot.active_layer_mask_enabled {
        "eye-off-line.svg"
    } else {
        "eye-line.svg"
    };
    let toggle_mask_label = if snapshot.active_layer_mask_enabled {
        "Mask Off"
    } else {
        "Mask On"
    };
    let toggle_target_label = if snapshot.active_edit_target_name == "Layer Mask" {
        "Edit Layer"
    } else {
        "Edit Mask"
    };

    vec![
        ("add-line.svg", "+ Layer".to_string(), LayerAction::Add),
        (
            "group-line.svg",
            "+ Group".to_string(),
            LayerAction::AddGroup,
        ),
        ("node-tree.svg", "Ungroup".to_string(), LayerAction::Ungroup),
        (
            "file-copy-line.svg",
            "Duplicate".to_string(),
            LayerAction::Duplicate,
        ),
        (
            "delete-bin-line.svg",
            "Delete".to_string(),
            LayerAction::Delete,
        ),
        ("text.svg", "Edit Text".to_string(), LayerAction::EditText),
        (
            "folder-add-line.svg",
            "Into Group".to_string(),
            LayerAction::MoveIntoGroup,
        ),
        (
            "folder-reduce-line.svg",
            "Out Group".to_string(),
            LayerAction::MoveOutOfGroup,
        ),
        ("add-line.svg", "+ Mask".to_string(), LayerAction::AddMask),
        (
            toggle_mask_icon,
            toggle_mask_label.to_string(),
            LayerAction::ToggleMask,
        ),
        (
            "edit-line.svg",
            toggle_target_label.to_string(),
            LayerAction::ToggleMaskTarget,
        ),
        ("arrow-up-line.svg", "Up".to_string(), LayerAction::MoveUp),
        (
            "arrow-down-line.svg",
            "Down".to_string(),
            LayerAction::MoveDown,
        ),
    ]
}

fn layer_action_sensitive(action: LayerAction, snapshot: &ShellSnapshot) -> bool {
    match action {
        LayerAction::Add | LayerAction::Delete => true,
        LayerAction::AddGroup => {
            snapshot.can_create_group_from_active_layer && !snapshot.text.selected
        }
        LayerAction::Ungroup => snapshot.can_ungroup_selected_group,
        LayerAction::Duplicate => !snapshot.text.selected,
        LayerAction::EditText => snapshot.text.selected && !snapshot.text.editing,
        LayerAction::MoveIntoGroup => {
            snapshot.can_move_active_layer_into_selected_group && !snapshot.text.selected
        }
        LayerAction::MoveOutOfGroup => {
            snapshot.can_move_active_layer_out_of_group && !snapshot.text.selected
        }
        LayerAction::AddMask => !snapshot.text.selected && !snapshot.active_layer_has_mask,
        LayerAction::ToggleMask | LayerAction::ToggleMaskTarget => {
            !snapshot.text.selected && snapshot.active_layer_has_mask
        }
        LayerAction::MoveUp | LayerAction::MoveDown => !snapshot.text.selected,
    }
}

fn run_layer_action(controller: &Rc<RefCell<dyn ShellController>>, action: LayerAction) {
    match action {
        LayerAction::Add => controller.borrow_mut().add_layer(),
        LayerAction::AddGroup => controller.borrow_mut().create_group_from_active_layer(),
        LayerAction::Ungroup => controller.borrow_mut().ungroup_selected_group(),
        LayerAction::Duplicate => controller.borrow_mut().duplicate_active_layer(),
        LayerAction::Delete => controller.borrow_mut().delete_active_layer(),
        LayerAction::EditText => controller.borrow_mut().begin_text_edit(),
        LayerAction::MoveIntoGroup => controller
            .borrow_mut()
            .move_active_layer_into_selected_group(),
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
    }
}

fn clear_box_children(container: &GtkBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

const DEFAULT_COLOR_SWATCHES: [[u8; 3]; 36] = [
    [0x00, 0x00, 0x00],
    [0x3a, 0x3a, 0x3a],
    [0x66, 0x66, 0x66],
    [0x99, 0x99, 0x99],
    [0xcc, 0xcc, 0xcc],
    [0xff, 0xff, 0xff],
    [0xff, 0x3b, 0x30],
    [0xff, 0x95, 0x00],
    [0xff, 0xea, 0x00],
    [0x8a, 0xff, 0x00],
    [0x00, 0xe6, 0x5a],
    [0x00, 0xc7, 0xff],
    [0x00, 0x7a, 0xff],
    [0x36, 0x3f, 0xe0],
    [0x7b, 0x2c, 0xff],
    [0xb1, 0x20, 0xff],
    [0xd0, 0x3b, 0xff],
    [0xff, 0x4d, 0x9d],
    [0x6d, 0x4c, 0x41],
    [0xa1, 0x88, 0x7f],
    [0xc0, 0xca, 0x33],
    [0x66, 0xbb, 0x6a],
    [0x26, 0xa6, 0x9a],
    [0x42, 0xa5, 0xf5],
    [0x5c, 0x6b, 0xc0],
    [0xab, 0x47, 0xbc],
    [0xef, 0x53, 0x50],
    [0xff, 0x70, 0x43],
    [0xff, 0xca, 0x28],
    [0xd4, 0xe1, 0x57],
    [0x9c, 0xcc, 0x65],
    [0x4d, 0xd0, 0xe1],
    [0x4f, 0xc3, 0xf7],
    [0x90, 0xa4, 0xae],
    [0xf4, 0xa2, 0xc5],
    [0xcf, 0xd8, 0xdc],
];

fn rgba_hex(rgba: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2])
}

fn rgba_to_cmyk(rgba: [u8; 4]) -> [u8; 4] {
    let r = rgba[0] as f32 / 255.0;
    let g = rgba[1] as f32 / 255.0;
    let b = rgba[2] as f32 / 255.0;
    let k = 1.0 - r.max(g).max(b);
    if k >= 0.999 {
        return [0, 0, 0, 100];
    }
    let c = ((1.0 - r - k) / (1.0 - k) * 100.0).round() as u8;
    let m = ((1.0 - g - k) / (1.0 - k) * 100.0).round() as u8;
    let y = ((1.0 - b - k) / (1.0 - k) * 100.0).round() as u8;
    [c, m, y, (k * 100.0).round() as u8]
}

fn rgba_to_hsv(rgba: [u8; 4]) -> (f64, f64, f64) {
    let r = rgba[0] as f64 / 255.0;
    let g = rgba[1] as f64 / 255.0;
    let b = rgba[2] as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta <= f64::EPSILON {
        0.0
    } else if (max - r).abs() < f64::EPSILON {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if (max - g).abs() < f64::EPSILON {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let saturation = if max <= f64::EPSILON {
        0.0
    } else {
        delta / max
    };
    (hue, saturation, max)
}

fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let c = value * saturation;
    let x = c * (1.0 - (((hue / 60.0) % 2.0) - 1.0).abs());
    let m = value - c;
    let (r1, g1, b1) = match hue as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    ]
}

fn build_color_patch(rgba: [u8; 4], size: i32) -> gtk4::DrawingArea {
    let patch = gtk4::DrawingArea::new();
    patch.set_content_width(size);
    patch.set_content_height(size);
    patch.set_draw_func(move |_, ctx, width, height| {
        let width = width as f64;
        let height = height as f64;
        ctx.set_source_rgba(
            rgba[0] as f64 / 255.0,
            rgba[1] as f64 / 255.0,
            rgba[2] as f64 / 255.0,
            rgba[3] as f64 / 255.0,
        );
        ctx.rectangle(0.5, 0.5, width - 1.0, height - 1.0);
        let _ = ctx.fill_preserve();
        ctx.set_source_rgba(0.15, 0.15, 0.15, 0.9);
        ctx.set_line_width(1.0);
        let _ = ctx.stroke();
    });
    patch
}

fn build_color_summary_chip(prefix: &str, rgba: [u8; 4]) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 6);
    row.add_css_class("color-summary-chip");
    row.append(&build_color_patch(rgba, 18));
    let label = Label::new(Some(&format!("{prefix}: {}", rgba_hex(rgba))));
    label.set_xalign(0.0);
    label.add_css_class("color-summary-label");
    row.append(&label);
    row
}

fn build_color_gradient_preview(rgba: [u8; 4]) -> gtk4::Overlay {
    let (hue, saturation, value) = rgba_to_hsv(rgba);
    let hue_rgb = hsv_to_rgb(hue, 1.0, 1.0);

    let overlay = gtk4::Overlay::new();
    overlay.add_css_class("color-gradient-frame");

    let surface = gtk4::DrawingArea::new();
    surface.set_content_width(220);
    surface.set_content_height(175);
    surface.set_draw_func(move |_, ctx, width, height| {
        let width = width as f64;
        let height = height as f64;
        let base = gtk4::cairo::LinearGradient::new(0.0, 0.0, width, 0.0);
        base.add_color_stop_rgb(0.0, 1.0, 1.0, 1.0);
        base.add_color_stop_rgb(
            1.0,
            hue_rgb[0] as f64 / 255.0,
            hue_rgb[1] as f64 / 255.0,
            hue_rgb[2] as f64 / 255.0,
        );
        let _ = ctx.set_source(&base);
        ctx.rectangle(0.0, 0.0, width, height);
        let _ = ctx.fill();

        let shade = gtk4::cairo::LinearGradient::new(0.0, 0.0, 0.0, height);
        shade.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.0);
        shade.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 1.0);
        let _ = ctx.set_source(&shade);
        ctx.rectangle(0.0, 0.0, width, height);
        let _ = ctx.fill();
    });
    overlay.set_child(Some(&surface));

    let cursor = GtkBox::new(Orientation::Horizontal, 0);
    cursor.add_css_class("color-picker-cursor");
    cursor.set_halign(Align::Start);
    cursor.set_valign(Align::Start);
    cursor.set_margin_start((saturation * 210.0).round() as i32);
    cursor.set_margin_top(((1.0 - value) * 165.0).round() as i32);
    overlay.add_overlay(&cursor);
    overlay
}

fn build_color_spectrum_preview(rgba: [u8; 4]) -> gtk4::Overlay {
    let (hue, _, _) = rgba_to_hsv(rgba);

    let overlay = gtk4::Overlay::new();
    overlay.add_css_class("color-spectrum-frame");

    let spectrum = gtk4::DrawingArea::new();
    spectrum.set_content_width(14);
    spectrum.set_content_height(175);
    spectrum.set_draw_func(move |_, ctx, width, height| {
        let width = width as f64;
        let height = height as f64;
        let gradient = gtk4::cairo::LinearGradient::new(0.0, 0.0, 0.0, height);
        for (offset, color) in [
            (0.0, [255, 0, 0]),
            (0.16, [255, 128, 0]),
            (0.32, [255, 255, 0]),
            (0.48, [0, 255, 0]),
            (0.64, [0, 255, 255]),
            (0.80, [0, 0, 255]),
            (1.0, [255, 0, 255]),
        ] {
            gradient.add_color_stop_rgb(
                offset,
                color[0] as f64 / 255.0,
                color[1] as f64 / 255.0,
                color[2] as f64 / 255.0,
            );
        }
        let _ = ctx.set_source(&gradient);
        ctx.rectangle(0.0, 0.0, width, height);
        let _ = ctx.fill();
    });
    overlay.set_child(Some(&spectrum));

    let cursor = GtkBox::new(Orientation::Horizontal, 0);
    cursor.add_css_class("color-spectrum-cursor");
    cursor.set_halign(Align::Start);
    cursor.set_valign(Align::Start);
    cursor.set_margin_top(((hue / 360.0) * 171.0).round() as i32);
    overlay.add_overlay(&cursor);
    overlay
}

fn build_color_value_row(fields: &[(&str, String)]) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 4);
    row.add_css_class("color-value-row");
    for (label_text, value_text) in fields {
        let field = GtkBox::new(Orientation::Horizontal, 4);
        field.add_css_class("color-value-field");

        let label = Label::new(Some(label_text));
        label.add_css_class("color-value-key");
        field.append(&label);

        let value = Label::new(Some(value_text));
        value.set_xalign(1.0);
        value.set_hexpand(true);
        value.add_css_class("color-value-text");
        field.append(&value);

        row.append(&field);
    }
    row
}

fn layer_matches_filter(layer: &LayerPanelItem, filter_text: &str) -> bool {
    if filter_text.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {}",
        layer.name,
        if layer.is_group { "group" } else { "layer" },
        if layer.is_text { "text" } else { "" },
        if layer.has_mask { "mask" } else { "" }
    )
    .to_ascii_lowercase();
    haystack.contains(filter_text)
}

fn layer_mask_suffix(layer: &LayerPanelItem) -> String {
    if !layer.has_mask {
        return String::new();
    }
    if layer.mask_target_active {
        return if layer.mask_enabled {
            "  [Mask Editing]".to_string()
        } else {
            "  [Mask Editing Off]".to_string()
        };
    }
    if layer.mask_enabled {
        "  [Mask]".to_string()
    } else {
        "  [Mask Off]".to_string()
    }
}

fn build_layer_preview(layer: &LayerPanelItem) -> GtkBox {
    let preview = GtkBox::new(Orientation::Vertical, 0);
    preview.add_css_class("layer-preview");
    if layer.is_group {
        preview.add_css_class("layer-preview-group");
    } else if layer.is_text {
        preview.add_css_class("layer-preview-text");
    } else {
        preview.add_css_class("layer-preview-raster");
    }
    if layer.has_mask {
        preview.add_css_class("layer-preview-masked");
    }

    let glyph = Label::new(Some(if layer.is_group {
        "G"
    } else if layer.is_text {
        "T"
    } else {
        "L"
    }));
    glyph.add_css_class("layer-preview-glyph");
    preview.append(&glyph);
    preview
}

fn build_layer_content_button<F>(
    layer: &LayerPanelItem,
    meta_text: &str,
    badge: Option<&str>,
    on_click: F,
) -> Button
where
    F: Fn() + 'static,
{
    let button = Button::new();
    button.add_css_class("layer-content-button");
    if layer.is_selected {
        button.add_css_class("layer-content-button-active");
    }

    let content = GtkBox::new(Orientation::Vertical, 2);
    content.set_hexpand(true);

    let title_row = GtkBox::new(Orientation::Horizontal, 4);
    title_row.set_hexpand(true);
    let title = Label::new(Some(&layer.name));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("layer-name-title");
    title_row.append(&title);

    if let Some(badge_text) = badge
        && !badge_text.is_empty()
    {
        let badge_label = Label::new(Some(badge_text));
        badge_label.add_css_class("layer-state-badge");
        title_row.append(&badge_label);
    }
    content.append(&title_row);

    let meta = Label::new(Some(meta_text));
    meta.set_xalign(0.0);
    meta.add_css_class("layer-meta-label");
    content.append(&meta);

    button.set_child(Some(&content));
    button.connect_clicked(move |_| on_click());
    button
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

fn wire_button_to_controller_action(
    button: &Button,
    controller: &Rc<RefCell<dyn ShellController>>,
    action: ControllerAction,
) {
    let controller = controller.clone();
    button.connect_clicked(move |_| action(&mut *controller.borrow_mut()));
}

/// Build an icon-only tool-chip button and wire a controller action to it.
fn wired_icon_chip(
    controller: &Rc<RefCell<dyn ShellController>>,
    icon: &str,
    tooltip: &str,
    action: ControllerAction,
) -> Button {
    let button = build_tool_chip_icon_button(icon, tooltip);
    wire_button_to_controller_action(&button, controller, action);
    button
}

/// Build a label-only tool-chip button, set its sensitivity, and wire a controller action to it.
fn wired_label_chip(
    label: &str,
    sensitive: bool,
    tooltip: Option<&str>,
    controller: &Rc<RefCell<dyn ShellController>>,
    action: ControllerAction,
) -> Button {
    let button = Button::with_label(label);
    button.add_css_class("tool-chip");
    button.set_sensitive(sensitive);
    button.set_tooltip_text(tooltip);
    wire_button_to_controller_action(&button, controller, action);
    button
}

/// Build a label-only tool-chip button whose label reflects a boolean state.
fn wired_toggle_label_chip(
    enabled_label: &str,
    disabled_label: &str,
    enabled: bool,
    tooltip: Option<&str>,
    controller: &Rc<RefCell<dyn ShellController>>,
    action: ControllerAction,
) -> Button {
    wired_label_chip(
        if enabled {
            enabled_label
        } else {
            disabled_label
        },
        true,
        tooltip,
        controller,
        action,
    )
}
