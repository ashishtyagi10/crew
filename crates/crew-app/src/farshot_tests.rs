//! Off-screen render of the `/far` file panel — crew's dual-pane file
//! manager, five thousand lines across twenty-one modules, and never once
//! drawn to a PNG and looked at.
//!
//! Two listings share the width, each with its own header, cursor, size and
//! date columns; a command line and a function-key bar share the bottom. What
//! any of that does at a *tile* width — a `/far` opened into a quarter of a
//! 2×2 grid — was nobody's test.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew far_shot -- --ignored`
use crate::farpane::FarPane;
use crate::shotgpu_tests::shot_at;

/// A panel rooted in this repo, so the listing is real names at real lengths.
fn pane() -> FarPane {
    // `Panel::new` reads the directory synchronously for a local path, so
    // the listing is real names at real lengths the moment it exists.
    FarPane::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn far_shot(name: &str, w: u32, h: u32, p: &FarPane) -> Option<Vec<u8>> {
    shot_at(name, w, h, 13.0, "far", |cols, rows, _| {
        (p.cells(cols, rows), Vec::new())
    })
}

/// The widths a `/far` pane is actually opened at: a full window, a half, and
/// one tile of a 2×2 grid. Two listings plus two gutters plus a size and a
/// date column all want the same columns.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn far_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    let p = pane();
    for (name, w, h) in [
        ("far-full", 1200, 700),
        ("far-half", 700, 560),
        ("far-tile", 460, 400),
    ] {
        let Some(px) = far_shot(name, w, h, &p) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 2000, "{name} drew");
    }
}

/// With something typed on the command line, which shares the bottom of the
/// pane with the function-key bar.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn far_shot_command_line() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.cmdline = "rg --hidden 'fn build_frame' crates/crew-app/src".into();
    let Some(px) = far_shot("far-cmdline", 1000, 560, &p) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 2000, "the pane drew");
}

/// On a light page and on a tube.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn far_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let p = pane();
    for (name, id) in [
        ("far-light", crew_theme::ThemeId::PaperLight),
        ("far-crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = far_shot(name, 1000, 560, &p) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 2000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
