use anyhow::{Result, anyhow};
use glib::Object;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Builder, Button, HeaderBar, Image, Label};

#[derive(Debug, Clone, Copy)]
enum UiTemplate {
    InfoDialog,
    PanelGroup,
    Titlebar,
    ToolOptionsBar,
    DocumentTabs,
    StatusBar,
}

impl UiTemplate {
    const fn path(self) -> &'static str {
        match self {
            Self::InfoDialog => "src/ui/dialogs/info-dialog.ui",
            Self::PanelGroup => "src/ui/fragments/panel-group.ui",
            Self::Titlebar => "src/ui/fragments/titlebar.ui",
            Self::ToolOptionsBar => "src/ui/fragments/tool-options-bar.ui",
            Self::DocumentTabs => "src/ui/fragments/document-tabs.ui",
            Self::StatusBar => "src/ui/fragments/status-bar.ui",
        }
    }

    const fn markup(self) -> &'static str {
        match self {
            Self::InfoDialog => include_str!("ui/dialogs/info-dialog.ui"),
            Self::PanelGroup => include_str!("ui/fragments/panel-group.ui"),
            Self::Titlebar => include_str!("ui/fragments/titlebar.ui"),
            Self::ToolOptionsBar => include_str!("ui/fragments/tool-options-bar.ui"),
            Self::DocumentTabs => include_str!("ui/fragments/document-tabs.ui"),
            Self::StatusBar => include_str!("ui/fragments/status-bar.ui"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct InfoDialogTemplate {
    pub(crate) root: GtkBox,
    pub(crate) title_label: Label,
    pub(crate) body_label: Label,
    pub(crate) secondary_label: Label,
    pub(crate) close_button: Button,
}

#[derive(Debug)]
pub(crate) struct PanelGroupTemplate {
    pub(crate) root: GtkBox,
    pub(crate) header: GtkBox,
    pub(crate) body: GtkBox,
    pub(crate) tab_buttons: [Button; 3],
}

#[derive(Debug)]
pub(crate) struct TitlebarTemplate {
    pub(crate) root: HeaderBar,
    pub(crate) logo_image: Image,
    pub(crate) app_name_label: Label,
    pub(crate) workspace_button: Button,
    pub(crate) search_button: Button,
    pub(crate) search_icon: Image,
}

#[derive(Debug)]
pub(crate) struct ToolOptionsBarTemplate {
    pub(crate) root: GtkBox,
    pub(crate) tool_icon: Image,
    pub(crate) tool_label: Label,
    pub(crate) option_chips: [Button; 6],
}

#[derive(Debug)]
pub(crate) struct DocumentTabsTemplate {
    pub(crate) root: GtkBox,
    pub(crate) active_tab_button: Button,
    pub(crate) active_tab_label: Label,
    pub(crate) add_tab_button: Button,
}

#[derive(Debug)]
pub(crate) struct StatusBarTemplate {
    pub(crate) root: GtkBox,
    pub(crate) doc_label: Label,
    pub(crate) zoom_label: Label,
    pub(crate) cursor_label: Label,
    pub(crate) notice_label: Label,
    pub(crate) mode_label: Label,
}

fn load_builder(template: UiTemplate) -> Builder {
    Builder::from_string(template.markup())
}

fn required_object<T: IsA<Object>>(builder: &Builder, template: UiTemplate, id: &str) -> Result<T> {
    builder.object(id).ok_or_else(|| {
        anyhow!(
            "failed to load `{}`: missing required object `{}`",
            template.path(),
            id
        )
    })
}

pub(crate) fn load_info_dialog_template() -> Result<InfoDialogTemplate> {
    let builder = load_builder(UiTemplate::InfoDialog);

    Ok(InfoDialogTemplate {
        root: required_object(&builder, UiTemplate::InfoDialog, "info_dialog_content")?,
        title_label: required_object(&builder, UiTemplate::InfoDialog, "info_dialog_title_label")?,
        body_label: required_object(&builder, UiTemplate::InfoDialog, "info_dialog_body_label")?,
        secondary_label: required_object(
            &builder,
            UiTemplate::InfoDialog,
            "info_dialog_secondary_label",
        )?,
        close_button: required_object(
            &builder,
            UiTemplate::InfoDialog,
            "info_dialog_close_button",
        )?,
    })
}

pub(crate) fn load_panel_group_template() -> Result<PanelGroupTemplate> {
    let builder = load_builder(UiTemplate::PanelGroup);

    Ok(PanelGroupTemplate {
        root: required_object(&builder, UiTemplate::PanelGroup, "panel_group_root")?,
        header: required_object(&builder, UiTemplate::PanelGroup, "panel_group_header")?,
        body: required_object(&builder, UiTemplate::PanelGroup, "panel_group_body")?,
        tab_buttons: [
            required_object(&builder, UiTemplate::PanelGroup, "panel_group_tab_1")?,
            required_object(&builder, UiTemplate::PanelGroup, "panel_group_tab_2")?,
            required_object(&builder, UiTemplate::PanelGroup, "panel_group_tab_3")?,
        ],
    })
}

pub(crate) fn load_titlebar_template() -> Result<TitlebarTemplate> {
    let builder = load_builder(UiTemplate::Titlebar);

    Ok(TitlebarTemplate {
        root: required_object(&builder, UiTemplate::Titlebar, "shell_titlebar_root")?,
        logo_image: required_object(&builder, UiTemplate::Titlebar, "shell_titlebar_logo")?,
        app_name_label: required_object(&builder, UiTemplate::Titlebar, "shell_titlebar_app_name")?,
        workspace_button: required_object(
            &builder,
            UiTemplate::Titlebar,
            "shell_titlebar_workspace_button",
        )?,
        search_button: required_object(
            &builder,
            UiTemplate::Titlebar,
            "shell_titlebar_search_button",
        )?,
        search_icon: required_object(&builder, UiTemplate::Titlebar, "shell_titlebar_search_icon")?,
    })
}

pub(crate) fn load_tool_options_bar_template() -> Result<ToolOptionsBarTemplate> {
    let builder = load_builder(UiTemplate::ToolOptionsBar);

    Ok(ToolOptionsBarTemplate {
        root: required_object(
            &builder,
            UiTemplate::ToolOptionsBar,
            "tool_options_bar_root",
        )?,
        tool_icon: required_object(&builder, UiTemplate::ToolOptionsBar, "tool_options_icon")?,
        tool_label: required_object(&builder, UiTemplate::ToolOptionsBar, "tool_options_label")?,
        option_chips: [
            required_object(&builder, UiTemplate::ToolOptionsBar, "tool_option_chip_1")?,
            required_object(&builder, UiTemplate::ToolOptionsBar, "tool_option_chip_2")?,
            required_object(&builder, UiTemplate::ToolOptionsBar, "tool_option_chip_3")?,
            required_object(&builder, UiTemplate::ToolOptionsBar, "tool_option_chip_4")?,
            required_object(&builder, UiTemplate::ToolOptionsBar, "tool_option_chip_5")?,
            required_object(&builder, UiTemplate::ToolOptionsBar, "tool_option_chip_6")?,
        ],
    })
}

pub(crate) fn load_document_tabs_template() -> Result<DocumentTabsTemplate> {
    let builder = load_builder(UiTemplate::DocumentTabs);

    Ok(DocumentTabsTemplate {
        root: required_object(&builder, UiTemplate::DocumentTabs, "document_tabs_root")?,
        active_tab_button: required_object(
            &builder,
            UiTemplate::DocumentTabs,
            "document_tab_active_button",
        )?,
        active_tab_label: required_object(
            &builder,
            UiTemplate::DocumentTabs,
            "document_tab_title",
        )?,
        add_tab_button: required_object(
            &builder,
            UiTemplate::DocumentTabs,
            "document_tab_add_button",
        )?,
    })
}

pub(crate) fn load_status_bar_template() -> Result<StatusBarTemplate> {
    let builder = load_builder(UiTemplate::StatusBar);

    Ok(StatusBarTemplate {
        root: required_object(&builder, UiTemplate::StatusBar, "status_bar_root")?,
        doc_label: required_object(&builder, UiTemplate::StatusBar, "status_doc_label")?,
        zoom_label: required_object(&builder, UiTemplate::StatusBar, "status_zoom_label")?,
        cursor_label: required_object(&builder, UiTemplate::StatusBar, "status_cursor_label")?,
        notice_label: required_object(&builder, UiTemplate::StatusBar, "status_notice_label")?,
        mode_label: required_object(&builder, UiTemplate::StatusBar, "status_mode_label")?,
    })
}

pub(crate) fn build_panel_group_shell(
    shell_name: &str,
    tabs: &[&str],
    body_spacing: i32,
    body_vexpand: bool,
) -> Result<(GtkBox, GtkBox)> {
    let template = load_panel_group_template()?;
    template
        .root
        .set_widget_name(&format!("{shell_name}-panel"));
    template
        .header
        .set_widget_name(&format!("{shell_name}-panel-header"));
    template
        .body
        .set_widget_name(&format!("{shell_name}-panel-body"));
    template.body.set_spacing(body_spacing);
    template.body.set_vexpand(body_vexpand);

    for (index, button) in template.tab_buttons.iter().enumerate() {
        let active = index == 0 && tabs.get(index).is_some();
        if let Some(tab) = tabs.get(index) {
            button.set_label(tab);
            button.set_visible(true);
            button.set_widget_name(&format!("{shell_name}-panel-tab-{}", index + 1));
        } else {
            button.set_visible(false);
        }

        if active {
            button.add_css_class("panel-tab-active");
        } else {
            button.remove_css_class("panel-tab-active");
        }
    }

    Ok((template.root, template.body))
}

#[cfg(test)]
mod tests {
    use super::{
        UiTemplate, build_panel_group_shell, load_builder, load_document_tabs_template,
        load_info_dialog_template, load_panel_group_template, load_status_bar_template,
        load_titlebar_template, load_tool_options_bar_template, required_object,
    };
    use gtk4::prelude::*;
    use gtk4::{Button, Label};
    use std::sync::OnceLock;

    fn gtk_available() -> bool {
        static GTK_AVAILABLE: OnceLock<bool> = OnceLock::new();
        *GTK_AVAILABLE.get_or_init(|| gtk4::init().is_ok())
    }

    #[test]
    fn info_dialog_template_source_contains_required_ids_and_classes() {
        let markup = UiTemplate::InfoDialog.markup();

        assert!(markup.contains("info_dialog_content"));
        assert!(markup.contains("info_dialog_title_label"));
        assert!(markup.contains("info_dialog_body_label"));
        assert!(markup.contains("info_dialog_secondary_label"));
        assert!(markup.contains("info_dialog_close_button"));
        assert!(markup.contains("template-dialog-content"));
        assert!(markup.contains("template-dialog-title"));
        assert!(markup.contains("template-dialog-body"));
        assert!(markup.contains("template-dialog-secondary"));
        assert!(markup.contains("tooltip-text\">Close dialog"));
    }

    #[test]
    fn panel_group_template_source_contains_required_ids_and_classes() {
        let markup = UiTemplate::PanelGroup.markup();

        assert!(markup.contains("panel_group_root"));
        assert!(markup.contains("panel_group_header"));
        assert!(markup.contains("panel_group_tab_1"));
        assert!(markup.contains("panel_group_tab_2"));
        assert!(markup.contains("panel_group_tab_3"));
        assert!(markup.contains("panel_group_body"));
        assert!(markup.contains("panel-group"));
        assert!(markup.contains("panel-group-header"));
        assert!(markup.contains("panel-group-body"));
        assert!(markup.contains("panel-tab-active"));
    }

    #[test]
    fn titlebar_template_source_contains_required_ids_and_classes() {
        let markup = UiTemplate::Titlebar.markup();

        assert!(markup.contains("shell_titlebar_root"));
        assert!(markup.contains("shell_titlebar_logo"));
        assert!(markup.contains("shell_titlebar_app_name"));
        assert!(markup.contains("shell_titlebar_workspace_button"));
        assert!(markup.contains("shell_titlebar_search_button"));
        assert!(markup.contains("shell_titlebar_search_icon"));
        assert!(markup.contains("titlebar"));
        assert!(markup.contains("app-brand"));
        assert!(markup.contains("chrome-icon-button"));
    }

    #[test]
    fn tool_options_template_source_contains_required_ids_and_classes() {
        let markup = UiTemplate::ToolOptionsBar.markup();

        assert!(markup.contains("tool_options_bar_root"));
        assert!(markup.contains("tool_options_icon"));
        assert!(markup.contains("tool_options_label"));
        assert!(markup.contains("tool_option_chip_1"));
        assert!(markup.contains("tool_option_chip_6"));
        assert!(markup.contains("tool-options-bar"));
        assert!(markup.contains("tool-options-label"));
        assert!(markup.contains("tool-option-chip"));
    }

    #[test]
    fn document_tabs_template_source_contains_required_ids_and_classes() {
        let markup = UiTemplate::DocumentTabs.markup();

        assert!(markup.contains("document_tabs_root"));
        assert!(markup.contains("document_tab_active_button"));
        assert!(markup.contains("document_tab_title"));
        assert!(markup.contains("document_tab_add_button"));
        assert!(markup.contains("document-tabs"));
        assert!(markup.contains("document-tab-active"));
        assert!(markup.contains("document-tab-title"));
    }

    #[test]
    fn status_bar_template_source_contains_required_ids_and_classes() {
        let markup = UiTemplate::StatusBar.markup();

        assert!(markup.contains("status_bar_root"));
        assert!(markup.contains("status_doc_label"));
        assert!(markup.contains("status_cursor_label"));
        assert!(markup.contains("status_notice_label"));
        assert!(markup.contains("status_zoom_label"));
        assert!(markup.contains("status_mode_label"));
        assert!(markup.contains("status-bar"));
        assert!(markup.contains("status-left"));
        assert!(markup.contains("status-notice"));
    }

    #[test]
    #[ignore = "GTK builder tests must run on the main thread"]
    fn info_dialog_template_loads_expected_structure_when_gtk_is_available() {
        if !gtk_available() {
            return;
        }

        let template = load_info_dialog_template().expect("info dialog template should load");

        assert_eq!(template.root.widget_name(), "info-dialog-content");
        assert!(template.root.has_css_class("template-dialog-content"));
        assert_eq!(template.title_label.widget_name(), "info-dialog-title");
        assert!(template.title_label.has_css_class("template-dialog-title"));
        assert!(template.body_label.has_css_class("template-dialog-body"));
        assert!(
            template
                .secondary_label
                .has_css_class("template-dialog-secondary")
        );
        assert!(!template.secondary_label.is_visible());
        assert_eq!(template.close_button.label().as_deref(), Some("Close"));
        assert_eq!(
            template.close_button.tooltip_text().as_deref(),
            Some("Close dialog")
        );
    }

    #[test]
    #[ignore = "GTK builder tests must run on the main thread"]
    fn panel_group_template_loads_expected_structure_when_gtk_is_available() {
        if !gtk_available() {
            return;
        }

        let template = load_panel_group_template().expect("panel group template should load");

        assert_eq!(template.root.widget_name(), "panel-group");
        assert!(template.root.has_css_class("panel-group"));
        assert_eq!(template.body.widget_name(), "panel-group-body");
        assert!(template.body.has_css_class("panel-group-body"));
        assert_eq!(template.tab_buttons[0].label().as_deref(), Some("Primary"));
    }

    #[test]
    #[ignore = "GTK builder tests must run on the main thread"]
    fn missing_template_object_reports_template_path_and_id_when_gtk_is_available() {
        if !gtk_available() {
            return;
        }

        let builder = load_builder(UiTemplate::InfoDialog);
        let error = required_object::<Label>(&builder, UiTemplate::InfoDialog, "missing_label")
            .expect_err("missing object should fail");

        let message = error.to_string();
        assert!(message.contains("src/ui/dialogs/info-dialog.ui"));
        assert!(message.contains("missing_label"));
    }

    #[test]
    #[ignore = "GTK builder tests must run on the main thread"]
    fn panel_group_shell_configures_tabs_when_gtk_is_available() {
        if !gtk_available() {
            return;
        }

        let (_root, body) = build_panel_group_shell("layers", &["Layers", "Channels"], 7, true)
            .expect("panel group shell should build");
        assert_eq!(body.widget_name(), "layers-panel-body");
        assert_eq!(body.spacing(), 7);
        assert!(body.is_vexpand_set());

        let builder = load_builder(UiTemplate::PanelGroup);
        let tab_button =
            required_object::<Button>(&builder, UiTemplate::PanelGroup, "panel_group_tab_1")
                .expect("panel group tab button should load");

        assert_eq!(tab_button.label().as_deref(), Some("Primary"));
        assert!(tab_button.has_css_class("panel-tab"));
        assert!(tab_button.has_css_class("panel-tab-active"));
    }

    #[test]
    #[ignore = "GTK builder tests must run on the main thread"]
    fn shell_chrome_templates_load_expected_structure_when_gtk_is_available() {
        if !gtk_available() {
            return;
        }

        let titlebar = load_titlebar_template().expect("titlebar template should load");
        assert!(titlebar.root.has_css_class("titlebar"));
        assert_eq!(titlebar.app_name_label.label(), "PhotoTux");

        let tool_options =
            load_tool_options_bar_template().expect("tool options template should load");
        assert!(tool_options.root.has_css_class("tool-options-bar"));
        assert_eq!(tool_options.option_chips.len(), 6);

        let document_tabs =
            load_document_tabs_template().expect("document tabs template should load");
        assert!(document_tabs.root.has_css_class("document-tabs"));
        assert_eq!(document_tabs.add_tab_button.label().as_deref(), Some("+"));

        let status_bar = load_status_bar_template().expect("status bar template should load");
        assert!(status_bar.root.has_css_class("status-bar"));
        assert_eq!(status_bar.notice_label.label(), "Ready");
    }
}
