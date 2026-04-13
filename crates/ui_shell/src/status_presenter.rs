use super::*;

pub(super) fn shell_status_hint(snapshot: &ShellSnapshot) -> String {
    let tool_hint = format!(
        "Tool: {} ({})",
        snapshot.active_tool_name,
        shell_tool_shortcut(snapshot.active_tool)
    );
    if snapshot.text.editing {
        return format!(
            "{} | Enter apply | Esc cancel | Font {} {}px | Align {:?}",
            tool_hint,
            snapshot.text.font_family,
            snapshot.text.font_size_px,
            snapshot.text.alignment,
        );
    }
    if matches!(snapshot.active_tool, ShellToolKind::Text) {
        return format!(
            "{} | Click canvas to place text | Enter edit dialog",
            tool_hint
        );
    }
    if matches!(
        snapshot.active_tool,
        ShellToolKind::Brush | ShellToolKind::Eraser
    ) && (snapshot.pressure_size_enabled || snapshot.pressure_opacity_enabled)
    {
        return format!(
            "{} | Radius {} | Hardness {}% | Spacing {} | Flow {}% | Pressure {}{}",
            tool_hint,
            snapshot.brush_radius,
            snapshot.brush_hardness_percent,
            snapshot.brush_spacing,
            snapshot.brush_flow_percent,
            if snapshot.pressure_size_enabled {
                "size"
            } else {
                ""
            },
            if snapshot.pressure_opacity_enabled {
                if snapshot.pressure_size_enabled {
                    " + opacity"
                } else {
                    "opacity"
                }
            } else {
                ""
            }
        );
    }
    if matches!(
        snapshot.active_tool,
        ShellToolKind::Brush | ShellToolKind::Eraser
    ) {
        return format!(
            "{} | Radius {} | Hardness {}% | Spacing {} | Flow {}%",
            tool_hint,
            snapshot.brush_radius,
            snapshot.brush_hardness_percent,
            snapshot.brush_spacing,
            snapshot.brush_flow_percent,
        );
    }
    if snapshot.active_edit_target_name == "Layer Mask" {
        return format!(
            "{} | Editing mask | Brush hides | Eraser reveals",
            tool_hint
        );
    }
    if snapshot.transform_active {
        if snapshot.snapping_enabled {
            return format!(
                "{} | Enter commit | Esc cancel | Snap {} | Hold Shift bypass",
                tool_hint,
                if snapshot.snapping_temporarily_bypassed {
                    "bypassed"
                } else {
                    "on"
                }
            );
        }
        return format!("{} | Enter commit | Esc cancel", tool_hint);
    }
    if matches!(snapshot.active_tool, ShellToolKind::Move) && snapshot.snapping_enabled {
        return format!(
            "{} | Snap {} | Hold Shift bypass",
            tool_hint,
            if snapshot.snapping_temporarily_bypassed {
                "bypassed"
            } else {
                "on"
            }
        );
    }
    if snapshot.selection_rect.is_some() {
        return format!("{} | Ctrl+D clear | Ctrl+I invert", tool_hint);
    }
    tool_hint
}

pub(super) fn shell_notice_text(snapshot: &ShellSnapshot) -> String {
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

pub(super) fn format_shell_alert_secondary_text(alert: &ShellAlert) -> Option<String> {
    alert.secondary_text.clone()
}

pub(super) fn format_import_report_details(report: &ShellImportReport) -> String {
    let mut details = report.summary.clone();
    if !report.diagnostics.is_empty() {
        details.push_str("\n\nDetails:");
        for diagnostic in &report.diagnostics {
            details.push_str("\n- ");
            details.push_str(&diagnostic.severity_label);
            details.push_str(": ");
            details.push_str(&diagnostic.message);
        }
    }
    details
}

pub(super) fn status_notice_class(snapshot: &ShellSnapshot) -> &'static str {
    if snapshot.file_job_active || snapshot.autosave_job_active || snapshot.filter_job_active {
        return "status-notice-busy";
    }

    if let Some(alert) = snapshot.latest_alert.as_ref() {
        return match alert.tone {
            ShellAlertTone::Info => "status-notice-success",
            ShellAlertTone::Warning => "status-notice-warning",
            ShellAlertTone::Error => "status-notice-error",
        };
    }

    if snapshot.recovery_offer_pending || snapshot.dirty {
        "status-notice-warning"
    } else {
        "status-notice-success"
    }
}

pub(super) fn apply_status_notice_style(label: &Label, snapshot: &ShellSnapshot) {
    for class_name in [
        "status-notice-busy",
        "status-notice-success",
        "status-notice-error",
        "status-notice-warning",
    ] {
        label.remove_css_class(class_name);
    }

    label.add_css_class(status_notice_class(snapshot));
}
