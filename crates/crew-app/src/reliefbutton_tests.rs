use super::*;

fn cell(col: u16, c: char, bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row: 3,
        c,
        bg,
        ..Default::default()
    }
}

const PAGE: (u8, u8, u8) = (234, 234, 236);
const FILL: (u8, u8, u8) = (67, 72, 190);

/// `  Save ⌘S  ` on a fill, then three spaces of page, then `  Cancel  `.
fn form() -> Vec<CellView> {
    let mut v = Vec::new();
    let mut col = 0;
    for (text, bg) in [
        ("\u{a0}\u{a0}Save\u{a0}\u{2318}S\u{a0}\u{a0}", FILL),
        ("   ", PAGE),
        ("\u{a0}\u{a0}Cancel\u{a0}esc\u{a0}\u{a0}", (200, 200, 204)),
    ] {
        for c in text.chars() {
            v.push(cell(col, c, bg));
            col += 1;
        }
    }
    v
}

#[test]
fn each_padded_run_of_fill_is_a_key() {
    let k = keys(&form(), PAGE);
    assert_eq!(k.len(), 2, "{k:?}");
    assert_eq!((k[0].c0, k[0].c1), (0, 10));
    assert_eq!((k[1].c0, k[1].c1), (14, 27));
}

#[test]
fn a_highlight_or_a_chip_is_not_a_key() {
    // A selection band: fill with letters to its ends, no padding.
    let band: Vec<_> = "selected row"
        .chars()
        .enumerate()
        .map(|(i, c)| cell(i as u16, c, FILL))
        .collect();
    assert!(keys(&band, PAGE).is_empty());
    // Inline code's capsule is marked: the renderer draws it, not a plate.
    let mut chip = form();
    chip.iter_mut().for_each(|c| c.mark = true);
    assert!(keys(&chip, PAGE).is_empty());
}

#[test]
fn the_plate_stands_out_past_the_fill() {
    let k = Key {
        row: 2,
        c0: 4,
        c1: 9,
    };
    let r = rect_of(&k, (100.0, 50.0), 8.0, 16.0);
    assert!(r.x < 100.0 + 4.0 * 8.0 && r.x + r.w > 100.0 + 10.0 * 8.0);
    // A key a little taller than its line, the line centred on it.
    assert!(r.y < 50.0 + 32.0 && r.y + r.h > 50.0 + 48.0);
    assert!(((r.y + r.h / 2.0) - (50.0 + 40.0)).abs() < 0.01);
}

#[test]
fn the_label_leaves_its_fill_and_the_primary_keeps_its_colour() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Porcelain);
    let page = crew_theme::theme().page_bg;
    let mut v = form();
    v.iter_mut()
        .for_each(|c| c.bg = if c.bg == PAGE { page } else { c.bg });
    v.iter_mut()
        .filter(|c| c.bg == FILL)
        .for_each(|c| c.fg = (255, 255, 255));
    let k = keys(&v, page);
    onto_plate(&mut v, &k, page);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert!(v.iter().all(|c| c.bg == page), "every fill is gone");
    let save = v[2].fg; // `S`
    let hue = |c| crew_theme::oklch::from_srgb(c);
    assert!(hue(save).c > 0.06, "Save is still the accent: {save:?}");
    assert!(crew_theme::contrast_ratio(save, page) >= 4.5, "{save:?}");
}
