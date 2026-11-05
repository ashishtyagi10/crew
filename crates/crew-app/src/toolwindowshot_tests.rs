//! A live turn's tool block with twelve calls: the head counting the nine
//! folded ones, then the last three, under smith's streaming card.
//!
//! `toolstrip_shot` is the same turn settled: the summary row that names
//! what the calls were, and its strip of where the time went.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew toolwindow_shot -- --ignored`
use crate::chatshot_tests::live_pane;
use crate::chattool::{ToolBlock, ToolDone, ToolLine};
use crate::shotgpu_tests::shot_at;

fn call(label: &str, subject: &str, ok: Option<bool>) -> ToolLine {
    timed(label, subject, ok, 800)
}

/// `started_ms` is epoch ms, as the live path stamps it (`chatswarmrec`):
/// the process clock here drew the running call as `· 99d`.
fn timed(label: &str, subject: &str, ok: Option<bool>, ms: u64) -> ToolLine {
    ToolLine {
        kind: crate::chattoolkind::LineKind::Tool,
        label: label.into(),
        args: String::new(),
        args_short: subject.into(),
        started_ms: crate::chattime::unix_now_ms(),
        done: ok.map(|ok| ToolDone {
            ok,
            ms,
            text: String::new(),
        }),
        show_text: false,
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn toolwindow_shot() {
    let _g = crate::app::theme_test_guard();
    let mut lines: Vec<ToolLine> = (0..9)
        .map(|i| call("Read", &format!("src/plot/part{i}.rs"), Some(i != 4)))
        .collect();
    lines.push(call("Grep", "fn cover", Some(true)));
    lines.push(call("Edit", "src/plot/sdf.rs", Some(true)));
    lines.push(call("Bash", "cargo test -p crew-plot", None));
    for (name, id) in [
        ("toolwindow-dark", crew_theme::ThemeId::PaperDark),
        ("toolwindow-glass-night", crew_theme::ThemeId::GlassNight),
    ] {
        crew_theme::set_theme(id);
        let mut p = live_pane();
        p.tools.blocks = vec![ToolBlock {
            agent_id: 1,
            agent: "smith".into(),
            lines: lines.clone(),
            anchor: None,
            settled: false,
            expanded: false,
        }];
        let drew = shot_at(name, 700, 760, 13.0, "crew", |cols, rows, aspect| {
            crate::chatview::art(&p, cols, rows, aspect)
        });
        if drew.is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn toolstrip_shot() {
    let _g = crate::app::theme_test_guard();
    let mut lines: Vec<ToolLine> = (0..6)
        .map(|i| timed("Read", &format!("src/plot/part{i}.rs"), Some(true), 700))
        .collect();
    lines.extend((0..3).map(|i| timed("Edit", &format!("src/plot/sdf{i}.rs"), Some(true), 2_500)));
    lines.push(timed(
        "Bash",
        "cargo test -p crew-plot",
        Some(false),
        14_000,
    ));
    lines.push(timed("Bash", "cargo test -p crew-plot", Some(true), 12_000));
    for (name, id) in [
        ("toolstrip-dark", crew_theme::ThemeId::PaperDark),
        ("toolstrip-light", crew_theme::ThemeId::PaperLight),
        ("toolstrip-glass-night", crew_theme::ThemeId::GlassNight),
        ("toolstrip-crt", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        let mut p = live_pane();
        p.tools.blocks = vec![ToolBlock {
            agent_id: 1,
            agent: "scout".into(),
            lines: lines.clone(),
            anchor: Some("1700000000000".into()),
            settled: true,
            expanded: false,
        }];
        let drew = shot_at(name, 700, 760, 13.0, "crew", |cols, rows, aspect| {
            crate::chatview::art(&p, cols, rows, aspect)
        });
        if drew.is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}
