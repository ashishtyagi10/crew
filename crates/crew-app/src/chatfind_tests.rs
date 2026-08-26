use super::*;
use crate::chatkeys::{ChatAction, ChatInput};

const LONG: &str = "one\ntwo\nthree\nfour needle here\nfive"; // folds: 5 body lines

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

/// A pane whose transcript is `texts`, oldest first.
fn pane_with(texts: &[(&str, &str)]) -> ChatPane {
    let mut p = crate::chat::tests::pane();
    for (s, t) in texts {
        p.push_capped(msg(s, t));
    }
    p
}

/// Open find over `p` and type `query` through the real key routing.
fn open_typing(p: &mut ChatPane, query: &str) {
    let cwd = std::env::temp_dir();
    assert!(p.on_input(ChatInput::FindNext, &cwd).is_none());
    assert!(p.find.is_some(), "Ctrl+F opens the find popup");
    for c in query.chars() {
        assert!(matches!(
            p.on_input(ChatInput::Char(c), &cwd),
            Some(ChatAction::FindJump)
        ));
    }
}

#[test]
fn filter_matches_case_insensitively_newest_first() {
    let msgs = [
        msg("coder", "Alpha Beta"),
        msg("user", "gamma"),
        msg("crew", "an ALPHA again"),
    ];
    let refs: Vec<&Message> = msgs.iter().collect();
    assert_eq!(filter(&refs, "alpha"), vec![2, 0], "newest match first");
    assert_eq!(filter(&refs, "zzz"), Vec::<usize>::new());
    assert_eq!(
        filter(&refs, ""),
        Vec::<usize>::new(),
        "an empty query matches nothing (no jumping before typing)"
    );
}

#[test]
fn ctrl_f_decodes_as_the_find_chord() {
    use winit::keyboard::Key;
    assert_eq!(
        crate::chatkeys::chat_key(&Key::Character("f".into()), true, false, true),
        ChatInput::FindNext
    );
    assert_eq!(
        crate::chatkeys::chat_key(&Key::Character("F".into()), true, false, true),
        ChatInput::FindNext
    );
    assert_eq!(
        crate::chatkeys::chat_key(&Key::Character("f".into()), true, false, false),
        ChatInput::Char('f')
    );
}

#[test]
fn typing_edits_the_query_and_esc_leaves_the_composer_draft_untouched() {
    let mut p = pane_with(&[("coder", "hello world")]);
    p.input = "my draft".to_string();
    open_typing(&mut p, "hello");
    let f = p.find.as_ref().unwrap();
    assert_eq!(f.query, "hello");
    assert_eq!(f.matches, vec![0]);
    assert_eq!(
        p.input, "my draft",
        "typing edits the query, not the composer"
    );
    let cwd = std::env::temp_dir();
    assert!(p.on_input(ChatInput::Close, &cwd).is_none());
    assert!(p.find.is_none(), "Esc closes the popup, not the pane");
    assert_eq!(p.input, "my draft", "Esc never rewrites the composer");
}

#[test]
fn enter_steps_older_up_steps_newer_both_wrap_and_the_title_counts() {
    let mut p = pane_with(&[
        ("coder", "needle a"),
        ("user", "nothing"),
        ("coder", "needle b"),
        ("crew", "needle c"),
    ]);
    open_typing(&mut p, "needle");
    let cwd = std::env::temp_dir();
    let sel = |p: &ChatPane| {
        let f = p.find.as_ref().unwrap();
        f.matches[f.sel]
    };
    assert_eq!(sel(&p), 3, "starts on the newest match");
    assert_eq!(title(p.find.as_ref().unwrap()), "find: needle (1/3)");
    assert!(matches!(
        p.on_input(ChatInput::Enter, &cwd),
        Some(ChatAction::FindJump)
    ));
    assert_eq!(sel(&p), 2, "Enter steps to the next older match");
    assert_eq!(title(p.find.as_ref().unwrap()), "find: needle (2/3)");
    p.on_input(ChatInput::FindNext, &cwd);
    assert_eq!(sel(&p), 0, "Ctrl+F steps older again");
    p.on_input(ChatInput::Down, &cwd);
    assert_eq!(sel(&p), 3, "Down wraps from the oldest back to the newest");
    p.on_input(ChatInput::Up, &cwd);
    assert_eq!(sel(&p), 0, "Up (newer) wraps from the newest to the oldest");
    p.on_input(ChatInput::Up, &cwd);
    assert_eq!(sel(&p), 2, "Up steps to the next newer match");
}

#[test]
fn scroll_for_puts_the_target_inside_the_window() {
    for (total, budget, target) in [
        (100usize, 10usize, 0usize), // very first line
        (100, 10, 99),               // very last line
        (100, 10, 50),               // mid-transcript
        (5, 10, 3),                  // everything fits
        (100, 1, 42),                // one-row window
    ] {
        let scroll = scroll_for(total, budget, target);
        let max_start = total.saturating_sub(budget);
        assert!(scroll <= max_start, "clamped to the scrollback");
        let start = max_start - scroll;
        assert!(
            (start..start + budget.max(1)).contains(&target),
            "target {target} outside window [{start}, {})",
            start + budget
        );
    }
    assert_eq!(scroll_for(100, 0, 5), 0, "a zero-row window never scrolls");
}

#[test]
fn jump_scrolls_the_match_line_into_the_drawn_window() {
    let (cols, rows) = (40u16, 12u16);
    let mut texts = vec![("coder", "the needle sits up here")];
    for _ in 0..30 {
        texts.push(("user", "filler chatter"));
    }
    let mut p = pane_with(&texts);
    open_typing(&mut p, "needle");
    jump(&mut p, cols, rows);
    // Assert against the span machinery: the matched message's line span
    // must intersect the drawn window under chatscroll's bottom-anchored
    // geometry.
    let visible = p.visible_messages();
    let view = crate::chatmsgs::View {
        gap_rows: crate::density::Density::Cozy.card_gap_rows(),
        source: false,
        compact: false,
        streaming_from: p.messages.len(),
    };
    let (lines, spans) = crate::chatmsgs::card_lines_spanned(&visible, cols as usize, 0, view);
    let budget = crate::chatplace::msg_rows_budget(&p, cols, rows) as usize;
    let start = lines.len().saturating_sub(budget).saturating_sub(p.scroll);
    let window = start..start + budget;
    assert!(
        spans[0].start >= window.start && spans[0].start < window.end,
        "match span {:?} outside window {window:?} (scroll {})",
        spans[0],
        p.scroll
    );
    // And in the actual placed render: some row shows the needle.
    let shown: Vec<String> = crate::chatplace::placed_lines(&p, cols, rows)
        .iter()
        .map(|(_, l)| l.iter().map(|c| c.c).collect())
        .collect();
    assert!(
        shown.iter().any(|l| l.contains("needle")),
        "jump must scroll the matched line on screen; visible: {shown:?}"
    );
}

#[test]
fn jump_expands_a_folded_system_card_so_the_hidden_line_exists() {
    let (cols, rows) = (40u16, 20u16);
    let mut p = pane_with(&[("crew", LONG), ("user", "later chatter")]);
    // Premise: the needle line is hidden while the card is folded.
    let folded: Vec<String> = crate::chatplace::placed_lines(&p, cols, rows)
        .iter()
        .map(|(_, l)| l.iter().map(|c| c.c).collect())
        .collect();
    assert!(
        !folded.iter().any(|l| l.contains("needle")),
        "premise: the match is inside the folded tail"
    );
    open_typing(&mut p, "needle");
    jump(&mut p, cols, rows);
    assert!(p.messages[0].expanded, "jump clicks the folded card open");
    let shown: Vec<String> = crate::chatplace::placed_lines(&p, cols, rows)
        .iter()
        .map(|(_, l)| l.iter().map(|c| c.c).collect())
        .collect();
    assert!(
        shown.iter().any(|l| l.contains("needle")),
        "the hidden line is on screen after the jump; visible: {shown:?}"
    );
}

#[test]
fn only_one_modal_opening_find_closes_histsearch_and_vice_versa() {
    let mut p = pane_with(&[("coder", "hello")]);
    let cwd = std::env::temp_dir();
    p.history.record("earlier prompt");
    p.input = "draft".to_string();
    p.on_input(ChatInput::HistSearch, &cwd);
    assert!(p.histsearch.is_some(), "premise: Ctrl+R opened histsearch");
    p.on_input(ChatInput::FindNext, &cwd);
    assert!(p.find.is_some(), "Ctrl+F opens find over histsearch");
    assert!(p.histsearch.is_none(), "opening find closes histsearch");
    assert_eq!(p.input, "draft", "histsearch close restored the draft");
    p.on_input(ChatInput::HistSearch, &cwd);
    assert!(
        p.histsearch.is_some(),
        "Ctrl+R reopens histsearch over find"
    );
    assert!(p.find.is_none(), "opening histsearch closes find");
}

#[test]
fn opening_find_disarms_the_palette_and_mention_popups() {
    let mut p = pane_with(&[("coder", "hello")]);
    let cwd = std::env::temp_dir();
    p.on_input(ChatInput::Char('/'), &cwd);
    assert!(p.palette.is_some(), "premise: '/' opened the palette");
    p.mention = Some(crate::chatmention::MentionState {
        entries: Vec::new(),
        matches: Vec::new(),
        sel: 0,
    });
    p.on_input(ChatInput::FindNext, &cwd);
    assert!(p.find.is_some());
    assert!(p.palette.is_none(), "opening find disarms the palette");
    assert!(
        p.mention.is_none(),
        "opening find disarms the mention popup"
    );
}

#[test]
fn a_shrinking_transcript_never_panics_and_matches_rescan() {
    let (cols, rows) = (40u16, 20u16);
    let mut p = pane_with(&[
        ("coder", "needle one"),
        ("user", "chatter"),
        ("coder", "needle two"),
    ]);
    open_typing(&mut p, "needle");
    // The transcript shrinks out from under the recorded matches.
    p.messages.clear();
    jump(&mut p, cols, rows); // must not panic
    assert!(
        p.find.as_ref().unwrap().matches.is_empty(),
        "jump rescans against the live transcript"
    );
    let cwd = std::env::temp_dir();
    p.on_input(ChatInput::Enter, &cwd); // must not panic either
    p.push_capped(msg("coder", "a fresh needle lands"));
    p.on_input(ChatInput::Enter, &cwd);
    let f = p.find.as_ref().unwrap();
    assert_eq!(
        f.matches,
        vec![0],
        "new messages are picked up on the next key"
    );
    assert_eq!(f.matches[f.sel], 0);
}

#[test]
fn fold_clicks_are_inert_while_find_is_open() {
    let (cols, rows) = (40u16, 20u16);
    let mut p = pane_with(&[("crew", LONG)]);
    let placed = crate::chatplace::placed_lines(&p, cols, rows);
    let suffix_row = placed
        .iter()
        .find(|(_, l)| {
            l.iter()
                .map(|c| c.c)
                .collect::<String>()
                .contains("\u{2026} +")
        })
        .map(|(r, _)| *r)
        .expect("a folded suffix row");
    p.find = Some(ChatFind {
        query: String::new(),
        matches: Vec::new(),
        sel: 0,
    });
    assert!(
        !p.toggle_fold_at(cols, rows, suffix_row),
        "find popup open: fold clicks are inert"
    );
    p.find = None;
    assert!(
        p.toggle_fold_at(cols, rows, suffix_row),
        "no popup: the same click toggles again"
    );
}

#[test]
fn the_current_match_substring_is_washed_with_the_find_highlight() {
    let _g = crate::app::theme_test_guard();
    let (cols, rows) = (40u16, 12u16);
    let mut p = pane_with(&[("coder", "a needle in the reply")]);
    open_typing(&mut p, "needle");
    jump(&mut p, cols, rows);
    let cells = crate::chatview::cells(&p, cols, rows);
    let hl = crew_theme::theme().find_hl_bg;
    let washed: String = {
        let mut w: Vec<(u16, u16, char)> = cells
            .iter()
            .filter(|c| c.bg == hl)
            .map(|c| (c.row, c.col, c.c))
            .collect();
        w.sort_unstable();
        w.into_iter().map(|(_, _, c)| c).collect()
    };
    assert_eq!(washed, "needle", "exactly the matched substring is washed");
}

#[test]
fn cmd_f_opens_find_on_the_focused_chat_pane_and_enter_jumps() {
    let _g = crate::app::theme_test_guard();
    let mut app = crate::app::CrewApp::default();
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    app.panes.push(crate::pane::Pane {
        content: crate::pane::PaneContent::Chat(crate::chat::ChatPane::new(plugin, "crew".into())),
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
    });
    assert!(!app.handle_super_chord("f"), "Cmd+F never exits the app");
    let crate::pane::PaneContent::Chat(c) = &app.panes[0].content else {
        panic!("chat pane replaced");
    };
    assert!(c.find.is_some(), "Cmd+F opens the find popup");
    // The FindJump action reaches chatfind::jump through the app.
    app.apply_chat_action(ChatAction::FindJump, 0); // no panic on empty query
}
