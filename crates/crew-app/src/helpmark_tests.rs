//! The wash lands on the match, in any case, and nowhere else.
use super::*;
use ratatui::style::{Color, Style};

fn washed(spans: &[Span<'_>]) -> Vec<String> {
    let hl = crew_theme::theme().find_hl_bg;
    spans
        .iter()
        .filter(|s| s.style.bg == Some(Color::Rgb(hl.0, hl.1, hl.2)))
        .map(|s| s.content.to_string())
        .collect()
}

#[test]
fn every_match_is_washed_whatever_its_case() {
    let _g = crate::app::theme_test_guard();
    let base = Style::new().fg(Color::Rgb(200, 200, 200));
    let s = marked("Swap the focused Pane with that pane", "pane", base);
    assert_eq!(washed(&s), ["Pane", "pane"]);
    let text: String = s.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(text, "Swap the focused Pane with that pane", "nothing lost");
}

#[test]
fn no_needle_and_no_match_mark_nothing() {
    let _g = crate::app::theme_test_guard();
    let base = Style::new();
    assert!(washed(&marked("Next / previous pane", "", base)).is_empty());
    assert!(washed(&marked("Next / previous pane", "zoom", base)).is_empty());
    assert!(
        washed(&marked("é", "ée", base)).is_empty(),
        "a needle longer than the text"
    );
}
