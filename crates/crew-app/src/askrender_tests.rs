use super::*;

#[test]
fn render_verdicts_and_codes() {
    let (t, c) = render(&Reply::Answered { text: "v2".into() });
    assert_eq!((t.as_str(), c), ("ANSWERED: v2", 0));

    let (t, c) = render(&Reply::NoAnswer {
        reason: NoAnswer::IdleNoEngage,
        partial: None,
    });
    assert!(t.contains("NO_ANSWER") && t.contains("idle"));
    assert_eq!(c, 2);

    let (t, c) = render(&Reply::NoAnswer {
        reason: NoAnswer::Unreachable,
        partial: None,
    });
    assert_eq!(c, 3, "unreachable is code 3: {t}");

    let (t, _) = render(&Reply::NoAnswer {
        reason: NoAnswer::Stalled,
        partial: Some("half an answer".into()),
    });
    assert!(t.contains("partial") && t.contains("half an answer"));
}

#[test]
fn render_roster_is_a_table_with_ids() {
    let out = render_roster(&[PaneCard {
        id: "p2".into(),
        label: Some("schema".into()),
        kind: "terminal".into(),
        running: Some("claude".into()),
        dir: Some("db".into()),
        busy: false,
    }]);
    assert!(out.contains("p2") && out.contains("schema") && out.contains("claude"));
    assert!(out.contains("idle"));
}

#[test]
fn cast_reply_renders_answers_and_reports_success() {
    let (t, c) = render(&Reply::Cast {
        answers: vec![
            CastAnswer {
                pane: "p0".into(),
                label: Some("schema".into()),
                text: Some("v2".into()),
                no_answer: None,
            },
            CastAnswer {
                pane: "p1".into(),
                label: None,
                text: None,
                no_answer: Some(NoAnswer::IdleNoEngage),
            },
        ],
    });
    assert!(t.contains("schema") && t.contains("v2"), "{t}");
    assert!(t.contains("p1") && t.contains("idle"), "{t}");
    assert_eq!(c, 0, "at least one answered → success");
}

#[test]
fn empty_cast_is_unreachable() {
    let (t, c) = render(&Reply::Cast { answers: vec![] });
    assert!(t.contains("no eligible panes"));
    assert_eq!(c, 3);
}

#[test]
fn roster_reply_renders_via_render() {
    // Reply::Roster routes through render_roster (exercises the render arm too).
    let (t, c) = render(&Reply::Roster {
        panes: vec![PaneCard {
            id: "p0".into(),
            label: None,
            kind: "terminal".into(),
            running: None,
            dir: None,
            busy: true,
        }],
    });
    assert!(t.contains("p0") && t.contains("busy"));
    assert_eq!(c, 0);
}
