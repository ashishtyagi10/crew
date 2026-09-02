//! Off-screen render of the two cards that sit over a crew pane's composer:
//! Cmd+F transcript find and Ctrl+R history search — each a `menu_card` with
//! a legend of its own and a placeholder row when nothing matches.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew composer_shot -- --ignored --nocapture`
use crate::composershot_tests::folded_pane;
use crate::goalshot_tests::dump;
use crew_render::PaneScene;

/// A card that draws its own frame, at the width a pane hands it.
fn card_shot(name: &str, w: u32, card: impl Fn(u16) -> (Vec<crew_render::CellView>, u16)) {
    let px = crate::shotdraw_tests::draw(w, 300, 13.0, |cw, ch| {
        let cols = (w as f32 / cw).floor() as u16;
        let (cells, rows) = card(cols);
        eprintln!("--- composer-{name} {cols}x{rows}");
        for l in dump(&cells, cols, rows) {
            eprintln!("|{l}");
        }
        vec![PaneScene {
            cells,
            x: 0.0,
            y: 0.0,
            w: w as f32,
            h: f32::from(rows) * ch,
            focused: false,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: true,
            paint: Vec::new(),
        }]
    });
    if let Some(px) = px {
        crate::shotdraw_tests::write_png(&format!("composer-{name}"), &px, w, 300);
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn composer_shot_find_and_history() {
    let _g = crate::app::theme_test_guard();
    let p = folded_pane();
    let visible = p.visible_messages();
    let find = crate::chatfind::ChatFind {
        query: "plot".into(),
        matches: crate::chatfind::filter(&visible, "plot"),
        sel: 0,
    };
    let none = crate::chatfind::ChatFind {
        query: "zebra".into(),
        matches: vec![],
        sel: 0,
    };
    for (name, w, f) in [
        ("find", 700, &find),
        ("find-narrow", 380, &find),
        ("find-none", 700, &none),
    ] {
        card_shot(name, w, |cols| crate::chatfind::card(f, &visible, cols));
    }
    let lines: Vec<String> = [
        "cargo test -p crew-app --bin crew chat",
        "@scout what changed in plot/ since v0.19.60?",
        "/doctor",
        "cargo clippy --workspace --all-targets",
        "why is the sidebar chart a smear at two rows?\nand the ring's track?",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let hist = crate::chathistsearch::HistSearch {
        query: "car".into(),
        saved: String::new(),
        matches: crate::chathistsearch::filter(&lines, "car"),
        sel: 1,
    };
    for (name, w) in [("history", 700), ("history-narrow", 380)] {
        card_shot(name, w, |cols| crate::chathistsearch::card(&hist, cols));
    }
}
