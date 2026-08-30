//! Off-screen render of the pane card — the frame every single pane in crew
//! wears, and the busiest surface in the app.
//!
//! Twenty-odd things ride these four borders: the numbered legend in the
//! pane's signature hue, the `[-][x]` buttons, the activity dot and unread
//! count, the bell, the broadcast mark, the pin, the git badge, the elapsed
//! clock, the OSC 9;4 progress bar along the bottom, the scroll thumb and its
//! landmark/search/command/error ticks down the sides, the focus brackets in
//! the corners, and the command-at-top label while scrolled. Each has its own
//! test. Nothing had ever drawn them together.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew card_shot -- --ignored`
use crew_render::PaneScene;

use crate::panecard::{pane_card, Bar};

const PAD: f32 = 14.0;

/// A quiet card: what a pane looks like when nothing is happening to it.
fn quiet(title: &'static str, focused: bool) -> Bar<'static> {
    Bar {
        index: Some(2),
        title,
        focused,
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: true,
        focus_t: if focused { 1.0 } else { 0.0 },
        assemble_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        pinned: false,
        at_cmd: None,
        fail_rows: &[],
        cmd_rows: &[],
        err_rows: &[],
        unread: 0,
        doc: false,
    }
}

fn card_shot(name: &str, w: u32, h: u32, b: &Bar) -> Option<Vec<u8>> {
    card_shot_via(name, w, h, b, false)
}

/// The same card through the CRT tube — the bloom, the scanlines and the
/// composite three of crew's themes wear. Every card shot before this took
/// the non-CRT path, so the look a third of the palette ships had never been
/// in the suite at all.
fn card_shot_crt(name: &str, w: u32, h: u32, b: &Bar) -> Option<Vec<u8>> {
    card_shot_via(name, w, h, b, true)
}

fn card_scene(w: u32, h: u32, cw: f32, ch: f32, b: &Bar) -> Vec<PaneScene> {
    let iw = w as f32 - 2.0 * PAD;
    let ih = h as f32 - 2.0 * PAD;
    let cols = (iw / cw).floor() as u16;
    let rows = (ih / ch).floor() as u16;
    vec![PaneScene {
        cells: pane_card(cols.saturating_sub(2), rows.saturating_sub(2), b),
        x: PAD,
        y: PAD,
        w: cols as f32 * cw,
        h: rows as f32 * ch,
        focused: b.focused,
        bordered: false,
        glass: true,
        scan: -1.0,
        overlay: false,
        // The thumb and the progress bar are Paint, not cells — a card shot
        // without its paint layer is missing two of the readings it exists to
        // give (see `cardpaint`).
        paint: crate::cardpaint::card_paint(cols, rows, b, ch / cw, 0),
    }]
}

fn card_shot_via(name: &str, w: u32, h: u32, b: &Bar, crt: bool) -> Option<Vec<u8>> {
    let px = match crt {
        true => crate::shotdraw_tests::draw_crt(w, h, 13.0, |cw, ch| card_scene(w, h, cw, ch, b)),
        false => crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| card_scene(w, h, cw, ch, b)),
    }?;
    crate::shotdraw_tests::write_png(name, &px, w, h);
    Some(px)
}

/// Everything at once, on one card: a focused pane mid-run, scrolled back,
/// with a git badge, a progress bar, unread lines, a bell, landmarks, search
/// hits, command marks and errors down the sides.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn card_shot_everything_at_once() {
    let _g = crate::app::theme_test_guard();
    let git = crate::git::GitInfo {
        branch: "feat/transparency-focus".into(),
        changed: 7,
        ahead: 2,
        behind: 1,
    };
    let loud = Bar {
        scroll: 120,
        total: 4_000,
        activity: true,
        bell: true,
        broadcast: true,
        pinned: true,
        unread: 37,
        git: Some(&git),
        elapsed: Some("2m14s".into()),
        progress: Some(crew_term::Progress {
            percent: Some(62),
            alarm: false,
        }),
        ticks: &[3, 9, 15],
        hits: &[6, 11],
        cmd_rows: &[2, 8],
        fail_rows: &[8],
        err_rows: &[12, 13],
        at_cmd: Some("cargo test --workspace"),
        ..quiet("crew \u{b7} claude", true)
    };
    let Some(px) = card_shot("card-loaded", 900, 420, &loud) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 1500, "the card drew");
}

/// Focused and not, side by side in two shots — the hierarchy the whole grid
/// depends on.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn card_shot_focus_hierarchy() {
    let _g = crate::app::theme_test_guard();
    for (name, focused) in [("card-focused", true), ("card-quiet", false)] {
        let Some(px) = card_shot(name, 700, 300, &quiet("zsh", focused)) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 200, "{name} drew");
    }
}

/// The widths a tile actually gets in a 2×2 and a 3×3 grid. The legend, the
/// buttons, the git badge and the clock all want the top border.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn card_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    let git = crate::git::GitInfo {
        branch: "main".into(),
        changed: 3,
        ahead: 0,
        behind: 0,
    };
    for (name, w) in [("card-tile-3x3", 340), ("card-tile-2x2", 520)] {
        let b = Bar {
            git: Some(&git),
            elapsed: Some("1m02s".into()),
            unread: 8,
            activity: true,
            ..quiet("crew \u{b7} claude-opus-5", true)
        };
        let Some(px) = card_shot(name, w, 260, &b) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 200, "{name} drew");
    }
}

/// The busiest card, through the tube. Nothing had ever looked at a real
/// frame this way: the headless chain test drives synthetic patterns, and
/// every shot went straight from the cell grid to the readback — the path a
/// NON-CRT theme takes.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn card_shot_through_the_tube() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
    let git = crate::git::GitInfo {
        branch: "feat/crisp".into(),
        changed: 3,
        ahead: 1,
        behind: 0,
    };
    let bar = Bar {
        scroll: 40,
        total: 400,
        activity: true,
        git: Some(&git),
        unread: 5,
        ..quiet("claude — crew", true)
    };
    let Some(px) = card_shot_crt("card-crt-green", 760, 420, &bar) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    // The tube adds light; it must not have taken the card away.
    assert!(
        crate::shotgpu_tests::ink(&px) > 2000,
        "the card vanished through the tube"
    );
}
