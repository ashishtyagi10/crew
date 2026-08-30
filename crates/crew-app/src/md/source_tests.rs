//! The offsets a cursor is made of. Every one of these asserts the same
//! thing in the end: the byte a rendered character claims to come from IS
//! that character in the file.
use crate::md::{render, MdLine, MdSpan};

/// Every (character, claimed offset) pair a render produces.
fn claims(text: &str, cols: usize) -> Vec<(char, u32)> {
    render(text, cols).iter().flat_map(pairs).collect()
}

fn pairs(line: &MdLine) -> Vec<(char, u32)> {
    line.spans.iter().flat_map(span_pairs).collect()
}

fn span_pairs(s: &MdSpan) -> Vec<(char, u32)> {
    let Some(start) = s.src else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut at = start as usize;
    for c in s.text.chars() {
        out.push((c, at as u32));
        at += c.len_utf8();
    }
    out
}

/// THE invariant. Nothing else in the editor is safe if this is not true.
fn every_claim_holds(src: &str, cols: usize) {
    for (c, at) in claims(src, cols) {
        let got = src[at as usize..].chars().next();
        assert_eq!(
            got,
            Some(c),
            "a render claimed {c:?} came from byte {at}, where the file has {got:?}\n{src:?}"
        );
    }
}

#[test]
fn prose_points_at_its_own_bytes() {
    every_claim_holds("hello there, this is a document.\n", 40);
}

#[test]
fn every_shape_in_the_grammar_points_at_its_own_bytes() {
    let doc = "\
# A heading

Some **bold** and *italic* and `code` in a paragraph that is long enough that
it has to wrap more than once at a narrow width.

- a list item
- another one, also long enough to wrap when the column count is small
  - and one nested under it

> a quotation, which the renderer draws a bar beside

| a | b |
|---|---|
| 1 | 2 |

```rust
fn main() { println!(\"hi\"); }
```

A [link](https://example.invalid) and a trailing line.
";
    for cols in [20, 40, 80] {
        every_claim_holds(doc, cols);
    }
}

/// A wrap cuts a span. The cut has to move the offset by the BYTES it
/// dropped, not by the characters — this is the assertion that fails if it
/// counts characters.
#[test]
fn a_wrapped_line_after_wide_characters_still_points_at_its_own_bytes() {
    every_claim_holds(
        "日本語のテキストが折り返されるとき、その後の文字も正しい位置を指す必要があります。\n",
        20,
    );
    every_claim_holds(
        "café naïve résumé, wrapped narrow enough to cut somewhere\n",
        12,
    );
}

#[test]
fn a_document_of_many_paragraphs_points_at_its_own_bytes() {
    let mut doc = String::new();
    for i in 0..30 {
        doc.push_str(&format!("Paragraph {i} with some words in it.\n\n"));
    }
    every_claim_holds(&doc, 24);
}

/// Text that is NOT its own bytes claims nothing rather than claiming
/// something four bytes out — an entity, an escape, a soft break.
#[test]
fn a_run_that_is_not_a_copy_of_its_source_claims_nothing() {
    for src in ["a &amp; b\n", "a \\* b\n", "one\ntwo\n"] {
        for (c, at) in claims(src, 40) {
            let got = src[at as usize..].chars().next();
            assert_eq!(got, Some(c), "{src:?} claimed {c:?} at {at}");
        }
    }
    // …and specifically: the entity run is not claimed at all.
    let ent: Vec<(char, u32)> = claims("&amp;\n", 40);
    assert!(ent.is_empty(), "an entity must claim no offsets: {ent:?}");
}

/// The offsets have to be there at all — a test that only checks "no wrong
/// claims" passes trivially when nothing claims anything.
#[test]
fn a_paragraph_actually_carries_offsets() {
    let got = claims("hello world\n", 40);
    assert_eq!(got.len(), "hello world".len(), "{got:?}");
    assert_eq!(got.first(), Some(&('h', 0)));
}
