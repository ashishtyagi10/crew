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
fn list_item_with_fenced_code_hoists_a_code_block() {
    let blocks = parse("1. First do X:\n\n   ```bash\n   cmd --flag\n   ```");
    let Block::List(items) = &blocks[0] else {
        panic!("expected a list first: {blocks:?}")
    };
    assert_eq!(items.len(), 1, "{items:?}");
    let texts: String = items[0].spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, "First do X:");
    assert_eq!(
        blocks.get(1),
        Some(&Block::CodeBlock {
            lang: "bash".into(),
            lines: vec!["cmd --flag".into()],
        }),
        "fenced code should hoist out as a sibling block: {blocks:?}"
    );
}

#[test]
fn successive_paragraphs_in_one_item_get_a_line_break_marker() {
    let blocks = parse("- a\n\n  b");
    let Block::List(items) = &blocks[0] else {
        panic!("expected a list: {blocks:?}")
    };
    assert_eq!(
        items[0]
            .spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "\n", "b"]
    );
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
fn default_render_joins_soft_breaks_with_a_space() {
    let lines = crate::md::render("one\ntwo", 40);
    // CommonMark: a single paragraph, soft break joined by a space.
    let flat: String = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.text.as_str())
        .collect::<String>();
    assert_eq!(flat, "one two");
    assert_eq!(lines.len(), 1, "{lines:?}");
}

#[test]
fn chat_render_keeps_soft_breaks_as_lines() {
    let lines = crate::md::render_chat("one\ntwo", 40);
    let body: Vec<&crate::md::MdLine> = lines
        .iter()
        .filter(|l| l.kind == crate::md::LineKind::Body)
        .collect();
    assert_eq!(body.len(), 2, "{lines:?}");
}

#[test]
fn pathological_nesting_does_not_overflow_the_stack() {
    // Abort-on-overflow can't be caught by #[test]; run in a small-stack
    // thread so overflow WOULD abort the child observably before the fix.
    let handle = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let quotes = ">".repeat(50_000) + "x";
            let _ = parse(&quotes);
            let mut deep = String::new();
            for i in 0..5_000 {
                deep.push_str(&"  ".repeat(i));
                deep.push_str("- x\n");
            }
            let _ = parse(&deep);
        })
        .unwrap();
    handle.join().expect("parser must not blow the stack");
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
