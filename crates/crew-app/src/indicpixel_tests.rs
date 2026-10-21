//! The Indic-text goal's proof, read off the pixels: Hindi drawn whole, on
//! the grid, on the light page and on glass.
//!
//! Each sample goes on its own row of one scene, placed by `place_row` as
//! every widget places text, and drawn through the real renderer. Then, per
//! row, a pixel column is ink when it differs from the page beside the scene:
//!
//! - **whole words** — a Devanagari word hangs from one headline, so its ink
//!   is one unbroken span of columns, and a row shows one span per word. A
//!   cell per letter (the bug) breaks every word into letters;
//! - **on its cells** — a row's ink starts at its first cell and ends within
//!   its last word's room (its cells, and the blanks it may spill into);
//! - **the grid holds** — Latin after Hindi on one line starts on its cell.
//!
//! `#[ignore]`d (needs a GPU adapter):
//! `cargo test -p crew-app --bin crew indic_pixels -- --ignored`
use std::cell::Cell;

use crew_render::{CellView, PaneScene};

const FONT: f32 = 15.0;
const X: f32 = 24.0;
const Y: f32 = 16.0;
const MIXED: &str = "crew में हिंदी text";

/// Rows of headline script, one span of ink per word.
const WHOLE: [&str; 3] = [
    "भारत क्ष हिंदी श्री द्वारा कृपया",
    "नई दिल्ली में आज नया उपग्रह भेजा",
    "বাংলা ভাষা",
];

fn row_cells(text: &str, row: u16) -> Vec<CellView> {
    let mut out = Vec::new();
    crate::chatwidth::place_row(
        0,
        200,
        text.chars().map(|c| (c, ())),
        |col, c, marks, ()| {
            out.push(CellView {
                col,
                row,
                c,
                marks,
                fg: crew_theme::theme().ink,
                bg: crew_theme::theme().page_bg,
                ..Default::default()
            })
        },
    );
    out
}

/// Per pixel column of row `r`'s band, whether it carries ink.
fn ink_columns(px: &[u8], w: u32, cell: (f32, f32), r: usize) -> Vec<bool> {
    let (top, bottom) = (
        (Y + r as f32 * cell.1) as u32,
        (Y + (r + 1) as f32 * cell.1) as u32,
    );
    let at = |x: u32, y: u32| &px[((y * w + x) * 4) as usize..][..3];
    let page: Vec<i32> = at(4, (top + bottom) / 2)
        .iter()
        .map(|&v| i32::from(v))
        .collect();
    (0..w)
        .map(|x| {
            (top..bottom).any(|y| {
                let p = at(x, y);
                (0..3)
                    .map(|i| (i32::from(p[i]) - page[i]).abs())
                    .sum::<i32>()
                    > 90
            })
        })
        .collect()
}

/// The `(start, end)` pixel spans of ink in `cols`.
fn spans(cols: &[bool]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (x, &on) in cols.iter().enumerate() {
        match (on, out.last_mut()) {
            (true, Some(s)) if s.1 == x => s.1 = x + 1,
            (true, _) => out.push((x, x + 1)),
            _ => {}
        }
    }
    out
}

#[test]
#[ignore = "needs a GPU adapter"]
fn indic_pixels_are_whole_words_on_their_cells() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in [
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::GlassNight,
    ] {
        crew_theme::set_theme(id);
        let rows: Vec<&str> = WHOLE.iter().copied().chain([MIXED]).collect();
        let (w, h) = (640u32, 160u32);
        let cell = Cell::new((0.0, 0.0));
        let Some(px) = crate::shotdraw_tests::draw(w, h, FONT, |cw, ch| {
            cell.set((cw, ch));
            let cells = (0..rows.len())
                .flat_map(|r| row_cells(rows[r], r as u16))
                .collect();
            vec![PaneScene {
                cells,
                x: X,
                y: Y,
                w: w as f32 - 2.0 * X,
                h: rows.len() as f32 * ch,
                ..Default::default()
            }]
        }) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let (cw, _) = cell.get();
        for (r, text) in rows.iter().enumerate() {
            let found = spans(&ink_columns(&px, w, cell.get(), r));
            let (first, last) = (found[0].0 as f32, found[found.len() - 1].1 as f32);
            let cols = crate::chatwidth::str_w(text) as f32;
            let last_word = crate::chatwidth::str_w(text.rsplit(' ').next().unwrap()) as f32;
            assert!(
                first >= X - 1.0,
                "{id:?} row {r}: ink at {first} before its first cell"
            );
            assert!(
                last <= X + (cols + last_word) * cw + 1.0,
                "{id:?} row {r}: ink to {last}, past its room"
            );
            if let Some(words) = WHOLE.get(r).map(|t| t.split(' ').count()) {
                assert_eq!(found.len(), words, "{id:?} {text}: {found:?}");
            }
        }
        // `text` on the mixed row starts on its own cell.
        let col = crate::chatwidth::str_w("crew में हिंदी ") as f32;
        let at = X + col * cw;
        let found = spans(&ink_columns(&px, w, cell.get(), rows.len() - 1));
        let text_x = found
            .iter()
            .find(|s| s.0 as f32 >= at - cw / 2.0)
            .unwrap()
            .0 as f32;
        assert!(
            text_x >= at - 1.0 && text_x <= at + 3.0,
            "{id:?}: `text` at {text_x}, its cell at {at}"
        );
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
