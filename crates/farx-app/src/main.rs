use std::io;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

mod keydebug;
mod update;

use farx_core::AppConfig;
use farx_ui::app::App;
use farx_ui::event::{Event, EventHandler};

#[derive(Parser)]
#[command(
    name = "farx",
    version,
    about = "Next-generation cross-platform file manager"
)]
struct Cli {
    /// Update farx to the latest release
    #[arg(long)]
    update: bool,

    /// Check if a newer version is available
    #[arg(long)]
    check_update: bool,

    /// Print version
    #[arg(long)]
    keydebug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // --keydebug: interactive key event debugger
    if cli.keydebug {
        keydebug::run_key_debug();
        return Ok(());
    }

    // --update: download and install the latest version, then exit
    if cli.update {
        println!("farx — checking for updates...");
        match update::perform_update()? {
            self_update::Status::UpToDate(v) => {
                println!("Already up to date (v{v}).");
            }
            self_update::Status::Updated(v) => {
                println!("Updated to v{v}! Restart farx to use the new version.");
            }
        }
        return Ok(());
    }

    // --check-update: just print whether an update exists
    if cli.check_update {
        update::print_version();
        let rx = update::check_and_auto_update_async();
        match rx.recv() {
            Ok(update::UpdateStatus::Updated(v)) => {
                println!("Updated to v{v}! Restart farx to use the new version.");
            }
            Ok(update::UpdateStatus::Available(v)) => {
                println!("New version available: v{v}");
                println!("Run `farx --update` to install it.");
            }
            Ok(update::UpdateStatus::UpToDate) => {
                println!("You are on the latest version.");
            }
            Ok(update::UpdateStatus::Failed(e)) => {
                eprintln!("Could not check for updates: {e}");
            }
            Err(_) => {
                eprintln!("Update check did not complete.");
            }
        }
        return Ok(());
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("farx=info".parse()?),
        )
        .with_writer(io::stderr)
        .init();

    // Load config
    let config = AppConfig::load();
    let tick_rate = Duration::from_millis(config.ui.tick_rate_ms);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let mut app = App::new(config)?;
    let mut events = EventHandler::new(tick_rate);

    // Main loop
    while app.running {
        // Render
        terminal.draw(|frame| {
            app.render(frame);
        })?;

        // Handle events
        match events.next().await {
            Some(Event::Key(key)) => {
                // Only handle key press events (not release/repeat)
                if key.kind == crossterm::event::KeyEventKind::Press {
                    let action = app.handle_key_event(key);
                    app.dispatch(action);
                }
            }
            Some(Event::Resize(_, _)) => {
                // Terminal will re-render on next loop iteration
            }
            Some(Event::Tick) => {
                app.tick();
            }
            Some(Event::Mouse(mouse)) => {
                app.handle_mouse_event(mouse);
            }
            None => {
                // Event stream ended
                break;
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
