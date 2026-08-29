use super::*;

/// The changelog is markdown; this screen has no renderer. The first frame of
/// the app was reading ``new in 0.19.53 · `/keys`, shot whole`` — backticks
/// and all — because the headline was lifted from the file verbatim.
#[test]
fn the_headline_arrives_as_text_not_as_markdown() {
    let line = whats_new(120).expect("a changelog headline");
    assert!(!line.contains('`'), "markdown leaked through: {line:?}");
    assert!(!line.contains('*'), "markdown leaked through: {line:?}");
    assert!(line.starts_with("new in "), "{line:?}");
}

#[test]
fn plain_strips_only_the_marks_markdown_uses() {
    assert_eq!(plain("`/keys`, **shot** whole"), "/keys, shot whole");
    assert_eq!(plain("nothing to strip"), "nothing to strip");
}

/// A line one column narrower than the card sits *against* the frame stroke.
/// Every centred line keeps air on both sides.
#[test]
fn no_line_is_allowed_to_touch_the_frame() {
    for cols in 20u16..90 {
        if let Some(h) = hint_for(cols) {
            let w = h.chars().count();
            assert!(
                w + 2 * MARGIN <= cols as usize,
                "{cols}: hint {w} wide leaves no margin"
            );
        }
        if let Some(line) = whats_new(cols as usize) {
            assert!(
                line.chars().count() + 2 * MARGIN <= cols as usize,
                "{cols}: headline {line:?} leaves no margin"
            );
        }
    }
}

/// The hint is the one line telling a new user what to press, and every
/// character of it used to be the same muted grey. The chords wear the
/// accent; the words do not.
#[test]
fn the_chords_are_the_only_coloured_thing_on_the_first_screen() {
    let key = (1, 2, 3);
    let word = (9, 9, 9);
    let hint = hint_for(80).expect("a hint fits 80 columns");
    let spans = hint_spans(hint, key, word);
    assert_eq!(spans.len(), hint.chars().count(), "every char is placed");
    let coloured: String = spans
        .iter()
        .filter(|&&(_, fg)| fg == key)
        .map(|&(c, _)| c)
        .collect();
    assert!(coloured.contains("Cmd+T"), "the shell chord: {coloured:?}");
    assert!(coloured.contains("Cmd+J"), "the agents chord: {coloured:?}");
    assert!(coloured.contains('/'), "the palette chord: {coloured:?}");
    assert!(
        !coloured.contains("shell") && !coloured.contains("agents"),
        "the words stayed muted: {coloured:?}"
    );
}

/// A hint too wide for the card is dropped, not squeezed.
#[test]
fn a_card_too_narrow_for_any_hint_shows_none() {
    assert!(hint_for(8).is_none());
}

/// The offer line's one typable token is coloured like every other.
#[test]
fn the_restore_command_is_a_chord_too() {
    let key = (1, 2, 3);
    let coloured: String = hint_spans(&restore_hint(3), key, (9, 9, 9))
        .iter()
        .filter(|&&(_, fg)| fg == key)
        .map(|&(c, _)| c)
        .collect();
    assert!(coloured.contains("/restore"), "{coloured:?}");
    assert!(!coloured.contains("panes"), "{coloured:?}");
}

#[test]
fn the_restore_offer_counts_its_panes() {
    assert!(restore_hint(1).contains("1 pane from"));
    assert!(restore_hint(4).contains("4 panes from"));
}
