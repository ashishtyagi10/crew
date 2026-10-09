//! The record card on a narrow pane.
use super::card_lines;

/// A task row too long for the card wraps under its title, never back under
/// the numbers (`light ← 1` landed at column 0, under `3 ×`, reading as a row
/// of its own); every row fits; nothing is lost.
#[test]
fn a_wrapped_row_hangs_under_its_title() {
    let text = "swarm \u{b7} 3 tasks \u{b7} 2 done \u{b7} 1 failed \u{b7} 12s\n \
                1 \u{2713} scout   find where the ring's track is drawn\n \
                2 \u{2713} coder   lift the tick marks to the accent floor\n \
                3 \u{2717} critic  check the ring track on paper light\u{a0}\u{2190}\u{a0}1";
    let width = 30;
    let lines = card_lines(text, width, (200, 200, 200));
    let rows: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect();
    for r in &rows {
        assert!(r.chars().count() <= width + 1, "overran: {r:?}");
    }
    // Past the head (prose: it wraps to the margin), a row that does not
    // start a task hangs.
    let first_task = rows
        .iter()
        .position(|r| r.trim_start().starts_with('1'))
        .unwrap();
    for r in &rows[first_task..] {
        let body = r.trim_start();
        let numbered = body.starts_with(|c: char| c.is_ascii_digit());
        let col = r.chars().count() - body.chars().count();
        assert!(
            numbered || col >= 4,
            "a continuation at column {col}: {r:?}"
        );
    }
    let words = |s: &str| s.split_whitespace().map(str::to_string).collect::<Vec<_>>();
    assert_eq!(words(&rows.join(" ")), words(text), "nothing lost");
}

/// Off the markdown path a title's inline code keeps no backticks.
#[test]
fn a_titles_code_is_quoted_not_ticked() {
    let text = "swarm \u{b7} 1 task \u{b7} 1 done\n 1 \u{2713} coder  run `cargo test`";
    let lines = card_lines(text, 60, (200, 200, 200));
    let all: String = lines.iter().flatten().map(|c| c.c).collect();
    assert!(
        !all.contains('`') && all.contains("\u{201c}cargo test\u{201d}"),
        "{all}"
    );
}

/// The head's wrapped tail hangs two columns in — at the margin `1 cancelled
/// · 1s` sat over `1 ✓ scout` and read as task 1 — and a task with a
/// specialist column hangs its title's tail under the title, not under the
/// specialist.
#[test]
fn the_heads_tail_and_a_titles_tail_hang_where_they_belong() {
    let nb = '\u{a0}';
    let text = format!(
        "swarm \u{b7} 3 tasks \u{b7} 1 done \u{b7} 1 failed \u{b7} 1 cancelled \u{b7} 1s\n \
         1 \u{2713} scout   find where the ring's track is drawn on paper{nb}\u{2190}{nb}2"
    );
    let lines = card_lines(&text, 30, (200, 200, 200));
    let rows: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect();
    let task = rows
        .iter()
        .position(|r| r.contains("scout"))
        .expect("the task row");
    assert!(task >= 2, "the head wrapped: {rows:?}");
    for tail in &rows[1..task] {
        assert!(tail.starts_with("   "), "the head's tail hangs: {tail:?}");
    }
    let title = rows[task].chars().position(|c| c == 'f').expect("title");
    let cont = &rows[task + 1];
    assert_eq!(
        cont.chars().take_while(|c| *c == ' ').count(),
        title,
        "{rows:?}"
    );
}
