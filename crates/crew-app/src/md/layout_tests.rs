use super::*;

fn flat(l: &MdLine) -> String {
    l.spans.iter().map(|s| s.text.as_str()).collect()
}

#[test]
fn prose_wraps_and_styles_survive_the_wrap() {
    let lines = render("**bold text that must wrap across lines**", 12);
    assert!(lines.len() >= 2);
    assert!(
        lines
            .iter()
            .filter(|l| l.kind == LineKind::Body)
            .all(|l| l.spans.iter().all(|s| s.style.bold)),
        "style lost at wrap"
    );
    assert!(lines.iter().all(|l| flat(l).chars().count() <= 12));
}

#[test]
fn heading_line_is_marked_and_bold() {
    let lines = render("# Title", 40);
    assert_eq!(lines[0].spans[0].style.heading, 1);
    assert!(lines[0].spans[0].style.bold);
    assert_eq!(flat(&lines[0]), "Title");
}

#[test]
fn code_block_chrome_lines() {
    let lines = render("```rust\nfn x() {}\n```", 40);
    let kinds: Vec<LineKind> = lines.iter().map(|l| l.kind).collect();
    assert_eq!(
        kinds,
        vec![LineKind::CodeHeader, LineKind::Code, LineKind::CodeFooter]
    );
    assert_eq!(flat(&lines[0]), "\u{256d}\u{2500} rust");
    assert_eq!(flat(&lines[1]), "fn x() {}");
    assert_eq!(flat(&lines[2]), "\u{2570}\u{2500}");
}

#[test]
fn code_hard_chunks_verbatim() {
    let lines = render("```\nlet a = 1;\n```", 6);
    let code: String = lines
        .iter()
        .filter(|l| l.kind == LineKind::Code)
        .map(flat)
        .collect();
    assert_eq!(code, "let a = 1;");
}

#[test]
fn lists_indent_and_number() {
    let lines = render("- a\n  - b\n\n1. one", 40);
    let texts: Vec<String> = lines
        .iter()
        .filter(|l| l.kind == LineKind::Body)
        .map(flat)
        .collect();
    assert!(texts.contains(&"• a".to_string()), "{texts:?}");
    assert!(texts.contains(&"  • b".to_string()), "{texts:?}");
    assert!(texts.contains(&"1. one".to_string()), "{texts:?}");
}

#[test]
fn blockquote_prefixes() {
    let lines = render("> quoted words", 40);
    assert!(
        flat(&lines[0]).starts_with("\u{258e} "),
        "{}",
        flat(&lines[0])
    );
}

#[test]
fn table_aligns_and_bolds_header() {
    let lines = render("| a | bb |\n|---|---|\n| ccc | d |", 40);
    let texts: Vec<String> = lines.iter().map(flat).collect();
    assert_eq!(texts[0], "a   \u{2502} bb");
    assert!(lines[0].spans.iter().any(|s| s.style.bold));
    assert!(texts[1].starts_with('\u{2500}'));
    assert_eq!(texts[2], "ccc \u{2502} d ");
}

#[test]
fn link_spans_survive_layout() {
    let lines = render("go to [site](https://s.io) now", 40);
    let link: Vec<&MdSpan> = lines[0].spans.iter().filter(|s| s.link.is_some()).collect();
    assert_eq!(link[0].text, "site");
    assert_eq!(link[0].link.as_deref(), Some("https://s.io"));
}

#[test]
fn blank_lines_separate_blocks_exactly_once() {
    let lines = render("a\n\nb", 40);
    let kinds: Vec<LineKind> = lines.iter().map(|l| l.kind).collect();
    assert_eq!(kinds, vec![LineKind::Body, LineKind::Blank, LineKind::Body]);
}

#[test]
fn byte_soup_never_panics_and_respects_cols() {
    let long = "a".repeat(10_000);
    let soups = ["\u{0}*[`|>#-~", "𓀀𓀁𓀂 **𓀃** https://𓀄", long.as_str()];
    for s in soups.iter() {
        for cols in [1usize, 4, 13, 80] {
            for l in render(s, cols) {
                if l.kind != LineKind::Code {
                    // code is verbatim-chunked by chars
                    assert!(flat(&l).chars().count() <= cols.max(1));
                }
            }
        }
    }
}

#[test]
fn code_chrome_lines_respect_cols() {
    for cols in [1usize, 4, 6] {
        for l in render("```averylonglanguagetag\nx\n```", cols) {
            assert!(
                flat(&l).chars().count() <= cols,
                "kind {:?} overflows at cols={cols}: {:?}",
                l.kind,
                flat(&l)
            );
        }
    }
}
