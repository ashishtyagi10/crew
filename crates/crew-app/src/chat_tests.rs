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
fn cells_render_session_line_agent_chips_and_turn_duration() {
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
    // The turn duration (1200ms -> "1.2s") now lives in the session line, not
    // a waterfall row.
    assert!(
        text.contains("1 turn") && text.contains("1.2s"),
        "turn duration in session line:\n{text}"
    );
    assert!(
        !text.contains('\u{2588}'),
        "no waterfall block glyphs left:\n{text}"
    );
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
    let avail = rows.saturating_sub(1 + crate::chatinput::composer_rows(&p.input, cols, rows));
    let lay = crate::chatchips::layout(&views, cols, avail).expect("some rows fit");
    assert_eq!(
        top,
        1 + lay.rows,
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
    let grid = crate::chatchips::row_cells(&views, cols, 1, &lay, crate::anim::now_ms());
    let max_row = grid.iter().map(|c| c.row).max().unwrap_or(0);
    assert!(
        max_row < top,
        "grid content stays inside the status zone: max_row={max_row} top={top}"
    );

    // Composer-overlap regression: the grid's budget must reserve the
    // composer's *real* height (`composer_rows`, 3 on this tall pane), not a
    // hardcoded stand-in — otherwise the last grid row lands on the
    // composer's top border. No rendered grid row may reach the composer's
    // first row.
    let composer_first_row = rows - crate::chatinput::composer_rows(&p.input, cols, rows);
    assert!(
        max_row < composer_first_row,
        "grid content stays clear of the composer: max_row={max_row} composer_first_row={composer_first_row}"
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
fn status_rows_counts_session_and_grid() {
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
    // Idle, wide+tall pane: session line + one row per agent (2 agents).
    assert_eq!(p.status_rows(200, 20), 1 + 2);
    // A turn ran → the count is unchanged; the duration now lives in the
    // session line, not an extra row.
    p.absorb_stats(950, String::new(), 0, 0);
    p.pulse.record_hop("planner", 1200);
    p.pulse.end_turn();
    assert_eq!(p.status_rows(200, 20), 1 + 2);
    // Too narrow for any card → just the session line.
    assert_eq!(p.status_rows(3, 20), 1);
    // A short pane (rows=3, composer=1 row here) reserves session(1) +
    // composer(1), leaving exactly one row for the grid — one agent row
    // fits with no overlap.
    assert_eq!(p.status_rows(200, 3), 1 + 1);
}

// `on_key` takes a winit `KeyEvent`, which is #[non_exhaustive] and awkward
// to construct in a unit test (see `chatkeys.rs`). These drive its testable
// half, `on_input(ChatInput, cwd)`, end-to-end — the real routing, including
// the return value that decides whether the pane closes.
#[test]
fn esc_interrupts_a_busy_connected_pane_instead_of_closing() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    // Busy via a live agent (not `awaiting`), so the send below is the only
    // thing that can flip `awaiting` true.
    p.absorb_activity("planner".into(), "thinking", "user".into());
    assert!(p.is_busy());
    assert!(!p.awaiting);

    let action = p.on_input(ChatInput::Close, &cwd);

    assert!(action.is_none(), "busy Esc must not close the pane");
    assert!(
        p.awaiting,
        "Esc sent /stop directly (bypassing the queue), latching awaiting like any direct send"
    );
    assert_eq!(p.messages.len(), 1, "one interrupt note pushed");
    assert_eq!(p.messages[0].sender, "crew");
    assert!(
        p.messages[0].text.contains("interrupting") && p.messages[0].text.contains("/stop"),
        "note text: {}",
        p.messages[0].text
    );
}

#[test]
fn esc_closes_the_pane_when_idle() {
    use crate::chatkeys::{ChatAction, ChatInput};

    let mut p = pane();
    let cwd = std::env::temp_dir();
    assert!(!p.is_busy());

    assert!(matches!(
        p.on_input(ChatInput::Close, &cwd),
        Some(ChatAction::Close)
    ));
    assert!(p.messages.is_empty(), "idle Esc writes nothing");
}

#[test]
fn esc_closes_when_busy_but_disconnected_and_writes_nothing() {
    use crate::chatkeys::{ChatAction, ChatInput};

    let mut p = pane();
    let cwd = std::env::temp_dir();
    // Busy via a live agent, but never connected — a dead pipe must not be
    // written to.
    p.absorb_activity("planner".into(), "thinking", "user".into());
    assert!(p.is_busy());
    assert!(!p.connected);

    assert!(matches!(
        p.on_input(ChatInput::Close, &cwd),
        Some(ChatAction::Close)
    ));
    assert!(!p.awaiting, "no send happened while disconnected");
    assert!(p.messages.is_empty(), "no note written while disconnected");
}

#[test]
fn double_esc_while_busy_resends_stop_but_notes_once() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    p.absorb_activity("planner".into(), "thinking", "user".into());

    assert!(p.on_input(ChatInput::Close, &cwd).is_none());
    assert_eq!(p.messages.len(), 1);
    let note = p.messages[0].text.clone();

    // Still busy (via `active`, untouched by the first Esc) — Esc again.
    assert!(p.is_busy());
    assert!(p.on_input(ChatInput::Close, &cwd).is_none());
    assert_eq!(p.messages.len(), 1, "no duplicate note on the second Esc");
    assert_eq!(p.messages[0].text, note, "the single note is unchanged");
}

// The dedup in `interrupt` only compares against the LAST transcript
// message — it must not suppress a note just because an earlier one exists.
// A broker `Message` landing between two Esc presses (the pane still busy
// throughout, e.g. via `active`/`swarm`) means the second Esc's note is not
// a dupe of the last message, so it must be pushed.
#[test]
fn esc_after_a_broker_message_pushes_a_second_interrupt_note() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    p.absorb_activity("planner".into(), "thinking", "user".into());

    // First Esc: interrupt note pushed.
    assert!(p.on_input(ChatInput::Close, &cwd).is_none());
    assert_eq!(p.messages.len(), 1, "one interrupt note pushed");
    let note = p.messages[0].text.clone();

    // A broker Message arrives mid-turn — mirrors chat.rs::poll's Message arm
    // (`note_reply` + `push_capped`) — from a sender unrelated to the still-
    // thinking `planner`, so the pane stays busy throughout (like the
    // swarm's own "crew" status lines).
    p.note_reply("crew");
    p.push_capped(Message {
        sender: "crew".into(),
        text: "swarm done \u{2014} 1 task(s), 0 failed".into(),
        ts: String::new(),
        meta: String::new(),
    });
    assert!(p.is_busy(), "pane must still be busy after the message");
    assert_eq!(p.messages.len(), 2);

    // Second Esc: the last message is the broker reply, not the note, so
    // dedup must not suppress this one — a second interrupt note is pushed.
    assert!(p.on_input(ChatInput::Close, &cwd).is_none());
    assert_eq!(
        p.messages.len(),
        3,
        "dedup only suppresses CONSECUTIVE notes, not one separated by a reply"
    );
    assert_eq!(p.messages[2].text, note, "same note text as the first");
}

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

#[test]
fn hive_events_are_pane_state_not_host_actions() {
    use crew_plugin::PluginEvent;
    assert!(crate::chat::classify(&PluginEvent::HivePlan { tasks: vec![] }).is_none());
    assert!(crate::chat::classify(&PluginEvent::Hive {
        event: crew_hive::HiveEvent::TaskStateChanged {
            task: crew_hive::TaskId(0),
            state: crew_hive::TaskState::Running,
        }
    })
    .is_none());
}

#[test]
fn slash_exit_closes_the_pane() {
    use crate::chatkeys::{ChatAction, ChatInput};
    let mut p = pane();
    let cwd = std::env::temp_dir();
    // `/exit` submitted from the composer closes the pane (like Escape), and
    // is never sent to the broker. Palette is closed here (the accept step
    // adds the trailing space, which `trim()` tolerates).
    p.input = "/exit".to_string();
    assert!(matches!(
        p.on_input(ChatInput::Enter, &cwd),
        Some(ChatAction::Close)
    ));
    // Trailing space (what the palette-accept produces) still closes.
    let mut p = pane();
    p.input = "/exit ".to_string();
    assert!(matches!(
        p.on_input(ChatInput::Enter, &cwd),
        Some(ChatAction::Close)
    ));
}

#[test]
fn slash_theme_lists_and_switches_without_reaching_the_broker() {
    use crate::chatkeys::ChatInput;
    let _g = crate::app::theme_test_guard();

    let mut p = pane();
    let cwd = std::env::temp_dir();

    // No arg: lists the themes locally, pane stays open, nothing sent.
    p.input = "/theme".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    let note = &p.messages.last().expect("a crew note was pushed").text;
    assert!(
        note.contains("paper-dark") && note.contains("crt-blue"),
        "got: {note}"
    );

    // A known name switches the live theme and echoes it.
    p.input = "/theme crt-amber".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::CrtAmber);
    let note = &p.messages.last().unwrap().text;
    assert!(
        note.contains("theme") && note.contains("crt-amber"),
        "got: {note}"
    );
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark); // reset (global atomic)

    // An unknown name reports the failure instead of switching.
    p.input = "/theme nope".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperDark);
    let note = &p.messages.last().unwrap().text;
    assert!(note.contains("unknown theme"), "got: {note}");
}

#[test]
fn slash_theme_random_enters_rotation_and_a_named_switch_clears_it() {
    use crate::chatkeys::ChatInput;
    let _g = crate::app::theme_test_guard();

    let mut p = pane();
    let cwd = std::env::temp_dir();
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );

    // `/theme random` enters rotation mode (the `random-dark` alias) and
    // echoes it, without reaching the broker.
    p.input = "/theme random".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert!(crew_theme::is_random());
    assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Dark));
    let note = &p.messages.last().unwrap().text;
    assert!(note.contains("random-dark"), "got: {note}");

    // The listing marks `random-dark`, not any fixed theme, while rotation is on.
    p.input = "/theme".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    let note = &p.messages.last().unwrap().text;
    assert!(note.contains("\u{25cf} random-dark"), "got: {note}");

    // Switching to a named theme turns rotation back off.
    p.input = "/theme paper-light".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert!(!crew_theme::is_random());
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);

    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    ); // reset (global atomic)
}

#[test]
fn slash_compact_folds_old_messages_without_reaching_the_broker() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for i in 0..30 {
        p.messages.push(crate::chatlayout::Message {
            sender: "user".into(),
            text: format!("m{i}"),
            ts: String::new(),
            meta: String::new(),
        });
    }
    p.scroll = 5;
    p.unread = 2;

    p.input = "/compact".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert_eq!(p.messages.len(), 21, "20 kept + 1 marker");
    assert!(
        p.messages[0].text.contains("compacted 10"),
        "got: {}",
        p.messages[0].text
    );
    assert_eq!(p.messages.last().unwrap().text, "m29");
    assert_eq!(p.scroll, 0, "compacting snaps back to the live bottom");
    assert_eq!(p.unread, 0, "compacting clears the unread count");

    // `/compact <n>` overrides the default keep count.
    let mut p = pane();
    for i in 0..10 {
        p.messages.push(crate::chatlayout::Message {
            sender: "user".into(),
            text: format!("m{i}"),
            ts: String::new(),
            meta: String::new(),
        });
    }
    p.input = "/compact 3".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert_eq!(p.messages.len(), 4, "3 kept + 1 marker");
    assert_eq!(p.messages.last().unwrap().text, "m9");
}

#[test]
fn show_source_false_renders_bold_markdown() {
    // When show_source is false (default), **bold** text should render with bold cells.
    let mut p = pane();
    p.messages.push(crate::chatlayout::Message {
        sender: "alice".into(),
        text: "**bold**".into(),
        ts: String::new(),
        meta: String::new(),
    });
    assert_eq!(p.show_source, false, "show_source defaults to false");

    let lines = crate::chatmsgs::card_lines(&p.messages, 80, 0, p.show_source);
    // First line is the header (▍ alice ...)
    // Remaining lines are the body with the bold text
    let body_lines: Vec<_> = lines.iter().skip(1).collect();
    assert!(!body_lines.is_empty(), "body should have lines");

    // Check that at least one cell in the body is bold
    let has_bold = body_lines.iter().any(|line| line.iter().any(|c| c.bold));
    assert!(
        has_bold,
        "preview mode should have bold cells for **bold** text"
    );

    // Check that the literal ** characters are NOT in the output
    let body_text: String = body_lines
        .iter()
        .flat_map(|l| l.iter().map(|c| c.c))
        .collect();
    assert!(
        !body_text.contains("**"),
        "preview mode should render markdown, not show literal ** chars"
    );
}

#[test]
fn show_source_true_shows_literal_text() {
    // When show_source is true, **bold** should be literal text with no bold cells.
    let mut p = pane();
    p.show_source = true;
    p.messages.push(crate::chatlayout::Message {
        sender: "alice".into(),
        text: "**bold**".into(),
        ts: String::new(),
        meta: String::new(),
    });

    let lines = crate::chatmsgs::card_lines(&p.messages, 80, 0, p.show_source);
    let body_lines: Vec<_> = lines.iter().skip(1).collect();
    assert!(!body_lines.is_empty(), "body should have lines");

    // Check that NO cell is bold
    let has_bold = body_lines.iter().any(|line| line.iter().any(|c| c.bold));
    assert!(
        !has_bold,
        "source mode should have no bold cells; all cells should be plain"
    );

    // Check that the literal ** characters ARE in the output
    let body_text: String = body_lines
        .iter()
        .flat_map(|l| l.iter().map(|c| c.c))
        .collect();
    assert!(
        body_text.contains("**"),
        "source mode should show literal ** chars: {body_text}"
    );
}

#[test]
fn ctrl_o_toggles_compact_view_and_back() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    assert_eq!(p.compact_view, false, "compact_view defaults to false");

    assert!(p.on_input(ChatInput::ToggleCompact, &cwd).is_none());
    assert!(p.compact_view, "first Ctrl+O turns compact view on");

    assert!(p.on_input(ChatInput::ToggleCompact, &cwd).is_none());
    assert!(
        !p.compact_view,
        "second Ctrl+O restores the full transcript"
    );
}

#[test]
fn ctrl_o_still_toggles_while_the_mention_popup_is_open() {
    // The @file popup only consumes Up/Down/Complete/Enter/Close (see
    // `chatmention::popup_key`) — anything else, including Ctrl+O, forwards
    // to the pane's own handling, matching the existing popup key contract.
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.mention = Some(crate::chatmention::MentionState {
        files: Vec::new(),
        matches: vec!["a.txt".into()],
        sel: 0,
    });

    assert!(p.on_input(ChatInput::ToggleCompact, &cwd).is_none());
    assert!(
        p.compact_view,
        "Ctrl+O reached the pane despite the open popup"
    );
}

#[test]
fn show_source_false_chat_title_has_no_suffix() {
    // When show_source is false (default), the title should be just "chat".
    let p = pane();
    assert_eq!(p.show_source, false);

    // Create a Pane wrapper to use title_text
    let pane = crate::pane::Pane {
        content: crate::pane::PaneContent::Chat(p),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
    };

    let title = pane.title_text();
    assert_eq!(
        title, "chat",
        "title should be just 'chat' when show_source is false"
    );
}

#[test]
fn absorb_stats_retargets_roster_anim() {
    let mut c = pane();
    // Two agents so share redistribution is observable.
    c.absorb_activity("planner".into(), "thinking", "user".into());
    c.absorb_stats(1200, "planner".into(), 800, 30_000);
    c.absorb_stats(400, "coder".into(), 200, 10_000);
    // Token target = the agent's live ctx (the tok column shows context fill).
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!((c.anim.tok("planner", now) - 30_000.0).abs() < 1.0);
    // Shares settle to ms proportions: planner 800/1000, coder 200/1000.
    assert!((c.anim.shr_target("planner") - 0.8).abs() < 1e-6);
    assert!((c.anim.shr_target("coder") - 0.2).abs() < 1e-6);
}

#[test]
fn absorb_stats_derives_ctx_pct_and_ignores_zero_ctx_retarget() {
    let mut c = pane();
    c.agents = vec![crew_plugin::AgentInfo {
        name: "planner".into(),
        role: String::new(),
        model: "claude".into(), // context_limit("claude") == 200_000
    }];
    // Half the model's context window: ctx% target should land on 0.5.
    c.absorb_stats(0, "planner".into(), 800, 100_000);
    assert!(
        (c.anim.ctx_target("planner") - 0.5).abs() < 1e-6,
        "ctx% derives from ctx/limit: {}",
        c.anim.ctx_target("planner")
    );
    // A follow-up event reporting no usage must not ease the bar toward 0.
    c.absorb_stats(0, "planner".into(), 200, 0);
    assert!(
        (c.anim.ctx_target("planner") - 0.5).abs() < 1e-6,
        "zero-ctx event leaves the retarget untouched: {}",
        c.anim.ctx_target("planner")
    );
}

#[test]
fn thinking_activity_records_flash() {
    let mut c = pane();
    c.absorb_activity("coder".into(), "thinking", "planner".into());
    let now = crate::anim::now_ms();
    assert!(c.anim.flash_t("coder", now) > 0.9, "fresh handoff flash");
}

#[test]
fn show_source_true_chat_title_has_source_suffix() {
    // When show_source is true, the title should be "chat · source".
    let mut p = pane();
    p.show_source = true;

    // Create a Pane wrapper to use title_text
    let pane = crate::pane::Pane {
        content: crate::pane::PaneContent::Chat(p),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
    };

    let title = pane.title_text();
    assert_eq!(
        title, "chat · source",
        "title should be 'chat · source' when show_source is true"
    );
}

#[test]
fn anim_active_tail_ends_after_settle() {
    let mut c = pane();
    let now = crate::anim::now_ms();
    assert!(!c.anim_active(now), "fresh pane is inactive");
    let before = crate::anim::now_ms();
    c.absorb_stats(100, "planner".into(), 50, 5_000);
    assert!(c.anim_active(before), "ease in flight");
    let after = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(!c.anim_active(after), "settled → no redraws");
}

#[test]
fn stats_tick_retargets_tok_while_reply_open() {
    let mut c = pane();
    c.absorb_stats(0, "planner".into(), 100, 30_000); // seed ctx ground truth
    c.absorb_activity("planner".into(), "thinking", "user".into()); // opens the reply
    c.absorb_stats_tick("planner".into(), 500);
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(
        (c.anim.tok("planner", now) - 30_500.0).abs() < 1.0,
        "tick target = last ctx + estimate"
    );
}

#[test]
fn stats_tick_ignored_when_no_reply_open() {
    let mut c = pane();
    c.absorb_stats(0, "planner".into(), 100, 30_000);
    // Reply closed by the per-agent Stats above (and never opened) — a
    // straggler tick must not move the target.
    c.absorb_stats_tick("planner".into(), 9_999);
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(
        (c.anim.tok("planner", now) - 30_000.0).abs() < 1.0,
        "stale tick ignored; Stats value stands"
    );
}

#[test]
fn per_agent_stats_closes_the_open_reply() {
    let mut c = pane();
    c.absorb_activity("planner".into(), "thinking", "user".into());
    c.absorb_stats_tick("planner".into(), 100);
    c.absorb_stats(0, "planner".into(), 50, 40_000); // closes + reconciles
    c.absorb_stats_tick("planner".into(), 20_000); // late tick from the finished reply
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(
        (c.anim.tok("planner", now) - 40_000.0).abs() < 1.0,
        "authoritative Stats wins over late ticks"
    );
}

// --- Queued messages while busy (see docs/superpowers/specs/2026-07-13-queued-messages-design.md) ---

/// Spawn a pane whose "broker" is a real child process that prints each of
/// `lines` as a JSON `PluginEvent` line on stdout (in order), then blocks
/// reading stdin forever — a controllable stand-in so `poll()` observes real
/// events (Ready/Roster/Message/Error/…) instead of us reaching into private
/// plugin internals.
fn pane_emitting(lines: &[&str]) -> ChatPane {
    let mut script = String::new();
    for l in lines {
        script.push_str("printf '%s\\n' '");
        script.push_str(&l.replace('\'', "'\\''"));
        script.push_str("';\n");
    }
    script.push_str("cat >/dev/null\n");
    let plugin = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

/// Poll `pane` until `done` holds or the deadline passes (the emitting
/// child's output lands on a background thread, so it isn't there on the
/// very first call).
fn poll_until(pane: &mut ChatPane, done: impl Fn(&ChatPane) -> bool) {
    let start = std::time::Instant::now();
    while !done(pane) {
        pane.poll();
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "poll_until timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn busy_send_is_queued_not_sent() {
    use crate::chatkeys::ChatInput;
    let mut p = pane();
    let cwd = std::env::temp_dir();
    // Busy via a live agent (not `awaiting`), so we can tell the two apart.
    p.absorb_activity("planner".into(), "thinking", "user".into());
    assert!(p.is_busy());
    assert!(!p.awaiting, "busy here comes from `active`, not `awaiting`");

    p.input = "hello while busy".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());

    assert_eq!(p.queued.len(), 1, "queued instead of sent");
    assert_eq!(p.queued[0], "hello while busy");
    assert!(!p.awaiting, "no send happened, so awaiting is untouched");
}

#[test]
fn idle_send_goes_direct_queue_stays_empty() {
    use crate::chatkeys::ChatInput;
    let mut p = pane();
    let cwd = std::env::temp_dir();
    assert!(!p.is_busy());

    p.input = "hello while idle".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());

    assert!(p.queued.is_empty(), "idle sends never touch the queue");
    assert!(p.awaiting, "direct send still latches awaiting as before");
}

#[test]
fn stop_bypasses_the_queue_while_busy() {
    use crate::chatkeys::ChatInput;
    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.absorb_activity("planner".into(), "thinking", "user".into());
    assert!(p.is_busy());

    p.input = "/stop".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());

    assert!(
        p.queued.is_empty(),
        "/stop must reach the broker, not queue"
    );
    assert!(
        p.awaiting,
        "the /stop send latched awaiting like any direct send"
    );
}

#[test]
fn flush_on_idle_sends_exactly_one_and_relatches_awaiting() {
    // The reply landing (a `Message` event) is what clears `awaiting` and
    // triggers poll()'s end-of-drain flush check.
    let mut p = pane_emitting(&[
        r#"{"type":"message","channel":"crew","sender":"crew","text":"reply one","ts":"t"}"#,
    ]);
    p.channel = "crew".into();
    p.connected = true; // simulate: already connected, mid-turn
    p.queued.push_back("first queued".into());
    p.queued.push_back("second queued".into());
    p.awaiting = true; // simulate: our earlier send is still in flight

    poll_until(&mut p, |p| p.queued.len() < 2);

    // Exactly one popped: the reply's Message cleared `awaiting`, poll's
    // flush then sent "first queued" and re-latched `awaiting` for it.
    assert_eq!(p.queued.len(), 1, "only one message flushed per turn");
    assert_eq!(p.queued[0], "second queued", "FIFO order preserved");
    assert!(
        p.awaiting,
        "the flushed send re-latches awaiting for its own reply"
    );
}

#[test]
fn queue_survives_broker_error() {
    // awaiting stays true across an Error (nothing clears it there), so
    // is_busy() stays true and the flush check never fires — the queue is
    // simply left alone, as the design requires.
    let mut p = pane_emitting(&[r#"{"type":"error","message":"broker died"}"#]);
    p.connected = true; // so we can observe the Error event actually landed
    p.awaiting = true;
    p.queued.push_back("still here".into());

    // Drive poll() until the Error event lands (it flips `connected` false);
    // bounded so a broken emitter fails the test instead of hanging.
    let start = std::time::Instant::now();
    while p.connected {
        p.poll();
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "never observed the Error event"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert_eq!(p.queued.len(), 1, "Error must not clear the queue");
    assert_eq!(p.queued[0], "still here");
    assert!(
        p.awaiting,
        "Error doesn't clear awaiting, so is_busy() stayed true — no flush attempted"
    );
}

#[test]
fn broker_death_mid_swarm_does_not_flush_queue_and_reconnect_unwedges() {
    // `queue_survives_broker_error` above passes for the wrong reason: with
    // `awaiting` as the busy signal, the Error arm never clears it, so
    // `is_busy()` stays true and the flush check never even runs. Busy via a
    // swarm instead: the Error arm's `fold_swarm`/`flush_active_hops` DO end
    // that busy state in the very same event drain that flips `connected`
    // false — the same-batch race the fix must close by also gating the
    // flush on `self.connected`.
    use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};

    let script = r#"printf '%s\n' '{"type":"error","message":"broker died"}'
sleep 0.3
printf '%s\n' '{"type":"roster","agents":[]}'
cat >/dev/null
"#;
    let plugin = crew_plugin::Plugin::spawn("sh", &["-c".to_string(), script.to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;

    p.absorb_hive_plan(vec![TaskSpec {
        id: TaskId(0),
        title: "research".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
    }]);
    assert!(p.is_busy(), "swarm in flight makes the pane busy");
    assert!(
        !p.awaiting,
        "busy here comes from the swarm, not `awaiting` — proves the fix, not the accidental guard"
    );

    p.queued.push_back("still here".into());

    // Drive poll() until the Error event lands and flips `connected` false.
    let start = std::time::Instant::now();
    while p.connected {
        p.poll();
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "never observed the Error event"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert!(p.swarm.is_none(), "Error arm folds the swarm");
    assert!(
        !p.is_busy(),
        "busy state cleared by the same Error arm that disconnected"
    );
    assert_eq!(
        p.queued.len(),
        1,
        "queue must survive broker death even though is_busy() went false in the same drain"
    );
    assert_eq!(p.queued[0], "still here");
    assert!(
        !p.awaiting,
        "nothing was sent — a real flush would have latched awaiting via send_now"
    );

    // Reconnect (direct field set is fine here — the existing tests in this
    // file do the same) and prove the queue isn't permanently wedged: the
    // next event to land should flush it once the pane is connected again.
    p.connected = true;
    let start = std::time::Instant::now();
    while !p.queued.is_empty() {
        p.poll();
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "queue never flushed after reconnecting"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        p.awaiting,
        "the post-reconnect flush re-latches awaiting for its own reply"
    );
}

#[test]
fn turn_over_flush_closes_open_reply_lifecycle_so_stray_ticks_are_ignored() {
    let mut c = pane();
    c.absorb_stats(0, "planner".into(), 100, 30_000); // seed ctx ground truth
    c.absorb_activity("planner".into(), "thinking", "user".into()); // opens the reply
                                                                    // Turn-over / empty-agent idle goes through absorb_activity's `_` arm,
                                                                    // which calls flush_active_hops — that must also clear tick_open so a
                                                                    // fan error (or the untracked goal-judge dial) can't leave a stale
                                                                    // lifecycle entry behind.
    c.absorb_activity(String::new(), "idle", String::new());
    c.absorb_stats_tick("planner".into(), 9_999); // straggler after turn-over
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(
        (c.anim.tok("planner", now) - 30_000.0).abs() < 1.0,
        "turn-over must close the reply lifecycle; straggler tick must not move the tok target"
    );
}
