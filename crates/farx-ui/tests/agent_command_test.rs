//! Integration tests for the agent-grid slash commands (`/agents`, `/focus`,
//! `/title`). The guard branches (no agents / no focus) are deterministic
//! without spawning real PTYs.

use farx_core::Action;

mod common;
use common::{make_app_in, setup_test_dir};

fn run(app: &mut farx_ui::app::App, input: &str) {
    app.command_line.input = input.to_string();
    app.dispatch(Action::CommandLineExecute);
}

fn last_message(app: &farx_ui::app::App) -> String {
    app.feedback
        .messages
        .last()
        .map(|m| m.text.clone())
        .unwrap_or_default()
}

#[test]
fn agents_command_reports_empty_grid() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    run(&mut app, "/agents");

    assert_eq!(last_message(&app), "No agents running");
}

#[test]
fn focus_command_without_agents_is_guarded() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    run(&mut app, "/focus 1");

    assert_eq!(last_message(&app), "No agents to focus");
}

#[test]
fn title_command_without_focus_errors() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    run(&mut app, "/title backend");

    assert_eq!(last_message(&app), "No focused agent to rename");
}

#[test]
fn title_command_requires_an_argument() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    run(&mut app, "/title");

    assert_eq!(last_message(&app), "Usage: /title <new tile name>");
}
