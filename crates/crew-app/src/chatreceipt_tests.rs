use super::{filed, is_receipt};

const RECEIPT: &str = "task #1: 1 file changed (+1 \u{2212}1): ~ lib.rs\n```diff\n-old\n+new\n```";

#[test]
fn a_note_is_filed_above_the_diff() {
    let got = filed(RECEIPT, "no diagnostics in the changed file");
    assert_eq!(
        got,
        "task #1: 1 file changed (+1 \u{2212}1): ~ lib.rs\nno diagnostics in the changed file\n```diff\n-old\n+new\n```"
    );
    let two = filed(
        &got,
        "\u{2713} checked: `cargo test` passed after the last edit",
    );
    let lines: Vec<&str> = two.lines().collect();
    assert_eq!(
        lines[2],
        "\u{2713} checked: `cargo test` passed after the last edit"
    );
    assert_eq!(lines[3], "```diff", "the diff stays last");
}

#[test]
fn a_receipt_without_a_diff_takes_the_note_at_its_end() {
    assert_eq!(
        filed("task #2: 1 file changed: + a.rs", "tip: x"),
        "task #2: 1 file changed: + a.rs\ntip: x"
    );
}

fn note(sender: &str, text: &str) -> (String, String) {
    (sender.into(), text.into())
}

#[test]
fn the_turns_later_notes_join_its_receipt_and_nothing_after_it_does() {
    let mut p = crate::chat::tests::pane();
    let feed = |p: &mut crate::chat::ChatPane, (s, t): (String, String)| {
        p.absorb_message(s, t, String::new(), String::new())
    };
    feed(&mut p, note("agent smith", RECEIPT));
    feed(
        &mut p,
        note("agent smith", "no diagnostics in the changed file"),
    );
    feed(
        &mut p,
        note("agent smith", "check: cargo test \u{2014} passed"),
    );
    assert_eq!(p.messages.len(), 1, "one card");
    assert!(is_receipt(&p.messages[0]));
    assert!(p.messages[0].text.contains("check: cargo test"));
    // An agent's answer ends the receipt; a smith note after it is its own.
    feed(&mut p, note("engineer \u{2192} user", "done"));
    feed(&mut p, note("agent smith", "routing: reply"));
    assert_eq!(p.messages.len(), 3);
    assert!(!p.messages[0].text.contains("routing"));
}

#[test]
fn the_checked_line_joins_the_receipt_when_the_task_ends() {
    let mut p = crate::chat::tests::pane();
    p.absorb_task(5, true);
    p.absorb_message(
        "agent smith".into(),
        RECEIPT.into(),
        String::new(),
        String::new(),
    );
    let edit = crew_plugin::PluginEvent::Hive {
        event: crew_hive::HiveEvent::ToolCall {
            agent: crew_hive::AgentId::minted("claude"),
            label: "Edit".into(),
            args: "{}".into(),
        },
    };
    p.heard(&edit);
    p.absorb_task(5, false);
    assert_eq!(p.messages.len(), 1, "the line joined the receipt");
    assert!(
        p.messages[0].text.contains("! not checked"),
        "{}",
        p.messages[0].text
    );
}

/// Folded, a receipt shows its head and the verdicts filed under it; only
/// the diff goes behind `… +N`.
#[test]
fn a_folded_receipt_keeps_its_verdicts_and_folds_its_diff() {
    let _g = crate::app::theme_test_guard();
    let mut p = crate::chat::tests::pane();
    let text = filed(
        RECEIPT,
        "\u{2713} checked: `cargo test` passed after the last edit",
    );
    p.absorb_message("agent smith".into(), text, String::new(), String::new());
    let rows: Vec<String> = p
        .cells(80, 20)
        .chunk_by(|a, b| a.row == b.row)
        .map(|r| r.iter().map(|c| c.c).collect::<String>())
        .collect();
    let all = rows.join("\n");
    assert!(all.contains("1 file changed"), "{all}");
    assert!(all.contains("checked:"), "the verdict shows folded:\n{all}");
    assert!(!all.contains("+new"), "the diff is folded:\n{all}");
}
