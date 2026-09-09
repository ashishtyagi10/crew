//! The badge shot: a pill's two ends, at the sizes crew is used at.
//!
//! The caps are the one piece of a badge that is NOT text, and the one
//! piece the font used to get wrong: a Nerd Font scales its powerline arcs
//! to its own cell, so a `` came back taller than the block and a ``
//! shorter. This shot renders the sender pill of a swarm hand-off — the
//! exact row the report came from — with the caps drawn by `boxglyph`, and
//! reads the cap column against the block column so a mismatch is a number.
//!
//! `#[ignore]`d (needs a GPU adapter):
//! `cargo test -p crew-app --bin crew badgeshot -- --ignored --test-threads=1`
use crew_render::{CellView, PaneScene};

use crate::segment::{self, Caps};

const PAD: f32 = 14.0;

fn row(
    cells: &mut Vec<CellView>,
    row: u16,
    col0: u16,
    badge: &[segment::Cell],
    page: (u8, u8, u8),
) {
    for (i, s) in badge.iter().enumerate() {
        cells.push(CellView {
            col: col0 + i as u16,
            row,
            c: s.c,
            fg: s.fg,
            bg: s.bg.unwrap_or(page),
            bold: s.bold,
            italic: false,
            ..Default::default()
        });
    }
}

/// The hand-off row: `editor → software-engineer`, once with the arcs and
/// once with the half blocks, so both switches of the set are on one sheet.
fn scene(w: u32, h: u32, cw: f32, ch: f32) -> Vec<PaneScene> {
    let t = crew_theme::theme();
    let page = t.page_bg;
    let cols = ((w as f32 - 2.0 * PAD) / cw).floor() as u16;
    let rows = ((h as f32 - 2.0 * PAD) / ch).floor() as u16;
    let teal = (30, 110, 110);
    let blue = (30, 70, 140);
    let mut cells = Vec::new();
    for (r, on) in [(1u16, true), (3u16, false)] {
        let _f = crate::glyphs::tests::force(on);
        let a = segment::badge("editor", segment::page_ink(teal), teal, Caps::BOTH);
        let b = segment::badge(
            "software-engineer",
            segment::page_ink(blue),
            blue,
            Caps::BOTH,
        );
        row(&mut cells, r, 1, &a, page);
        let arrow = [segment::Cell {
            c: '\u{2192}',
            fg: t.text_muted,
            bg: None,
            bold: false,
        }];
        row(&mut cells, r, 1 + a.len() as u16 + 1, &arrow, page);
        row(&mut cells, r, 1 + a.len() as u16 + 3, &b, page);
    }
    vec![PaneScene {
        cells,
        x: PAD,
        y: PAD,
        w: cols as f32 * cw,
        h: rows as f32 * ch,
        focused: false,
        bordered: false,
        glass: false,
        scan: -1.0,
        overlay: false,
        paint: Vec::new(),
    }]
}

/// Rows in `y0..y0 + rows` at column `x` painted in `block` (within a
/// tolerance the paper grain does not reach).
fn block_rows(px: &[u8], w: u32, x: u32, y0: u32, rows: u32, block: (u8, u8, u8)) -> u32 {
    (y0..y0 + rows)
        .filter(|y| {
            let i = ((y * w + x) * 4) as usize;
            let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).abs();
            d(px[i], block.0) < 40 && d(px[i + 1], block.1) < 40 && d(px[i + 2], block.2) < 40
        })
        .count() as u32
}

#[test]
#[ignore = "needs a GPU adapter"]
fn badge_caps_match_their_block() {
    let _g = crate::app::theme_test_guard();
    for font in [13.0f32, 26.0] {
        let (w, h) = (700u32, 160u32);
        let cell = std::cell::Cell::new((0.0f32, 0.0f32));
        let Some(px) = crate::shotdraw_tests::draw(w, h, font, |cw, ch| {
            cell.set((cw, ch));
            scene(w, h, cw, ch)
        }) else {
            eprintln!("no GPU adapter — skipped");
            return;
        };
        crate::shotdraw_tests::write_png(&format!("badge_caps_{font}"), &px, w, h);
        let (cw, ch) = cell.get();
        let teal = (30, 110, 110);
        // Row 1 is the arc pill; column 1 its left cap, column 2 the pad cell
        // that is pure block.
        let y0 = (PAD + ch).round() as u32;
        let rows = ch.round() as u32;
        let col = |c: f32| (PAD + c * cw).round() as u32;
        let block = block_rows(&px, w, col(2.5), y0, rows, teal);
        let cap_mid = block_rows(&px, w, col(1.5), y0, rows, teal);
        let cap_edge = block_rows(&px, w, col(1.0) + 1, y0, rows, teal);
        println!("font {font}: block {block} rows, cap mid {cap_mid}, cap edge {cap_edge}");
        assert!(
            block + 1 >= rows,
            "the block does not fill its cell: {block}/{rows}"
        );
        // The report: the font's arc stood taller than the block on one end.
        // A drawn cap can never be taller than the block it caps…
        assert!(
            cap_mid <= block,
            "cap {cap_mid} rows taller than block {block}"
        );
        // …and it is round: half the block or more at its middle, thinning
        // toward its own edge, and nothing at all in the corner.
        assert!(
            cap_mid * 2 > block,
            "cap too thin at its middle: {cap_mid}/{block}"
        );
        assert!(
            cap_edge < cap_mid,
            "cap not rounded: edge {cap_edge} vs mid {cap_mid}"
        );
        assert_eq!(
            block_rows(&px, w, col(1.0) + 1, y0, 1, teal),
            0,
            "square corner"
        );
    }
}
