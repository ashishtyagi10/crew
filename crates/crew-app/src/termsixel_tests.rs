//! A sixel from a program's output to pixels: the terminal cuts it out, a
//! worker decodes it, and the bitmap the pane would paint holds the colours
//! the stream defined.
use super::*;

/// Six columns, six rows, P2 = 1: default blue, an RGB define, an HLS define
/// (DEC hue 120 is red), a repeat of a green, and a lone top pixel.
const SIXEL: &str = "\x1bP0;1;0q\"1;1;6;6#1~#2;2;100;0;50~#3;1;120;50;100~#5;2;0;100;0!2~#1@\x1b\\";

fn landed(feed: &str) -> Bitmap {
    use crew_term::TermModel;
    let mut term = crew_term::HeadlessTerm::new(crew_term::GridSize { cols: 40, rows: 12 });
    term.feed(feed.as_bytes());
    let mut t = TermImages::default();
    t.sync(term.take_image_wipes(), term.take_images());
    for _ in 0..400 {
        t.poll();
        if !t.loading() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    match t.shown.remove(0).art {
        Art::Ready(bm) => bm,
        _ => panic!("the sixel did not decode"),
    }
}

#[test]
fn a_sixel_decodes_to_the_colours_it_defined() {
    let bm = landed(&format!("$ img2sixel x.png\r\n{SIXEL}$ "));
    assert_eq!((bm.w, bm.h), (6, 6));
    let at = |x: usize, y: usize| bm.px[y * 6 + x];
    assert_eq!(at(0, 0), [0x33, 0x33, 0xcc, 255], "VT340 register 1");
    assert_eq!(at(1, 3), [255, 0, 128, 255], "RGB 100/0/50 percent");
    assert_eq!(
        at(2, 5),
        [255, 0, 0, 255],
        "HLS hue 120 is red on DEC's wheel"
    );
    assert_eq!(
        at(3, 0),
        [0, 255, 0, 255],
        "a repeat paints both columns..."
    );
    assert_eq!(
        at(4, 4),
        [0, 255, 0, 255],
        "...down to the band's last pixel"
    );
    assert_eq!(at(5, 0), [0x33, 0x33, 0xcc, 255], "the lone pixel");
    assert_eq!(at(5, 1)[3], 0, "P2=1: what no band touched stays clear");
}

/// Garbage after `q`, and a raster header claiming 100 megapixels, must be
/// refused up front: not a panic, not a hang, not a worker.
#[test]
fn an_unsizable_sixel_is_never_placed() {
    use crew_term::TermModel;
    let mut term = crew_term::HeadlessTerm::new(crew_term::GridSize { cols: 40, rows: 12 });
    term.feed(b"\x1bPq#;;;;;!!!!\x1b\\\x1bP0;0;0q\"1;1;99999;99999~\x1b\\");
    let mut t = TermImages::default();
    t.sync(term.take_image_wipes(), term.take_images());
    assert!(t.shown.is_empty(), "no size, no placement, no worker");
}
