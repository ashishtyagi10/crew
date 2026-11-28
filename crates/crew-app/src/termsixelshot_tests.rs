//! A sixel in a terminal pane, photographed: `$ img2sixel x.png`, the picture,
//! then the prompt again. The picture is built here, not read from a file — a
//! rainbow of hue-defined (HLS) tiles on a diagonal, a row of the VT340
//! default registers, under raster attributes that fix the size up front.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew sixel_shot -- --ignored`
use crew_term::{GridSize, HeadlessTerm, TermModel};

use crate::session::to_cellviews;
use crate::shotgpu_tests::{ink, shot_at};

/// 192 x 108 px: 16 x 9 tiles of 12 px, then a 16-register strip, 6 px tall (114 rows in all).
fn sixel() -> String {
    let mut s = String::from("\x1bP0;1;0q\"1;1;192;114");
    for n in 0..8 {
        // DEC hue: 0 is blue, so the wheel starts cold and goes around.
        s.push_str(&format!("#{};1;{};50;100", n + 20, n * 45));
    }
    for band in 0..18 {
        // Two bands per 12 px tile row.
        for tile in 0..16 {
            s.push_str(&format!("#{}!12~", 20 + (band / 2 + tile) % 8));
        }
        s.push('-');
    }
    // The default registers, as they come, one 12 px swatch each.
    for reg in 0..16 {
        s.push_str(&format!("#{reg}!12~"));
    }
    s.push_str("\x1b\\");
    s
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sixel_shot() {
    let _g = crate::app::theme_test_guard();
    let body = format!(
        "\x1b[36m$\x1b[0m img2sixel rainbow.png\r\n{}\r\n\x1b[36m$\x1b[0m echo done\r\ndone\r\n",
        sixel()
    );
    let shot = shot_at("term-sixel", 700, 560, 13.0, "zsh", |cols, rows, _| {
        let mut t = HeadlessTerm::new(GridSize { cols, rows });
        t.feed(body.as_bytes());
        let cells = to_cellviews(&t.cells(true));
        let mut store = crate::termimg::TermImages::default();
        store.sync(t.take_image_wipes(), t.take_images());
        for _ in 0..400 {
            if store.poll() || !store.loading() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let paint = store.paint(t.screen_line(), t.display_offset(), cols, rows, 2.1);
        (cells, paint)
    });
    match shot {
        Some(px) => assert!(ink(&px) > 5_000, "the picture is not on the page"),
        None => eprintln!("no GPU adapter — skipping (this is a skip, not a pass)"),
    }
}
