//! Each section's division of the panel.
use super::*;

const GLANCE: Tail = Tail::Glance {
    waiting: 2,
    weather: 0,
    inbox: 0,
    serving: 4,
};

#[test]
fn only_the_panes_section_has_a_pane_list() {
    for s in NavSection::ALL {
        let l = layout_for(s, 40, GLANCE, 10, 3);
        assert_eq!(l.panes_top.is_some(), s == NavSection::Panes, "{s:?}");
    }
    let empty = layout_for(NavSection::Panes, 40, GLANCE, 10, 0);
    assert_eq!(empty.panes_top, None, "no panes, no list");
}

#[test]
fn only_activity_has_a_log_or_cards() {
    for s in NavSection::ALL {
        let l = layout_for(s, 40, GLANCE, 10, 3);
        let any = l.log_lines + l.waiting_lines + usize::from(l.serving_rows);
        assert_eq!(any > 0, s == NavSection::Activity, "{s:?}: {l:?}");
    }
}

/// Under the cards, the LOG takes the rest of the column and no more.
#[test]
fn activity_puts_the_log_under_the_cards_and_inside_the_panel() {
    for rows in [12u16, 20, 40, 90] {
        let l = layout_for(NavSection::Activity, rows, GLANCE, 200, 0);
        let cards_end = l.waiting_top + l.waiting_lines as u16 + 2;
        if l.log_lines > 0 {
            assert!(l.log_top >= cards_end, "rows={rows}: {l:?}");
            assert!(l.log_top + l.log_block() <= rows, "rows={rows}: {l:?}");
        }
    }
    let tall = layout_for(NavSection::Activity, 90, GLANCE, 200, 0);
    assert!(
        tall.log_lines > LOG_MIN * 10,
        "a tall panel is mostly LOG: {tall:?}"
    );
}

#[test]
fn with_nav_log_activity_is_the_log_alone() {
    let l = layout_for(NavSection::Activity, 30, Tail::Log(100), 100, 3);
    assert_eq!((l.log_top, l.waiting_lines, l.serving_rows), (0, 0, 0));
    assert_eq!(
        l.log_lines, 28,
        "the whole panel, less the rule and the gap"
    );
}
