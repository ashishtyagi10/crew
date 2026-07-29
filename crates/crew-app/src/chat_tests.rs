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
    pane.absorb_stats(950, String::new(), 0, 0, 0, 0, 0); // turn-level
    pane.absorb_stats(0, "planner".into(), 4_200, 8_100, 0, 0, 0); // reply-level, ctx 8.1k
    pane.absorb_stats(0, "planner".into(), 2_200, 9_400, 0, 0, 0);
    assert_eq!((pane.tokens, pane.turns), (950, 1));
    assert_eq!(pane.agent_stats.get("planner"), Some(&(2, 6_400)));
    // The latest reply's prompt size is the live context fill.
    assert_eq!(pane.ctx.get("planner"), Some(&9_400));
}

pub(crate) fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

#[test]
fn delta_opens_a_provisional_card_then_appends_to_it() {
    let mut p = pane();
    p.absorb_delta("coder".into(), "Hello".into());
    p.absorb_delta("coder".into(), ", world".into());
    assert!(
        p.messages.is_empty(),
        "nothing settled reaches the transcript"
    );
    assert_eq!(p.streaming.len(), 1, "one card per agent, not per delta");
    assert_eq!(p.streaming[0].text, "Hello, world");
    let visible = p.visible_messages();
    assert_eq!(visible.len(), 1, "the provisional card is drawn");
    assert_eq!(visible[0].text, "Hello, world");
}

#[test]
fn settled_message_replaces_the_provisional_card() {
    let mut p = pane();
    p.absorb_delta("coder".into(), "Hel".into());
    p.settle_stream("coder \u{2192} user");
    p.push_capped(Message {
        sender: "coder \u{2192} user".into(),
        text: "Hello, world".into(),
        ts: "1".into(),
        meta: String::new(),
    });
    assert!(
        p.streaming.is_empty(),
        "a relay sender settles the bare-name card"
    );
    assert_eq!(
        p.messages.len(),
        1,
        "exactly one card, not the stream plus the reply"
    );
    assert_eq!(p.visible_messages().len(), 1);
    assert_eq!(p.messages[0].text, "Hello, world");
}

#[test]
fn two_agents_stream_into_separate_cards() {
    let mut p = pane();
    p.absorb_delta("planner".into(), "plan".into());
    p.absorb_delta("coder".into(), "code".into());
    p.absorb_delta("planner".into(), "ning".into());
    assert_eq!(p.streaming.len(), 2);
    let texts: Vec<&str> = p.streaming.iter().map(|m| m.text.as_str()).collect();
    assert!(
        texts.contains(&"planning") && texts.contains(&"code"),
        "{texts:?}"
    );
}

/// The next task's UI finds the most-recently-updated agent via
/// `streaming.last()`, with no timestamp field — so touching an OLDER card
/// must move it to the end, not just mutate it in place. This would still
/// pass `two_agents_stream_into_separate_cards` above (which only checks
/// membership) even if `absorb_delta` regressed to an in-place mutation; this
/// test asserts POSITION and fails on that regression.
#[test]
fn absorb_delta_moves_the_touched_card_to_the_end() {
    let mut p = pane();
    p.absorb_delta("planner".into(), "plan".into());
    p.absorb_delta("coder".into(), "code".into());
    // `planner` is the OLDER card (opened first); touching it again must move
    // it to the end, ahead of `coder`, which is untouched since.
    p.absorb_delta("planner".into(), "ning".into());
    assert_eq!(
        p.streaming
            .iter()
            .map(|m| m.sender.as_str())
            .collect::<Vec<_>>(),
        vec!["coder", "planner"],
        "the touched (planner) card must be last, not left in its original slot"
    );
    assert_eq!(p.streaming.last().unwrap().text, "planning");
}

#[test]
fn provisional_card_never_bumps_the_unread_pill() {
    let mut p = pane();
    p.scroll = 5; // scrolled up: a settled reply WOULD count as unread
    p.absorb_delta("coder".into(), "text".into());
    assert_eq!(p.unread, 0, "only settled replies are 'new'");
}

#[test]
fn turn_end_clears_a_stranded_provisional_card() {
    let mut p = pane();
    p.absorb_delta("coder".into(), "half a reply".into());
    // The empty-agent idle: the turn is over and no Message ever arrived.
    p.absorb_activity(String::new(), "idle", String::new());
    assert!(
        p.streaming.is_empty(),
        "an interrupted hop leaves no stranded card"
    );
}

#[test]
fn export_never_contains_a_provisional_card() {
    let mut p = pane();
    p.push_capped(Message {
        sender: "coder".into(),
        text: "SETTLED".into(),
        ts: "1".into(),
        meta: String::new(),
    });
    p.absorb_delta("coder".into(), "HALFWRITTEN".into());
    let md = crate::chatexport::transcript_markdown("c", &p.messages, &chrono::Local::now());
    assert!(md.contains("SETTLED"));
    assert!(
        !md.contains("HALFWRITTEN"),
        "provisional cards live outside `messages`, so export cannot see them"
    );
}

#[test]
fn cells_render_header_and_summary_footer_not_agent_chips() {
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
    p.absorb_stats(950, String::new(), 0, 0, 600, 350, 0);
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
    // The header row is liveness-only now — the fieldset legend names the
    // pane, so no `agent smith` title text renders inside it.
    assert!(
        !text.contains("agent smith"),
        "header title must be gone:\n{text}"
    );
    // The summary footer replaces the per-agent chip grid: a 3-line
    // statusline with the shared model, the token split, and the (empty,
    // since no fill recorded) context bar. A tall pane (20 rows) shows all
    // three lines.
    assert!(text.contains("qwen"), "footer model:\n{text}");
    assert!(text.contains("0% (ctx)"), "footer context row:\n{text}");
    assert!(
        text.contains("600 in / 350 out"),
        "footer token split:\n{text}"
    );
    // The retired chip markers must be gone.
    assert!(
        !text.contains('\u{25b8}') && !text.contains('\u{25aa}'),
        "agent chip markers must not render:\n{text}"
    );
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
    p.note_reply("agent smith");
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
fn status_rows_is_header_only() {
    // The per-agent grid was retired, so the status zone above the transcript
    // is now just the single header row (roster size no longer changes it).
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
    assert_eq!(p.status_rows(200, 20), 1, "just the header on a tall pane");
    p.absorb_stats(950, String::new(), 0, 0, 0, 0, 0);
    p.pulse.record_hop("planner", 1200);
    p.pulse.end_turn();
    assert_eq!(p.status_rows(200, 20), 1, "unchanged after a turn");
    assert_eq!(
        p.status_rows(200, 3),
        1,
        "header still shows on a short pane"
    );
    // Too short for the card view at all → plain fallback (no status rows).
    assert_eq!(p.status_rows(200, 2), 0);
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
    assert_eq!(p.messages[0].sender, "agent smith");
    assert!(
        p.messages[0].text.contains("interrupting") && p.messages[0].text.contains("/stop"),
        "note text: {}",
        p.messages[0].text
    );
}

#[test]
fn submitting_a_prompt_echoes_it_into_the_transcript_as_the_user() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;

    // Type a prompt and press Enter — idle, so it sends immediately.
    p.input = "hello crew".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());

    // The user's own message must appear in the transcript. Previously only
    // agent output (the `PluginEvent::Message` arm) was ever appended, so the
    // submitted prompt vanished and the pane showed only replies.
    assert_eq!(p.messages.len(), 1, "the submitted prompt is echoed once");
    assert_eq!(p.messages[0].sender, "user");
    assert_eq!(p.messages[0].text, "hello crew");
    assert!(p.awaiting, "an idle send still latches awaiting");
}

#[test]
fn a_prompt_queued_while_busy_still_echoes_immediately() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    // Busy via a live agent, so a fresh submit is queued, not sent.
    p.absorb_activity("planner".into(), "thinking", "user".into());
    assert!(p.is_busy());

    p.input = "do the thing".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());

    // Echoed to the transcript now — the user sees their message the instant
    // they hit Enter, even though the send itself waits behind the busy turn.
    assert_eq!(
        p.messages.len(),
        1,
        "the queued prompt is echoed immediately"
    );
    assert_eq!(p.messages[0].sender, "user");
    assert_eq!(p.messages[0].text, "do the thing");
    assert_eq!(p.queued.len(), 1, "the send is queued behind the busy turn");
}

#[test]
fn locally_handled_slash_commands_are_not_echoed_as_user_messages() {
    use crate::chatkeys::ChatInput;

    // `/theme` is answered locally and must not leave a stray "user: /theme"
    // line — only genuine prompts get echoed.
    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.input = "/theme".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert!(
        !p.messages.iter().any(|m| m.sender == "user"),
        "a locally-intercepted command must not echo as a user message"
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
    // swarm's own "agent smith" status lines).
    p.note_reply("agent smith");
    p.push_capped(Message {
        sender: "agent smith".into(),
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
    crate::chatpalette::after_edit(&mut p.palette, &p.input, None, Vec::new);
    crate::chatmention::after_edit(&mut p.mention, &p.input, || {
        vec![crate::chatmention::MentionEntry::File("mod.rs".into())]
    });
    assert!(p.palette.is_some());
    assert!(p.mention.is_none());

    let mut p = pane();
    p.input = "hey @mo".to_string();
    crate::chatpalette::after_edit(&mut p.palette, &p.input, None, || {
        vec![crate::chatmention::MentionEntry::File("mod.rs".into())]
    });
    crate::chatmention::after_edit(&mut p.mention, &p.input, || {
        vec![crate::chatmention::MentionEntry::File("mod.rs".into())]
    });
    assert!(p.palette.is_none());
    assert!(p.mention.is_some());
}

#[test]
fn model_pick_sends_one_broker_command() {
    let mut chat = pane();
    let cwd = std::path::Path::new(".");
    for c in "/model ".chars() {
        chat.on_input(crate::chatkeys::ChatInput::Char(c), cwd);
    }
    assert!(chat.palette.is_some(), "picker should be open");
    // Move off the opening `default` row before picking — Enter on an
    // untouched, still-empty query has its own behavior (see the next test).
    chat.on_input(crate::chatkeys::ChatInput::Down, cwd);
    chat.on_input(crate::chatkeys::ChatInput::Enter, cwd);
    // The pick ran: input cleared, and the echoed message is a `/model all …`.
    assert!(chat.input.is_empty());
    let last = chat.messages.last().expect("the pick was echoed");
    assert!(last.text.starts_with("/model all "), "{}", last.text);
}

#[test]
fn bare_model_space_enter_lists_instead_of_resetting_every_pin() {
    // A bare `/model ` + Enter, selection never touched, must send the raw
    // `/model` listing — NOT the highlighted `default` row's destructive
    // `/model all default`, which would clear every agent's pin (see
    // `chatpalettemodel_tests.rs` for the pure-logic half of this).
    let mut chat = pane();
    let cwd = std::path::Path::new(".");
    for c in "/model ".chars() {
        chat.on_input(crate::chatkeys::ChatInput::Char(c), cwd);
    }
    assert!(chat.palette.is_some(), "picker should be open");
    chat.on_input(crate::chatkeys::ChatInput::Enter, cwd);
    assert!(chat.input.is_empty());
    let last = chat.messages.last().expect("the raw command was echoed");
    assert_eq!(last.text, "/model ");
    assert!(
        !last.text.starts_with("/model all"),
        "must not clear every agent's pin: {}",
        last.text
    );
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

    // No arg: lists the three themes locally, pane stays open, nothing sent.
    p.input = "/theme".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    let note = &p.messages.last().expect("a crew note was pushed").text;
    assert!(
        note.contains("dark") && note.contains("light") && note.contains("crt"),
        "got: {note}"
    );

    // A known name switches the live theme, echoes it, and asks the app to
    // persist the switch (the pane can't reach the config).
    p.input = "/theme crt-amber".to_string();
    assert!(matches!(
        p.on_input(ChatInput::Enter, &cwd),
        Some(ChatAction::PersistTheme)
    ));
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

    // `/theme random` enters the dark rotation (back-compat alias for `dark`)
    // and echoes it, without reaching the broker.
    p.input = "/theme random".to_string();
    assert!(matches!(
        p.on_input(ChatInput::Enter, &cwd),
        Some(ChatAction::PersistTheme)
    ));
    assert!(crew_theme::is_random());
    assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Dark));
    let note = &p.messages.last().unwrap().text;
    assert!(note.contains("dark"), "got: {note}");

    // The listing marks `dark`, the active mode, while rotation is on.
    p.input = "/theme".to_string();
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    let note = &p.messages.last().unwrap().text;
    assert!(note.contains("\u{25cf} dark"), "got: {note}");

    // Switching to a named theme turns rotation back off (and persists too —
    // a fixed pick must not leave a stale rotation mode in the config).
    p.input = "/theme paper-light".to_string();
    assert!(matches!(
        p.on_input(ChatInput::Enter, &cwd),
        Some(ChatAction::PersistTheme)
    ));
    assert!(!crew_theme::is_random());
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);

    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    ); // reset (global atomic)
}

/// The marker reports the RUNNING total, not whatever the last pass dropped.
/// Folding to exactly the cap made every subsequent message fold again, and
/// each pass reported only its own one or two — a hundred messages past the
/// cap the marker still read "compacted 2 earlier messages".
#[test]
fn the_fold_marker_counts_everything_it_ever_folded() {
    let mut p = pane();
    let cap = crate::chat::ChatPane::TRANSCRIPT_CAP;
    let extra = 100;
    for i in 0..cap + extra {
        p.push_capped(crate::chatlayout::Message {
            sender: "user".into(),
            text: format!("m{i}"),
            ts: String::new(),
            meta: String::new(),
        });
    }
    let marker = &p.messages[0].text;
    let n: usize = marker
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .expect("a count in the marker");
    // Everything not still present has been counted.
    let kept = p.messages.len() - 1;
    assert_eq!(
        n,
        cap + extra - kept,
        "marker says {n}, but {} were dropped",
        cap + extra - kept
    );
    assert_eq!(
        p.folded, n,
        "the pane's running total agrees with the marker"
    );
}

/// Folding happens in batches, so a long session does not re-copy the whole
/// transcript on every single message once it passes the cap.
#[test]
fn folding_is_batched_not_per_message() {
    let mut p = pane();
    let cap = crate::chat::ChatPane::TRANSCRIPT_CAP;
    let push = |p: &mut crate::chat::ChatPane, i: usize| {
        p.push_capped(crate::chatlayout::Message {
            sender: "user".into(),
            text: format!("m{i}"),
            ts: String::new(),
            meta: String::new(),
        });
    };
    for i in 0..cap {
        push(&mut p, i);
    }
    assert_eq!(p.folded, 0, "nothing folds before the cap");
    push(&mut p, cap);
    let after_first = p.folded;
    assert!(after_first > 1, "a fold takes a batch, not one message");
    // The next few messages ride in the slack without another fold.
    for i in 1..10 {
        push(&mut p, cap + i);
    }
    assert_eq!(p.folded, after_first, "folded again inside the slack");
}

/// The transcript folds itself, and SAYS it did. The cap has always been
/// automatic; before v0.6.55 it drained old messages in silence, so a long
/// session quietly lost its beginning with nothing to mark the loss.
#[test]
fn the_transcript_folds_itself_with_a_marker() {
    let mut p = pane();
    let cap = crate::chat::ChatPane::TRANSCRIPT_CAP;
    for i in 0..cap + 10 {
        p.push_capped(crate::chatlayout::Message {
            sender: "user".into(),
            text: format!("m{i}"),
            ts: String::new(),
            meta: String::new(),
        });
    }
    // Folding takes a batch, so the length lands between the low-water mark
    // and the cap rather than on the cap exactly.
    let low = crate::chat::ChatPane::TRANSCRIPT_CAP - crate::chat::ChatPane::FOLD_SLACK;
    assert!(
        p.messages.len() > low && p.messages.len() <= cap + 1,
        "length {} outside the fold band ({low}..={}]",
        p.messages.len(),
        cap + 1
    );
    assert!(
        p.messages[0].text.contains("compacted"),
        "the fold must announce itself: {}",
        p.messages[0].text
    );
    // The newest message always survives — folding must never eat the reply
    // the user is waiting for.
    assert_eq!(p.messages.last().unwrap().text, format!("m{}", cap + 9));
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
    assert!(!p.show_source, "show_source defaults to false");

    let refs: Vec<&crate::chatlayout::Message> = p.messages.iter().collect();
    let lines = crate::chatmsgs::card_lines(
        &refs,
        80,
        0,
        crate::chatmsgs::View {
            source: p.show_source,
            compact: p.compact_view,
        },
    );
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

    let refs: Vec<&crate::chatlayout::Message> = p.messages.iter().collect();
    let lines = crate::chatmsgs::card_lines(
        &refs,
        80,
        0,
        crate::chatmsgs::View {
            source: p.show_source,
            compact: p.compact_view,
        },
    );
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
fn show_source_false_chat_title_has_no_suffix() {
    // When show_source is false (default), the title should be just "chat".
    let p = pane();
    assert!(!p.show_source);

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
        born_ms: crate::anim::now_ms(),
    };

    let title = pane.title_text();
    assert_eq!(
        title, "chat",
        "title should be just 'chat' when show_source is false"
    );
}

#[test]
fn absorb_stats_records_ctx_and_keeps_last_nonzero() {
    // The summary footer reads `ctx` to derive the remaining context window,
    // so a per-agent Stats must record real usage — and a follow-up event
    // reporting no usage must NOT clear it (else the footer would flap to full).
    let mut c = pane();
    c.absorb_stats(0, "planner".into(), 800, 100_000, 0, 0, 0);
    assert_eq!(
        c.ctx.get("planner").copied(),
        Some(100_000),
        "ctx fill recorded from the Stats event"
    );
    c.absorb_stats(0, "planner".into(), 200, 0, 0, 0, 0);
    assert_eq!(
        c.ctx.get("planner").copied(),
        Some(100_000),
        "a zero-ctx event leaves the last known fill untouched"
    );
    // Replies still accumulate regardless of ctx.
    assert_eq!(c.agent_stats.get("planner").map(|(n, _)| *n), Some(2));
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
        born_ms: crate::anim::now_ms(),
    };

    let title = pane.title_text();
    assert_eq!(
        title, "chat · source",
        "title should be 'chat · source' when show_source is true"
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
        specialty: String::new(),
        expertise: String::new(),
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
fn accepting_the_model_row_from_the_slash_palette_opens_the_picker() {
    // The natural flow: type `/mod`, see the slash palette, press Enter on
    // the `/model` row. The input becomes `/model ` — and the grouped model
    // picker must be showing, not nothing.
    let mut p = pane();
    let cwd = std::path::Path::new(".");
    for c in "/model".chars() {
        p.on_input(crate::chatkeys::ChatInput::Char(c), cwd);
    }
    assert_eq!(
        p.palette.as_ref().map(|x| x.kind),
        Some(crate::chatpalette::Kind::Slash),
        "the slash palette should be listing /model"
    );
    p.on_input(crate::chatkeys::ChatInput::Enter, cwd);
    assert_eq!(p.input, "/model ");
    assert_eq!(
        p.palette.as_ref().map(|x| x.kind),
        Some(crate::chatpalette::Kind::Model),
        "accepting /model must open the model picker"
    );
}

// `pane()` is the existing helper at the top of this file — it spawns an idle
// child as a stand-in broker and returns a `ChatPane`. Use it; do NOT call
// `ChatPane::new` directly (it takes a `Plugin` and a channel).

#[test]
fn the_key_prompt_is_modal_and_escape_closes_it_not_the_pane() {
    let mut p = pane();
    p.keyentry = Some(crate::keyentry::KeyEntry::new("ANTHROPIC_API_KEY".into()));
    assert!(p
        .on_input(ChatInput::Close, std::path::Path::new("."))
        .is_none());
    assert!(p.keyentry.is_none(), "escape closes the prompt");
}

#[test]
fn the_prompt_swallows_keys_meant_for_the_composer() {
    let mut p = pane();
    let before = p.input.clone();
    p.keyentry = Some(crate::keyentry::KeyEntry::new("ANTHROPIC_API_KEY".into()));
    p.on_input(ChatInput::Char('x'), std::path::Path::new("."));
    assert_eq!(p.input, before, "a typed key must not reach the composer");
    assert!(p.keyentry.is_some(), "the prompt stays open");
}

// (An OpenRouter key stored through `store_provider_key_at` is covered by
// `chatkeystore_tests`, which owns that path. The test that used to sit here
// named OAuth but exercised none of it — it called the store directly, like
// the typed path does, so it duplicated that coverage while claiming more.)

#[test]
fn escaping_the_prompt_cancels_an_in_flight_sign_in() {
    // Dropping the receiver is what makes the worker thread's send fail and
    // exit, so a cancelled flow must not leave it set.
    let mut p = pane();
    p.keyentry = Some(crate::keyentry::KeyEntry::new("OPENROUTER_API_KEY".into()));
    let (_tx, rx) = std::sync::mpsc::channel();
    p.oauth = Some(rx);
    p.on_input(ChatInput::Close, std::path::Path::new("."));
    assert!(p.keyentry.is_none(), "escape closes the prompt");
    assert!(p.oauth.is_none(), "escape cancels the sign-in");
    // And never silently: the exchange may already have minted a key on the
    // user's OpenRouter account, so a cancel they cannot see is a key they
    // have to go and revoke by hand.
    let note = p
        .messages
        .last()
        .map(|m| m.text.clone())
        .unwrap_or_default();
    assert!(note.contains("cancelled"), "{note:?}");
}

#[test]
fn cancelling_says_so_only_when_a_sign_in_was_actually_in_flight() {
    // The chokepoint both dismissal paths (Escape, Submit) go through. A
    // prompt with no browser flow behind it — every provider but OpenRouter —
    // must not announce cancelling one.
    let mut p = pane();
    p.cancel_oauth();
    assert!(p.messages.is_empty(), "nothing to cancel, nothing to say");

    let (_tx, rx) = std::sync::mpsc::channel();
    p.oauth = Some(rx);
    p.cancel_oauth();
    assert!(p.oauth.is_none(), "the receiver is dropped: the flow ends");
    assert_eq!(p.messages.len(), 1);
    assert!(p.messages[0].text.contains("openrouter sign-in cancelled"));
}

/// The pane tracks the broker's task lifecycle so the footer can show it.
/// An end event for a task the pane never saw start (a broker that outlived a
/// reconnect) must not leave a phantom.
#[test]
fn task_events_track_what_is_running() {
    let mut p = pane();
    p.absorb_task(1, true);
    p.absorb_task(2, true);
    assert_eq!(p.running_tasks, vec![1, 2]);
    p.absorb_task(1, false);
    assert_eq!(p.running_tasks, vec![2]);
    p.absorb_task(99, false);
    assert_eq!(p.running_tasks, vec![2], "unknown end disturbed the list");
    // A duplicate start does not double-count.
    p.absorb_task(2, true);
    assert_eq!(p.running_tasks, vec![2]);
}

/// Enter and Esc answer a pending plan — but only on an empty composer.
/// Half-typed text means the user moved on, and neither key should throw a
/// plan away behind it.
#[test]
fn a_pending_plan_binds_enter_and_esc() {
    let cwd = std::path::PathBuf::from("/tmp");
    let mut p = pane();
    p.plan_pending = true;
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert!(!p.plan_pending, "enter answered the plan");

    let mut p = pane();
    p.plan_pending = true;
    assert!(
        p.on_input(ChatInput::Close, &cwd).is_none(),
        "esc answered the plan instead of closing the pane"
    );
    assert!(!p.plan_pending);

    // With text typed, both keys keep their ordinary meaning.
    let mut p = pane();
    p.plan_pending = true;
    p.input = "wait, actually".into();
    p.on_input(ChatInput::Enter, &cwd);
    assert!(p.plan_pending, "sending text must not answer the plan");
    let mut p = pane();
    p.plan_pending = true;
    p.input = "hm".into();
    assert!(
        p.on_input(ChatInput::Close, &cwd).is_some(),
        "esc with text typed still closes the pane"
    );
    assert!(p.plan_pending, "closing must not answer the plan");
}

/// State that belonged to a dead broker must not outlive it. A task killed
/// with its process never sends its end event, so the footer would offer
/// `/stop #3` for a task that no longer exists — and a plan it was holding
/// would still answer to enter, against a broker that never saw it.
#[test]
fn a_lost_broker_takes_its_running_tasks_and_pending_plan_with_it() {
    let mut p = pane();
    p.absorb_task(3, true);
    p.absorb_task(5, true);
    p.plan_pending = true;
    assert_eq!(p.running_tasks, vec![3, 5]);

    p.reset_broker_state();
    assert!(p.running_tasks.is_empty(), "phantom tasks survived");
    assert!(!p.plan_pending, "a plan survived the broker holding it");
}

/// …and a FRESH broker starts counting from 1, so anything left over would
/// both lie and collide with a real id.
#[test]
fn a_fresh_broker_starts_from_an_empty_task_list() {
    let mut p = pane();
    p.absorb_task(1, true);
    p.reset_broker_state();
    p.absorb_task(1, true);
    assert_eq!(p.running_tasks, vec![1], "an id was double-counted");
}

/// Up recalls what was sent, through the real key path — and the palette,
/// which is open whenever the composer leads with `/`, must get the arrows
/// FIRST. The history's own walk is unit-tested in `chathistory`; what is
/// under test here is the wiring: what gets recorded, and who owns the key.
#[test]
fn up_recalls_the_last_prompt_once_no_popup_wants_the_key() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for c in "hello there".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);
    assert!(p.input.is_empty(), "sending clears the composer");

    p.on_input(ChatInput::Up, &cwd);
    assert_eq!(p.input, "hello there", "Up did not recall the prompt");
    p.on_input(ChatInput::Down, &cwd);
    assert!(p.input.is_empty(), "Down returned the empty draft");
}

/// A locally answered command never reaches the broker, and is exactly as
/// worth recalling as one that does — `record` runs before every intercept.
#[test]
fn a_locally_intercepted_command_is_still_recalled() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for c in "/export".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    // Escape the palette the leading '/' opened, so Enter is the pane's.
    p.on_input(ChatInput::Close, &cwd);
    p.on_input(ChatInput::Enter, &cwd);
    p.on_input(ChatInput::Up, &cwd);
    assert_eq!(p.input, "/export");
}

/// The arrows belong to an open popup. Recall must not fight the palette for
/// them — this is the same routing-order guard as `esc_closes_the_open_
/// palette_then_the_pane`, for the keys added with history.
#[test]
fn an_open_palette_keeps_the_arrows() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for c in "sent once".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);
    p.on_input(ChatInput::Char('/'), &cwd);
    assert!(p.palette.is_some(), "leading '/' opens the palette");
    p.on_input(ChatInput::Up, &cwd);
    assert_eq!(
        p.input, "/",
        "the palette moved its selection, not the input"
    );
}

/// Typing over a recalled line adopts it: Down must not put the stashed draft
/// back on top of what the user has just written.
#[test]
fn editing_a_recalled_prompt_keeps_the_edit() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for c in "run it".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);
    p.on_input(ChatInput::Up, &cwd);
    p.on_input(ChatInput::Char('!'), &cwd);
    p.on_input(ChatInput::Down, &cwd);
    assert_eq!(p.input, "run it!");
}

/// The rest of a previous prompt, offered as you retype its beginning — the
/// input bar's fish-style autosuggestion, which the composer did not have.
#[test]
fn typing_the_start_of_a_past_prompt_suggests_the_rest() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for c in "refactor the parser".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);
    for c in "refac".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    assert_eq!(p.ghost().as_deref(), Some("tor the parser"));

    // Right takes it, and the result is the user's own text — Down must not
    // then swap it for a stashed draft.
    p.on_input(ChatInput::Accept, &cwd);
    assert_eq!(p.input, "refactor the parser");
    p.on_input(ChatInput::Down, &cwd);
    assert_eq!(p.input, "refactor the parser");
}

/// Tab already meant "complete the leading token". It keeps that meaning and
/// gains the ghost only where it previously did nothing at all.
#[test]
fn tab_completes_a_token_first_and_the_suggestion_otherwise() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for c in "deploy to staging".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);
    for c in "dep".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Complete, &cwd);
    assert_eq!(p.input, "deploy to staging", "Tab took the suggestion");
}

/// A leading `/` belongs to the palette, which is already showing the same
/// answer as a popup — two answers to one question is worse than one.
#[test]
fn a_slash_command_is_the_palettes_to_answer_not_the_ghosts() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    for c in "/doctor".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Close, &cwd); // dismiss the palette
    p.on_input(ChatInput::Enter, &cwd);
    for c in "/doc".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    assert_eq!(p.ghost(), None, "the palette owns a leading slash");
}

#[test]
fn nothing_typed_and_nothing_matching_suggest_nothing() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    assert_eq!(p.ghost(), None, "an empty composer suggests nothing");
    for c in "one".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);
    for c in "zzz".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    assert_eq!(p.ghost(), None);
    // Accepting nothing is not an error — both keys are pressed speculatively.
    p.on_input(ChatInput::Accept, &cwd);
    p.on_input(ChatInput::Complete, &cwd);
    assert_eq!(p.input, "zzz");
}

/// Esc means stop. Left alone it stopped one run and immediately started
/// every follow-up queued behind it — cancelling is exactly what makes the
/// pane idle, which is exactly what flushes the queue.
#[test]
fn interrupting_drops_what_was_queued_behind_the_run() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    p.absorb_activity("planner".into(), "thinking", "user".into());
    assert!(p.is_busy());

    // Two follow-ups typed while it worked: queued, not sent.
    for text in ["and then deploy it", "then tag a release"] {
        for c in text.chars() {
            p.on_input(ChatInput::Char(c), &cwd);
        }
        p.on_input(ChatInput::Enter, &cwd);
    }
    assert_eq!(p.queued.len(), 2, "the follow-ups should have queued");

    p.on_input(ChatInput::Close, &cwd);
    assert!(p.queued.is_empty(), "queued work survived the interrupt");
    // …and says so, because two things the user typed just stopped existing.
    let note = &p.messages.last().expect("a note").text;
    assert!(note.contains("dropped 2 queued messages"), "{note}");
}

/// A second Esc has nothing left to drop, so it writes the plain note — which
/// is deduped against the one already there, exactly as before.
#[test]
fn a_second_interrupt_with_an_empty_queue_says_the_plain_thing_once() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    p.absorb_activity("planner".into(), "thinking", "user".into());
    for c in "one more thing".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);

    p.on_input(ChatInput::Close, &cwd);
    let first = p.messages.len();
    assert!(p
        .messages
        .last()
        .unwrap()
        .text
        .contains("dropped 1 queued message"));

    p.on_input(ChatInput::Close, &cwd);
    assert_eq!(p.messages.len(), first + 1, "the plain note is a new note");
    p.on_input(ChatInput::Close, &cwd);
    assert_eq!(p.messages.len(), first + 1, "…and then deduped");
}

/// Nothing queued, nothing said about it — the note is unchanged for the
/// ordinary interrupt.
#[test]
fn interrupting_with_an_empty_queue_reads_exactly_as_it_did() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    p.connected = true;
    p.absorb_activity("planner".into(), "thinking", "user".into());
    p.on_input(ChatInput::Close, &std::env::temp_dir());
    let note = &p.messages.last().unwrap().text;
    assert!(
        note.contains("interrupting") && !note.contains("dropped"),
        "{note}"
    );
}

/// Typed rather than pressed is not a different intention: a bare `/stop`
/// takes the queue with it exactly as Esc does. Esc sends this very string.
#[test]
fn a_typed_stop_drops_the_queue_too() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    p.absorb_activity("planner".into(), "thinking", "user".into());
    for text in ["and then deploy", "then tag it"] {
        for c in text.chars() {
            p.on_input(ChatInput::Char(c), &cwd);
        }
        p.on_input(ChatInput::Enter, &cwd);
    }
    assert_eq!(p.queued.len(), 2);

    for c in "/stop".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    // Dismiss the palette a leading `/` opened — and ONLY if it is open. Esc
    // with no popup reaches the pane, which while busy is the interrupt, and
    // the interrupt drops the queue: the test would have passed without the
    // typed path doing anything at all.
    assert!(p.palette.is_some());
    p.on_input(ChatInput::Close, &cwd);
    p.on_input(ChatInput::Enter, &cwd);

    assert!(p.queued.is_empty(), "queued work survived a typed /stop");
    assert!(
        p.messages
            .iter()
            .any(|m| m.text.contains("dropped 2 queued messages")),
        "the drop was silent: {:?}",
        p.messages.iter().map(|m| &m.text).collect::<Vec<_>>()
    );
}

/// `/stop #2` calls off ONE of several parallel tasks. The rest of the
/// session is still meant, and so is the rest of the queue — dropping it
/// would cancel work the user never asked to cancel.
#[test]
fn stopping_one_task_by_id_leaves_the_queue_alone() {
    use crate::chatkeys::ChatInput;

    let mut p = pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    p.absorb_activity("planner".into(), "thinking", "user".into());
    for c in "keep this one".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    p.on_input(ChatInput::Enter, &cwd);
    assert_eq!(p.queued.len(), 1);

    for c in "/stop #2".chars() {
        p.on_input(ChatInput::Char(c), &cwd);
    }
    // No palette here (the argument closed it), so no Esc — sending one
    // would be the interrupt, not the command under test.
    assert!(p.palette.is_none());
    p.on_input(ChatInput::Enter, &cwd);

    assert_eq!(p.queued.len(), 1, "a targeted stop cancelled everything");
}
