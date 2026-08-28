//! Off-screen render of the WHOLE left nav — every section stacked in one tall
//! narrow card, the shape the app actually docks — so the column can be looked
//! at as a column. The per-widget shots in `chartshot_tests` each pass on a
//! wide card and still stack into something that reads badly: a chart that is
//! a smear at 2 rows, a log line that clips mid-word, a donut that overpowers
//! the legend beside it. Only the full stack shows that.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `cargo test -p crew-app --bin crew sidebar_shot -- --ignored --nocapture`
use crate::applog::{LogEntry, LogLevel};
use crate::panelist::PaneRow;
use crate::shotgpu_tests::shot_at;
use crate::statspane::StatsPane;

/// The nav at its default docked width and a full-height window: 260×1000 at
/// 13px is ~24 cols × ~57 rows, which is what the screenshot in the bug report
/// shows.
const W: u32 = 268;
const H: u32 = 1000;

fn log(lines: &[(&str, LogLevel)]) -> Vec<LogEntry> {
    lines
        .iter()
        .map(|(t, level)| LogEntry {
            text: (*t).to_string(),
            level: *level,
        })
        .collect()
}

/// A minute of plausible CPU readings: a laptop idling in the teens with a
/// build spike in it — the trace the report's screenshot was taken on.
fn cpu_trace() -> Vec<u64> {
    (0..64)
        .map(|i| {
            let t = i as f32 / 64.0;
            let base = 11.0 + 6.0 * (t * 9.0).sin();
            let spike = if (0.55..0.68).contains(&t) { 46.0 } else { 0.0 };
            (base + spike).clamp(2.0, 99.0) as u64
        })
        .collect()
}

fn pane(index: usize, title: &str, focused: bool, busy: bool) -> PaneRow {
    PaneRow {
        index,
        title: title.into(),
        focused,
        activity: false,
        minimized: false,
        attention: None,
        busy,
        unread: 0,
        hovered: false,
    }
}

/// The column as the report shows it: one idle pane, two log lines long enough
/// to overflow the nav, a live git repo.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sidebar_shot_full_column() {
    let _g = crate::app::theme_test_guard();
    let mut sp = StatsPane::new();
    sp.refresh(std::path::Path::new("."), 0);
    sp.seed_history(&cpu_trace(), &[0; 64]);
    let entries = log(&[
        ("23:20 updated to crew v0.19.38", LogLevel::Info),
        ("23:20 font → MonoLisa 13px", LogLevel::Info),
    ]);
    let panes = vec![pane(1, "dash", true, false)];
    let px = shot_at(
        "sidebar-full",
        W,
        H,
        13.0,
        concat!("crew v", env!("CARGO_PKG_VERSION")),
        |cols, rows, aspect| {
            (
                sp.cells(cols, rows, &panes, &entries, 0),
                sp.chart_paint(cols, rows, aspect, &panes, entries.len()),
            )
        },
    );
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 4000, "the column drew");
}

/// The same column with a busy crew: several panes, a bell, a deep log.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sidebar_shot_busy_crew() {
    let _g = crate::app::theme_test_guard();
    let mut sp = StatsPane::new();
    sp.refresh(std::path::Path::new("."), 3);
    sp.seed_history(&cpu_trace(), &[0, 0, 1, 3, 4, 4, 2, 1, 1, 2, 3, 3, 2, 2]);
    let entries = log(&[
        ("23:18 swarm planner started", LogLevel::Info),
        ("23:19 mcp server 'files' connected", LogLevel::Info),
        ("23:19 provider anthropic → claude-opus-5", LogLevel::Info),
        ("23:20 build failed: 2 errors in crew-app", LogLevel::Error),
        ("23:20 font → MonoLisa 13px", LogLevel::Info),
    ]);
    let mut panes = vec![
        pane(1, "claude — crew", true, true),
        pane(2, "cargo watch", false, true),
        pane(3, "zsh", false, false),
        pane(4, "dash", false, false),
    ];
    panes[2].attention = Some(('!', true));
    let px = shot_at(
        "sidebar-busy",
        W,
        H,
        13.0,
        concat!("crew v", env!("CARGO_PKG_VERSION")),
        |cols, rows, aspect| {
            (
                sp.cells(cols, rows, &panes, &entries, 0),
                sp.chart_paint(cols, rows, aspect, &panes, entries.len()),
            )
        },
    );
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 4000, "the column drew");
}
