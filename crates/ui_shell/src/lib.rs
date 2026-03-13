#![allow(missing_docs)]

//! Slint-based application shell scaffolding for PhotoTux.

pub mod theme;

use crate::theme::{DARK_PRO, SlintTheme, ThemeMappingError};
use i_slint_backend_winit::WinitWindowAccessor;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, SharedString};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::info;

slint::include_modules!();

const INTERACTIVE_STATE_APPLY_INTERVAL: Duration = Duration::from_millis(33);

/// Result type used by the UI shell.
pub type Result<T> = std::result::Result<T, UiShellError>;

/// Errors that can occur while launching the UI shell.
#[derive(Debug, Error)]
pub enum UiShellError {
    /// Slint returned a platform or event-loop error.
    #[error(transparent)]
    SlintPlatform(#[from] slint::PlatformError),
    /// Theme token conversion failed.
    #[error(transparent)]
    ThemeMapping(#[from] ThemeMappingError),
}

/// Mutable shell text state controlled by the application layer.
#[derive(Clone)]
pub struct UiShellState {
    /// Active document title shown in the shell.
    pub document_title: String,
    /// Whether the current document has unsaved changes.
    pub document_dirty: bool,
    /// Active tool label for the options strip.
    pub active_tool_label: String,
    /// Status line text.
    pub status_message: String,
    /// Active short tool key used for shell selection state.
    pub active_tool_key: String,
    /// Tool preset label.
    pub tool_preset_label: String,
    /// Primary tool option label.
    pub tool_option_primary_label: String,
    /// Secondary tool option label.
    pub tool_option_secondary_label: String,
    /// Tertiary tool option label.
    pub tool_option_tertiary_label: String,
    /// Quaternary tool option label.
    pub tool_option_quaternary_label: String,
    /// Formatted size label.
    pub tool_size_label: String,
    /// Formatted hardness label.
    pub tool_hardness_label: String,
    /// Formatted opacity label.
    pub tool_opacity_label: String,
    /// Formatted flow label.
    pub tool_flow_label: String,
    /// Formatted zoom label.
    pub zoom_label: String,
    /// Formatted canvas size label.
    pub canvas_size_label: String,
    /// Canvas width in pixels.
    pub canvas_width_px: i32,
    /// Canvas height in pixels.
    pub canvas_height_px: i32,
    /// Formatted cursor position label.
    pub cursor_position_label: String,
    /// Formatted selection bounds label.
    pub selection_bounds_label: String,
    /// Foreground color readout.
    pub foreground_color_label: String,
    /// Background color readout.
    pub background_color_label: String,
    /// Active layer name.
    pub active_layer_name: String,
    /// Active layer blend mode label.
    pub layer_blend_mode_label: String,
    /// Active layer opacity label.
    pub layer_opacity_label: String,
    /// Active layer visibility label.
    pub layer_visibility_label: String,
    /// Layer stack count label.
    pub layer_count_label: String,
    /// Latest history row.
    pub history_entry_primary: String,
    /// Second history row.
    pub history_entry_secondary: String,
    /// Third history row.
    pub history_entry_tertiary: String,
    /// Fourth history row.
    pub history_entry_quaternary: String,
    /// Fifth history row.
    pub history_entry_quinary: String,
    /// Raster preview shown in the canvas area.
    pub canvas_preview: Image,
}

impl Default for UiShellState {
    fn default() -> Self {
        Self {
            document_title: "untitled.ptx".to_string(),
            document_dirty: false,
            active_tool_label: "Brush Tool".to_string(),
            status_message: "Shell ready".to_string(),
            active_tool_key: "Brush".to_string(),
            tool_preset_label: "Round Brush".to_string(),
            tool_option_primary_label: "Size".to_string(),
            tool_option_secondary_label: "Hardness".to_string(),
            tool_option_tertiary_label: "Opacity".to_string(),
            tool_option_quaternary_label: "Flow".to_string(),
            tool_size_label: "24 px".to_string(),
            tool_hardness_label: "85%".to_string(),
            tool_opacity_label: "100%".to_string(),
            tool_flow_label: "100%".to_string(),
            zoom_label: "100%".to_string(),
            canvas_size_label: "1920x1080 px".to_string(),
            canvas_width_px: 1920,
            canvas_height_px: 1080,
            cursor_position_label: "Cursor -, -".to_string(),
            selection_bounds_label: "No Selection".to_string(),
            foreground_color_label: "#4F8CFF".to_string(),
            background_color_label: "#101216".to_string(),
            active_layer_name: "Surface".to_string(),
            layer_blend_mode_label: "Normal".to_string(),
            layer_opacity_label: "100%".to_string(),
            layer_visibility_label: "Visible".to_string(),
            layer_count_label: "1 Layer".to_string(),
            history_entry_primary: "No actions yet".to_string(),
            history_entry_secondary: "Brush strokes and edits appear here".to_string(),
            history_entry_tertiary: "Undo and redo update this list".to_string(),
            history_entry_quaternary: "Save and export are shown in status".to_string(),
            history_entry_quinary: "Keyboard shortcuts stay available".to_string(),
            canvas_preview: default_canvas_preview(),
        }
    }
}

/// Delegate for shell command and tool events.
pub trait UiShellDelegate {
    /// Return the initial text state for the shell.
    fn initial_state(&self) -> UiShellState {
        UiShellState::default()
    }

    /// Handle a menu command and return the updated shell state.
    fn on_command(&mut self, command: &str, current_state: &UiShellState) -> UiShellState;

    /// Handle a tool selection and return the updated shell state.
    fn on_tool(&mut self, tool: &str, current_state: &UiShellState) -> UiShellState;

    /// Handle an adjustment to the active tool options.
    fn on_tool_option_adjusted(
        &mut self,
        option: &str,
        delta: i32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let _ = (option, delta);
        current_state.clone()
    }

    /// Reset the current tool options.
    fn on_tool_options_reset(&mut self, current_state: &UiShellState) -> UiShellState {
        current_state.clone()
    }

    /// Handle a pointer press inside the canvas interaction region.
    fn on_canvas_pressed(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let _ = (normalized_x, normalized_y);
        current_state.clone()
    }

    /// Handle a pointer drag inside the canvas interaction region.
    fn on_canvas_dragged(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let _ = (normalized_x, normalized_y);
        current_state.clone()
    }

    /// Handle a pointer release inside the canvas interaction region.
    fn on_canvas_released(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let _ = (normalized_x, normalized_y);
        current_state.clone()
    }

    /// Handle a pointer hover inside the canvas interaction region.
    fn on_canvas_hovered(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        current_state: &UiShellState,
    ) -> UiShellState {
        let _ = (normalized_x, normalized_y);
        current_state.clone()
    }
}

#[derive(Default)]
struct LoggingDelegate;

impl UiShellDelegate for LoggingDelegate {
    fn on_command(&mut self, command: &str, current_state: &UiShellState) -> UiShellState {
        info!(target: "phototux::ui_shell", %command, "menu command triggered");
        current_state.clone()
    }

    fn on_tool(&mut self, tool: &str, current_state: &UiShellState) -> UiShellState {
        info!(target: "phototux::ui_shell", %tool, "tool selected");
        current_state.clone()
    }
}

/// Launch the application shell.
pub fn launch() -> Result<()> {
    launch_with_delegate(LoggingDelegate)
}

/// Launch the application shell with an event delegate.
pub fn launch_with_delegate<D>(delegate: D) -> Result<()>
where
    D: UiShellDelegate + 'static,
{
    let theme = SlintTheme::try_from(DARK_PRO)?;
    let window = PhotoTuxWindow::new()?;
    let shared_delegate = Rc::new(RefCell::new(delegate));
    let shell_state = Rc::new(RefCell::new(shared_delegate.borrow().initial_state()));

    apply_theme(&window, &theme);
    apply_shell_state(&window, &shell_state.borrow());
    log_startup_diagnostics(&theme);

    {
        let shell_state = Rc::clone(&shell_state);
        let shared_delegate = Rc::clone(&shared_delegate);
        let weak_window = window.as_weak();
        window.on_command_triggered(move |command| {
            if let Some(window) = weak_window.upgrade() {
                if handle_window_command(&window, command.as_str()) {
                    return;
                }
            }

            let current_state = shell_state.borrow().clone();
            let next_state = shared_delegate
                .borrow_mut()
                .on_command(command.as_str(), &current_state);
            *shell_state.borrow_mut() = next_state.clone();

            if let Some(window) = weak_window.upgrade() {
                apply_shell_state(&window, &next_state);
            }
        });
    }

    {
        let weak_window = window.as_weak();
        window.on_titlebar_dragged(move || {
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            let _ = window.window().with_winit_window(|winit_window| winit_window.drag_window());
        });
    }

    {
        let shell_state = Rc::clone(&shell_state);
        let shared_delegate = Rc::clone(&shared_delegate);
        let weak_window = window.as_weak();
        window.on_tool_triggered(move |tool| {
            let current_state = shell_state.borrow().clone();
            let next_state = shared_delegate
                .borrow_mut()
                .on_tool(tool.as_str(), &current_state);
            *shell_state.borrow_mut() = next_state.clone();

            if let Some(window) = weak_window.upgrade() {
                apply_shell_state(&window, &next_state);
            }
        });
    }

    {
        let shell_state = Rc::clone(&shell_state);
        let shared_delegate = Rc::clone(&shared_delegate);
        let weak_window = window.as_weak();
        window.on_tool_option_adjusted(move |option, delta| {
            let current_state = shell_state.borrow().clone();
            let next_state = shared_delegate.borrow_mut().on_tool_option_adjusted(
                option.as_str(),
                delta,
                &current_state,
            );
            *shell_state.borrow_mut() = next_state.clone();

            if let Some(window) = weak_window.upgrade() {
                apply_shell_state(&window, &next_state);
            }
        });
    }

    {
        let shell_state = Rc::clone(&shell_state);
        let shared_delegate = Rc::clone(&shared_delegate);
        let weak_window = window.as_weak();
        window.on_tool_options_reset(move || {
            let current_state = shell_state.borrow().clone();
            let next_state = shared_delegate
                .borrow_mut()
                .on_tool_options_reset(&current_state);
            *shell_state.borrow_mut() = next_state.clone();

            if let Some(window) = weak_window.upgrade() {
                apply_shell_state(&window, &next_state);
            }
        });
    }

    {
        let shell_state = Rc::clone(&shell_state);
        let shared_delegate = Rc::clone(&shared_delegate);
        let weak_window = window.as_weak();
        window.on_canvas_pressed(move |normalized_x, normalized_y| {
            let next_state = {
                let current_state = shell_state.borrow();
                shared_delegate.borrow_mut().on_canvas_pressed(
                    normalized_x,
                    normalized_y,
                    &current_state,
                )
            };
            *shell_state.borrow_mut() = next_state.clone();

            if let Some(window) = weak_window.upgrade() {
                apply_shell_state(&window, &next_state);
            }
        });
    }

    {
        let drag_shell_state = Rc::clone(&shell_state);
        let drag_shared_delegate = Rc::clone(&shared_delegate);
        let drag_weak_window = window.as_weak();
        let last_drag_apply_at = Rc::new(Cell::new(None::<Instant>));
        let drag_apply_gate = Rc::clone(&last_drag_apply_at);
        window.on_canvas_dragged(move |normalized_x, normalized_y| {
            let next_state = {
                let current_state = drag_shell_state.borrow();
                drag_shared_delegate.borrow_mut().on_canvas_dragged(
                    normalized_x,
                    normalized_y,
                    &current_state,
                )
            };
            *drag_shell_state.borrow_mut() = next_state.clone();

            let should_apply = drag_apply_gate
                .get()
                .is_none_or(|instant| instant.elapsed() >= INTERACTIVE_STATE_APPLY_INTERVAL);

            if should_apply {
                drag_apply_gate.set(Some(Instant::now()));
            }

            if should_apply {
                if let Some(window) = drag_weak_window.upgrade() {
                    apply_shell_state(&window, &next_state);
                }
            }
        });

        let release_shell_state = Rc::clone(&shell_state);
        let release_shared_delegate = Rc::clone(&shared_delegate);
        let release_weak_window = window.as_weak();
        let release_drag_apply_gate = Rc::clone(&last_drag_apply_at);
        window.on_canvas_released(move |normalized_x, normalized_y| {
            release_drag_apply_gate.set(None);
            let next_state = {
                let current_state = release_shell_state.borrow();
                release_shared_delegate.borrow_mut().on_canvas_released(
                    normalized_x,
                    normalized_y,
                    &current_state,
                )
            };
            *release_shell_state.borrow_mut() = next_state.clone();

            if let Some(window) = release_weak_window.upgrade() {
                apply_shell_state(&window, &next_state);
            }
        });
    }

    {
        let shell_state = Rc::clone(&shell_state);
        let shared_delegate = Rc::clone(&shared_delegate);
        let weak_window = window.as_weak();
        window.on_canvas_hovered(move |normalized_x, normalized_y| {
            let next_state = {
                let current_state = shell_state.borrow();
                shared_delegate.borrow_mut().on_canvas_hovered(
                    normalized_x,
                    normalized_y,
                    &current_state,
                )
            };
            *shell_state.borrow_mut() = next_state.clone();

            if let Some(window) = weak_window.upgrade() {
                apply_shell_state(&window, &next_state);
            }
        });
    }

    info!(target: "phototux::ui_shell", "launching PhotoTux shell window");
    window.run()?;
    Ok(())
}

fn apply_theme(window: &PhotoTuxWindow, theme: &SlintTheme) {
    window.set_bg_app(theme.bg_app);
    window.set_bg_chrome(theme.bg_chrome);
    window.set_bg_panel(theme.bg_panel);
    window.set_bg_panel_header(theme.bg_panel_header);
    window.set_bg_canvas_surround(theme.bg_canvas_surround);
    window.set_bg_menu(theme.bg_menu);
    window.set_text_primary(theme.text_primary);
    window.set_text_secondary(theme.text_secondary);
    window.set_text_muted(theme.text_muted);
    window.set_accent_primary(theme.accent_primary);
    window.set_border_subtle(theme.border_subtle);
    window.set_border_default(theme.border_default);
    window.set_border_strong(theme.border_strong);
    window.set_button_bg_hover(theme.button_bg_hover);
    window.set_button_bg_active(theme.button_bg_active);
    window.set_toolbar_width(theme.toolbar_width.into());
    window.set_right_dock_width(theme.right_dock_width.into());
    window.set_titlebar_height(theme.titlebar_height.into());
    window.set_menu_bar_height(theme.menu_bar_height.into());
    window.set_tool_options_height(theme.tool_options_height.into());
    window.set_panel_header_height(theme.panel_header_height.into());
    window.set_panel_padding(theme.panel_padding.into());
    window.set_tab_height(theme.tab_height.into());
    window.set_status_bar_height(theme.status_bar_height.into());
}

fn apply_shell_state(window: &PhotoTuxWindow, state: &UiShellState) {
    window.set_document_title(SharedString::from(state.document_title.as_str()));
    window.set_document_dirty(state.document_dirty);
    window.set_active_tool_label(SharedString::from(state.active_tool_label.as_str()));
    window.set_status_message(SharedString::from(state.status_message.as_str()));
    window.set_active_tool_key(SharedString::from(state.active_tool_key.as_str()));
    window.set_tool_preset_label(SharedString::from(state.tool_preset_label.as_str()));
    window.set_tool_option_primary_label(SharedString::from(
        state.tool_option_primary_label.as_str(),
    ));
    window.set_tool_option_secondary_label(SharedString::from(
        state.tool_option_secondary_label.as_str(),
    ));
    window.set_tool_option_tertiary_label(SharedString::from(
        state.tool_option_tertiary_label.as_str(),
    ));
    window.set_tool_option_quaternary_label(SharedString::from(
        state.tool_option_quaternary_label.as_str(),
    ));
    window.set_tool_size_label(SharedString::from(state.tool_size_label.as_str()));
    window.set_tool_hardness_label(SharedString::from(state.tool_hardness_label.as_str()));
    window.set_tool_opacity_label(SharedString::from(state.tool_opacity_label.as_str()));
    window.set_tool_flow_label(SharedString::from(state.tool_flow_label.as_str()));
    window.set_zoom_label(SharedString::from(state.zoom_label.as_str()));
    window.set_canvas_size_label(SharedString::from(state.canvas_size_label.as_str()));
    window.set_canvas_width_px(state.canvas_width_px);
    window.set_canvas_height_px(state.canvas_height_px);
    window.set_cursor_position_label(SharedString::from(state.cursor_position_label.as_str()));
    window.set_selection_bounds_label(SharedString::from(state.selection_bounds_label.as_str()));
    window.set_foreground_color_label(SharedString::from(state.foreground_color_label.as_str()));
    window.set_background_color_label(SharedString::from(state.background_color_label.as_str()));
    window.set_active_layer_name(SharedString::from(state.active_layer_name.as_str()));
    window.set_layer_blend_mode_label(SharedString::from(state.layer_blend_mode_label.as_str()));
    window.set_layer_opacity_label(SharedString::from(state.layer_opacity_label.as_str()));
    window.set_layer_visibility_label(SharedString::from(state.layer_visibility_label.as_str()));
    window.set_layer_count_label(SharedString::from(state.layer_count_label.as_str()));
    window.set_history_entry_primary(SharedString::from(state.history_entry_primary.as_str()));
    window.set_history_entry_secondary(SharedString::from(state.history_entry_secondary.as_str()));
    window.set_history_entry_tertiary(SharedString::from(state.history_entry_tertiary.as_str()));
    window
        .set_history_entry_quaternary(SharedString::from(state.history_entry_quaternary.as_str()));
    window.set_history_entry_quinary(SharedString::from(state.history_entry_quinary.as_str()));
    window.set_canvas_preview(state.canvas_preview.clone());
}

fn handle_window_command(window: &PhotoTuxWindow, command: &str) -> bool {
    match command {
        "Window::Minimize" => {
            window.window().set_minimized(true);
            true
        }
        "Window::ToggleMaximize" => {
            let next_state = !window.window().is_maximized();
            window.window().set_maximized(next_state);
            true
        }
        "Window::Close" => {
            let _ = window.hide();
            true
        }
        _ => false,
    }
}

fn default_canvas_preview() -> Image {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(2, 2);
    buffer.make_mut_bytes().copy_from_slice(&[
        0x50, 0x55, 0x5F, 0xFF, 0x3E, 0x43, 0x4C, 0xFF, 0x3E, 0x43, 0x4C, 0xFF, 0x50, 0x55, 0x5F,
        0xFF,
    ]);

    Image::from_rgba8(buffer)
}

fn log_startup_diagnostics(theme: &SlintTheme) {
    if cfg!(debug_assertions) {
        info!(
            target: "phototux::ui_shell",
            toolbar_width = theme.toolbar_width,
            right_dock_width = theme.right_dock_width,
            titlebar_height = theme.titlebar_height,
            menu_bar_height = theme.menu_bar_height,
            tool_options_height = theme.tool_options_height,
            status_bar_height = theme.status_bar_height,
            "debug shell diagnostics"
        );
    }
}
