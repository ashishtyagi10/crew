//! Off-screen render of the minimized strip — the row of thumbnails a pane
//! is demoted to when the grid runs out of full tiles, plus the `+N` tile
//! standing in for the panes that had no room even there.
//!
//! Nobody chooses to look at this strip; it appears when you open one pane too
//! many, which is exactly why it had never been rendered and inspected. Three
//! things share a thumbnail's one content row (the marker, the unread count,
//! and the title on its border) and the `+N` tile is the only place in crew
//! that says *which* panes are behind it.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew min_shot -- --ignored`
use crate::shotgpu_tests::shot_at;

/// One thumbnail at the size the strip actually gives it.
fn thumb_shot(
    name: &str,
    w: u32,
    title: &str,
    marker: Option<(char, (u8, u8, u8))>,
    unread: usize,
) -> Option<Vec<u8>> {
    shot_at(name, w, 74, 13.0, title, |cols, _rows, _| {
        (crate::minstrip::strip_row(cols, marker, unread), Vec::new())
    })
}

/// The states a thumbnail carries: quiet, active, busy, needing attention,
/// and with a pile of unread lines behind it.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn min_shot_thumbnails() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    for (name, w, title, marker, unread) in [
        ("min-quiet", 240, "3 zsh", None, 0),
        (
            "min-active",
            240,
            "4 cargo watch",
            Some((crate::shapecues::dot(false), t.activity)),
            12,
        ),
        (
            "min-attention",
            240,
            "5 crew \u{b7} claude",
            Some(('\u{25c9}', t.bell)),
            999,
        ),
        (
            "min-narrow",
            120,
            "6 a-very-long-pane-title-nobody-can-fit",
            Some((crate::shapecues::dot(true), t.activity)),
            7,
        ),
    ] {
        let Some(px) = thumb_shot(name, w, title, marker, unread) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 100, "{name} drew");
    }
}

/// The `+N` tile: the one surface that answers "which panes are behind this?"
/// — with more names than it has rows, and one longer than it has columns.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn min_shot_overflow_tile() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = vec![
        "7 zsh".into(),
        "8 crew \u{b7} claude-opus-5 reviewing the diff".into(),
        "9 cargo test --workspace".into(),
        "10 ssh build-box".into(),
        "11 tail -f /var/log/system.log".into(),
        "12 htop".into(),
    ];
    for (name, w, h) in [("min-overflow", 260, 130), ("min-overflow-wide", 520, 130)] {
        let names = names.clone();
        let Some(px) = shot_at(name, w, h, 13.0, "+6", move |cols, rows, _| {
            (
                crate::minstrip::overflow_cells(&names, cols, rows),
                Vec::new(),
            )
        }) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 200, "{name} drew");
    }
}
