use super::*;

#[test]
fn classify_spawn_pane_returns_host_action() {
    let ev = PluginEvent::SpawnPane {
        command: "sh".into(),
        args: vec![],
        label: "x".into(),
    };
    let result = classify(&ev);
    assert_eq!(
        result,
        Some(HostAction::SpawnPane {
            command: "sh".into(),
            args: vec![],
            label: "x".into(),
        })
    );
}

#[test]
fn classify_message_returns_none() {
    let ev = PluginEvent::Message {
        channel: "general".into(),
        sender: "bob".into(),
        text: "hello".into(),
        ts: "t".into(),
        meta: String::new(),
    };
    assert_eq!(classify(&ev), None);
}

#[test]
fn stats_events_split_turn_and_agent_totals() {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut pane = ChatPane::new(plugin, "crew".into());
    pane.absorb_stats(950, String::new(), 0); // turn-level
    pane.absorb_stats(0, "planner".into(), 4_200); // reply-level
    pane.absorb_stats(0, "planner".into(), 2_200);
    assert_eq!((pane.tokens, pane.turns), (950, 1));
    assert_eq!(pane.agent_stats.get("planner"), Some(&(2, 6_400)));
}

#[test]
fn classify_send_pane_returns_host_action() {
    let ev = PluginEvent::SendPane {
        label: "a".into(),
        text: "hi".into(),
    };
    assert_eq!(
        classify(&ev),
        Some(HostAction::SendPane {
            label: "a".into(),
            text: "hi".into(),
        })
    );
}
