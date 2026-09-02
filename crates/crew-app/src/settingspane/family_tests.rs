//! The font family list as a thing to browse — found by shooting the open
//! dropdown (`edgeshot_tests`): it opened on a one-row list.
use super::keys::step_family;
use super::{SettingsPane, DEFAULT_FAMILY_LABEL};
use crate::config::CrewConfig;

fn pane() -> SettingsPane {
    let cfg = CrewConfig {
        font_family: Some("Lilex".into()),
        ..CrewConfig::default()
    };
    SettingsPane::new(
        cfg,
        ["Lilex", "MonoLisa", "SF Mono"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    )
}

/// The field holds the current family when you arrive; that exact name is
/// not a filter, it is where you are.
#[test]
fn a_query_that_is_a_family_lists_every_family() {
    let p = pane();
    assert_eq!(p.family_query, "Lilex");
    let names = p.filtered();
    assert_eq!(
        names,
        [DEFAULT_FAMILY_LABEL, "Lilex", "MonoLisa", "SF Mono"]
    );
    let mut p = pane();
    p.family_query = "mono".into();
    assert_eq!(p.filtered(), [DEFAULT_FAMILY_LABEL, "MonoLisa", "SF Mono"]);
}

#[test]
fn the_first_arrow_opens_on_the_current_family_and_the_next_moves() {
    let mut p = pane();
    assert!(!p.family_open);
    step_family(&mut p, 1);
    assert!(p.family_open);
    assert_eq!(p.family_sel, 1, "the cursor starts on Lilex, not the top");
    step_family(&mut p, 1);
    assert_eq!(p.family_sel, 2);
    step_family(&mut p, -1);
    step_family(&mut p, -1);
    step_family(&mut p, -1);
    assert_eq!(p.family_sel, 0, "clamped at the top");
    step_family(&mut p, 9);
    assert_eq!(p.family_sel, 3, "clamped at the bottom");
}

#[test]
fn opening_on_a_query_that_matches_nothing_exactly_starts_at_the_top() {
    let mut p = pane();
    p.family_query = "mono".into();
    step_family(&mut p, -1);
    assert!(p.family_open);
    assert_eq!(p.family_sel, 0);
}
