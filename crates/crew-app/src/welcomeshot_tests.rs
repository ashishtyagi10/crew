//! Off-screen render of the empty-screen welcome — the first thing anybody
//! ever sees of crew, and a surface whose whole content is *centred*: a rain
//! field, a nameplate over it, a tagline, a hint, a version stamp, and (on a
//! relaunch) the restore offer. Six things negotiating for the middle of one
//! card is exactly the arrangement that goes wrong at a size nobody tested.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew welcome_shot -- --ignored`
use crate::shotgpu_tests::shot_at;

fn welcome_shot(name: &str, w: u32, h: u32, restore: Option<usize>) -> Option<Vec<u8>> {
    shot_at(name, w, h, 13.0, "crew", |cols, rows, _| {
        (
            crate::welcome::welcome_cells_animated(cols, rows, 7, restore),
            Vec::new(),
        )
    })
}

/// The window shapes a first launch actually arrives in: a default window, a
/// laptop half-screen, a tall narrow column, and a letterbox short enough
/// that the rain has to give up its rows.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn welcome_shot_shapes() {
    let _g = crate::app::theme_test_guard();
    for (name, w, h) in [
        ("welcome-default", 1100, 760),
        ("welcome-half", 640, 760),
        ("welcome-column", 420, 900),
        ("welcome-letterbox", 1100, 300),
    ] {
        let Some(px) = welcome_shot(name, w, h, None) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 1000, "{name} drew");
    }
}

/// A relaunch offers the session back. That line lands under the hint, in the
/// same centred stack everything else is in.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn welcome_shot_restore_offer() {
    let _g = crate::app::theme_test_guard();
    let Some(px) = welcome_shot("welcome-restore", 1100, 760, Some(4)) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 1000, "the offer drew");
}

/// The first impression on a light page and on a tube.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn welcome_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("welcome-light", crew_theme::ThemeId::PaperLight),
        ("welcome-crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = welcome_shot(name, 1100, 760, None) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 1000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
