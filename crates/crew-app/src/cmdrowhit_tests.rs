//! The mark a matched character wears (`cmdrow::hit_style`). Split from
//! `cmdrow_tests.rs`, which is past the line cap.
use super::*;
use crate::suggest::MenuItem;

fn item(label: &str, hit: Vec<usize>) -> MenuItem {
    MenuItem {
        label: label.into(),
        hit,
        ..MenuItem::default()
    }
}

/// A matched character wears the search wash, not just bold: bold is also
/// how the selected row is drawn, so bold marks vanished on the one row
/// being looked at.
#[test]
fn a_matched_character_wears_the_search_wash() {
    let _g = crate::app::theme_test_guard();
    let hl = crew_theme::theme().find_hl_bg;
    let line = spans(&item("/dump", vec![1]), 5, 0, 40, Color::Gray);
    let bg = |i: usize| line.spans[i].style.bg;
    assert_eq!(bg(1), Some(Color::Rgb(hl.0, hl.1, hl.2)), "the hit");
    assert_eq!(bg(0), None, "an unmatched char");
}
