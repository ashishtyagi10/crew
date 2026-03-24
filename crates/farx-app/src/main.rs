use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

mod keydebug;

use farx_core::AppConfig;
use farx_ui::app::App;
use farx_ui::event::{Event, EventHandler};

#[tokio::main]
async fn main() -> Result<()> {
    // Key debug mode: cargo run -- --keydebug
    if std::env::args().any(|a| a == "--keydebug") {
        keydebug::run_key_debug();
        return Ok(());
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("farx=info".parse()?),
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
                // Check for completed AI responses
                app.check_ai_response();
            }
            Some(Event::Mouse(_)) => {
                // Mouse support later
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
