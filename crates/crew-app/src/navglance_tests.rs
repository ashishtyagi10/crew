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
