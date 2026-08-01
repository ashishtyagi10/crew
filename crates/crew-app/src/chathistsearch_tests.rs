use super::*;
use crate::chatkeys::ChatInput;

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Press Ctrl+R over `input` with `hist` recorded; the popup must open.
fn open_over(input: &str, hist: &[&str]) -> (Option<HistSearch>, String, Vec<String>) {
    let mut state = None;
    let mut inp = input.to_string();
    let l = lines(hist);
    let k = popup_key(&mut state, &mut inp, &l, &ChatInput::HistSearch);
    assert!(matches!(k, HistKey::Consumed), "Ctrl+R must be consumed");
    assert!(state.is_some(), "Ctrl+R must open the search popup");
    (state, inp, l)
}

#[test]
fn ctrl_r_opens_listing_all_history_newest_first() {
    let (state, inp, _) = open_over("draft", &["one", "two", "three"]);
    let s = state.unwrap();
    assert_eq!(s.matches, lines(&["three", "two", "one"]));
    assert_eq!(s.sel, 0, "selection starts on the newest entry");
    assert_eq!(s.query, "", "the query starts empty");
    assert_eq!(inp, "draft", "opening must not touch the composer");
}

#[test]
fn ctrl_r_with_no_history_opens_nothing() {
    let mut state = None;
    let mut inp = String::new();
    let k = popup_key(&mut state, &mut inp, &[], &ChatInput::HistSearch);
    assert!(matches!(k, HistKey::Consumed));
    assert!(state.is_none(), "nothing to search — no popup");
}

#[test]
fn other_keys_forward_while_closed() {
    let mut state = None;
    let mut inp = String::new();
    let k = popup_key(&mut state, &mut inp, &lines(&["x"]), &ChatInput::Enter);
    assert!(matches!(k, HistKey::Forward));
    assert!(state.is_none());
}

#[test]
fn typing_builds_the_query_and_filters_newest_first() {
    let (mut state, mut inp, l) = open_over("", &["git status", "run tests", "git push"]);
    for c in "git".chars() {
        popup_key(&mut state, &mut inp, &l, &ChatInput::Char(c));
    }
    let s = state.as_ref().unwrap();
    assert_eq!(s.query, "git");
    assert_eq!(s.matches, lines(&["git push", "git status"]));
    assert_eq!(
        s.sel, 0,
        "narrowing resets the selection to the newest match"
    );
    assert_eq!(inp, "", "typing edits the query, not the composer");
}

#[test]
fn backspace_edits_the_query_and_widens_back_to_everything() {
    let (mut state, mut inp, l) = open_over("", &["alpha", "beta"]);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Char('a'));
    popup_key(&mut state, &mut inp, &l, &ChatInput::Char('l'));
    assert_eq!(state.as_ref().unwrap().matches, lines(&["alpha"]));
    popup_key(&mut state, &mut inp, &l, &ChatInput::Backspace);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Backspace);
    let s = state.as_ref().unwrap();
    assert_eq!(s.query, "", "backspace removed both chars");
    assert_eq!(
        s.matches,
        lines(&["beta", "alpha"]),
        "empty query lists all"
    );
}

#[test]
fn filter_ranks_substring_hits_before_subsequence_hits() {
    // "abc marker" contains "abc"; "axbxc" only has it as a subsequence —
    // even though "axbxc" is more recent, the substring hit ranks first.
    let got = filter(&lines(&["abc marker", "axbxc"]), "abc");
    assert_eq!(got, lines(&["abc marker", "axbxc"]));
    assert!(filter(&lines(&["one"]), "zzz").is_empty());
}

#[test]
fn ctrl_r_again_steps_to_the_next_older_match_and_clamps() {
    let (mut state, mut inp, l) = open_over("", &["one", "two", "three"]);
    popup_key(&mut state, &mut inp, &l, &ChatInput::HistSearch);
    assert_eq!(state.as_ref().unwrap().sel, 1, "second Ctrl+R steps older");
    popup_key(&mut state, &mut inp, &l, &ChatInput::HistSearch);
    assert_eq!(state.as_ref().unwrap().sel, 2);
    popup_key(&mut state, &mut inp, &l, &ChatInput::HistSearch);
    assert_eq!(
        state.as_ref().unwrap().sel,
        2,
        "clamped at the oldest match"
    );
}

#[test]
fn up_down_move_the_selection() {
    let (mut state, mut inp, l) = open_over("", &["one", "two"]);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Down);
    assert_eq!(state.as_ref().unwrap().sel, 1);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Down);
    assert_eq!(state.as_ref().unwrap().sel, 1, "clamped at the bottom");
    popup_key(&mut state, &mut inp, &l, &ChatInput::Up);
    assert_eq!(state.as_ref().unwrap().sel, 0);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Up);
    assert_eq!(state.as_ref().unwrap().sel, 0, "clamped at the top");
}

#[test]
fn enter_accepts_the_selection_into_the_input() {
    let (mut state, mut inp, l) = open_over("draft", &["one", "two"]);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Down);
    let k = popup_key(&mut state, &mut inp, &l, &ChatInput::Enter);
    assert!(matches!(k, HistKey::Accepted));
    assert_eq!(inp, "one", "the selected entry replaces the composer text");
    assert!(state.is_none(), "accepting closes the popup");
}

#[test]
fn esc_restores_the_prior_input_and_closes() {
    let (mut state, mut inp, l) = open_over("my draft", &["one", "two"]);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Char('o'));
    popup_key(&mut state, &mut inp, &l, &ChatInput::Down);
    let k = popup_key(&mut state, &mut inp, &l, &ChatInput::Close);
    assert!(matches!(k, HistKey::Consumed));
    assert_eq!(
        inp, "my draft",
        "Esc restores what was typed before opening"
    );
    assert!(state.is_none(), "Esc closes the popup, not the pane");
}

#[test]
fn items_flatten_newlines_and_show_a_placeholder_when_nothing_matches() {
    let (mut state, mut inp, l) = open_over("", &["one\ntwo"]);
    let rows = items(state.as_ref().unwrap());
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].label.contains('\n'),
        "multiline prompts render flat"
    );
    assert!(!rows[0].header);
    popup_key(&mut state, &mut inp, &l, &ChatInput::Char('z'));
    let rows = items(state.as_ref().unwrap());
    assert_eq!(rows.len(), 1, "an unmatched query still shows one row");
    assert!(rows[0].header, "the placeholder is a dim non-choice row");
    assert_eq!(title(state.as_ref().unwrap()), "history search: z");
}

// The pane-level routing: Ctrl+R must reach the popup before the palette and
// the composer, and accepting must NOT send anything to the broker.
#[test]
fn on_input_opens_search_and_enter_accepts_without_sending() {
    let mut p = crate::chat::tests::pane();
    let cwd = std::env::temp_dir();
    p.connected = true;
    p.history.record("earlier prompt");
    p.input = "draft".to_string();

    assert!(p.on_input(ChatInput::HistSearch, &cwd).is_none());
    assert!(p.histsearch.is_some(), "Ctrl+R opens the search popup");

    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert!(p.histsearch.is_none(), "Enter closes the popup");
    assert_eq!(p.input, "earlier prompt", "the entry lands in the composer");
    assert!(p.messages.is_empty(), "accepting must not send or echo");
    assert!(!p.is_busy(), "nothing was sent to the broker");
}

#[test]
fn opening_search_closes_the_palette_and_mention_popups() {
    // Ctrl+R is modal over both popups; leaving one armed underneath is
    // state the renderer hides but the key routing would resurrect.
    let mut p = crate::chat::tests::pane();
    let cwd = std::env::temp_dir();
    p.history.record("earlier prompt");
    p.on_input(ChatInput::Char('/'), &cwd);
    assert!(p.palette.is_some(), "premise: '/' opened the palette");
    p.mention = Some(crate::chatmention::MentionState {
        entries: Vec::new(),
        matches: Vec::new(),
        sel: 0,
    });
    assert!(p.on_input(ChatInput::HistSearch, &cwd).is_none());
    assert!(p.histsearch.is_some(), "Ctrl+R opened the search");
    assert!(p.palette.is_none(), "opening search disarms the palette");
    assert!(
        p.mention.is_none(),
        "opening search disarms the mention popup"
    );
}

#[test]
fn accepting_a_search_entry_disarms_stale_popups() {
    // A palette/mention still armed when an entry is accepted would eat the
    // next Enter — possibly submitting a stale palette row against the
    // recalled line. Accept must clear both, like Up/Down recall does.
    let mut p = crate::chat::tests::pane();
    let cwd = std::env::temp_dir();
    p.history.record("earlier prompt");
    p.on_input(ChatInput::HistSearch, &cwd);
    assert!(p.histsearch.is_some());
    p.palette = Some(crate::chatpalette::PaletteState {
        kind: crate::chatpalette::Kind::Slash,
        items: Vec::new(),
        sel: 0,
        entries: Vec::new(),
        touched: false,
    });
    p.mention = Some(crate::chatmention::MentionState {
        entries: Vec::new(),
        matches: Vec::new(),
        sel: 0,
    });
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert!(
        p.histsearch.is_none(),
        "Enter accepted and closed the search"
    );
    assert_eq!(p.input, "earlier prompt");
    assert!(p.palette.is_none(), "accept disarms a stale palette");
    assert!(p.mention.is_none(), "accept disarms a stale mention popup");
}

#[test]
fn on_input_esc_closes_the_popup_not_the_pane() {
    let mut p = crate::chat::tests::pane();
    let cwd = std::env::temp_dir();
    p.history.record("earlier prompt");
    p.input = "draft".to_string();
    p.on_input(ChatInput::HistSearch, &cwd);
    assert!(
        p.on_input(ChatInput::Close, &cwd).is_none(),
        "Esc with the popup open must not close the pane"
    );
    assert!(p.histsearch.is_none());
    assert_eq!(p.input, "draft");
}
