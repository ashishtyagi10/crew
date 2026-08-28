//! Off-screen render of the WHOLE left nav — every section stacked in one tall
//! narrow card, the shape the app actually docks — so the column can be looked
//! at as a column. The per-widget shots in `chartshot_tests` each pass on a
//! wide card and still stack into something that reads badly: a chart that is
//! a smear at 2 rows, a log line that clips mid-word, a mark that overpowers
//! the legend beside it. Only the full stack shows that.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `cargo test -p crew-app --bin crew sidebar_shot -- --ignored --nocapture`
use crate::applog::{LogEntry, LogLevel};
use crate::panelist::PaneRow;
use crate::shotgpu_tests::shot_at;
use crate::statspane::StatsPane;

/// A full-height window. Widths come from the resize edge's own range
/// (`navresize::MIN_W`..`MAX_W`), which is the only range a user can produce.
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

/// One nav shot: a full session's worth of state, at `w` logical px and
/// whatever theme is set, so the same column can be looked at across the
/// widths the resize edge allows and the themes it can be wearing.
fn nav_shot(name: &str, w: u32) -> Option<Vec<u8>> {
    let mut sp = StatsPane::new();
    sp.refresh(std::path::Path::new("."), 3);
    // GIT is polled off the main thread and has not answered by the time a
    // shot is taken, so it was the one section never in any of these frames.
    sp.set_git(Some(crate::git::GitInfo {
        branch: "main".into(),
        changed: 9,
        ahead: 1,
        behind: 0,
    }));
    sp.seed_history(&cpu_trace(), &[0, 0, 1, 3, 4, 4, 2, 1, 1, 2, 3, 3, 2, 2]);
    let entries = session_log();
    let mut panes = vec![
        pane(1, "claude — crew", true, true),
        pane(2, "cargo watch", false, true),
        pane(3, "zsh", false, false),
        pane(4, "dash", false, false),
    ];
    panes[2].attention = Some(('!', true));
    shot_at(
        name,
        w,
        H,
        13.0,
        concat!("crew v", env!("CARGO_PKG_VERSION")),
        |cols, rows, aspect| {
            (
                sp.cells(cols, rows, &panes, &entries, 0),
                sp.chart_paint(cols, rows, aspect, &panes, entries.len()),
            )
        },
    )
}

fn session_log() -> Vec<LogEntry> {
    log(&[
        ("23:11 crew v0.19.38 started", LogLevel::Info),
        ("23:12 restored 4 panes from session", LogLevel::Info),
        ("23:12 shell probe: zsh, 41 vars", LogLevel::Info),
        ("23:13 mcp server 'files' connected", LogLevel::Info),
        ("23:13 mcp server 'github' connected", LogLevel::Info),
        ("23:14 provider anthropic → claude-opus-5", LogLevel::Info),
        ("23:14 roster: 6 agents, 3 skills", LogLevel::Info),
        ("23:15 rclone remote 'gdrive' ready", LogLevel::Info),
        ("23:16 relay listening on crew.sock", LogLevel::Info),
        ("23:17 update check: up to date", LogLevel::Info),
        ("23:18 swarm planner started", LogLevel::Info),
        ("23:18 swarm: 4 tasks, width 3", LogLevel::Info),
        ("23:19 cargo check finished in 4.2s", LogLevel::Info),
        ("23:19 git: main ↑1, 9 changed", LogLevel::Info),
        ("23:20 build failed: 2 errors in crew-app", LogLevel::Error),
        ("23:20 font → MonoLisa 13px", LogLevel::Info),
    ])
}

/// The nav at every width its resize edge allows, on the default theme. A
/// section that only reads at one width is a section that reads at no width
/// the user actually chose — this repo has shipped a badge invisible for four
/// releases behind exactly that.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sidebar_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    // MIN_W / the default / MAX_W, plus the card's own 24px of margin.
    for (name, w) in [
        ("sidebar-narrow", 160 + 24),
        ("sidebar-default", 210 + 24),
        ("sidebar-wide", 320 + 24),
    ] {
        let Some(px) = nav_shot(name, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
}

/// The same column on a light page, twice: once with the theme's own accent,
/// and once with crew's brand green — which reads at 1.2 against every light
/// page in the set, and took the whole nav with it before `palette::accent`
/// grew a floor.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sidebar_shot_light_theme() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    for (name, accent) in [
        ("sidebar-light", crew_theme::theme().accent_default),
        ("sidebar-light-crewgreen", crate::palette::DEFAULT_ACCENT),
    ] {
        crate::palette::set_accent(accent);
        let Some(px) = nav_shot(name, 210 + 24) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// The same column on the phosphor tubes. The dial's scale is the finest
/// thing the nav draws — a tick a device pixel wide — and the CRT pass puts
/// bloom, scanlines and curvature over it, which is exactly the combination
/// that can turn a scale into a smear. Shot so it can be looked at.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sidebar_shot_crt_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("sidebar-crt-green", crew_theme::ThemeId::CrtGreen),
        ("sidebar-crt-amber", crew_theme::ThemeId::CrtAmber),
    ] {
        crew_theme::set_theme(id);
        // Each tube's own accent, not whichever one a previous shot left
        // behind: a fresh install on the amber tube is amber throughout, and
        // a green needle on it would be the user's choice, not the theme's.
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = nav_shot(name, 210 + 24) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// A fresh launch: two log lines, one pane, no history behind any chart. The
/// state the bug report's screenshot was taken in.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sidebar_shot_fresh_launch() {
    let _g = crate::app::theme_test_guard();
    let mut sp = StatsPane::new();
    sp.refresh(std::path::Path::new("."), 0);
    let entries = log(&[
        ("23:20 updated to crew v0.19.38", LogLevel::Info),
        ("23:20 font → MonoLisa 13px", LogLevel::Info),
    ]);
    let panes = vec![pane(1, "dash", true, false)];
    let px = shot_at(
        "sidebar-fresh",
        210 + 24,
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
    assert!(crate::shotgpu_tests::ink(&px) > 2000, "the column drew");
}
