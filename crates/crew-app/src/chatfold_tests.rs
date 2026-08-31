use super::*;
use crate::chatmsgs::{card_line_count, card_lines};

const LONG: &str = "one\ntwo\nthree\nfour\nfive"; // 5 body lines > FOLD_LINES
const SHORT: &str = "one\ntwo\nthree"; // exactly FOLD_LINES — stays open

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

fn text(line: &crate::chatbody::CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

#[test]
fn long_system_card_auto_folds_to_header_first_line_and_suffix() {
    let m = msg("crew", LONG);
    let lines = card_lines(&[&m], 40, 0, View::default());
    assert_eq!(
        lines.len(),
        2,
        "header + clamped first body line only, got: {:?}",
        lines.iter().map(text).collect::<Vec<_>>()
    );
    let body = text(&lines[1]);
    assert!(body.starts_with(" one"), "first body line kept: {body}");
    assert!(body.contains("\u{2026} +4"), "4 hidden lines noted: {body}");
}

#[test]
fn system_card_at_the_fold_threshold_stays_open() {
    let m = msg("crew", SHORT);
    let lines = card_lines(&[&m], 40, 0, View::default());
    assert_eq!(lines.len(), 4, "header + all 3 body lines");
    assert!(!text(&lines[1]).contains('\u{2026}'), "no hidden suffix");
}

#[test]
fn long_agent_reply_never_auto_folds() {
    let m = msg("coder", LONG);
    let lines = card_lines(&[&m], 40, 0, View::default());
    assert_eq!(lines.len(), 6, "header + all 5 body lines");
}

#[test]
fn splash_nameplate_never_folds() {
    // Box art clamped to one `╔` line would be destroyed, not summarized.
    let art = "\u{2554}aa\n\u{2551}bb\n\u{2551}cc\n\u{2551}dd\n\u{255a}ee";
    let lines = card_lines(&[&msg("crew", art)], 40, 0, View::default());
    assert_eq!(lines.len(), 5, "headerless splash keeps every line");
}

#[test]
fn expanded_system_card_renders_its_full_body() {
    let mut m = msg("crew", LONG);
    m.expanded = true;
    let lines = card_lines(&[&m], 40, 0, View::default());
    assert_eq!(
        lines.len(),
        6,
        "header + all 5 body lines once clicked open"
    );
}

#[test]
fn compact_view_wins_over_per_message_expansion() {
    let mut m = msg("crew", LONG);
    m.expanded = true;
    let view = View {
        compact: true,
        ..View::default()
    };
    let lines = card_lines(&[&m], 40, 0, view);
    assert_eq!(lines.len(), 2, "Ctrl+O clamps even a clicked-open card");
    assert!(text(&lines[1]).contains("\u{2026} +4"));
}

#[test]
fn card_line_count_agrees_with_card_lines_in_both_fold_states() {
    for expanded in [false, true] {
        let mut m = msg("crew", LONG);
        m.expanded = expanded;
        let v = View::default();
        assert_eq!(
            card_line_count(&[&m], 40, v),
            card_lines(&[&m], 40, 0, v).len(),
            "count/lines drift with expanded={expanded}"
        );
    }
}

#[test]
fn folded_and_foldable_gate_on_voice_length_and_expansion() {
    let v = View::default();
    assert!(folded(&msg("crew", LONG), 5) && foldable(&msg("crew", LONG), 40, v));
    assert!(!folded(&msg("coder", LONG), 5), "agents never fold");
    assert!(!foldable(&msg("coder", LONG), 40, v));
    assert!(!folded(&msg("crew", SHORT), 3), "3 lines is not long");
    assert!(!foldable(&msg("crew", SHORT), 40, v));
    let mut open = msg("crew", LONG);
    open.expanded = true;
    assert!(!folded(&open, 5), "clicked open renders full");
    assert!(foldable(&open, 40, v), "but stays toggleable");
}

#[test]
fn line_index_at_mirrors_the_bottom_anchored_window() {
    // 10 lines in 4 rows starting at row 1: rows 1..5 show lines 6..10.
    assert_eq!(line_index_at(10, 4, 1, 0, 1), Some(6));
    assert_eq!(line_index_at(10, 4, 1, 0, 4), Some(9));
    assert_eq!(line_index_at(10, 4, 1, 0, 0), None, "the header row");
    assert_eq!(line_index_at(10, 4, 1, 0, 5), None, "below the window");
    // Scrolled 2 up: the window slides back to lines 4..8.
    assert_eq!(line_index_at(10, 4, 1, 2, 1), Some(4));
    // Fewer lines than rows: the three lines sit on the BOTTOM of the ten,
    // so rows 2..9 are the slack above them and row 9 is the last line.
    assert_eq!(line_index_at(3, 10, 2, 0, 2), None, "slack above the cards");
    assert_eq!(line_index_at(3, 10, 2, 0, 9), Some(0));
    assert_eq!(line_index_at(3, 10, 2, 0, 11), Some(2));
    assert_eq!(line_index_at(3, 10, 2, 0, 12), None, "past the last line");
    // An overshooting scroll clamps like `window` does.
    assert_eq!(line_index_at(10, 4, 1, 100, 1), Some(0));
}

/// Pane-relative text rows as drawn — click targets are located in the real
/// render, never hardcoded (mirrors `clickopen`'s own click-target lookups).
fn rendered(p: &ChatPane, cols: u16, rows: u16) -> std::collections::BTreeMap<u16, String> {
    let mut m: std::collections::BTreeMap<u16, Vec<(u16, char)>> = Default::default();
    for c in crate::chatview::cells(p, cols, rows) {
        m.entry(c.row).or_default().push((c.col, c.c));
    }
    m.into_iter()
        .map(|(r, mut v)| {
            v.sort_unstable();
            (r, v.into_iter().map(|(_, c)| c).collect())
        })
        .collect()
}

fn row_with(p: &ChatPane, cols: u16, rows: u16, needle: &str) -> u16 {
    *rendered(p, cols, rows)
        .iter()
        .find(|(_, t)| t.contains(needle))
        .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        .0
}

#[test]
fn clicking_the_folded_card_expands_and_its_header_refolds() {
    let (cols, rows) = (40u16, 20u16);
    let mut p = crate::chat::tests::pane();
    p.push_capped(msg("crew", LONG));
    let suffix_row = row_with(&p, cols, rows, "\u{2026} +4");
    assert!(p.toggle_fold_at(cols, rows, suffix_row), "the fold toggles");
    assert!(p.messages[0].expanded, "the card is clicked open");
    // Body clicks on the open card do NOT refold — they stay free for
    // text selection; only the header folds it back.
    let body_row = row_with(&p, cols, rows, "four");
    assert!(!p.toggle_fold_at(cols, rows, body_row), "body click inert");
    assert!(p.messages[0].expanded);
    let header_row = row_with(&p, cols, rows, "\u{2506}crew");
    assert!(p.toggle_fold_at(cols, rows, header_row), "header refolds");
    assert!(!p.messages[0].expanded);
}

#[test]
fn agent_cards_and_compact_view_never_fold_toggle() {
    let (cols, rows) = (40u16, 20u16);
    let mut p = crate::chat::tests::pane();
    p.push_capped(msg("coder", LONG));
    let body_row = row_with(&p, cols, rows, "four");
    assert!(!p.toggle_fold_at(cols, rows, body_row), "agent cards inert");
    p.push_capped(msg("crew", LONG));
    p.compact_view = true;
    let suffix_row = row_with(&p, cols, rows, "\u{2026} +4");
    assert!(
        !p.toggle_fold_at(cols, rows, suffix_row),
        "Ctrl+O compact view wins — nothing toggles under it"
    );
}

#[test]
fn fold_clicks_are_inert_while_a_popup_overlays_the_transcript() {
    // A click on a Ctrl+R/palette/mention popup row must not fall through
    // and toggle the fold of the card invisibly beneath it.
    let (cols, rows) = (40u16, 20u16);
    let mut p = crate::chat::tests::pane();
    p.push_capped(msg("crew", LONG));
    let suffix_row = row_with(&p, cols, rows, "\u{2026} +4");
    p.histsearch = Some(crate::chathistsearch::HistSearch {
        query: String::new(),
        saved: String::new(),
        matches: vec!["x".into()],
        sel: 0,
    });
    assert!(
        !p.toggle_fold_at(cols, rows, suffix_row),
        "search popup open: fold clicks are inert"
    );
    p.histsearch = None;
    p.mention = Some(crate::chatmention::MentionState {
        entries: Vec::new(),
        matches: Vec::new(),
        sel: 0,
    });
    assert!(
        !p.toggle_fold_at(cols, rows, suffix_row),
        "mention popup open: fold clicks are inert"
    );
    p.mention = None;
    p.palette = Some(crate::chatpalette::PaletteState {
        kind: crate::chatpalette::Kind::Slash,
        items: Vec::new(),
        sel: 0,
        entries: Vec::new(),
        touched: false,
    });
    assert!(
        !p.toggle_fold_at(cols, rows, suffix_row),
        "palette open: fold clicks are inert"
    );
    p.palette = None;
    assert!(
        p.toggle_fold_at(cols, rows, suffix_row),
        "no popup: the same click toggles again"
    );
}

// --- Press/release split: the toggle fires on mouse RELEASE, and only when
// the gesture stayed a plain click (see `events.rs` / `fold_release`) ---

/// An app whose pane 0 is a chat pane showing one folded system card, plus
/// the absolute row of the card's fold suffix (located in the real render).
fn app_with_folded_card() -> (crate::app::CrewApp, u16) {
    let (cols, rows) = (40u16, 20u16);
    let mut chat = crate::chat::tests::pane();
    chat.push_capped(msg("crew", LONG));
    let suffix_row = row_with(&chat, cols, rows, "\u{2026} +4");
    let mut app = crate::app::CrewApp::default();
    app.panes.push(crate::pane::Pane {
        glide: crate::glide::Glide::default(),
        content: crate::pane::PaneContent::Chat(chat),
        grid: crew_term::GridSize { cols, rows },
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
    (app, suffix_row)
}

fn card_expanded(app: &crate::app::CrewApp) -> bool {
    match &app.panes[0].content {
        crate::pane::PaneContent::Chat(c) => c.messages[0].expanded,
        _ => unreachable!("pane 0 is a chat pane"),
    }
}

#[test]
fn a_stationary_click_release_fires_the_armed_fold_toggle() {
    // The press arms a candidate (geometry needs a live renderer, so tests
    // arm it directly, as `fold_press_at_cursor` would); a release that
    // never became a drag fires it.
    let (mut app, row) = app_with_folded_card();
    app.fold_click = Some((0, row));
    assert!(app.fold_release(false), "a plain click toggles on release");
    assert!(card_expanded(&app), "the folded card is clicked open");
    assert!(app.fold_click.is_none(), "the candidate is consumed");
}

#[test]
fn a_drag_release_consumes_the_candidate_without_toggling() {
    // Starting a drag-selection ON a folded card must not expand it — the
    // layout would shift under the cursor mid-gesture. The drag still copies
    // its selection (`selection_release`, untouched); the fold stays put.
    let (mut app, row) = app_with_folded_card();
    app.fold_click = Some((0, row));
    assert!(!app.fold_release(true), "a moved drag never toggles");
    assert!(!card_expanded(&app), "the card stays folded");
    assert!(app.fold_click.is_none(), "consumed, not left armed");
    assert!(
        !app.fold_release(false),
        "and the next plain release has nothing left to fire"
    );
}

#[test]
fn fold_armed_clicks_stay_out_of_the_double_click_zoom_count() {
    // No armed drag, so these presses read as landing on the card's border —
    // where the double click still toggles zoom.
    let mut app = crate::app::CrewApp::default();
    app.click_gesture(0, true); // the press that armed a fold toggle
    app.click_gesture(0, false); // a plain press right after
    assert!(
        !app.zoomed,
        "a fold click must not seed a double-click zoom"
    );
    app.click_gesture(0, false); // second plain press within the window
    assert!(app.zoomed, "two plain clicks in a row still zoom");
}

#[test]
fn a_streaming_system_card_toggles_behind_the_settled_transcript() {
    // Index math across the `messages` → `streaming` seam: the streaming
    // card is visible index 1 but lives in `streaming[0]`.
    let (cols, rows) = (40u16, 20u16);
    let mut p = crate::chat::tests::pane();
    p.push_capped(msg("coder", "hi"));
    p.absorb_delta("crew".into(), LONG.into());
    let suffix_row = row_with(&p, cols, rows, "\u{2026} +4");
    assert!(p.toggle_fold_at(cols, rows, suffix_row));
    assert!(p.streaming[0].expanded, "the streaming card toggled");
    assert!(p.messages.iter().all(|m| !m.expanded));
}

// ---------------------------------------------------------------------------
// Tool cards fold at their first line
// ---------------------------------------------------------------------------

#[test]
fn a_tool_result_folds_to_its_outcome_line() {
    let mut m = tool_msg();
    // One line is the whole card: nothing to fold.
    assert!(!folded(&m, 1));
    // Two is already output under the outcome line.
    assert!(folded(&m, 2));
    // …and it stays folded where a system card would still be open, because
    // three lines of a JSON body is not a preview.
    assert!(folded(&m, FOLD_LINES));
    m.expanded = true;
    assert!(!folded(&m, 40), "a clicked-open card shows everything");
}

/// The drift these two predicates exist to prevent: a card `folded` collapses
/// but `foldable` does not consider foldable is collapsed with NO WAY TO OPEN
/// IT.
#[test]
fn every_folded_tool_card_is_also_clickable() {
    let m = tool_msg();
    let cols = 80;
    let view = View {
        source: false,
        compact: false,
        gap_rows: 1,
        streaming_from: usize::MAX,
    };
    let body_len = crate::chatmsgs::full_body(&m, cols, view).len();
    assert!(body_len > 1, "fixture must be long enough to fold");
    assert_eq!(folded(&m, body_len), foldable(&m, cols, view));
}

fn tool_msg() -> Message {
    Message {
        sender: "api-consumer".into(),
        text: "[tool] sys:run \u{2713} 1.2s\nOslo: +56F\nTokyo: +78F".into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}
