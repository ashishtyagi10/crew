//! The activity bar's Panes section and rail with two tabs: what is drawn is what a click finds.
use super::fix::{app, drawn, CELL};
use crate::app::CrewApp;
use crate::pane::PaneContent;

/// Two tabs: three named panes, two in the first tab and one in the second.
fn tabbed(collapsed: bool) -> CrewApp {
    let mut a = app(720.0, collapsed, 0);
    let named = |n: &str| {
        let mut p = crate::glassworkshot_tests::pane(
            PaneContent::Todo(crate::todopane::TodoPane::new()),
            None,
        );
        p.name = Some(n.into());
        p
    };
    a.panes = vec![named("alpha"), named("bravo")];
    a.focused = 1;
    a.reconcile_grid();
    let second = a.tabs.add();
    a.activate_tab(second);
    a.panes.push(named("charlie"));
    a.focused = 2;
    a.reconcile_grid();
    a
}

/// Where `word` is drawn whole on a pane's ROW of the nav: the lowest place it is written, since
/// a tab card's legend names the tab after its first pane on the edge above.
fn word_at(a: &mut CrewApp, word: &str) -> Option<(f32, f32)> {
    let right = a.nav_hit_geometry()?.0;
    let scenes = a.build_frame();
    let first = word.chars().next()?;
    let inside = |s: &&crew_render::PaneScene| s.x < right.x + right.w && s.x + s.w > right.x;
    scenes
        .iter()
        .filter(inside)
        .flat_map(|s| {
            s.cells
                .iter()
                .filter(|c| c.c == first)
                .filter_map(move |c| {
                    let whole = word.chars().enumerate().all(|(i, ch)| {
                        s.cells
                            .iter()
                            .any(|d| d.row == c.row && d.col == c.col + i as u16 && d.c == ch)
                    });
                    whole.then(|| {
                        let x = s.x + (f32::from(c.col) + 0.5) * CELL.0;
                        (x, s.y + (f32::from(c.row) + 0.5) * CELL.1)
                    })
                })
        })
        .max_by(|p, q| p.1.total_cmp(&q.1))
}

/// With tabs, the Panes section draws each tab as a card around its panes. A click on a pane's
/// drawn title is that pane; a click on a card's edge is that tab.
#[test]
fn with_two_tabs_the_panes_section_answers_for_what_it_drew() {
    let _g = crate::app::theme_test_guard();
    let mut a = tabbed(false);
    a.config.nav_section = "panes".into();
    for (k, name) in ["alpha", "bravo", "charlie"].into_iter().enumerate() {
        let (x, y) = word_at(&mut a, name).unwrap_or_else(|| panic!("{name} drawn"));
        a.cursor = (x, y);
        assert_eq!(a.pane_at_sidebar(), Some(k), "{name}");
    }
    // Every tab edge line the list holds answers for its tab, on the row it is drawn on.
    let (sb, ch, l) = a.nav_hit_geometry().expect("open nav");
    let top = l.panes_top.expect("a list");
    for (k, line) in a.nav_lines().into_iter().enumerate() {
        let (crate::tabnav::Line::Tab(id) | crate::tabnav::Line::Edge(id)) = line else {
            continue;
        };
        let row = top + 1 + k as u16;
        a.cursor = (sb.x + 3.0 * CELL.0, sb.y + (f32::from(row) + 1.5) * ch);
        assert_eq!(a.tab_at_nav(), Some(id), "line {k}");
    }
}

/// On the rail, with tabs, the pane numbers sit under the icons and still go to their panes.
#[test]
fn with_two_tabs_the_rail_numbers_go_to_their_panes() {
    let _g = crate::app::theme_test_guard();
    let mut a = tabbed(true);
    let rule = f32::from(crate::navbar::HEAD_ROWS + 2) * CELL.1;
    // A tab's `─N─` divider carries a number too; a pane's number is the one not on a rule.
    let rules: Vec<f32> = drawn(&mut a, '\u{2500}').iter().map(|p| p.1).collect();
    for (k, n) in ['1', '2', '3'].into_iter().enumerate() {
        let at = drawn(&mut a, n);
        let pane = at
            .iter()
            .filter(|(x, y)| *x < 60.0 && *y > rule && !rules.contains(y));
        let &(x, y) = pane
            .min_by(|p, q| p.1.total_cmp(&q.1))
            .expect("drawn on the rail");
        a.cursor = (x, y);
        assert_eq!(a.pane_at_rail(), Some(k), "pane number {n}");
    }
}
