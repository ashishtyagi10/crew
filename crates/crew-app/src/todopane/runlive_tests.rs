use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};
use crate::todopane::item::{TodoItem, TodoRun};
use crate::todopane::rowchips::{chip_name, RAN, RUN};
use crate::todopane::{test_pane, TodoPane};

fn ran(id: u64, pane: &str) -> TodoItem {
    TodoItem {
        id,
        title: format!("task {id}"),
        project: Some("x".into()),
        created_ms: id,
        run: Some(TodoRun {
            agent: "claude".into(),
            started_ms: 1,
            pane: pane.into(),
        }),
        ..Default::default()
    }
}

fn text(cells: &[crew_render::CellView]) -> String {
    cells.iter().map(|c| c.c).collect()
}

/// v0.22.79 sliced the chip at byte 1 for its colour; `▶` is three bytes.
/// Drawing a row that had run panicked the pane. Now it draws, in either
/// state, and the sigil says which.
#[test]
fn a_row_that_ran_draws_and_its_sigil_follows_the_agent() {
    assert_eq!(chip_name("\u{25b6}claude"), "claude");
    assert_eq!(chip_name("@crew"), "crew");
    assert_eq!(chip_name("@"), "");
    let mut p: TodoPane = test_pane(vec![ran(1, "@x \u{b7} claude")]);
    let idle = text(&p.cells(60, 8));
    assert!(idle.contains(&format!("{RAN}claude")), "{idle}");
    p.live.insert("@x \u{b7} claude".into());
    let busy = text(&p.cells(60, 8));
    assert!(busy.contains(&format!("{RUN}claude")), "{busy}");
    assert!(!busy.contains(RAN), "{busy}");
}

fn todo_pane(app: &mut CrewApp, items: Vec<TodoItem>) {
    app.panes.push(Pane {
        glide: Default::default(),
        content: PaneContent::Todo(test_pane(items)),
        grid: crate::app::FALLBACK_SIZE,
        rect: crate::spawn::PLACEHOLDER_RECT,
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    });
}

fn live(app: &CrewApp) -> Vec<String> {
    app.panes
        .iter()
        .find_map(|p| match &p.content {
            PaneContent::Todo(t) => Some(t.live.iter().cloned().collect()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn a_list_with_no_runs_never_asks_and_never_changes() {
    let mut app = CrewApp::default();
    todo_pane(&mut app, vec![TodoItem::default()]);
    assert!(!app.sync_todo_runs());
    assert!(live(&app).is_empty());
}

/// A real pane: while its command runs the label is live and the chip is
/// `▶`; once the wrapper drops to the prompt the label leaves the set and
/// the chip is `▷`. Drives a PTY: Unix-only by construction.
#[cfg(unix)]
#[test]
fn the_live_set_follows_the_pane_s_foreground_process() {
    let mut app = CrewApp::default();
    let label = "@x \u{b7} sh";
    todo_pane(&mut app, vec![ran(1, label)]);
    let (_, program, script) =
        crate::runpane::run_parts("sleep 2", "/bin/sh", crate::runpane::bash_path());
    app.spawn_labeled_terminal(&program, &["-c".to_string(), script], label.to_string());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut seen_running = false;
    loop {
        app.sync_todo_runs();
        let now = live(&app);
        if now == vec![label.to_string()] {
            seen_running = true;
        } else if seen_running && now.is_empty() {
            break; // ran, then came back to the prompt
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never saw the run end (running seen: {seen_running}, live: {now:?})"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
