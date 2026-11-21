use crate::app::CrewApp;
use crate::motion::{set_level, MotionLevel};
use crate::pane::{spawn_pane, PaneContent};
use crew_term::{GridSize, TermModel};

const CH: f32 = 17.0;

fn app_with_history() -> CrewApp {
    let mut app = CrewApp {
        geo_override: Some((8.0, CH, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    let grid = GridSize { cols: 40, rows: 10 };
    let mut p = spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap();
    if let PaneContent::Terminal(t) = &mut p.content {
        t.pty.feed(b"\x1b[H\x1b[2J\x1b[3J");
        for i in 0..300 {
            t.pty.feed(format!("L{i}\r\n").as_bytes());
        }
    }
    app.panes.push(p);
    app.build_frame();
    app
}

fn offset(app: &CrewApp) -> usize {
    match &app.panes[0].content {
        PaneContent::Terminal(t) => t.pty.display_offset(),
        _ => unreachable!(),
    }
}

/// The shifted content scenes of a frame: `(scroll_px, max row, rows)`.
fn shifted(app: &mut CrewApp) -> Vec<(f32, u16)> {
    app.build_frame()
        .iter()
        .filter(|s| s.scroll_px != 0.0)
        .map(|s| {
            (
                s.scroll_px,
                s.cells.iter().map(|c| c.row).max().unwrap_or(0),
            )
        })
        .collect()
}

/// Mid-glide, the terminal is drawn up by part of a row with the line under
/// its viewport added as one more row; at rest it is on the grid again with
/// no extra row — the frame a pane at rest always drew.
#[test]
fn mid_glide_the_terminal_is_drawn_between_rows() {
    let _g = crate::appearanceguard::theme_test_guard();
    set_level(MotionLevel::Full);
    let mut app = app_with_history();
    crate::scrollglide::start(&mut app.panes[0], 3);
    let mut seen = Vec::new();
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(4));
        seen.extend(shifted(&mut app));
        if !app.panes[0].glide.scroll.moving() {
            break;
        }
    }
    let rows = app.panes[0].grid.rows;
    assert!(!seen.is_empty(), "some frame was drawn between rows");
    for &(px, max_row) in &seen {
        assert!(px < 0.0 && px > -CH, "under a row, upward: {px}");
        assert_eq!(max_row, rows, "the line below rides as row {rows}");
    }
    assert!(shifted(&mut app).is_empty(), "at rest, on the grid");
    assert!(app.panes[0].glide.scroll.edge.is_none());
}

/// The row added is the right line: the one just under the last row shown.
#[test]
fn the_added_row_is_the_next_line_down() {
    let _g = crate::appearanceguard::theme_test_guard();
    set_level(MotionLevel::Full);
    let mut app = app_with_history();
    crate::scrollglide::start(&mut app.panes[0], 20);
    crate::scrollglide::step_all(&mut app.panes, 16);
    let p = &app.panes[0];
    let rows = p.grid.rows;
    let (cells, _) = super::art(p, false, 8.0, CH);
    let line = |r: u16| -> String {
        let mut cs: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
        cs.sort_by_key(|c| c.col);
        cs.iter()
            .map(|c| c.c)
            .collect::<String>()
            .trim()
            .to_string()
    };
    let last: u32 = line(rows - 1)[1..].parse().unwrap();
    assert_eq!(line(rows), format!("L{}", last + 1));
}

/// Motion off: nothing is ever drawn between rows.
#[test]
fn motion_off_draws_on_the_grid() {
    let _g = crate::appearanceguard::theme_test_guard();
    set_level(MotionLevel::Off);
    let mut app = app_with_history();
    crate::scrollglide::start(&mut app.panes[0], 3);
    assert!(shifted(&mut app).is_empty());
}

/// Through `build_frame`, after an idle stretch: a three-row notch is DRAWN
/// under a row along on its first frame — the glide starts from rest rather than
/// being stepped through the idle gap (`glide::first_dt`).
#[test]
fn a_notch_after_idle_starts_gently() {
    let _g = crate::appearanceguard::theme_test_guard();
    set_level(MotionLevel::Full);
    let mut app = app_with_history();
    std::thread::sleep(std::time::Duration::from_millis(150));
    crate::scrollglide::start(&mut app.panes[0], 3);
    app.build_frame();
    let drawn = offset(&app) as f32 + app.panes[0].glide.scroll.residual();
    assert!(drawn < 0.6, "first frame of a notch drawn at {drawn} rows");
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.build_frame();
        if !app.panes[0].glide.scroll.moving() {
            break;
        }
    }
    assert_eq!(offset(&app), 3);
}

/// Mid-scroll frames, rendered: the top row cut at the card's frame, the
/// line coming in at the bottom, on a plain page and on glass.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew scroll_px_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn scroll_px_shot_mid_scroll() {
    use crew_theme::ThemeId;
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let cases = [
        (ThemeId::PaperDark, "mid", -0.6..=-0.4),
        (ThemeId::GlassNight, "mid", -0.6..=-0.4),
        (ThemeId::PaperLight, "mid", -0.6..=-0.4),
        // The tail of a glide, and rest: no sliver in the slack under the
        // last row (`scrollclip::window`).
        (ThemeId::PaperDark, "tail", -0.12..=-0.03),
        (ThemeId::PaperDark, "rest", 0.0..=0.0),
    ];
    for (id, name, want) in cases {
        crew_theme::set_theme(id);
        let px = crate::shotdraw_tests::draw(1280, 712, 13.0, |cw, ch| {
            let mut app = CrewApp {
                geo_override: Some((cw, ch, 1280.0, 712.0, 1.0)),
                ..Default::default()
            };
            let grid = GridSize { cols: 40, rows: 10 };
            let mut p = spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap();
            if let PaneContent::Terminal(t) = &mut p.content {
                t.pty.feed(b"\x1b[H\x1b[2J\x1b[3J");
                for i in 0..300 {
                    let bg = if i % 7 == 0 { "\x1b[44m" } else { "" };
                    t.pty
                        .feed(format!("{bg}line {i:03} of the history\x1b[0m\r\n").as_bytes());
                }
            }
            app.panes.push(p);
            app.input.focused = false;
            for _ in 0..40 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                app.build_frame();
            }
            crate::scrollglide::start(&mut app.panes[0], 6);
            // Step until the frame is drawn about half a row off the grid.
            let mut scenes = app.build_frame();
            for _ in 0..200 {
                let r = app.panes[0].glide.scroll.residual();
                let rest = *want.end() == 0.0 && !app.panes[0].glide.scroll.moving();
                if rest || (*want.end() < 0.0 && want.contains(&r)) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(3));
                scenes = app.build_frame();
            }
            scenes
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let shot = format!("scrollpx-{name}-{}", id.as_str());
        crate::shotdraw_tests::write_png(&shot, &px, 1280, 712);
    }
}
