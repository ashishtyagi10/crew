use super::*;
use crate::cmddefs::commands;

#[test]
fn expands_agrees_with_options_for_across_every_command() {
    // `expands` is a cheap shortcut for "does options_for(cmd).is_some()" —
    // duplicated so the palette never has to build rows just to answer a
    // bool. The two lists must never drift apart: a command in one but not
    // the other would silently gain or lose its value picker.
    for c in commands() {
        assert_eq!(
            expands(c.name),
            options_for(c.name).is_some(),
            "{} : expands() and options_for() disagree",
            c.name
        );
    }
}

/// A closed-set picker that does not mark the value you are on is asking you
/// to remember the choice it exists to save you from making twice.
#[test]
fn the_current_value_is_named_for_the_pickers_that_have_one() {
    use crate::config::CrewConfig;
    let cfg = CrewConfig {
        theme: Some("crt".into()),
        motion: "subtle".into(),
        border_marks: false,
        ..CrewConfig::default()
    };
    assert_eq!(current_value("/theme", &cfg).as_deref(), Some("crt"));
    assert_eq!(current_value("/motion", &cfg).as_deref(), Some("subtle"));
    assert_eq!(current_value("/marks", &cfg).as_deref(), Some("off"));
    // A command whose "current" is not one value has none to mark.
    assert_eq!(current_value("/view", &cfg), None);
    assert_eq!(current_value("/out", &cfg), None);
}

/// A named gradient pair is what the picker offers, so it is what "current"
/// has to say — not the level underneath it.
#[test]
fn a_pinned_gradient_pair_is_the_current_value_rather_than_the_level() {
    use crate::config::CrewConfig;
    let mut cfg = CrewConfig {
        gradient: "lively".into(),
        ..CrewConfig::default()
    };
    assert_eq!(current_value("/gradient", &cfg).as_deref(), Some("lively"));
    let (a, b) = crew_theme::gradients::by_name("ember").unwrap();
    cfg.gradient_poles = Some(crate::gradientcmd::format_poles((a, b)));
    assert_eq!(current_value("/gradient", &cfg).as_deref(), Some("ember"));
}

/// Marking touches the row it names and nothing else — headings included.
#[test]
fn only_the_matching_row_is_marked() {
    let mut items = crate::suggest::menu_items("/motion ");
    crate::suggest::mark_current(&mut items, Some("subtle"));
    let marked: Vec<&str> = items
        .iter()
        .filter(|i| i.desc.ends_with("current"))
        .map(|i| i.label.as_str())
        .collect();
    assert_eq!(marked, vec!["subtle"], "{marked:?}");
    let mut none = crate::suggest::menu_items("/motion ");
    crate::suggest::mark_current(&mut none, None);
    assert!(none.iter().all(|i| !i.desc.ends_with("current")));
}
