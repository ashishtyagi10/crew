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
fn cells_render_session_line_agent_chips_and_waterfall() {
    let mut p = pane();
    p.agents = vec![
        crew_plugin::AgentInfo {
            name: "planner".into(),
            role: String::new(),
            model: "qwen".into(),
        },
        crew_plugin::AgentInfo {
            name: "coder".into(),
            role: String::new(),
            model: "qwen".into(),
        },
    ];
    p.absorb_stats(950, String::new(), 0, 0);
    p.pulse.record_hop("planner", 1200);
    p.pulse.end_turn();
    let cells = p.cells(120, 20);
    let text: String = {
        let mut rows: std::collections::BTreeMap<u16, Vec<(u16, char)>> = Default::default();
        for c in &cells {
            rows.entry(c.row).or_default().push((c.col, c.c));
        }
        rows.into_values()
            .map(|mut r| {
                r.sort();
                r.into_iter().map(|(_, c)| c).collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(text.contains("crew"), "session line present:\n{text}");
    assert!(
        text.contains("\u{25b8}planner") || text.contains("\u{25aa}planner"),
        "planner card:\n{text}"
    );
    assert!(
        text.contains("\u{25aa}coder") || text.contains("\u{25b8}coder"),
        "coder card:\n{text}"
    );
    assert!(text.contains("idle"), "state token:\n{text}");
    assert!(text.contains("turn"), "waterfall row:\n{text}");
}

/// Overdraw regression: `status_rows` used to clamp the grid's row count
/// while `cells()`'s renderer drew the grid's *unclamped* row count, so a
/// short pane with more agents than fit could bleed the card grid into the
/// message body. Both now derive from one `chatchips::layout` call with
/// identical `avail`, so the drawn extent can never exceed what `status_rows`
/// reports. Force the cap (8 agents, a short/narrow pane) and confirm it.
#[test]
fn cells_grid_never_overdraws_past_status_rows() {
    let mut p = pane();
    p.agents = (0..8)
        .map(|i| crew_plugin::AgentInfo {
            name: format!("agent{i}"),
            role: String::new(),
            model: "m".into(),
        })
        .collect();
    p.absorb_stats(950, String::new(), 0, 0);
    p.pulse.record_hop("agent0", 1200);
    p.pulse.end_turn();
    let (cols, rows) = (40u16, 9u16);
    let top = p.status_rows(cols, rows);

    let views = p.agent_views();
    let wf = u16::from(!p.pulse.hops().is_empty() && cols >= 30);
    let avail = rows.saturating_sub(2).saturating_sub(1 + wf);
    let lay = crate::chatchips::layout(&views, cols, avail).expect("some rows fit");
    assert_eq!(
        top,
        1 + lay.rows + wf,
        "status_rows matches the shared layout's extent exactly"
    );
    assert!(
        lay.shown < views.len(),
        "the short pane forces capping below all 8 agents: shown={} of {}",
        lay.shown,
        views.len()
    );

    // The renderer draws with this exact `lay` — its cells must never reach
    // or exceed `top` (no rendered row exceeds `status_rows`).
    let grid = crate::chatchips::row_cells(&views, cols, 1, &lay);
    let max_row = grid.iter().map(|c| c.row).max().unwrap_or(0);
    assert!(
        max_row < top,
        "grid content stays inside the status zone: max_row={max_row} top={top}"
    );

    // And the full render pipeline still produces a sane frame at this size.
    let cells = p.cells(cols, rows);
    assert!(!cells.is_empty());
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
}

#[test]
fn status_rows_counts_session_grid_and_waterfall() {
    let mut p = pane();
    p.agents = vec![
        crew_plugin::AgentInfo {
            name: "planner".into(),
            role: String::new(),
            model: "m".into(),
        },
        crew_plugin::AgentInfo {
            name: "coder".into(),
            role: String::new(),
            model: "m".into(),
        },
    ];
    // Idle, wide+tall pane: session line + one row per agent (2 agents), no
    // waterfall yet (no turn has run).
    assert_eq!(p.status_rows(200, 20), 1 + 2);
    // A turn ran → the waterfall row is added.
    p.absorb_stats(950, String::new(), 0, 0);
    p.pulse.record_hop("planner", 1200);
    p.pulse.end_turn();
    assert_eq!(p.status_rows(200, 20), 1 + 2 + 1);
    // Too narrow for any card → just the session line.
    assert_eq!(p.status_rows(3, 20), 1);
    // Too short for even one card row → capped down.
    assert_eq!(p.status_rows(200, 4), 1);
}

// `on_key` takes a winit `KeyEvent`, which is #[non_exhaustive] and awkward
// to construct in a unit test (see `chatkeys.rs`). These drive its testable
// half, `on_input(ChatInput, cwd)`, end-to-end — the real routing, including
// the return value that decides whether the pane closes.
#[test]
fn esc_closes_the_open_palette_then_the_pane() {
    use crate::chatkeys::{ChatAction, ChatInput};

    let mut p = pane();
    let cwd = std::env::temp_dir();

    // Typing '/' opens the command palette (goes through the real edit path).
    assert!(p.on_input(ChatInput::Char('/'), &cwd).is_none());
    assert!(p.palette.is_some(), "leading '/' opens the command palette");

    // First Esc: consumed by the palette. on_input returns None — the pane
    // stays open — and the popup closes. This is the exact swallowed-Esc
    // regression the routing order guards against.
    assert!(
        p.on_input(ChatInput::Close, &cwd).is_none(),
        "Esc with an open palette must NOT close the pane"
    );
    assert!(p.palette.is_none(), "Esc closed the popup");

    // Second Esc: no popup now, so it reaches the pane and asks to close.
    assert!(matches!(
        p.on_input(ChatInput::Close, &cwd),
        Some(ChatAction::Close)
    ));
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
