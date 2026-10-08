//! Card-scale corners, on real pixels: every arc's tails have to meet the
//! rule glyphs they run into to the pixel — a tail a row off is a step at
//! every corner of every card — and the corner has to have actually turned.
//! Shot at 1x, 2x and an odd size, on whole and fractional origins, plain and
//! stretched, because those are what move a glyph's pixels.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew corner -- --ignored`
use crew_render::{CellView, PaneScene};

const W: u32 = 420;
const H: u32 = 260;
const COLS: u16 = 14;
const ROWS: u16 = 7;

/// A plain frame: rounded corners, rules, nothing on them.
fn frame() -> Vec<CellView> {
    let rule = |col: u16, row: u16, c: char| CellView {
        col,
        row,
        c,
        fg: (255, 255, 255),
        bg: crew_theme::theme().page_bg,
        ..Default::default()
    };
    let (r, b) = (COLS - 1, ROWS - 1);
    let mut v = Vec::new();
    for col in 0..COLS {
        for row in 0..ROWS {
            let c = match (col, row) {
                (0, 0) => '\u{256D}',
                (c, 0) if c == r => '\u{256E}',
                (0, y) if y == b => '\u{2570}',
                (c, y) if c == r && y == b => '\u{256F}',
                _ if row == 0 || row == b => '\u{2500}',
                _ if col == 0 || col == r => '\u{2502}',
                _ => ' ',
            };
            v.push(rule(col, row, c));
        }
    }
    v
}

/// The pixels of `px` holding the rule's white along a line through it.
fn inked(px: &[u8], pts: impl Iterator<Item = (usize, usize)>) -> Vec<(usize, usize)> {
    pts.filter(|&(x, y)| px[(y * W as usize + x) * 4] > 128)
        .collect()
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn corner_tails_meet_their_rules() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    for font_px in [13.0f32, 26.0, 15.0] {
        for (ox, oy) in [(12.0f32, 10.0f32), (12.3, 9.6), (11.9, 10.95)] {
            for (sx, sy) in [(0.0f32, 0.0f32), (5.3, 3.7)] {
                let mut fw = 0.0;
                let mut fh = 0.0;
                let mut cell = (0.0, 0.0);
                let Some(px) = crate::shotdraw_tests::draw(W, H, font_px, |cw, ch| {
                    cell = (cw, ch);
                    fw = f32::from(COLS) * cw + sx;
                    fh = f32::from(ROWS) * ch + sy;
                    vec![PaneScene {
                        cells: frame(),
                        x: ox,
                        y: oy,
                        w: fw,
                        h: fh,
                        stretch: sx > 0.0,
                        ..Default::default()
                    }]
                }) else {
                    eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                    return;
                };
                let name = format!("corner-{font_px}-{ox}-{oy}-{sx}");
                crate::shotdraw_tests::write_png(&name, &px, W, H);
                check(&px, &name, (ox, oy, fw, fh), cell);
            }
        }
    }
}

/// The four rules hold one band end to end, through both arcs' tails and
/// their joins, and two pixels in from each corner that band is empty.
fn check(px: &[u8], name: &str, (ox, oy, fw, fh): (f32, f32, f32, f32), (cw, ch): (f32, f32)) {
    let (x0, y0) = (ox as usize, oy as usize);
    let (x1, y1) = ((ox + fw) as usize, (oy + fh) as usize);
    let (xm, ym) = ((x0 + x1) / 2, (y0 + y1) / 2);
    let span = |a: usize, n: f32| a.saturating_sub(1)..a + n.ceil() as usize + 1;
    // Each rule's band where nothing but the rule can be: mid-run.
    let top: Vec<usize> = inked(px, span(y0, ch).map(|y| (xm, y)))
        .iter()
        .map(|p| p.1)
        .collect();
    let bottom: Vec<usize> = inked(px, span(y1 - ch as usize, ch).map(|y| (xm, y)))
        .iter()
        .map(|p| p.1)
        .collect();
    let left: Vec<usize> = inked(px, span(x0, cw).map(|x| (x, ym)))
        .iter()
        .map(|p| p.0)
        .collect();
    let right: Vec<usize> = inked(px, span(x1 - cw as usize, cw).map(|x| (x, ym)))
        .iter()
        .map(|p| p.0)
        .collect();
    for (what, band) in [
        ("top", &top),
        ("bottom", &bottom),
        ("left", &left),
        ("right", &right),
    ] {
        assert!(!band.is_empty(), "{name}: no {what} rule");
    }
    let mid = |b: &[usize]| b.iter().sum::<usize>() as f32 / b.len() as f32;
    let (lx, rx, ty, by) = (mid(&left), mid(&right), mid(&top), mid(&bottom));
    // From the very end of each arc — its tail and the join into the next
    // rule glyph included.
    let reach = crew_render::corner_radius(cw, ch).ceil();
    let along = |a: f32, b: f32| (a + reach) as usize..=(b - reach) as usize;
    for x in along(lx, rx) {
        for (what, band, at) in [("top", &top, y0), ("bottom", &bottom, y1 - ch as usize)] {
            let got: Vec<usize> = inked(px, span(at, ch).map(|y| (x, y)))
                .iter()
                .map(|p| p.1)
                .collect();
            assert_eq!(&got, band, "{name}: the {what} rule steps at x={x}");
        }
    }
    for y in along(ty, by) {
        for (what, band, at) in [("left", &left, x0), ("right", &right, x1 - cw as usize)] {
            let got: Vec<usize> = inked(px, span(at, cw).map(|x| (x, y)))
                .iter()
                .map(|p| p.0)
                .collect();
            assert_eq!(&got, band, "{name}: the {what} rule steps at y={y}");
        }
    }
    // Two pixels in from each corner the rule has already turned away.
    for (x, band) in [
        ((lx + 2.0).round() as usize, &top),
        ((rx - 2.0).round() as usize, &top),
        ((lx + 2.0).round() as usize, &bottom),
        ((rx - 2.0).round() as usize, &bottom),
    ] {
        let lit = inked(px, band.iter().map(|&y| (x, y)));
        assert!(lit.is_empty(), "{name}: a square corner at x={x}: {lit:?}");
    }
}
