//! The revealed block drawn as its source, among rows that are not.
use crate::md::reveal::tests::{at, DOC};
use crate::md::reveal::{block_at, Reveal};
use crate::md::{render_revealed, LineKind, MdLine};

fn text_of(l: &MdLine) -> String {
    l.spans.iter().map(|s| s.text.as_str()).collect()
}

/// Every reveal the document can produce, one per block.
fn every_reveal() -> Vec<Reveal> {
    let mut out: Vec<Reveal> = Vec::new();
    for (i, _) in DOC.char_indices() {
        if let Some(r) = block_at(DOC, i as u32) {
            if !out.contains(&r) {
                out.push(r);
            }
        }
    }
    out
}

/// THE invariant, with each block revealed in turn: every claimed byte holds its character…
/// and every byte of the revealed lines is claimed, bar the newlines between them and the
/// spaces a wrap consumes. Without the second half, a reveal that drew nothing would pass.
#[test]
fn every_revealed_block_claims_every_byte_and_each_claim_holds() {
    let reveals = every_reveal();
    assert!(reveals.len() >= 8, "{reveals:?}");
    for r in reveals {
        for cols in [20, 40, 80] {
            let lines = render_revealed(DOC, cols, Some(r));
            let mut claimed = std::collections::BTreeSet::new();
            for s in lines.iter().flat_map(|l| &l.spans) {
                let Some(mut b) = s.src else { continue };
                for c in s.text.chars() {
                    let got = DOC[b as usize..].chars().next();
                    assert_eq!(got, Some(c), "{r:?} at {cols}: byte {b}");
                    claimed.insert(b);
                    b += c.len_utf8() as u32;
                }
            }
            for (i, c) in DOC[r.from as usize..r.to as usize].char_indices() {
                let b = r.from + i as u32;
                if c != '\n' && c != ' ' {
                    assert!(
                        claimed.contains(&b),
                        "{r:?} at {cols}: {c:?} at {b} unclaimed"
                    );
                }
            }
        }
    }
}

/// The rest of the document is untouched — a reveal replaces its own rows and no others.
#[test]
fn a_reveal_leaves_every_other_row_as_it_rendered() {
    let plain = crate::md::render(DOC, 80);
    let r = block_at(DOC, at("a quotation")).unwrap();
    let shown = render_revealed(DOC, 80, Some(r));
    let quote = shown
        .iter()
        .position(|l| text_of(l).starts_with("> a quotation"))
        .expect("the quote is drawn as written");
    let was = plain
        .iter()
        .position(|l| text_of(l).contains("a quotation"))
        .unwrap();
    assert_eq!(quote, was, "rows above it moved");
    assert_eq!(shown[..quote], plain[..was]);
    assert_eq!(shown[quote + 1..], plain[was + 1..]);
}

/// Bold stays bold, the `**` around it is a marker, and a heading's `#` is a HEADING's marker.
#[test]
fn a_revealed_block_keeps_its_styles_and_marks_its_markers() {
    let r = block_at(DOC, at("italic")).unwrap();
    let lines = render_revealed(DOC, 200, Some(r));
    let spans: Vec<_> = lines.iter().flat_map(|l| &l.spans).collect();
    let bold = spans.iter().find(|s| s.text == "bold").expect("bold run");
    assert!(bold.style.bold && !bold.style.marker);
    let stars = spans
        .iter()
        .find(|s| s.text.contains("**"))
        .expect("** shown");
    assert!(stars.style.marker, "{stars:?}");
    let r = block_at(DOC, at("heading")).unwrap();
    let lines = render_revealed(DOC, 80, Some(r));
    let h = lines.iter().find(|l| text_of(l) == "# A heading").unwrap();
    assert!(h.spans[0].style.marker && h.spans[0].style.heading == 1);
    assert_eq!(
        crate::md::heading::level_of(h),
        1,
        "still an outline landmark"
    );
}

/// A fence keeps its code field: the fence lines are the field's top and bottom rows.
#[test]
fn a_revealed_fence_is_still_drawn_on_its_field() {
    let r = block_at(DOC, at("fn main")).unwrap();
    let lines = render_revealed(DOC, 80, Some(r));
    let kinds: Vec<(LineKind, String)> = lines
        .iter()
        .filter(|l| text_of(l).contains("```") || text_of(l).contains("fn main"))
        .map(|l| (l.kind, text_of(l)))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (LineKind::CodeHeader, "```rust".into()),
            (LineKind::Code, "fn main() { println!(\"hi\"); }".into()),
            (LineKind::CodeFooter, "```".into()),
        ]
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.kind == LineKind::CodeHeader)
            .count(),
        1,
        "the rendered label row is replaced, not kept beside the fence"
    );
}
