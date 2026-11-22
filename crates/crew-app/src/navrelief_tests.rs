use super::*;

fn row(out: &mut Vec<CellView>, r: u16, s: &str) {
    for (i, c) in s.chars().enumerate() {
        out.push(CellView {
            col: i as u16,
            row: r,
            c,
            ..Default::default()
        });
    }
}

fn text(cells: &[CellView], r: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
    v.sort_by_key(|c| c.col);
    let s: String = v.iter().map(|c| c.c).collect();
    s.trim_end().to_string()
}

/// Two sections a gap row apart, a third hard under the second, a PANES
/// heading, and two tab cards on adjacent rows.
fn nav() -> Vec<CellView> {
    let mut v = Vec::new();
    for (r, s) in [
        " \u{2500} TIME \u{2500}\u{2500}\u{2500} ",
        "   16:45     ",
        "",
        " \u{2500} LOAD \u{2500}\u{2500}\u{2500} ",
        "   1.4 2.4   ",
        " \u{2500} GIT \u{2500}\u{2500}\u{2500}\u{2500} ",
        "   main      ",
        "",
        " \u{2500} PANES \u{2500}\u{2500} ",
        " \u{256d} 1 \u{b7} a \u{2500}\u{2500}\u{256e} ",
        " \u{2502} 1 far   \u{2502} ",
        " \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f} ",
        " \u{256d} 2 \u{b7} b \u{2500}\u{2500}\u{256e} ",
        " \u{2502} 2 a\u{2500}b   \u{2502} ",
        " \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f} ",
    ]
    .iter()
    .enumerate()
    {
        row(&mut v, r as u16, s);
    }
    v
}

/// Every section but the first gets a groove: mid-gap where a blank row
/// parts it from the one above, on the heading's top edge where none does.
/// A tab card's edges open no section.
#[test]
fn a_groove_above_every_section_but_the_first() {
    assert_eq!(heads(&nav(), 0), [0, 3, 5, 8]);
    assert_eq!(grooves(&nav(), 0), [2.5, 5.0, 7.5]);
}

/// The section strokes go and the words stay; a tab card's own strokes
/// and a rule inside a pane's name are not this module's to blank.
#[test]
fn the_section_rules_are_blanked_and_the_words_kept() {
    let mut v = nav();
    unruled(&mut v, 0);
    assert_eq!(text(&v, 0), "   TIME");
    assert_eq!(text(&v, 8), "   PANES");
    assert_eq!(text(&v, 13), " \u{2502} 2 a\u{2500}b   \u{2502}");
}

/// A groove is two hairlines, the dark one resting on the light one, as
/// wide as the rule it replaces.
#[test]
fn a_groove_is_a_dark_line_over_a_light_one() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Porcelain);
    let p = groove_paint(&[2.5], 0, 13, 16.0);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert_eq!(p.len(), 2);
    let (dark, lite) = (&p[0], &p[1]);
    assert_eq!((dark.color, lite.color), ((0, 0, 0), (255, 255, 255)));
    assert!((dark.y + dark.h - lite.y).abs() < 1e-6 && lite.y == 2.5);
    assert!(p.iter().all(|l| l.x == 1.0 && l.w == 11.0));
    // One px of a 16 px row.
    assert!((dark.h - 1.0 / 16.0).abs() < 1e-6);
}

/// Two tab cards on adjacent rows end up `gap` px apart: a rib of the
/// panel between their wells.
#[test]
fn neighbouring_tab_wells_keep_a_rib_between_them() {
    let (cw, ch, gap) = (8.0, 17.0, 6.0);
    let b = crate::reliefwell::boxes(&nav());
    assert_eq!(b.len(), 2);
    let r: Vec<Rect> = b
        .iter()
        .map(|b| well_rect(b, (12.0, 27.0), cw, ch, gap))
        .collect();
    assert!((r[1].y - (r[0].y + r[0].h) - gap).abs() < 1e-3, "{r:?}");
    assert_eq!(r[0].y, 27.0 + 9.0 * ch);
}

/// Beside the activity bar the panel starts `x0` columns in: its sections are found there, the
/// bar's own rule (a vertical stroke, and its icons) is no section, and only the panel's
/// rules are blanked.
#[test]
fn beside_the_bar_the_sections_are_found_where_the_panel_starts() {
    let x0 = 6u16;
    let mut v: Vec<CellView> = nav()
        .into_iter()
        .map(|c| CellView {
            col: c.col + x0,
            ..c
        })
        .collect();
    // The bar: a horizontal rule at ITS column 1 on every row must not read as a heading.
    for r in 0..12 {
        row(&mut v, r, " \u{2500}   \u{2502}");
    }
    assert_eq!(heads(&nav(), 0), heads(&v, x0));
    unruled(&mut v, x0);
    let bar_rule = v.iter().filter(|c| c.col == 1 && c.c == '\u{2500}').count();
    assert_eq!(bar_rule, 12, "the bar's cells are not the panel's to blank");
    let p = groove_paint(&[2.5], x0, 20, 16.0);
    assert!(
        p.iter().all(|p| p.x >= f32::from(x0)),
        "grooves start past the bar"
    );
}
