//! A pane's card: what each line says, where, and when it fits.
use super::*;

fn row(index: usize, card: Card) -> PaneRow {
    PaneRow {
        index,
        title: card.name.clone(),
        focused: false,
        activity: false,
        minimized: false,
        attention: None,
        busy: false,
        unread: 0,
        hovered: false,
        card: Some(card),
    }
}

fn card(name: &str, detail: Option<&str>, state: EntryState) -> Card {
    Card {
        name: name.into(),
        detail: detail.map(str::to_string),
        glyph: '✳',
        state,
    }
}

/// The text of `row`, columns in order (gaps as spaces).
pub(crate) fn line(cells: &[CellView], row: u16, cols: u16) -> String {
    let mut s = vec![' '; usize::from(cols)];
    for c in cells.iter().filter(|c| c.row == row && c.col < cols) {
        s[usize::from(c.col)] = c.c;
    }
    s.into_iter().collect::<String>().trim_end().to_string()
}

#[test]
fn a_card_says_who_on_line_one_and_what_on_line_two() {
    let _g = crate::app::theme_test_guard();
    let r = row(7, card("sentinel", Some("Pondering… · 12s"), EntryState::Working));
    let cells = card_cells(&r, r.card.as_ref().unwrap(), 26, 3);
    assert_eq!(line(&cells, 3, 26), " \u{258e} 7 sentinel     working");
    assert_eq!(line(&cells, 4, 26), " \u{258e}   ✳ Pondering… · 12s");
    let word = cells.iter().find(|c| c.row == 3 && c.c == 'w').unwrap();
    assert_eq!(word.fg, crate::palette::accent(), "working wears the accent");
}

/// The state word is kept over the name: a long name is cut, the word stays.
#[test]
fn a_long_name_is_cut_before_the_state_word() {
    let _g = crate::app::theme_test_guard();
    let r = row(1, card("PROVISIONAL-SPECIFICATION", None, EntryState::Waiting));
    let cells = card_cells(&r, r.card.as_ref().unwrap(), 24, 0);
    let one = line(&cells, 0, 24);
    assert!(one.ends_with(" you"), "{one}");
    assert!(one.contains('…'), "{one}");
    let spine = cells.iter().find(|c| c.row == 0 && c.col == 1).unwrap();
    assert_eq!(spine.fg, crew_theme::theme().bell, "waiting: the bell's spine");
}

/// Idle says nothing on line one; the restore `+` and the unread count end
/// line two; the focused card's spine is the wide one, in the accent.
#[test]
fn markers_and_focus() {
    let _g = crate::app::theme_test_guard();
    let mut r = row(2, card("crew", Some("~/code/crew"), EntryState::Idle));
    r.minimized = true;
    r.unread = 12;
    let cells = card_cells(&r, r.card.as_ref().unwrap(), 26, 0);
    assert_eq!(line(&cells, 0, 26), " \u{258e} 2 crew");
    let two = line(&cells, 1, 26);
    assert!(two.ends_with("12 +"), "{two}");
    r.focused = true;
    let cells = card_cells(&r, r.card.as_ref().unwrap(), 26, 0);
    let spine = cells.iter().find(|c| c.row == 0 && c.col == 1).unwrap();
    assert_eq!((spine.c, spine.fg), ('\u{258c}', crate::palette::accent()));
    assert!(!line(&cells, 1, 26).contains("12"), "focused: read, no count");
}

#[test]
fn cards_fit_only_when_every_line_has_a_row() {
    // Flat, 3 panes: 3 rows + 3 second lines + 2 gaps = 8, under a rule.
    assert!(fits(9, 3, 3, 1));
    assert!(!fits(8, 3, 3, 1), "one row short: one-line rows");
    // Two tabs (2 + 1 panes): 3 rows + 4 edges, + 3 second lines + 1 gap.
    assert!(fits(12, 7, 3, 2));
    assert!(!fits(11, 7, 3, 2));
    assert!(!fits(40, 0, 0, 1), "no panes, no cards");
}
