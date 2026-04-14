use super::*;
use crate::ui_templates::TitlebarTemplate;

pub(super) fn build_ui(
    application: &Application,
    shell_state: Rc<ShellUiState>,
    on_map: Option<StartupWindowHook>,
) {
    let header_bar = build_header_bar();

    let window = ApplicationWindow::builder()
        .application(application)
        .title(APP_NAME)
        .default_width(MAIN_WINDOW_DEFAULT_WIDTH)
        .default_height(MAIN_WINDOW_DEFAULT_HEIGHT)
        .build();
    window.add_css_class("app-window");
    window.set_titlebar(Some(&header_bar));

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.add_css_class("app-root");
    root.append(&menus::build_menu_bar(&window, shell_state.clone()));
    root.append(&shell_state.tool_options_bar);
    let workspace = build_workspace_body(&shell_state);
    root.append(&workspace);
    root.append(&shell_state.status_bar);

    window.set_child(Some(&root));
    shell_state.attach_window(window.clone());
    wire_window_shortcuts(&window, shell_state.clone());
    wire_window_close_request(&window, shell_state.clone());
    wire_window_map(&window, shell_state.clone(), on_map);
    window.present();

    shell_state.refresh();
    glib::timeout_add_local(Duration::from_millis(33), move || {
        shell_state.refresh();
        ControlFlow::Continue
    });
}

fn build_header_bar() -> HeaderBar {
    load_titlebar_template()
        .map(build_header_bar_from_template)
        .unwrap_or_else(|error| {
            tracing::error!(%error, "failed to load titlebar template");
            build_header_bar_fallback()
        })
}

fn build_header_bar_from_template(template: TitlebarTemplate) -> HeaderBar {
    let spacer = build_header_bar_spacer();
    template.root.set_title_widget(Some(&spacer));
    template.app_name_label.set_visible(true);
    template.workspace_button.set_visible(false);
    set_image_resource_or_fallback(
        &template.logo_image,
        logo_icon_resource_path(true),
        APP_NAME,
        16,
    );
    template.search_icon.add_css_class("remix-icon");
    ui_support::set_remix_icon_or_fallback(&template.search_icon, "search-line.svg", "Search", 12);
    template.search_button.set_tooltip_text(Some("Search"));
    template.root
}

fn build_header_bar_spacer() -> GtkBox {
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    spacer
}

fn wire_window_map(
    window: &ApplicationWindow,
    shell_state: Rc<ShellUiState>,
    on_map: Option<StartupWindowHook>,
) {
    let on_map = Rc::new(RefCell::new(on_map));
    window.connect_map(move |window| {
        shell_state.focus_canvas();
        if let Some(on_map) = on_map.borrow_mut().take() {
            on_map(window);
        }
    });
}

fn build_header_bar_fallback() -> HeaderBar {
    let header = HeaderBar::new();
    header.add_css_class("titlebar");
    header.set_show_title_buttons(true);
    let spacer = build_header_bar_spacer();
    header.set_title_widget(Some(&spacer));

    let title_row = GtkBox::new(Orientation::Horizontal, 6);
    title_row.add_css_class("app-brand");
    let title_icon = build_logo_icon(true, APP_NAME, 16);
    title_icon.add_css_class("titlebar-icon");
    title_row.append(&title_icon);
    let title_label = Label::new(Some(APP_NAME));
    title_label.add_css_class("titlebar-app-name");
    title_row.append(&title_label);
    header.pack_start(&title_row);

    let actions = GtkBox::new(Orientation::Horizontal, 6);
    actions.add_css_class("titlebar-actions");
    let search = build_icon_only_button("search-line.svg", "Search", "chrome-button", 12);
    search.add_css_class("chrome-icon-button");
    actions.append(&search);
    header.pack_end(&actions);
    header
}

fn build_workspace_body(shell_state: &Rc<ShellUiState>) -> GtkBox {
    let outer = GtkBox::new(Orientation::Horizontal, 0);
    outer.add_css_class("workspace-body");
    outer.append(&shell_state.tool_rail);

    let inner = Paned::new(Orientation::Horizontal);
    inner.set_wide_handle(true);
    inner.set_focusable(false);
    inner.set_start_child(Some(&shell_chrome::build_document_region(shell_state)));
    inner.set_end_child(Some(&shell_chrome::build_right_sidebar(shell_state)));
    inner.set_position(1120);
    inner.set_hexpand(true);
    inner.set_vexpand(true);

    outer.append(&inner);
    outer
}
