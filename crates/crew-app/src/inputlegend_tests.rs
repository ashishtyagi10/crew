use super::*;
use std::path::PathBuf;

/// A pane whose title is a whole command line is a tag, not a paragraph: it
/// gets a third of the bar at most, and the rule keeps the rest.
#[test]
fn a_long_pane_name_cannot_eat_the_bottom_rule() {
    let cols = 52u16;
    let (label, _) = bottom(
        None,
        Some("cargo test --workspace -p crew-app --bin crew"),
        cols,
    )
    .expect("a pane name is a tag");
    assert!(
        label.chars().count() <= tag_budget(cols) + 2,
        "tag {label:?} overran its budget"
    );
    assert!(
        label.ends_with("\u{2026} "),
        "clipped names ellipsize: {label:?}"
    );
    // Rule on both sides of it: the label plus its two corners and spacer
    // columns must still leave the majority of the row to the border.
    assert!(
        label.chars().count() * 2 < cols as usize,
        "the tag took more than half the rule"
    );
}

/// A short name is not padded out to the budget — it is exactly itself.
#[test]
fn a_short_pane_name_is_left_alone() {
    assert_eq!(bottom(None, Some("zsh"), 52).unwrap().0, " zsh ");
}

/// A flashing status borrows the slot — and gets the rule's real budget, not
/// the tag's: it is a sentence the bar is saying once, not a standing label.
#[test]
fn a_status_flash_borrows_the_slot_and_may_use_the_rule() {
    let _g = crate::app::theme_test_guard();
    let msg = "copied 4 lines to the clipboard";
    let (label, fg) = bottom(Some(msg), Some("zsh"), 52).unwrap();
    assert!(
        label.contains(msg),
        "status was clipped to a tag: {label:?}"
    );
    assert_eq!(fg, crew_theme::theme().status_fg);
    assert!(
        label.chars().count() > tag_budget(52),
        "a status may outrun the tag budget"
    );
}

/// No pane and no status leaves the rule unbroken.
#[test]
fn nothing_to_say_draws_nothing() {
    assert!(bottom(None, None, 52).is_none());
    assert!(bottom(None, Some("   "), 52).is_none());
}

/// The narrowest bar still gets a usable tag rather than a bare ellipsis.
#[test]
fn a_narrow_bar_keeps_a_readable_tag() {
    assert!(tag_budget(12) >= TAG_MIN);
    assert!(tag_budget(4000) <= TAG_MAX);
}

/// The top legend keeps the tail of a deep path — the directory you are in is
/// the part that matters.
#[test]
fn the_top_legend_keeps_the_current_directory() {
    let deep = PathBuf::from("/one/two/three/four/five/six/seven/eight/settingspane");
    let legend = top(&deep, 40);
    assert!(
        legend.ends_with("settingspane"),
        "the tail survived: {legend:?}"
    );
}
