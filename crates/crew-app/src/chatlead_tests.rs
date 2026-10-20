use crate::chatlayout::Message;
use crate::chatmsgs::{card_lines, View};

fn msg(text: &str) -> Message {
    Message {
        sender: "smith".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

fn rows(text: &str, view: View) -> Vec<String> {
    let m = [msg(text)];
    let refs: Vec<&Message> = m.iter().collect();
    card_lines(&refs, 40, 0, view)
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect::<String>())
        .collect()
}

fn normal() -> View<'static> {
    View {
        streaming_from: usize::MAX,
        ..View::default()
    }
}

#[test]
fn the_blocks_that_draw_a_frame_are_told_from_prose() {
    for t in [
        "# Release notes",
        "\n  > a quote",
        "```rust\nfn a() {}\n```",
        "| a | b |",
    ] {
        assert!(super::framed(t), "{t:?}");
    }
    for t in [
        "Release notes",
        "## a subheading",
        "#hashtag",
        "- a list",
        "",
    ] {
        assert!(!super::framed(t), "{t:?}");
    }
}

#[test]
fn an_h1_or_a_quote_opens_a_row_under_the_header_and_prose_does_not() {
    let _g = crate::app::theme_test_guard();
    for text in ["# Release notes\n\nbody", "> quoted\n\nbody"] {
        let r = rows(text, normal());
        assert!(r[1].trim().is_empty(), "{text:?}: {r:#?}");
        assert!(!r[2].trim().is_empty(), "{text:?}: {r:#?}");
    }
    let r = rows("Release notes\n\nbody", normal());
    assert!(r[1].contains("Release notes"), "{r:#?}");
}

#[test]
fn compact_and_source_views_stay_tight() {
    let _g = crate::app::theme_test_guard();
    let compact = View {
        compact: true,
        ..normal()
    };
    let source = View {
        source: true,
        ..normal()
    };
    for view in [compact, source] {
        let r = rows("# Release notes\n\nbody", view);
        assert!(r[1].contains("Release notes"), "{r:#?}");
    }
}
