//! The activity bar, as the app draws it and as a click finds it.
use crate::navbar::BarHit;
use crate::navsection::NavSection;

use super::fix::{app, drawn, drawn_icon, hits, CELL};

/// THE seam: every icon the frame draws answers a click on itself — open and collapsed, a tall
/// window and a short one, with and without the update card standing over the nav.
#[test]
fn every_drawn_icon_answers_a_click_on_itself() {
    let _g = crate::app::theme_test_guard();
    for collapsed in [false, true] {
        for h in [720.0, 260.0] {
            for parked in [false, true] {
                let mut a = app(h, collapsed, 3);
                if parked {
                    a.parked_update = Some(("9.9.9".into(), 0));
                }
                let mut found = 0;
                for hit in hits() {
                    let Some((x, y)) = drawn_icon(&mut a, hit) else {
                        continue; // no room for it in this window
                    };
                    found += 1;
                    a.cursor = (x, y);
                    assert_eq!(
                        a.bar_at_cursor(),
                        Some(hit),
                        "collapsed={collapsed} h={h} parked={parked}: {hit:?} at {x},{y}"
                    );
                }
                assert!(
                    found >= 4,
                    "collapsed={collapsed} h={h}: only {found} icons drawn"
                );
            }
        }
    }
}

/// A click on the bar is never a click on the panel beside it, even on the row the pane list
/// starts on.
#[test]
fn the_bar_is_not_the_panel() {
    let _g = crate::app::theme_test_guard();
    let mut a = app(720.0, false, 3);
    let (x, y) = drawn_icon(&mut a, BarHit::Section(NavSection::Panes)).expect("drawn");
    a.cursor = (x, y);
    assert!(
        a.pane_at_sidebar().is_none(),
        "the Panes icon is not pane 1"
    );
}

/// The pane numbers under the collapsed bar's icons still go to their panes.
#[test]
fn a_collapsed_pane_number_still_goes_to_its_pane() {
    let _g = crate::app::theme_test_guard();
    let mut a = app(720.0, true, 3);
    for (k, n) in ['1', '2', '3'].into_iter().enumerate() {
        let at = drawn(&mut a, n);
        // Under the rule: a badge on the icons above can be a digit too.
        let rule = f32::from(crate::navbar::HEAD_ROWS + 2) * CELL.1;
        let below = at.iter().filter(|(x, y)| *x < 60.0 && *y > rule);
        let &(x, y) = below
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("drawn on the rail");
        a.cursor = (x, y);
        assert_eq!(a.pane_at_rail(), Some(k), "pane number {n}");
    }
}

/// VS Code's rules: another icon switches; the open one puts the panel away; any icon on a
/// put-away panel opens it on that section.
#[test]
fn icons_switch_close_and_open_the_panel() {
    let _g = crate::app::theme_test_guard();
    let mut a = app(720.0, false, 0);
    a.pick_nav_section(NavSection::System);
    assert_eq!(
        (a.nav_section(), a.config.nav_collapsed),
        (NavSection::System, false)
    );
    a.pick_nav_section(NavSection::System);
    assert!(
        a.config.nav_collapsed,
        "the open section's icon puts the panel away"
    );
    a.pick_nav_section(NavSection::Activity);
    assert_eq!(
        (a.nav_section(), a.config.nav_collapsed),
        (NavSection::Activity, false)
    );
}

/// The panel keeps the width it was given; the bar stands beside it, and the resize edge moves
/// with the bar so dragging it still sets the panel's width.
#[test]
fn the_bar_widens_the_nav_and_the_edge_follows() {
    let mut a = app(720.0, false, 0);
    let bar = a.nav_bar_w();
    assert!((bar - 6.0 * CELL.0).abs() < 0.01, "{bar}");
    assert!((a.nav_px(1.0) - (a.config.nav_width + bar)).abs() < 0.01);
    a.cursor = (a.config.nav_width + bar, 300.0);
    assert!(a.cursor_on_nav_edge());
    a.nav_drag = true;
    a.cursor = (250.0 + bar, 300.0);
    a.nav_edge_drag();
    assert!(
        (a.config.nav_width - 250.0).abs() < 0.01,
        "{}",
        a.config.nav_width
    );
}

/// The gear is on screen in a full-height window, open and collapsed — the collapsed card is
/// dropped level with the panes, and the gear on its last row used to be clipped off.
#[test]
fn the_gear_is_on_screen_in_both_states() {
    let _g = crate::app::theme_test_guard();
    for collapsed in [false, true] {
        let mut a = app(720.0, collapsed, 2);
        let gear = drawn_icon(&mut a, BarHit::Settings);
        assert!(gear.is_some(), "collapsed={collapsed}");
    }
}
