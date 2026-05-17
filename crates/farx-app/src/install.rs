use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use farx_core::update;
use ratatui::prelude::*;

/// Temporarily restore the normal terminal so `perform_update`'s stdout
/// (download progress, install-path notes) is visible to the user, run
/// the blocking installer, then return to the alternate screen with raw
/// mode and mouse capture re-enabled. Returns `Ok(version)` on success
/// or `Err(message)` on any failure — never propagates, because the TUI
/// must always be restored.
pub fn run_install_with_screen_break<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
) -> Result<String, String> {
    // Leave the TUI.
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );

    println!();
    println!("farx — installing update…");
    let install_result = update::perform_update();
    println!();
    println!("Returning to farx…");

    // Re-enter the TUI regardless of install outcome.
    let _ = enable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    );
    let _ = terminal.clear();

    match install_result {
        Ok(self_update::Status::Updated(v)) => Ok(v),
        Ok(self_update::Status::UpToDate(v)) => Ok(v),
        Err(e) => Err(e.to_string()),
    }
}
