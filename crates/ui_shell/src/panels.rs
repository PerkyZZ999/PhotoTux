use super::*;

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

    pub(super) fn refresh_properties_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.properties_body);
        let editing_mask = snapshot.active_edit_target_name == "Layer Mask";
        for row in [
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
        ] {
            let label = Label::new(Some(&row));
            label.set_xalign(0.0);
            label.add_css_class("panel-row");
            self.properties_body.append(&label);
        }

        if snapshot.text.selected || snapshot.text.editing {
            for row in [
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
            ] {
                let label = Label::new(Some(&row));
                label.set_xalign(0.0);
                label.add_css_class("panel-row");
                self.properties_body.append(&label);
            }

            let text_controls = GtkBox::new(Orientation::Horizontal, 6);
            let edit_text = Button::with_label(if snapshot.text.editing {
                "Editing Text"
            } else {
                "Edit Text"
            });
            edit_text.add_css_class("tool-chip");
            edit_text.set_sensitive(snapshot.text.selected && !snapshot.text.editing);
            {
                let controller = self.controller.clone();
                edit_text.connect_clicked(move |_| controller.borrow_mut().begin_text_edit());
            }
            text_controls.append(&edit_text);
            self.properties_body.append(&text_controls);
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
        self.properties_body.append(&mask_banner);

        if let Some(selection) = snapshot.selection_rect {
            for row in [
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
                if snapshot.guides_visible {
                    "Visible"
                } else {
                    "Hidden"
                },
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
            opacity_down
                .connect_clicked(move |_| controller.borrow_mut().decrease_active_layer_opacity());
        }
        controls.append(&opacity_down);

        let opacity_up = Button::with_label("Opacity +");
        opacity_up.add_css_class("tool-chip");
        opacity_up.set_tooltip_text(Some("Increase active layer opacity"));
        {
            let controller = self.controller.clone();
            opacity_up
                .connect_clicked(move |_| controller.borrow_mut().increase_active_layer_opacity());
        }
        controls.append(&opacity_up);
        self.properties_body.append(&controls);

        let blend_controls = GtkBox::new(Orientation::Horizontal, 6);
        let blend_prev = Button::with_label("Blend -");
        blend_prev.add_css_class("tool-chip");
        blend_prev.set_tooltip_text(Some("Previous blend mode"));
        {
            let controller = self.controller.clone();
            blend_prev.connect_clicked(move |_| {
                controller.borrow_mut().previous_active_layer_blend_mode()
            });
        }
        blend_controls.append(&blend_prev);

        let blend_next = Button::with_label("Blend +");
        blend_next.add_css_class("tool-chip");
        blend_next.set_tooltip_text(Some("Next blend mode"));
        {
            let controller = self.controller.clone();
            blend_next
                .connect_clicked(move |_| controller.borrow_mut().next_active_layer_blend_mode());
        }
        blend_controls.append(&blend_next);
        self.properties_body.append(&blend_controls);

        let mask_controls = GtkBox::new(Orientation::Horizontal, 6);

        let add_mask = Button::with_label("Add Mask");
        add_mask.add_css_class("tool-chip");
        add_mask.set_sensitive(!snapshot.text.selected && !snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            add_mask.connect_clicked(move |_| controller.borrow_mut().add_active_layer_mask());
        }
        mask_controls.append(&add_mask);

        let toggle_mask = Button::with_label(if snapshot.active_layer_mask_enabled {
            "Mask Off"
        } else {
            "Mask On"
        });
        toggle_mask.add_css_class("tool-chip");
        toggle_mask.set_sensitive(snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            toggle_mask.connect_clicked(move |_| {
                controller.borrow_mut().toggle_active_layer_mask_enabled()
            });
        }
        mask_controls.append(&toggle_mask);

        let remove_mask = Button::with_label("Delete Mask");
        remove_mask.add_css_class("tool-chip");
        remove_mask.set_sensitive(snapshot.active_layer_has_mask);
        {
            let controller = self.controller.clone();
            remove_mask
                .connect_clicked(move |_| controller.borrow_mut().remove_active_layer_mask());
        }
        mask_controls.append(&remove_mask);
        self.properties_body.append(&mask_controls);

        let target_controls = GtkBox::new(Orientation::Horizontal, 6);
        let edit_pixels = Button::with_label("Edit Layer");
        edit_pixels.add_css_class("tool-chip");
        edit_pixels
            .set_sensitive(!snapshot.text.selected && snapshot.active_edit_target_name != "Layer Pixels");
        {
            let controller = self.controller.clone();
            edit_pixels
                .connect_clicked(move |_| controller.borrow_mut().edit_active_layer_pixels());
        }
        target_controls.append(&edit_pixels);

        let edit_mask = Button::with_label("Edit Mask");
        edit_mask.add_css_class("tool-chip");
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

        let brush_preset_controls = GtkBox::new(Orientation::Horizontal, 6);

        let preset_prev = Button::with_label("Preset -");
        preset_prev.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            preset_prev.connect_clicked(move |_| controller.borrow_mut().previous_brush_preset());
        }
        brush_preset_controls.append(&preset_prev);

        let preset_current = Label::new(Some(&format!("Preset: {}", snapshot.brush_preset_name)));
        preset_current.set_xalign(0.0);
        preset_current.add_css_class("panel-row");
        brush_preset_controls.append(&preset_current);

        let preset_next = Button::with_label("Preset +");
        preset_next.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            preset_next.connect_clicked(move |_| controller.borrow_mut().next_brush_preset());
        }
        brush_preset_controls.append(&preset_next);

        self.properties_body.append(&brush_preset_controls);

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
            hardness_down
                .connect_clicked(move |_| controller.borrow_mut().decrease_brush_hardness());
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
            pressure_size
                .connect_clicked(move |_| controller.borrow_mut().toggle_pressure_size_enabled());
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
            pressure_opacity.connect_clicked(move |_| {
                controller.borrow_mut().toggle_pressure_opacity_enabled()
            });
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

        let toggle_guides = Button::with_label(if snapshot.guides_visible {
            "Hide Guides"
        } else {
            "Show Guides"
        });
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

        let toggle_snapping = Button::with_label(if snapshot.snapping_enabled {
            "Snap On"
        } else {
            "Snap Off"
        });
        toggle_snapping.add_css_class("tool-chip");
        {
            let controller = self.controller.clone();
            toggle_snapping
                .connect_clicked(move |_| controller.borrow_mut().toggle_snapping_enabled());
        }
        guide_controls.append(&toggle_snapping);
        self.properties_body.append(&guide_controls);

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

    pub(super) fn refresh_layers_panel(&self, snapshot: &ShellSnapshot) {
        clear_box_children(&self.layers_body);

        let actions = GtkBox::new(Orientation::Horizontal, 4);
        actions.add_css_class("layers-toolbar");
        for (label, action) in [
            ("+ Layer", LayerAction::Add),
            ("+ Group", LayerAction::AddGroup),
            ("Ungroup", LayerAction::Ungroup),
            ("Duplicate", LayerAction::Duplicate),
            ("Delete", LayerAction::Delete),
            ("Edit Text", LayerAction::EditText),
            ("Into Group", LayerAction::MoveIntoGroup),
            ("Out Group", LayerAction::MoveOutOfGroup),
            ("+ Mask", LayerAction::AddMask),
            (
                if snapshot.active_layer_mask_enabled {
                    "Mask Off"
                } else {
                    "Mask On"
                },
                LayerAction::ToggleMask,
            ),
            (
                if snapshot.active_edit_target_name == "Layer Mask" {
                    "Edit Layer"
                } else {
                    "Edit Mask"
                },
                LayerAction::ToggleMaskTarget,
            ),
            ("Up", LayerAction::MoveUp),
            ("Down", LayerAction::MoveDown),
        ] {
            let button = Button::with_label(label);
            button.add_css_class("tool-chip");
            button.add_css_class("layer-action-chip");
            match action {
                LayerAction::AddGroup => button.set_sensitive(
                    snapshot.can_create_group_from_active_layer && !snapshot.text.selected,
                ),
                LayerAction::Ungroup => button.set_sensitive(snapshot.can_ungroup_selected_group),
                LayerAction::Duplicate => button.set_sensitive(!snapshot.text.selected),
                LayerAction::EditText => {
                    button.set_sensitive(snapshot.text.selected && !snapshot.text.editing)
                }
                LayerAction::MoveIntoGroup => button.set_sensitive(
                    snapshot.can_move_active_layer_into_selected_group && !snapshot.text.selected,
                ),
                LayerAction::MoveOutOfGroup => button.set_sensitive(
                    snapshot.can_move_active_layer_out_of_group && !snapshot.text.selected,
                ),
                LayerAction::AddMask => {
                    button.set_sensitive(!snapshot.text.selected && !snapshot.active_layer_has_mask)
                }
                LayerAction::ToggleMask => {
                    button.set_sensitive(!snapshot.text.selected && snapshot.active_layer_has_mask)
                }
                LayerAction::ToggleMaskTarget => {
                    button.set_sensitive(!snapshot.text.selected && snapshot.active_layer_has_mask)
                }
                LayerAction::MoveUp | LayerAction::MoveDown => {
                    button.set_sensitive(!snapshot.text.selected)
                }
                _ => {}
            }
            let controller = self.controller.clone();
            button.connect_clicked(move |_| match action {
                LayerAction::Add => controller.borrow_mut().add_layer(),
                LayerAction::AddGroup => controller.borrow_mut().create_group_from_active_layer(),
                LayerAction::Ungroup => controller.borrow_mut().ungroup_selected_group(),
                LayerAction::Duplicate => controller.borrow_mut().duplicate_active_layer(),
                LayerAction::Delete => controller.borrow_mut().delete_active_layer(),
                LayerAction::EditText => controller.borrow_mut().begin_text_edit(),
                LayerAction::MoveIntoGroup => controller
                    .borrow_mut()
                    .move_active_layer_into_selected_group(),
                LayerAction::MoveOutOfGroup => {
                    controller.borrow_mut().move_active_layer_out_of_group()
                }
                LayerAction::AddMask => controller.borrow_mut().add_active_layer_mask(),
                LayerAction::ToggleMask => {
                    controller.borrow_mut().toggle_active_layer_mask_enabled()
                }
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
            row.append(&visibility);

            if layer.is_group {
                let target_strip = GtkBox::new(Orientation::Horizontal, 3);
                target_strip.add_css_class("layer-target-strip");
                let group_chip =
                    build_target_chip("G", "Select this group", layer.is_selected, true);
                if let Some(group_id) = layer.group_id {
                    let controller = self.controller.clone();
                    group_chip
                        .connect_clicked(move |_| controller.borrow_mut().select_group(group_id));
                }
                target_strip.append(&group_chip);
                row.append(&target_strip);

                let select = Button::with_label(&format!(
                    "{}  ({}%) [Group]",
                    layer.name, layer.opacity_percent
                ));
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

                if layer.is_text {
                    let text_target =
                        build_target_chip("T", "Select this text layer", layer.is_selected, true);
                    if let Some(layer_id) = layer.layer_id {
                        let controller = self.controller.clone();
                        text_target.connect_clicked(move |_| {
                            controller.borrow_mut().select_layer(layer_id)
                        });
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

                    let select = Button::with_label(&format!(
                        "{}  ({}%) [Text]",
                        layer.name, layer.opacity_percent
                    ));
                    select.add_css_class("layer-select-button");
                    if layer.is_selected {
                        select.add_css_class("layer-select-button-active");
                    }
                    if let Some(layer_id) = layer.layer_id {
                        let controller = self.controller.clone();
                        select.connect_clicked(move |_| {
                            controller.borrow_mut().select_layer(layer_id)
                        });
                    }
                    row.append(&select);
                } else {
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

                    let mask_suffix = if !layer.has_mask {
                        String::new()
                    } else if layer.mask_target_active {
                        if layer.mask_enabled {
                            "  [Mask Editing]".to_string()
                        } else {
                            "  [Mask Editing Off]".to_string()
                        }
                    } else if layer.mask_enabled {
                        "  [Mask]".to_string()
                    } else {
                        "  [Mask Off]".to_string()
                    };
                    let select = Button::with_label(&format!(
                        "{}  ({}%){}",
                        layer.name, layer.opacity_percent, mask_suffix
                    ));
                    select.add_css_class("layer-select-button");
                    if layer.is_selected {
                        select.add_css_class("layer-select-button-active");
                    }
                    if let Some(layer_id) = layer.layer_id {
                        let controller = self.controller.clone();
                        select.connect_clicked(move |_| {
                            controller.borrow_mut().select_layer(layer_id)
                        });
                    }
                    row.append(&select);
                }
            }

            self.layers_body.append(&row);
        }
    }

    pub(super) fn refresh_history_panel(&self, snapshot: &ShellSnapshot) {
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
