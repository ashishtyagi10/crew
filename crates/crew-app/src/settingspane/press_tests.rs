//! Clicks, end to end: the hit test (`super::click`) plus what pressing the
//! thing it found does.
//!
//! The geometry is read back out of the same layout the renderer draws from,
//! so these tests ask the layout where a control is and then click there —
//! never a hard-coded row, which would pass while the form moved under it.
use super::*;
use crate::config::CrewConfig;
use crate::settingspane::labels::value_of;

const COLS: u16 = 160;
const ROWS: u16 = 200; // tall enough that nothing scrolls

fn pane() -> SettingsPane {
    SettingsPane::new(
        CrewConfig::default(),
        vec!["Lilex".into(), "SF Mono".into()],
    )
}

/// The middle of `f`'s box, in pane coordinates.
fn at(f: Field) -> (u16, u16) {
    let r = form::layout(COLS).rect_of(f).expect("a rect");
    (r.y + r.height / 2, r.x + r.width / 2)
}

#[test]
fn clicking_a_box_focuses_that_field() {
    let mut p = pane();
    for f in [Field::Accent, Field::NotifyMinSecs, Field::Theme] {
        let (row, col) = at(f);
        p.click(COLS, ROWS, row, col);
        assert_eq!(
            p.focused_field(),
            f,
            "click on {f:?} focused something else"
        );
    }
}

#[test]
fn clicking_a_checkbox_flips_it() {
    // A one-row `[x] Label` exists to be pressed; a click that only focused
    // it would visibly do nothing.
    let mut p = pane();
    let before = p.draft.show_nav;
    let (row, col) = at(Field::ShowNav);
    p.click(COLS, ROWS, row, col);
    assert_eq!(p.focused_field(), Field::ShowNav);
    assert_ne!(p.draft.show_nav, before, "the checkbox did not flip");
}

#[test]
fn the_chevrons_of_a_picker_step_it_both_ways() {
    let mut p = pane();
    let r = form::layout(COLS).rect_of(Field::Motion).expect("a rect");
    let (value, _) = value_of(&p, Field::Motion);
    assert!(value.starts_with('\u{2039}'), "not a picker: {value}");
    let (left, right) = (r.x + 1, r.x + value.chars().count() as u16);
    let start = p.draft.motion.clone();

    p.click(COLS, ROWS, r.y + 1, right);
    let forward = p.draft.motion.clone();
    assert_ne!(forward, start, "the right chevron did not step it");

    p.click(COLS, ROWS, r.y + 1, left);
    assert_eq!(p.draft.motion, start, "the left chevron did not step back");
}

#[test]
fn clicking_between_the_chevrons_only_focuses() {
    // The value itself is not a button: pressing the middle of a picker to
    // read it must not change it.
    let mut p = pane();
    let (row, col) = at(Field::Motion);
    let before = p.draft.motion.clone();
    p.click(COLS, ROWS, row, col);
    assert_eq!(p.focused_field(), Field::Motion);
    assert_eq!(p.draft.motion, before);
}

#[test]
fn clicking_the_family_box_opens_its_list_and_a_row_picks_a_font() {
    let mut p = pane();
    let (row, col) = at(Field::FontFamily);
    p.click(COLS, ROWS, row, col);
    assert!(p.family_open, "the list did not open");
    let anchor = form::layout(COLS).rect_of(Field::FontFamily).unwrap();
    // First row under the list's border; index 0 is the system-default label.
    p.click(COLS, ROWS, anchor.y + anchor.height + 1 + 1, anchor.x + 2);
    assert!(!p.family_open, "picking a row left the list open");
    assert_eq!(p.draft.font_family.as_deref(), Some("Lilex"));
}

#[test]
fn the_buttons_answer_on_the_row_they_are_drawn_on() {
    let save = "[ Save \u{2318}S ]".chars().count() as u16;
    let cancel = "[ Cancel esc ]".chars().count() as u16;
    let x0 = COLS - (save + 3 + cancel + 2);

    let mut p = pane();
    assert!(matches!(
        p.click(COLS, ROWS, ROWS - 1, x0 + 1),
        Some(SettingsAction::Apply(_))
    ));
    let mut p = pane();
    assert!(matches!(
        p.click(COLS, ROWS, ROWS - 1, x0 + save + 3 + 1),
        Some(SettingsAction::Cancel)
    ));
    // The gap between them is not a button.
    let mut p = pane();
    assert!(p.click(COLS, ROWS, ROWS - 1, x0 + save + 1).is_none());
}

#[test]
fn a_click_in_the_gap_between_controls_changes_nothing() {
    let mut p = pane();
    let before = p.focused_field();
    // Column 0 is the card's own border, left of every control.
    assert!(p.click(COLS, ROWS, 5, 0).is_none());
    assert_eq!(p.focused_field(), before);
}

#[test]
fn a_click_follows_the_form_when_it_has_scrolled() {
    // The scroll is focus-driven, so a form scrolled to its foot must answer
    // clicks against what is ON SCREEN, not against the virtual rows.
    let mut p = pane();
    let rows = 24; // short enough that the tail scrolls into view
    p.focus = FIELDS.iter().position(|f| *f == Field::Budget7d).unwrap();
    let lay = form::layout(COLS);
    let viewport = rows - 2;
    let off = form::scroll_for(lay.rect_of(Field::Budget7d).unwrap(), lay.height, viewport);
    assert!(off > 0, "nothing scrolled; the test proves nothing");
    let r = lay.rect_of(Field::Budget5h).unwrap();
    p.click(COLS, rows, r.y + 1 - off, r.x + 2);
    assert_eq!(p.focused_field(), Field::Budget5h);
}
