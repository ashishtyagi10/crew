//! The input bar's own palette: the "commands" card that stands over the
//! docked bar (`render.rs`, the `ib` block) while a slash command is typed
//! there. It is the one pop-up not over a pane — placed above the bar with
//! the grid gap between — and it was never shot. `#[ignore]`d: needs a GPU.
//! `cargo test -p crew-app --bin crew bar_popup_shot -- --ignored --nocapture`
use crate::goalshot_tests::dump;
use crate::inputbar::InputBar;
use crew_render::PaneScene;

const PAD: f32 = 12.0;

fn shot(name: &str, w: u32, text: &str) -> Option<Vec<u8>> {
    let b = InputBar {
        text: text.into(),
        focused: true,
        cwd: "/Users/me/code/crew".into(),
        ..Default::default()
    };
    let matches = crate::cmdnote::rows(text, std::path::Path::new("/Users/me/code/crew"));
    assert!(!matches.is_empty(), "{text:?} lists commands");
    let h = 14 * 18 + 2 * PAD as u32;
    let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
        let bh = 3.0 * ch;
        let bw = w as f32 - 2.0 * PAD;
        let cols = (bw / cw).floor() as u16;
        let (ib_x, ib_y) = (PAD, h as f32 - PAD - bh);
        let pop = crate::cmdmenu::popup("commands", &matches, 1, cols);
        eprintln!("--- {name} {}x{}", pop.cols, pop.rows);
        for l in dump(&pop.cells, pop.cols, pop.rows) {
            eprintln!("|{l}");
        }
        let mh = f32::from(pop.rows) * ch;
        vec![
            PaneScene {
                cells: b.cells(cols, 3, None, None, Some("1 crew")),
                x: ib_x,
                y: ib_y,
                w: bw,
                h: bh,
                focused: true,
                bordered: false,
                glass: true,
                scan: -1.0,
                overlay: false,
                paint: Vec::new(),
            },
            PaneScene {
                cells: pop.cells,
                x: ib_x,
                y: (ib_y - mh - crate::app::gap()).max(0.0),
                w: crate::popupplace::scene_w(pop.cols, cols, cw),
                h: mh,
                focused: false,
                bordered: false,
                glass: false,
                scan: -1.0,
                overlay: true,
                paint: Vec::new(),
            },
        ]
    })?;
    crate::shotdraw_tests::write_png(name, &px, w, h);
    Some(px)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn bar_popup_shot() {
    let _g = crate::app::theme_test_guard();
    for (name, w, text) in [
        ("bar-popup", 1000, "/th"),
        ("bar-popup-all", 1000, "/"),
        ("bar-popup-narrow", 420, "/th"),
    ] {
        if shot(name, w, text).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}
