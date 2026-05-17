use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use farx_core::AppConfig;
use farx_ui::app::App;
use farx_ui::event::{Event, EventHandler};

use crate::install::run_install_with_screen_break;

/// Run the full TUI: setup terminal, run event loop, then restore terminal.
pub async fn run_tui() -> Result<()> {
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

    let loop_result = event_loop(&mut terminal, &mut app, &mut events).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    loop_result
}

async fn event_loop<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &mut EventHandler,
) -> Result<()> {
    while app.running {
        terminal.draw(|frame| {
            app.render(frame);
        })?;

        match events.next().await {
            Some(Event::Key(key)) => {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    let action = app.handle_key_event(key);
                    app.dispatch(action);
                }
            }
            Some(Event::Resize(_, _)) => {}
            Some(Event::Tick) => {
                app.tick();
            }
            Some(Event::Mouse(mouse)) => {
                app.handle_mouse_event(mouse);
            }
            None => break,
        }

        // If the user just confirmed an in-TUI update, leave the alternate
        // screen so `perform_update`'s stdout (download progress, install
        // path notes) is visible, run the installer synchronously, then
        // restore the TUI and let App render the result modal.
        if app.pending_install {
            app.pending_install = false;
            let result = run_install_with_screen_break(terminal);
            app.complete_install(result);
        }
    }
    Ok(())
}
