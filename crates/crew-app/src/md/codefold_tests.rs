//! A code block's lines, and where each one starts in the source.
use super::super::{parse, Block};

#[test]
fn fenced_code_keeps_verbatim_lines_and_lang() {
    let blocks = parse("```rust\nfn x() {}\n  indented\n```");
    assert_eq!(
        blocks[0],
        Block::CodeBlock {
            lang: "rust".into(),
            lines: vec!["fn x() {}".into(), "  indented".into()],
            // Each line's own byte: the opening fence is eight.
            starts: vec![Some(8), Some(18)],
        }
    );
}

/// A fence inside a quote: each line's text follows a `> ` it does not contain, so each starts
/// two bytes into its source line — and the line still claims its own byte.
#[test]
fn a_quoted_fence_s_lines_start_after_the_bar() {
    let src = "> ```\n> one\n> two\n> ```\n";
    let Some(Block::BlockQuote(inner)) = parse(src).into_iter().next() else {
        panic!("a quote")
    };
    let Some(Block::CodeBlock { lines, starts, .. }) = inner.into_iter().next() else {
        panic!("a fence in it")
    };
    for (line, start) in lines.iter().zip(&starts) {
        let at = start.expect("verbatim") as usize;
        assert_eq!(&src[at..at + line.len()], line);
    }
}
