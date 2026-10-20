//! A card body keeps the pane's last column as margin, as the header, the
//! code fields, the swarm rows and the composer's stroke do. Rules, tables,
//! a word that landed on the last column and the fold's ` … +N` used to run
//! one column past all of them.
use super::*;
use crate::chatbody::body_lines;
use crate::chatlayout::Message;

fn text(l: &[crate::chatbody::CardCell]) -> String {
    l.iter().map(|c| c.c).collect()
}

#[test]
fn nothing_in_a_body_takes_the_cards_last_column() {
    let md = format!(
        "intro\n\n---\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n{}",
        "x".repeat(70)
    );
    let lines = body_lines(&md, 30, (9, 9, 9), false);
    let all: Vec<String> = lines.iter().map(|l| text(l)).collect();
    assert!(lines.iter().all(|l| l.len() <= 29), "{all:#?}");
    assert!(
        lines.iter().any(|l| l.len() == 29),
        "the rule runs to the margin: {all:#?}"
    );
}

#[test]
fn a_folded_line_ends_its_count_before_the_margin() {
    let _g = crate::app::theme_test_guard();
    let m = [Message {
        sender: "coder".into(),
        text: format!("{}\ntwo\nthree", "word ".repeat(12).trim_end()),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }];
    let refs: Vec<&Message> = m.iter().collect();
    let view = View {
        compact: true,
        streaming_from: usize::MAX,
        ..View::default()
    };
    let lines = card_lines(&refs, 30, 0, view);
    let folded = text(&lines[1]);
    // Twelve words wrap to three rows: four rows fold away.
    assert!(folded.ends_with("\u{2026} +4"), "{folded:?}");
    assert!(lines[1].len() <= 29, "{folded:?}");
}
