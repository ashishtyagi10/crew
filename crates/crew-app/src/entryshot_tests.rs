//! Mid-trip shots of the two entrances (`paneentry`, `tabentry`): the frame
//! a user actually sees while a split slides out of its parent and while a
//! tab drops in, on a plain page and on glass.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew entry_shot -- --ignored`
use crate::app::CrewApp;
use crate::pane::{spawn_pane, PaneContent};
use crew_term::{GridSize, TermModel};
use crew_theme::ThemeId;

const W: u32 = 1280;
const H: u32 = 720;

fn shell(text: &str) -> crate::pane::Pane {
    let grid = GridSize { cols: 40, rows: 10 };
    let mut p = spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap();
    p.rect = crate::spawn::PLACEHOLDER_RECT;
    if let PaneContent::Terminal(t) = &mut p.content {
        t.pty.feed(b"\x1b[H\x1b[2J");
        for i in 0..30 {
            t.pty.feed(format!("{text} line {i}\r\n").as_bytes());
        }
    }
    p
}

fn settle(app: &mut CrewApp) {
    for _ in 0..80 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.build_frame();
    }
}

/// Shoot the frame `ms` after `trigger` fires on a settled `setup`.
fn mid(name: &str, id: ThemeId, ms: u64, setup: fn(&mut CrewApp), trigger: fn(&mut CrewApp)) {
    crew_theme::set_theme(id);
    let px = crate::shotdraw_tests::draw(W, H, 13.0, |cw, ch| {
        let mut app = CrewApp {
            geo_override: Some((cw, ch, W as f32, H as f32, 1.0)),
            ..Default::default()
        };
        setup(&mut app);
        settle(&mut app);
        trigger(&mut app);
        std::thread::sleep(std::time::Duration::from_millis(120));
        let mut scenes = app.build_frame();
        let until = crate::anim::now_ms() + ms;
        while crate::anim::now_ms() < until {
            std::thread::sleep(std::time::Duration::from_millis(16));
            scenes = app.build_frame();
        }
        scenes
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    crate::shotdraw_tests::write_png(name, &px, W, H);
}

fn one(app: &mut CrewApp) {
    app.panes.push(shell("parent"));
    app.focused = 0;
    app.input.focused = false;
}

fn split(app: &mut CrewApp) {
    app.panes.push(shell("CHILD"));
    app.focused = 1;
}

fn two_tabs(app: &mut CrewApp) {
    one(app);
    settle(app);
    app.own_tab();
    app.panes.push(shell("second tab"));
    app.focused = 1;
}

fn back_to_first(app: &mut CrewApp) {
    let id = app.tabs.id_at(1).unwrap();
    app.activate_tab(id);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn entry_shot_split_and_tab_mid_trip() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::PaperDark, ThemeId::GlassNight] {
        for ms in [0, 60] {
            let n = id.as_str();
            mid(&format!("entry-split-{n}-{ms}ms"), id, ms, one, split);
            mid(
                &format!("entry-tab-{n}-{ms}ms"),
                id,
                ms,
                two_tabs,
                back_to_first,
            );
        }
    }
}
