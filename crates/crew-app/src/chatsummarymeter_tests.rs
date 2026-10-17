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
