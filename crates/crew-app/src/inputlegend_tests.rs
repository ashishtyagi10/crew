use super::*;
use std::path::PathBuf;

/// A pane whose title is a whole command line is a tag, not a paragraph: it
/// gets a third of the bar at most, and the rule keeps the rest.
#[test]
fn a_long_pane_name_cannot_eat_the_bottom_rule() {
    let cols = 52u16;
    let (label, _) = bottom(
        None,
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
    assert_eq!(bottom(None, None, Some("zsh"), 52).unwrap().0, " zsh ");
}

/// A flashing status borrows the slot — and gets the rule's real budget, not
/// the tag's: it is a sentence the bar is saying once, not a standing label.
#[test]
fn a_status_flash_borrows_the_slot_and_may_use_the_rule() {
    let _g = crate::app::theme_test_guard();
    let msg = "copied 4 lines to the clipboard";
    let (label, fg) = bottom(None, Some(msg), Some("zsh"), 52).unwrap();
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
    assert!(bottom(None, None, None, 52).is_none());
    assert!(bottom(None, None, Some("   "), 52).is_none());
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
    let legend = top(&deep, 40, 0);
    assert!(
        legend.ends_with("settingspane"),
        "the tail survived: {legend:?}"
    );
}

/// `/closeall` arms a ten-second window in which running it again closes
/// every pane. That is a state, not a moment, and it outranks both the
/// transient status and the standing pane name.
#[test]
fn a_pending_confirmation_owns_the_slot_and_wears_the_bell() {
    let _g = crate::app::theme_test_guard();
    let ask = "close all 4 panes? /closeall again";
    let (label, fg) = bottom(Some(ask), Some("copied 4 lines"), Some("zsh"), 60).unwrap();
    assert!(label.contains(ask), "the question is what shows: {label:?}");
    assert_eq!(fg, crew_theme::theme().bell, "a warning wears the bell");
}

/// Once it is answered or the window shuts, the slot goes back.
#[test]
fn the_slot_goes_back_when_nothing_is_pending() {
    let _g = crate::app::theme_test_guard();
    assert_eq!(bottom(None, None, Some("zsh"), 60).unwrap().0, " zsh ");
}

/// Up does not walk back through everything typed: it recalls only lines
/// starting with what was in the bar when browsing began. Both halves of that
/// were invisible — the recalled line looks exactly like one you typed.
#[test]
fn browsing_history_says_where_you_are_and_what_is_filtering_it() {
    let _g = crate::app::theme_test_guard();
    let hist: Vec<String> = ["git status", "ls -la", "git push", "cargo test", "git log"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    // Not browsing: the slot stays empty, as it always has been.
    assert!(history_tag(&hist, "", None, 60).is_none());

    // One Up with "git" typed lands on the newest match (index 4 of 5) —
    // which is the FIRST step back, not the fifth.
    let (label, fg) = history_tag(&hist, "git", Some(4), 60).unwrap();
    assert!(label.contains("hist 1/3"), "{label:?}");
    assert!(label.contains("git"), "the prefix is named: {label:?}");
    assert_eq!(fg, crew_theme::theme().status_fg);

    // Two more Ups reach the oldest match, which is where Up stops doing
    // anything — the state that used to be indistinguishable from a dead key.
    assert!(history_tag(&hist, "git", Some(0), 60)
        .unwrap()
        .0
        .contains("hist 3/3"));
}

/// Plain recall (nothing typed) counts the whole history and names no filter.
#[test]
fn plain_recall_needs_no_prefix_in_the_tag() {
    let _g = crate::app::theme_test_guard();
    let hist: Vec<String> = (0..12).map(|i| format!("cmd{i}")).collect();
    let label = history_tag(&hist, "", Some(9), 60).unwrap().0;
    assert_eq!(label.trim(), "hist 3/12", "{label:?}");
}

/// A deep path gives way to the tag rather than being overwritten by it.
#[test]
fn the_path_makes_room_for_the_tag() {
    let _g = crate::app::theme_test_guard();
    let deep = std::path::PathBuf::from("/one/two/three/four/five/six/seven/settingspane");
    let cols = 44u16;
    let (tag, _) = history_tag(&[String::from("x")], "", Some(0), cols).unwrap();
    let reserved = crate::chatwidth::str_w(&tag) + 1;
    let with = top(&deep, cols, reserved);
    let without = top(&deep, cols, 0);
    assert!(
        crate::chatwidth::str_w(&with) < crate::chatwidth::str_w(&without),
        "the legend did not give way: {with:?}"
    );
    // Both still end in the directory you are actually in.
    assert!(with.ends_with("settingspane"), "{with:?}");
}
