use super::*;
use gtk4::{Dialog, FileChooserAction, FileChooserNative, FileFilter};
use std::path::{Path, PathBuf};

pub(super) fn choose_open_project(parent: &ApplicationWindow, shell_state: Rc<ShellUiState>) {
    choose_path(
        parent,
        "Open Project",
        FileChooserAction::Open,
        "Open",
        None,
        &[build_extension_filter("PhotoTux Project", &["*.ptx"])],
        move |path| {
            shell_state.request_document_replacement(PendingDocumentAction::OpenProject(path))
        },
    );
}

pub(super) fn choose_import_image(parent: &ApplicationWindow, shell_state: Rc<ShellUiState>) {
    choose_path(
        parent,
        "Import Image Or PSD",
        FileChooserAction::Open,
        "Import",
        None,
        &[build_extension_filter(
            "Supported Imports",
            &["*.png", "*.jpg", "*.jpeg", "*.webp", "*.psd"],
        )],
        move |path| {
            shell_state.request_document_replacement(PendingDocumentAction::ImportImage(path))
        },
    );
}

pub(super) fn choose_export_path(
    parent: &ApplicationWindow,
    controller: Rc<RefCell<dyn ShellController>>,
    extension: &'static str,
) {
    let suggested_name =
        suggested_export_name(&controller.borrow().snapshot().document_title, extension);
    choose_path(
        parent,
        "Export Image",
        FileChooserAction::Save,
        "Export",
        Some(&suggested_name),
        &[build_extension_filter(
            &format!("{}.{}", extension.to_ascii_uppercase(), extension),
            &[&format!("*.{}", extension)],
        )],
        move |path| {
            controller
                .borrow_mut()
                .export_document(ensure_extension(&path, extension))
        },
    );
}

pub(super) fn choose_save_project_path(
    parent: &ApplicationWindow,
    controller: Rc<RefCell<dyn ShellController>>,
) {
    choose_save_project_path_with_callback(parent, controller, None);
}

pub(super) fn choose_save_project_path_with_callback(
    parent: &ApplicationWindow,
    controller: Rc<RefCell<dyn ShellController>>,
    on_requested: Option<Rc<dyn Fn()>>,
) {
    let snapshot = controller.borrow().snapshot();
    let suggested_name = snapshot
        .project_path
        .as_ref()
        .and_then(|path| path.file_name().and_then(|name| name.to_str()))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| suggested_export_name(&snapshot.document_title, "ptx"));

    choose_path(
        parent,
        "Save Project As",
        FileChooserAction::Save,
        "Save",
        Some(&suggested_name),
        &[build_extension_filter("PhotoTux Project", &["*.ptx"])],
        move |path| {
            controller
                .borrow_mut()
                .save_document_as(ensure_extension(&path, "ptx"));
            if let Some(callback) = on_requested.as_ref() {
                callback();
            }
        },
    );
}

pub(super) fn show_info_dialog(
    parent: &ApplicationWindow,
    title: &str,
    text: &str,
    secondary_text: Option<&str>,
) {
    let template = match load_info_dialog_template() {
        Ok(template) => template,
        Err(error) => {
            tracing::error!(%error, "failed to load info dialog template");
            let dialog = MessageDialog::builder()
                .transient_for(parent)
                .modal(true)
                .message_type(MessageType::Error)
                .buttons(ButtonsType::Close)
                .text("Failed to load dialog UI")
                .secondary_text(format!(
                    "{} could not be shown because its UI template failed to load: {}",
                    title, error
                ))
                .build();
            dialog.set_title(Some(title));
            dialog.connect_response(|dialog, _| dialog.destroy());
            dialog.show();
            return;
        }
    };

    template.title_label.set_label(title);
    template.body_label.set_label(text);
    let secondary_text = secondary_text.unwrap_or("");
    template.secondary_label.set_label(secondary_text);
    template
        .secondary_label
        .set_visible(!secondary_text.is_empty());

    let dialog = Dialog::builder()
        .transient_for(parent)
        .modal(true)
        .resizable(false)
        .title(title)
        .build();
    dialog.content_area().append(&template.root);
    {
        let dialog = dialog.clone();
        template
            .close_button
            .connect_clicked(move |_| dialog.destroy());
    }
    dialog.connect_close_request(|dialog| {
        dialog.destroy();
        glib::Propagation::Stop
    });
    dialog.present();
}

fn build_extension_filter(name: &str, patterns: &[&str]) -> FileFilter {
    let filter = FileFilter::new();
    filter.set_name(Some(name));
    for pattern in patterns {
        filter.add_pattern(pattern);
    }
    filter
}

fn ensure_extension(path: &Path, extension: &str) -> PathBuf {
    match path.extension().and_then(|value| value.to_str()) {
        Some(existing) if existing.eq_ignore_ascii_case(extension) => path.to_path_buf(),
        _ => path.with_extension(extension),
    }
}

fn suggested_export_name(document_title: &str, extension: &str) -> String {
    let stem = document_title
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(document_title);
    format!("{}.{}", stem, extension)
}

fn choose_path<F>(
    parent: &ApplicationWindow,
    title: &str,
    action: FileChooserAction,
    accept_label: &str,
    suggested_name: Option<&str>,
    filters: &[FileFilter],
    on_accept: F,
) where
    F: Fn(PathBuf) + 'static,
{
    let dialog = FileChooserNative::new(
        Some(title),
        Some(parent),
        action,
        Some(accept_label),
        Some("Cancel"),
    );
    dialog.set_modal(true);
    if let Some(name) = suggested_name {
        dialog.set_current_name(name);
    }
    for filter in filters {
        dialog.add_filter(filter);
    }

    let on_accept: Rc<dyn Fn(PathBuf)> = Rc::new(on_accept);
    let parent = parent.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept
            && let Some(path) = dialog.file().and_then(|file| file.path())
        {
            if action == FileChooserAction::Save && path.exists() {
                confirm_overwrite(&parent, path, on_accept.clone());
            } else {
                on_accept(path);
            }
        }
        dialog.destroy();
    });

    dialog.show();
}

fn confirm_overwrite(parent: &ApplicationWindow, path: PathBuf, on_accept: Rc<dyn Fn(PathBuf)>) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(MessageType::Question)
        .buttons(ButtonsType::None)
        .text("Replace existing file?")
        .secondary_text(format!(
            "{} already exists. Do you want to replace it?",
            path.display()
        ))
        .build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Replace", ResponseType::Accept);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            on_accept(path.clone());
        }
        dialog.destroy();
    });

    dialog.show();
}
