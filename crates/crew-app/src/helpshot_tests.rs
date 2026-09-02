//! Off-screen render of the `/keys` overlay — the surface whose whole job is
//! to teach the app, and the one place where a clipped line teaches the half
//! that fits (the v0.6.52 lesson, which the *width* of this panel learned
//! the hard way and nothing has looked at since).
//!
//! The overlay draws its own ratatui `Block`, so it is shot through
//! `shotdraw_tests` at the full canvas rather than nested in the harness's
//! card.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew help_shot -- --ignored`
use crew_render::PaneScene;

const PAD: f32 = 12.0;

fn help_shot(name: &str, w: u32, h: u32, scroll: usize, needle: &str) -> Option<Vec<u8>> {
    help_shot_at(name, w, h, 13.0, scroll, needle)
}

fn help_shot_at(
    name: &str,
    w: u32,
    h: u32,
    font: f32,
    scroll: usize,
    needle: &str,
) -> Option<Vec<u8>> {
    let px = crate::shotdraw_tests::draw(w, h, font, |cw, ch| {
        let iw = w as f32 - 2.0 * PAD;
        let ih = h as f32 - 2.0 * PAD;
        let cols = (iw / cw).floor() as u16;
        let rows = (ih / ch).floor() as u16;
        vec![PaneScene {
            cells: crate::help::help_cells(cols, rows, scroll, needle),
            x: PAD,
            y: PAD,
            w: cols as f32 * cw,
            h: rows as f32 * ch,
            focused: true,
            bordered: false,
            glass: true,
            scan: -1.0,
            overlay: true,
            paint: Vec::new(),
        }]
    })?;
    crate::shotdraw_tests::write_png(name, &px, w, h);
    Some(px)
}

/// The overlay at the size it asks for, and at a window that cannot give it
/// that. The key column is a fixed 26, so the description column is whatever
/// the window has left — the width where that stops being a sentence is the
/// width where `/keys` stops teaching.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn help_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    for (name, w, h) in [
        ("help-preferred", 1000, 760),
        ("help-narrow", 620, 760),
        ("help-short", 1000, 420),
    ] {
        let Some(px) = help_shot(name, w, h, 0, "") else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 3000, "{name} drew");
    }
}

/// Scrolled into the middle, and filtered down to a few rows — the two states
/// the panel spends most of its life in once it has more list than window.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn help_shot_states() {
    let _g = crate::app::theme_test_guard();
    for (name, scroll, needle) in [
        ("help-scrolled", 14usize, ""),
        // The last section — the document window's — is the one a fresh
        // shot at scroll 0 never reaches.
        ("help-tail", usize::MAX, ""),
        ("help-filtered", 0, "pane"),
        ("help-nomatch", 0, "zzz"),
    ] {
        let Some(px) = help_shot(name, 1000, 620, scroll, needle) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 800, "{name} drew");
    }
}

/// The same panel on a light page and on a tube.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn help_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("help-light", crew_theme::ThemeId::PaperLight),
        ("help-crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = help_shot(name, 1000, 620, 0, "") else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 3000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// `/keys` at every font size, in one window.
///
/// The panel's key column is a fixed 26 COLUMNS and its descriptions take
/// whatever is left; a column is a different number of pixels at every font
/// size, so the same window is a wide panel at 10px and a cramped one at 26.
/// The width sweep above says the same thing in pixels — this says it the way
/// a reader actually changes it, with `/font`.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn help_shot_font_sweep() {
    let _g = crate::app::theme_test_guard();
    for font in [10.0f32, 13.0, 16.0, 19.0, 22.0, 26.0] {
        let name = format!("help-font-{font:.0}");
        if help_shot_at(&name, 900, 460, font, 0, "").is_none() {
            eprintln!("no GPU adapter — skipped");
            return;
        }
    }
}
