//! Application startup and top-level orchestration for PhotoTux.

mod session;

use anyhow::{Context, Result};
use session::AppSession;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Start the PhotoTux application.
pub fn run() -> Result<()> {
    init_tracing();

    info!(target: "phototux::app_core", "starting PhotoTux");
    ui_shell::launch_with_delegate(AppSession::new()).context("launching ui shell")?;
    info!(target: "phototux::app_core", "PhotoTux exited cleanly");

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .try_init();
}
