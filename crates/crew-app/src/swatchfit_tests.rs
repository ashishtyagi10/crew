//! `/theme` at a quarter tile — found by shooting it at 380 px
//! (`pickshot_tests`, `pick-theme-narrow.png`).
use crate::cmdrow::{label_col, spans, swatch_col};
use ratatui::style::Color;

const DIM: Color = Color::Rgb(120, 130, 140);

fn text(l: &ratatui::text::Line<'static>) -> String {
    l.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// The first two words of a description, as a wide enough row shows them.
fn lead(desc: &str) -> String {
    let two = desc.split(' ').take(2).collect::<Vec<_>>().join(" ");
    // A cut on a word drops the comma after it: `Tokyo Night…`.
    two.trim_end_matches([',', ';']).to_string()
}

/// At 40 columns `auto`'s fifteen chips set a column that left every
/// description ten columns (`black…`, `rotatin…`): each row now shows its
/// first word whole, in a column with room for a few words of it.
#[test]
fn a_narrow_theme_list_keeps_its_descriptions() {
    let _g = crate::app::theme_test_guard();
    let items = crate::suggest::menu_items("/theme ");
    assert!(swatch_col(&items) > 8, "auto's strip is the widest");
    let avail = 40;
    let lw = label_col(&items, avail);
    let sw = super::col(&items, lw, avail);
    for i in items.iter().filter(|i| !i.header && !i.desc.is_empty()) {
        let row = text(&spans(i, lw, sw, avail, DIM));
        assert!(row.chars().count() <= avail, "{row:?}");
        // Its first word whole, and room past it: `black…` said nothing.
        let first = i
            .desc
            .split(' ')
            .next()
            .unwrap()
            .trim_end_matches([',', ':']);
        let at = row
            .find(first)
            .unwrap_or_else(|| panic!("{}: {row:?}", i.label));
        let room = avail - row[..at].chars().count();
        assert!(room >= 16, "{}: {room} columns: {row:?}", i.label);
        let chips = i.swatch.len().min(sw);
        assert!(
            i.swatch.is_empty() || chips > 0,
            "{}: lost its colour",
            i.label
        );
    }
}

/// A card wide enough for the whole strip keeps every chip: the cut is for
/// narrow cards only.
#[test]
fn a_wide_list_keeps_the_whole_strip() {
    let _g = crate::app::theme_test_guard();
    let items = crate::suggest::menu_items("/theme ");
    let lw = label_col(&items, 100);
    assert_eq!(super::col(&items, lw, 100), swatch_col(&items));
    assert_eq!(super::col(&[], lw, 10), 0, "no swatch, no column");
}

/// A row's strip longer than the column is cut to it, so its description
/// starts where the one-chip rows' do.
#[test]
fn a_strip_longer_than_the_column_is_cut_to_it() {
    let _g = crate::app::theme_test_guard();
    let items = crate::suggest::menu_items("/theme ");
    let auto = items.iter().find(|i| i.label == "auto").unwrap();
    let dark = items.iter().find(|i| i.label == "dark").unwrap();
    let lw = label_col(&items, 40);
    let sw = super::col(&items, lw, 40);
    let at = |i: &crate::suggest::MenuItem| {
        let t = text(&spans(i, lw, sw, 40, DIM));
        t[..t.find(&lead(&i.desc)).unwrap()].chars().count()
    };
    assert_eq!(at(auto), at(dark));
}
