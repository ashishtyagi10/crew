use super::*;
use crate::{CliAdapter, Normalize};

fn reg(names: &[&str]) -> Registry {
    Registry::new(
        names
            .iter()
            .map(|n| {
                Box::new(CliAdapter {
                    name: (*n).into(),
                    program: "true".into(),
                    args: vec![],
                    normalize: Normalize::Raw,
                }) as Box<dyn crate::Adapter>
            })
            .collect(),
    )
}

fn text(ev: &PluginEvent) -> (&str, &str) {
    match ev {
        PluginEvent::Message { sender, text, .. } => (sender, text),
        _ => ("", ""),
    }
}

#[test]
fn split_target_defaults_to_first_agent() {
    let (s, b) = split_target("do the thing", &reg(&["claude", "codex"]));
    assert_eq!((s.as_str(), b.as_str()), ("claude", "do the thing"));
}

#[test]
fn split_target_honours_at_selector() {
    let (s, b) = split_target("@codex review this", &reg(&["claude", "codex"]));
    assert_eq!((s.as_str(), b.as_str()), ("codex", "review this"));
}

#[test]
fn split_target_ignores_unknown_selector() {
    let (s, b) = split_target("@ghost hi", &reg(&["claude"]));
    assert_eq!((s.as_str(), b.as_str()), ("claude", "@ghost hi"));
}

#[test]
fn dialing_becomes_a_thinking_activity() {
    let hop = Hop {
        from: "broker".into(),
        to: "codex".into(),
        hop: 1,
        kind: HopKind::Dialing,
        text: String::new(),
    };
    match hop_to_msg(&hop, None) {
        PluginEvent::Activity { agent, state, from } => {
            assert_eq!((agent.as_str(), state.as_str()), ("codex", "thinking"));
            assert_eq!(from, "broker", "activity names who dialed the agent");
        }
        ev => panic!("expected Activity, got {ev:?}"),
    }
}

#[test]
fn reply_hop_is_labelled_from_to() {
    let hop = Hop {
        from: "claude".into(),
        to: "codex".into(),
        hop: 0,
        kind: HopKind::Reply,
        text: "here is my analysis".into(),
    };
    let ev = hop_to_msg(&hop, Some(Duration::from_millis(4200)));
    assert_eq!(text(&ev), ("claude → codex", "here is my analysis"));
    match &ev {
        PluginEvent::Message { meta, ts, .. } => {
            assert_eq!(meta, "4.2s");
            assert!(ts.parse::<u64>().is_ok(), "ts should be epoch ms: {ts}");
        }
        _ => panic!("expected Message"),
    }
}

#[test]
fn done_and_error_markers() {
    let mk = |kind, t: &str| Hop {
        from: "a".into(),
        to: "b".into(),
        hop: 0,
        kind,
        text: t.into(),
    };
    assert_eq!(text(&hop_to_msg(&mk(HopKind::Done, ""), None)).1, "[done]");
    assert_eq!(
        text(&hop_to_msg(&mk(HopKind::Error, "x"), None)).1,
        "[error] x"
    );
    assert_eq!(
        text(&hop_to_msg(&mk(HopKind::Terminated, "y"), None)).1,
        "[stopped] y"
    );
}

#[test]
fn turn_summary_times_each_agent_in_order() {
    let segs = vec![
        ("planner".to_string(), Duration::from_millis(4200)),
        ("coder".to_string(), Duration::from_millis(8100)),
    ];
    let s = turn_summary(&segs, 2, 950);
    assert!(s.contains("planner 4.2s → coder 8.1s"), "{s}");
    assert!(s.contains("2 exchange(s)"), "{s}");
    assert!(s.contains("~950 tok"), "{s}");
}

#[test]
fn turn_summary_without_segments_still_reports_cost() {
    let s = turn_summary(&[], 0, 0);
    assert!(s.starts_with("turn done"), "{s}");
    assert!(s.contains("~0 tok"), "{s}");
}

#[test]
fn multi_targets_parses_plus_joined_agents() {
    let r = reg(&["planner", "coder", "reviewer"]);
    let (names, body) = multi_targets("@planner+coder review this", &r).unwrap();
    assert_eq!(names, vec!["planner".to_string(), "coder".to_string()]);
    assert_eq!(body, "review this");
    // Case-insensitive + de-duplicated.
    let (names, _) = multi_targets("@Coder+coder+REVIEWER x", &r).unwrap();
    assert_eq!(names, vec!["coder".to_string(), "reviewer".to_string()]);
}

#[test]
fn multi_targets_rejects_singles_typos_and_plain_tasks() {
    let r = reg(&["planner", "coder"]);
    assert!(
        multi_targets("@planner do it", &r).is_none(),
        "no + selector"
    );
    assert!(
        multi_targets("@planner+ghost do it", &r).is_none(),
        "typo member"
    );
    assert!(
        multi_targets("do it @planner+coder", &r).is_none(),
        "not leading"
    );
    assert!(
        multi_targets("@planner+coder", &r).is_none(),
        "no task body"
    );
}
