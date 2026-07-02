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
    pane.absorb_stats(950, String::new(), 0, 0); // turn-level
    pane.absorb_stats(0, "planner".into(), 4_200, 8_100); // reply-level, ctx 8.1k
    pane.absorb_stats(0, "planner".into(), 2_200, 9_400);
    assert_eq!((pane.tokens, pane.turns), (950, 1));
    assert_eq!(pane.agent_stats.get("planner"), Some(&(2, 6_400)));
    // The latest reply's prompt size is the live context fill.
    assert_eq!(pane.ctx.get("planner"), Some(&9_400));
}

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

#[test]
fn relay_reply_ends_the_hop_and_records_it() {
    let mut p = pane();
    p.absorb_activity("planner".into(), "thinking", "user".into());
    assert_eq!(p.active_names(), ["planner"]);
    // In a relay there is no per-agent idle: the reply message is the signal.
    p.note_reply("planner \u{2192} user");
    assert!(p.active_names().is_empty(), "reply stops the clock");
    assert_eq!(p.pulse.hops().len(), 1, "hop recorded for the waterfall");
    assert!(p.pulse.hist("planner").is_some());
    // Unrelated senders (system notices) never match an agent.
    p.note_reply("crew");
    assert_eq!(p.pulse.hops().len(), 1);
}

#[test]
fn per_agent_idle_records_the_hop_for_fans() {
    let mut p = pane();
    p.absorb_activity("coder".into(), "thinking", "user".into());
    p.absorb_activity("coder".into(), "idle", String::new());
    assert!(p.active_names().is_empty());
    assert_eq!(p.pulse.hops().len(), 1);
}

#[test]
fn turn_over_flushes_stragglers_and_next_turn_resets() {
    let mut p = pane();
    p.absorb_activity("planner".into(), "thinking", "user".into());
    p.absorb_activity(String::new(), "idle", String::new()); // turn over
    assert!(p.active_names().is_empty());
    assert_eq!(
        p.pulse.hops().len(),
        1,
        "cancelled hop still on the waterfall"
    );
    // The next turn's first hop starts a fresh waterfall.
    p.absorb_activity("coder".into(), "thinking", "user".into());
    assert!(p.pulse.hops().is_empty());
    assert!(p.engaged(), "an active agent keeps the pulse block on");
}

#[test]
fn pulse_lanes_gate_on_height_and_engagement() {
    let mut p = pane();
    p.agents = vec![
        AgentInfo {
            name: "planner".into(),
            role: String::new(),
            model: "m".into(),
        },
        AgentInfo {
            name: "coder".into(),
            role: String::new(),
            model: "m".into(),
        },
    ];
    assert_eq!(p.pulse_lanes(20), 0, "fresh pane keeps the roster rows");
    assert_eq!(p.top_rows(20), 2);
    p.absorb_stats(950, String::new(), 0, 0); // a turn ran
    assert_eq!(p.pulse_lanes(20), 2, "engaged + tall → one lane per agent");
    assert_eq!(p.top_rows(20), 4, "header + 2 lanes + waterfall");
    assert_eq!(p.pulse_lanes(10), 0, "short pane falls back");
}

// `on_key` takes a winit `KeyEvent`, which is #[non_exhaustive] and awkward
// to construct in a unit test (see `chatkeys.rs`'s comment on the same
// tradeoff). These tests instead exercise the routing boundary `on_key`
// calls: `chatpalette::popup_key`/`after_edit` against a real `ChatPane`'s
// `palette`/`input` fields, which is what `on_key` does internally.
#[test]
fn typing_slash_opens_the_palette_and_esc_closes_it_not_the_pane() {
    use crate::chatkeys::ChatInput;
    use crate::chatpalette::{self, PaletteKey};

    let mut p = pane();
    p.input.push('/');
    chatpalette::after_edit(&mut p.palette, &p.input, &p.agents);
    assert!(p.palette.is_some(), "leading '/' opens the command palette");

    // Esc is consumed by the palette (mirrors on_key's routing: the palette
    // gets first look at every key), so on_key would return None here, never
    // `Some(ChatAction::Close)` — Esc closes the popup, not the pane.
    assert!(matches!(
        chatpalette::popup_key(&mut p.palette, &mut p.input, &ChatInput::Close),
        PaletteKey::Consumed
    ));
    assert!(p.palette.is_none(), "Esc closed the popup");
}

#[test]
fn palette_and_file_mention_are_mutually_exclusive() {
    // A leading '/' or '@' opens the palette but never the file mention
    // (pending_mention requires a non-leading token); a mid-line '@' opens
    // the mention but never the palette (pending_palette requires the
    // leading token). At most one popup is ever open.
    let mut p = pane();
    p.input = "/mo".to_string();
    crate::chatpalette::after_edit(&mut p.palette, &p.input, &p.agents);
    crate::chatmention::after_edit(&mut p.mention, &p.input, || vec!["mod.rs".into()]);
    assert!(p.palette.is_some());
    assert!(p.mention.is_none());

    let mut p = pane();
    p.input = "hey @mo".to_string();
    crate::chatpalette::after_edit(&mut p.palette, &p.input, &p.agents);
    crate::chatmention::after_edit(&mut p.mention, &p.input, || vec!["mod.rs".into()]);
    assert!(p.palette.is_none());
    assert!(p.mention.is_some());
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
