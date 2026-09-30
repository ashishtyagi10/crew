//! The joined note as it is drawn. The rules behind it — what is offered,
//! what a `Steered` takes from the queue — are tested beside them in
//! `chatsteer_tests`.
use super::*;
use crate::chatkeys::ChatInput;
use crew_plugin::Plugin;

const TEXT: &str = "also check the tests";

/// The pane's rows as text, each with its cells, on a 60 × 20 pane.
fn screen(p: &ChatPane) -> Vec<(String, Vec<crew_render::CellView>)> {
    let cells = p.cells(60, 20);
    (0..20u16)
        .map(|r| {
            let mut row: Vec<_> = cells.iter().filter(|c| c.row == r).cloned().collect();
            row.sort_by_key(|c| c.col);
            (row.iter().map(|c| c.c).collect(), row)
        })
        .collect()
}

#[test]
fn the_joined_note_draws_quietly_under_the_message_and_the_queue_line_goes() {
    let _g = crate::app::theme_test_guard();
    // An idle child stands in for the broker; only what is drawn is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    (p.connected, p.awaiting) = (true, true);
    p.input = TEXT.into();
    p.on_input(ChatInput::Enter, &std::env::temp_dir());
    let before: Vec<String> = screen(&p).into_iter().map(|(t, _)| t).collect();
    assert!(before.iter().any(|t| t.contains("queued")), "{before:#?}");

    p.absorb_steered(TEXT.into());
    p.messages.last_mut().unwrap().ts.clear(); // past its fade-in
    let after = screen(&p);
    let rows: Vec<&str> = after.iter().map(|(t, _)| t.as_str()).collect();
    assert!(!rows.iter().any(|t| t.contains("queued")), "{rows:#?}");
    let asked = rows
        .iter()
        .position(|t| t.contains(TEXT))
        .expect("the message");
    let joined = rows
        .iter()
        .position(|t| t.contains(&joined_note(TEXT)))
        .unwrap_or_else(|| panic!("no joined note: {rows:#?}"));
    assert!(joined > asked, "under the message: {rows:#?}");
    let arrow = after[joined]
        .1
        .iter()
        .find(|c| c.c == '\u{21b3}')
        .expect("the arrow");
    assert_eq!(arrow.fg, crew_theme::theme().text_muted, "the quiet voice");
}
