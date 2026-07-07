//! Task 6: `link_at` resolves a markdown link's URL from a (row, col) in the
//! rendered message body — the click hit-test `clickopen` drives. Tests locate
//! the link text in the rendered `CellView`s rather than hardcoding layout
//! constants, so they stay independent of header/status-row geometry.
use super::*;
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crew_plugin::Plugin;

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
    }
}

fn test_pane(messages: Vec<Message>) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut pane = ChatPane::new(plugin, "crew".into());
    pane.messages = messages;
    pane
}

#[test]
fn link_at_resolves_the_clicked_link_and_misses_off_link() {
    // Link label "k" is rare enough not to collide with header/status text
    // ("crew", "1 msg", the connection dot) so the search below is unambiguous.
    let pane = test_pane(vec![msg("user", "see [k](https://x.io/p)")]);
    let (cols, rows) = (40u16, 20u16);
    let cells = cells(&pane, cols, rows);
    let k = cells
        .iter()
        .find(|c| c.c == 'k')
        .expect("link text 'k' rendered somewhere");
    assert_eq!(
        link_at(&pane, cols, rows, k.row, k.col).as_deref(),
        Some("https://x.io/p")
    );
    // Column 0 of the same row is the body's indentation cell — no link there.
    assert_eq!(link_at(&pane, cols, rows, k.row, 0), None);
}

#[test]
fn link_at_resolves_after_scrolling() {
    // Enough filler lines before the link message to overflow the row budget
    // (so scrolling actually moves the window), and exactly one filler
    // message after it, so the link's line stays a few lines shy of the
    // live-bottom edge — window() only drops lines from that edge as scroll
    // grows, so a scroll of 1 shifts the link's row without hiding it.
    let mut messages: Vec<Message> = (0..5)
        .map(|i| msg("planner", &format!("line {i}")))
        .collect();
    messages.push(msg("user", "see [k](https://x.io/p)"));
    messages.push(msg("planner", "tail"));
    let mut pane = test_pane(messages);
    let (cols, rows) = (40u16, 10u16);

    let before = cells(&pane, cols, rows);
    let k0 = before
        .iter()
        .find(|c| c.c == 'k')
        .expect("link visible before scroll");
    assert_eq!(
        link_at(&pane, cols, rows, k0.row, k0.col).as_deref(),
        Some("https://x.io/p")
    );

    pane.scroll = 1;
    let after = cells(&pane, cols, rows);
    let k1 = after
        .iter()
        .find(|c| c.c == 'k')
        .expect("link still visible after scrolling");
    assert_ne!(
        k1.row, k0.row,
        "scrolling should actually shift the link's row"
    );
    assert_eq!(
        link_at(&pane, cols, rows, k1.row, k1.col).as_deref(),
        Some("https://x.io/p"),
        "link must resolve at its shifted row after scrolling"
    );
}

#[test]
fn link_after_wide_glyphs_resolves_at_its_display_column() {
    // "中文 " advances the DISPLAY column by 4 (two CJK cells, 2 cols each)
    // plus 1 for the space, before the link starts — but only 3 Vec slots
    // (one per char). Raw `Vec` indexing by display column therefore misses
    // the link cell entirely; the fix must walk the line by display width.
    let pane = test_pane(vec![msg("user", "中文 [k](https://x.io/p)")]);
    let (cols, rows) = (40u16, 20u16);
    let cells_out = cells(&pane, cols, rows);
    let k = cells_out
        .iter()
        .find(|c| c.c == 'k')
        .expect("link text 'k' rendered somewhere");
    assert_eq!(
        link_at(&pane, cols, rows, k.row, k.col).as_deref(),
        Some("https://x.io/p"),
        "click on the link's own display column must resolve its URL"
    );
    // A click on the first CJK glyph's own cell must not resolve to the link.
    let cjk = cells_out
        .iter()
        .find(|c| c.c == '中')
        .expect("CJK glyph rendered");
    assert_eq!(link_at(&pane, cols, rows, cjk.row, cjk.col), None);
}
