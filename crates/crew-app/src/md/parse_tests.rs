use super::*;

#[test]
fn paragraph_inline_styles_nest() {
    let blocks = parse("plain **bold _both_** `code`");
    let Block::Paragraph(spans) = &blocks[0] else {
        panic!("not a paragraph: {blocks:?}")
    };
    let texts: Vec<(&str, bool, bool, bool)> = spans
        .iter()
        .map(|s| (s.text.as_str(), s.style.bold, s.style.italic, s.style.code))
        .collect();
    assert!(texts.contains(&("bold ", true, false, false)), "{texts:?}");
    assert!(texts.contains(&("both", true, true, false)), "{texts:?}");
    assert!(texts.contains(&("code", false, false, true)), "{texts:?}");
}

#[test]
fn heading_levels_carry_through() {
    let blocks = parse("## Two\n\ntext");
    assert!(matches!(&blocks[0], Block::Heading(2, s) if s[0].text == "Two"));
}

#[test]
fn fenced_code_keeps_verbatim_lines_and_lang() {
    let blocks = parse("```rust\nfn x() {}\n  indented\n```");
    assert_eq!(
        blocks[0],
        Block::CodeBlock {
            lang: "rust".into(),
            lines: vec!["fn x() {}".into(), "  indented".into()],
        }
    );
}

#[test]
fn markdown_link_and_bare_url_become_link_spans() {
    let blocks = parse("see [docs](https://ex.am/d) and https://ex.am/raw now");
    let Block::Paragraph(spans) = &blocks[0] else {
        panic!()
    };
    let links: Vec<(&str, &str)> = spans
        .iter()
        .filter_map(|s| s.link.as_deref().map(|u| (s.text.as_str(), u)))
        .collect();
    assert_eq!(
        links,
        vec![
            ("docs", "https://ex.am/d"),
            ("https://ex.am/raw", "https://ex.am/raw")
        ]
    );
}

#[test]
fn nested_and_ordered_lists_carry_depth_and_index() {
    let blocks = parse("- a\n  - b\n1. one");
    let items: Vec<(Option<u64>, u8, String)> = blocks
        .iter()
        .flat_map(|b| match b {
            Block::List(items) => items
                .iter()
                .map(|i| {
                    (
                        i.ordered_idx,
                        i.depth,
                        i.spans.iter().map(|s| s.text.clone()).collect::<String>(),
                    )
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect();
    assert!(items.contains(&(None, 0, "a".into())), "{items:?}");
    assert!(items.contains(&(None, 1, "b".into())), "{items:?}");
    assert!(items.contains(&(Some(1), 0, "one".into())), "{items:?}");
}

#[test]
fn blockquote_wraps_inner_blocks() {
    let blocks = parse("> quoted");
    assert!(matches!(&blocks[0], Block::BlockQuote(inner)
        if matches!(&inner[0], Block::Paragraph(s) if s[0].text == "quoted")));
}

#[test]
fn table_splits_header_and_rows() {
    let blocks = parse("| a | b |\n|---|---|\n| 1 | 2 |");
    let Block::Table { header, rows } = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert_eq!(header.len(), 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][1][0].text, "2");
}

#[test]
fn rule_and_strikethrough() {
    let blocks = parse("---\n\n~~gone~~");
    assert!(matches!(blocks[0], Block::Rule));
    assert!(matches!(&blocks[1], Block::Paragraph(s) if s[0].style.italic));
}

#[test]
fn hard_break_becomes_newline_span() {
    let blocks = parse("a  \nb"); // two trailing spaces = hard break
    let Block::Paragraph(spans) = &blocks[0] else {
        panic!()
    };
    assert!(spans.iter().any(|s| s.text == "\n"), "{spans:?}");
}

#[test]
fn garbage_never_panics() {
    for s in [
        "",
        "``",
        "**",
        "[a](",
        "|",
        ">>>",
        "#".repeat(300).as_str(),
        "\u{0}\u{fffd}*_`[",
        "- \n- \n  1. \n```",
    ] {
        let _ = parse(s);
    }
}
