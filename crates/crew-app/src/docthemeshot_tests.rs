//! The document window on the themes it was not built on. Split from
//! [`crate::docshot_tests`], which draws its states on the dark theme.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew doc_shot_themes -- --ignored`
use crate::docshot_tests::{doc, MD};
use crate::layout::Rect;

/// The document window on a light page and through a green tube — the two
/// places a surface built and eyed on the dark theme goes wrong.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let mut view = doc(MD);
    view.scroll = 6;
    for (suffix, id) in [
        ("light", crew_theme::ThemeId::PaperLight),
        ("crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let (w, h) = (720u32, 560u32);
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let m = 12.0;
            let rect = Rect {
                x: m,
                y: m,
                w: w as f32 - m * 2.0,
                h: h as f32 - m * 2.0,
            };
            crate::docwin::draw::scenes(rect, cw, ch, "window.md \u{00b7} 22%", &view)
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(&format!("doc-{suffix}"), &px, w, h);
        assert!(crate::shotgpu_tests::ink(&px) > 3_000, "doc-{suffix} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// A document window at the proportions it opens at (a reading measure, taller
/// than wide) and at a shape somebody has dragged it into.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_window() {
    let _g = crate::app::theme_test_guard();
    let view = doc(MD);
    for (name, w, h) in [
        ("doc-window", 720u32, 900u32),
        ("doc-window-wide", 1100, 620),
        ("doc-window-narrow", 460, 760),
    ] {
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let m = 12.0;
            let rect = Rect {
                x: m,
                y: m,
                w: w as f32 - m * 2.0,
                h: h as f32 - m * 2.0,
            };
            crate::docwin::draw::scenes(rect, cw, ch, "window.md \u{00b7} 34%", &view)
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(name, &px, w, h);
        assert!(
            crate::shotgpu_tests::ink(&px) > 3_000,
            "{name} drew a document"
        );
    }
}
