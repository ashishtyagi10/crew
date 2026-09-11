use super::*;

/// Blocked panes lead, then plans, then running counts — however the panes
/// are ordered — each row naming its pane; nothing at all is one quiet row.
#[test]
fn rows_are_ordered_by_urgency_and_never_empty() {
    let s = |blocked, plan, running| Signal {
        blocked,
        plan,
        running,
    };
    let rows = rows_from(vec![
        (0, "smith".to_string(), s(false, false, 2)),
        (1, "zsh".to_string(), s(true, false, 0)),
        (2, "smith 2".to_string(), s(false, true, 1)),
    ]);
    let kinds: Vec<Wait> = rows.iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        [Wait::Blocked, Wait::Plan, Wait::Running, Wait::Running]
    );
    assert_eq!(rows[0].text, "\u{2691} zsh");
    assert_eq!(rows[0].pane, Some(1));
    assert_eq!(rows[1].text, "\u{21b5} plan \u{00b7} smith 2");
    assert_eq!(rows[2].text, "\u{25b6} 2 running \u{00b7} smith");
    let quiet = rows_from(vec![(0, "zsh".to_string(), Signal::default())]);
    assert_eq!(quiet.len(), 1);
    assert_eq!(quiet[0].kind, Wait::Quiet);
    assert_eq!(quiet[0].pane, None);
    assert_eq!(rows_from(Vec::new())[0].kind, Wait::Quiet);
}

/// The layout (`nav_tail`) and the click (`waiting_pane_at`) read the cards
/// BEFORE the pointer is applied. The hover walks `sidebar_row` →
/// `nav_hit_geometry` → `nav_tail`; if `nav_tail` read `glance()` the two
/// would recurse until the stack was gone — v0.22.0 to v0.22.2 aborted on
/// the first frame that way. Pinned at the source, since the cycle needs a
/// live renderer to run.
#[test]
fn layout_and_click_read_the_cards_before_the_hover() {
    let src = include_str!("navglance.rs");
    let body = |name: &str| {
        let at = src.find(&format!("fn {name}(")).expect(name);
        let open = src[at..].find('{').unwrap() + at;
        let close = src[open..].find("\n    }\n").unwrap() + open;
        &src[open..close]
    };
    for f in ["nav_tail", "waiting_pane_at", "glance_base"] {
        assert!(
            !body(f).contains("self.glance()") && !body(f).contains("hovered_waiting_row"),
            "{f} must not read the hovered cards"
        );
    }
    assert!(body("nav_tail").contains("self.glance_base()"));
}
