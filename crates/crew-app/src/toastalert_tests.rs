//! A card asking for an answer is not drawn as one reporting a failure.
use crate::toastcard::{card_cells, CardText};

/// The colour of the card's top-left corner (its stroke) and of the legend's
/// first letter, as `card_cells` draws them.
fn stroke_and_legend(legend: &str) -> ((u8, u8, u8), (u8, u8, u8)) {
    let card = CardText {
        text: "agent-7 is waiting for an answer",
        legend,
        repeats: 1,
        alert: true,
        actionable: true,
    };
    let cells = card_cells(&card, 40, 0.0, false);
    let corner = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap().fg;
    let first = legend.chars().next().unwrap();
    let word = cells
        .iter()
        .find(|c| c.row == 0 && c.c == first)
        .unwrap()
        .fg;
    (corner, word)
}

#[test]
fn waiting_is_drawn_in_the_attention_ink_not_the_error_red() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES.iter() {
        crew_theme::set_theme(*id);
        let t = crew_theme::theme();
        let waiting = stroke_and_legend("waiting");
        let failed = stroke_and_legend("failed");
        assert_ne!(waiting.0, failed.0, "{id:?}: waiting wears failed's stroke");
        assert_ne!(waiting.1, failed.1, "{id:?}: waiting wears failed's legend");
        assert_eq!(waiting, (t.ansi[11], t.ansi[11]), "{id:?}");
        assert_eq!(failed, (t.bell, t.bell), "{id:?}: a failure stays red");
        let r = crew_theme::contrast_ratio(t.ansi[11], t.page_bg);
        assert!(r >= 4.5, "{id:?}: the waiting legend reads at {r:.2}");
    }
}

#[test]
fn only_waiting_asks() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    for legend in ["failed", "exited", "error"] {
        assert_eq!(super::ink(legend), t.bell, "{legend}");
    }
}
