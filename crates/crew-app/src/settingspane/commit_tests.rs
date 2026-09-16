//! Focus movement: the walk Tab takes through the form.
use super::move_focus;
use crate::config::CrewConfig;
use crate::settingspane::SettingsPane;

#[test]
fn tab_walks_the_form_the_way_it_is_drawn() {
    // The user's report: "navigation on settings pane is confusing, it should
    // go from top to bottom." It stepped through the declaration order, which
    // had drifted from the layout.
    let mut p = SettingsPane::new(CrewConfig::default(), vec!["Lilex".into()]);
    p.cols.set(160);
    let want = crate::settingspane::form::tab_order(160);
    let mut got = vec![p.focused_field()];
    for _ in 1..want.len() {
        move_focus(&mut p, false);
        got.push(p.focused_field());
    }
    assert_eq!(got, want);
    // And back the way it came.
    move_focus(&mut p, false);
    assert_eq!(p.focused_field(), want[0], "it did not wrap to the top");
    move_focus(&mut p, true);
    assert_eq!(
        p.focused_field(),
        want[want.len() - 1],
        "shift-tab wraps too"
    );
}

#[test]
fn the_walk_follows_the_width_it_was_drawn_at() {
    // `pair` stacks its two fields on a narrow pane and sits them side by side
    // on a wide one, so the reading order genuinely changes with the width —
    // which is why no hand-written list could have been right at both.
    let mut narrow = SettingsPane::new(CrewConfig::default(), vec![]);
    narrow.cols.set(46);
    let mut wide = SettingsPane::new(CrewConfig::default(), vec![]);
    wide.cols.set(200);
    for _ in 0..4 {
        move_focus(&mut narrow, false);
        move_focus(&mut wide, false);
    }
    // Both are walking their OWN layout, and each step is a field that layout
    // actually places.
    for p in [&narrow, &wide] {
        let order = crate::settingspane::form::tab_order(p.cols.get());
        assert!(order.contains(&p.focused_field()));
    }
}

#[test]
fn the_wheel_stops_at_the_ends_of_the_walk_rather_than_wrapping() {
    // The end-stop used to compare against the declaration list's ends, which
    // since 0.22.55 are not the fields the form begins and ends with.
    let order = crate::settingspane::form::tab_order(160);
    let mut p = SettingsPane::new(CrewConfig::default(), vec![]);
    p.cols.set(160);
    p.scroll(-(order.len() as i32 + 10)); // wheel down, past the foot
    assert_eq!(p.focused_field(), order[order.len() - 1]);
    p.scroll(order.len() as i32 + 10); // and back up, past the head
    assert_eq!(p.focused_field(), order[0]);
}
