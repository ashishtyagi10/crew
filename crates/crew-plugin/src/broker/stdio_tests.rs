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
fn dialing_names_the_agent_being_called() {
    let hop = Hop {
        from: "broker".into(),
        to: "codex".into(),
        hop: 1,
        kind: HopKind::Dialing,
        text: String::new(),
    };
    let (sender, t) = {
        let ev = hop_to_msg(&hop);
        let (s, x) = text(&ev);
        (s.to_string(), x.to_string())
    };
    assert_eq!(sender, "crew");
    assert!(t.starts_with("calling codex"), "{t}");
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
    let ev = hop_to_msg(&hop);
    assert_eq!(text(&ev), ("claude → codex", "here is my analysis"));
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
    assert_eq!(text(&hop_to_msg(&mk(HopKind::Done, ""))).1, "[done]");
    assert_eq!(text(&hop_to_msg(&mk(HopKind::Error, "x"))).1, "[error] x");
    assert_eq!(
        text(&hop_to_msg(&mk(HopKind::Terminated, "y"))).1,
        "[stopped] y"
    );
}

#[test]
fn roster_lists_or_explains() {
    assert!(roster(&reg(&["claude", "codex"])).contains("claude, codex"));
    assert!(roster(&reg(&[])).contains("ANTHROPIC_API_KEY"));
}
