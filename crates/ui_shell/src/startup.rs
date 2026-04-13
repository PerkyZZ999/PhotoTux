use super::*;
use gtk4::Spinner;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupPhase {
    Launching,
    LoadingShell,
    WarmingCanvas,
    OpeningWorkspace,
}

impl StartupPhase {
    fn title(self) -> &'static str {
        match self {
            Self::Launching => "Launching PhotoTux",
            Self::LoadingShell => "Loading shell",
            Self::WarmingCanvas => "Warming renderer",
            Self::OpeningWorkspace => "Opening workspace",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::Launching => "Preparing startup surfaces.",
            Self::LoadingShell => "Building the GTK shell and document session.",
            Self::WarmingCanvas => "Compiling the first canvas frame before handoff.",
            Self::OpeningWorkspace => "Presenting the workspace and transferring focus.",
        }
    }
}

#[derive(Clone)]
struct StartupSplash {
    window: ApplicationWindow,
    phase_label: Label,
    detail_label: Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StartupTimingSummary {
    shell_init_ms: u128,
    warmup_ms: u128,
    handoff_ms: u128,
    total_ms: u128,
    renderer_warmed: bool,
}

impl StartupTimingSummary {
    fn format_log_line(self) -> String {
        format!(
            "shell_init={}ms warmup={}ms handoff={}ms total={}ms renderer_warmed={}",
            self.shell_init_ms,
            self.warmup_ms,
            self.handoff_ms,
            self.total_ms,
            self.renderer_warmed
        )
    }
}

impl StartupSplash {
    fn new(application: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(application)
            .title(APP_NAME)
            .default_width(420)
            .default_height(280)
            .resizable(false)
            .decorated(false)
            .build();
        window.add_css_class("startup-splash");

        let content = GtkBox::new(Orientation::Vertical, 12);
        content.add_css_class("startup-splash-content");
        content.set_valign(Align::Center);
        content.set_halign(Align::Center);
        content.set_margin_top(28);
        content.set_margin_bottom(28);
        content.set_margin_start(28);
        content.set_margin_end(28);

        let logo = build_logo_icon(APP_NAME, 72);
        logo.add_css_class("startup-splash-logo");
        content.append(&logo);

        let title = Label::new(Some(APP_NAME));
        title.add_css_class("startup-splash-title");
        content.append(&title);

        let phase_label = Label::new(None);
        phase_label.add_css_class("startup-splash-phase");
        content.append(&phase_label);

        let detail_label = Label::new(None);
        detail_label.add_css_class("startup-splash-detail");
        detail_label.set_wrap(true);
        detail_label.set_justify(gtk4::Justification::Center);
        content.append(&detail_label);

        let spinner = Spinner::new();
        spinner.add_css_class("startup-splash-spinner");
        spinner.start();
        content.append(&spinner);

        window.set_child(Some(&content));

        let splash = Self {
            window,
            phase_label,
            detail_label,
        };
        splash.update_phase(StartupPhase::Launching);
        splash
    }

    fn update_phase(&self, phase: StartupPhase) {
        self.phase_label.set_label(phase.title());
        self.detail_label.set_label(phase.detail());
    }
}

pub(super) fn begin_startup(application: &Application, controller: Rc<RefCell<dyn ShellController>>) {
    install_theme();
    let splash = StartupSplash::new(application);
    splash.update_phase(StartupPhase::LoadingShell);
    splash.window.present();

    let application = application.clone();
    let shell_state = Rc::new(RefCell::new(None::<Rc<ShellUiState>>));
    let startup_started_at = Instant::now();
    glib::idle_add_local_once(move || {
        let shell_started_at = Instant::now();
        let built_shell_state = ShellUiState::new(controller.clone());
        let shell_init_ms = shell_started_at.elapsed().as_millis();
        tracing::info!(shell_init_ms, "startup shell initialized");
        *shell_state.borrow_mut() = Some(built_shell_state);
        splash.update_phase(StartupPhase::WarmingCanvas);

        let application = application.clone();
        let shell_state = shell_state.clone();
        let splash = splash.clone();
        let startup_started_at = startup_started_at;
        glib::idle_add_local_once(move || {
            let warmup_started_at = Instant::now();
            let mut renderer_warmed = false;
            if let Some(shell_state) = shell_state.borrow().as_ref() {
                renderer_warmed = shell_state
                    .canvas_state
                    .borrow_mut()
                    .warm_up_startup(STARTUP_WARMUP_WIDTH, STARTUP_WARMUP_HEIGHT);
            }
            let warmup_ms = warmup_started_at.elapsed().as_millis();
            tracing::info!(warmup_ms, renderer_warmed, "startup canvas warm-up complete");
            splash.update_phase(StartupPhase::OpeningWorkspace);

            let application = application.clone();
            let shell_state = shell_state.clone();
            let splash = splash.clone();
            glib::idle_add_local_once(move || {
                let handoff_started_at = Instant::now();
                if let Some(shell_state) = shell_state.borrow_mut().take() {
                    let splash_window = splash.window.clone();
                    let startup_started_at = startup_started_at;
                    layout::build_ui(
                        &application,
                        shell_state,
                        Some(Box::new(move |_| {
                            let summary = StartupTimingSummary {
                                shell_init_ms,
                                warmup_ms,
                                handoff_ms: handoff_started_at.elapsed().as_millis(),
                                total_ms: startup_started_at.elapsed().as_millis(),
                                renderer_warmed,
                            };
                            tracing::info!(
                                handoff_ms = summary.handoff_ms,
                                startup_total_ms = summary.total_ms,
                                renderer_warmed = summary.renderer_warmed,
                                startup_summary = %summary.format_log_line(),
                                "startup handoff complete"
                            );
                            splash_window.close();
                        })),
                    );
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{StartupPhase, StartupTimingSummary};

    #[test]
    fn startup_phase_copy_is_non_empty_and_distinct() {
        let phases = [
            StartupPhase::Launching,
            StartupPhase::LoadingShell,
            StartupPhase::WarmingCanvas,
            StartupPhase::OpeningWorkspace,
        ];

        for phase in phases {
            assert!(!phase.title().is_empty());
            assert!(!phase.detail().is_empty());
        }

        assert_ne!(StartupPhase::Launching.title(), StartupPhase::LoadingShell.title());
        assert_ne!(
            StartupPhase::WarmingCanvas.detail(),
            StartupPhase::OpeningWorkspace.detail()
        );
    }

    #[test]
    fn startup_timing_summary_formats_machine_readable_line() {
        let summary = StartupTimingSummary {
            shell_init_ms: 12,
            warmup_ms: 34,
            handoff_ms: 5,
            total_ms: 51,
            renderer_warmed: true,
        };

        let line = summary.format_log_line();
        assert!(line.contains("shell_init=12ms"));
        assert!(line.contains("warmup=34ms"));
        assert!(line.contains("handoff=5ms"));
        assert!(line.contains("total=51ms"));
        assert!(line.contains("renderer_warmed=true"));
    }
}
