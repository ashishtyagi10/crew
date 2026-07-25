//! Bare `/model ` + Enter must list every agent's model, never reset it —
//! split from `chatpalette_tests.rs` (child test module) to keep that file
//! under the house line cap, same pattern as `modelpick.rs`'s split
//! children.
use super::*;
use crate::chatkeys::ChatInput;

#[test]
fn bare_model_space_untouched_enter_submits_the_raw_listing() {
    let mut p = None;
    after_edit(&mut p, "/model ", None, Vec::new);
    assert_eq!(p.as_ref().unwrap().kind, Kind::Model);
    assert!(!p.as_ref().unwrap().touched);

    let mut input = "/model ".to_string();
    let key = popup_key(&mut p, &mut input, &ChatInput::Enter);
    assert!(matches!(key, PaletteKey::Submit));
    // The raw input is untouched — NOT rewritten to the destructive
    // "/model all default", which would clear every agent's pin.
    assert_eq!(input, "/model ");
    assert!(p.is_none());
}

#[test]
fn model_selection_moved_then_enter_picks_the_highlighted_row() {
    let mut p = None;
    after_edit(&mut p, "/model ", None, Vec::new);
    let mut input = "/model ".to_string();
    // Move the selection at least once — now Enter must pick, not list.
    assert!(matches!(
        popup_key(&mut p, &mut input, &ChatInput::Down),
        PaletteKey::Consumed
    ));
    assert!(p.as_ref().unwrap().touched);

    let key = popup_key(&mut p, &mut input, &ChatInput::Enter);
    assert!(matches!(key, PaletteKey::Submit));
    assert!(
        input.starts_with("/model all "),
        "a touched selection still picks the highlighted row: {input}"
    );
    assert!(p.is_none());
}

#[test]
fn typing_past_the_empty_query_also_makes_enter_pick_not_list() {
    // Even with the selection never moved, once the user has typed a
    // character the query is no longer empty — Enter must pick that filtered
    // row rather than falling through to the raw-listing shortcut.
    let mut p = None;
    after_edit(&mut p, "/model son", None, Vec::new);
    assert!(!p.as_ref().unwrap().touched);
    let mut input = "/model son".to_string();
    let key = popup_key(&mut p, &mut input, &ChatInput::Enter);
    assert!(matches!(key, PaletteKey::Submit));
    assert!(input.starts_with("/model all "), "{input}");
}
