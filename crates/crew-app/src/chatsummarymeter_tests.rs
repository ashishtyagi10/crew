//! The footer's meters on a narrow tile. Split from `chatsummary_tests`
//! for the line cap; its fixtures are shared.
use crate::chatsummary::tests::{agent, ctx, fc};

/// A narrow tile keeps the pane's own context meter, with its own fraction,
/// and gives up the account's 5h one first (survey H4: a hard 60-column gate
/// hid both, with 27 columns free).
#[test]
fn a_narrow_tile_keeps_its_context_meter() {
    let _g = crate::app::theme_test_guard();
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    let ctx = ctx(&[("smith", 100_000)]);
    let lines_at = |cols: usize| {
        let mut meters = Vec::new();
        let lines = crate::chatsummary::footer_text_lines(&fc(&agents, &ctx), cols, &mut meters);
        let text: String = lines[1].iter().map(|(c, _)| *c).collect();
        (text, meters)
    };
    let (wide, all) = lines_at(120);
    assert!(wide.contains("(5h)") && wide.contains("(ctx)"), "{wide}");
    let (narrow, kept) = lines_at(44);
    assert!(
        narrow.contains("(ctx)") && !narrow.contains("(5h)"),
        "{narrow}"
    );
    assert_eq!(kept, [all[1]], "the ctx meter keeps the ctx fraction");
    let (tiny, none) = lines_at(30);
    assert!(!tiny.contains("(ctx)") && none.is_empty(), "{tiny}");
}

/// Claude Code's relay agent names no model (`claudeagent` reports `""`), so
/// the model table never knew its window and its pane drew no gauge. The
/// window its `result` line reported is enough: 310k of 1M reads 31%.
#[test]
fn a_reported_window_gauges_an_agent_with_no_model() {
    let _g = crate::app::theme_test_guard();
    let agents = [agent("claude", "")];
    let ctx = ctx(&[("claude", 310_000)]);
    let none = std::collections::HashMap::new();
    assert_eq!(crate::summaryfit::ctx_fill(&agents, &ctx, &none), None);
    let window = crate::chatsummary::tests::ctx(&[("claude", 1_000_000)]);
    assert_eq!(
        crate::summaryfit::ctx_fill(&agents, &ctx, &window),
        Some(31)
    );
    let mut f = fc(&agents, &ctx);
    f.ctx_window = &window;
    let mut meters = Vec::new();
    let lines = crate::chatsummary::footer_text_lines(&f, 120, &mut meters);
    let text: String = lines[1].iter().map(|(c, _)| *c).collect();
    assert!(text.contains("31% (ctx)"), "{text}");
}

/// A stat that reported no window keeps the one an earlier reply said.
#[test]
fn a_stat_without_a_window_keeps_the_known_one() {
    let mut pane = crate::chat::tests::pane();
    pane.note_window("claude", 1_000_000);
    pane.note_window("claude", 0);
    assert_eq!(pane.ctx_window.get("claude"), Some(&1_000_000));
}
