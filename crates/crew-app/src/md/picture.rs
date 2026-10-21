//! `![alt](src)` — a picture named in a document, and the rows it claims.
//!
//! Until now an image in markdown rendered as its alt text and nothing else:
//! the one piece of a document that is not words came out as words. Crew draws
//! real pictures on the paint layer (the `/view` image rung, a program's own
//! output), so a picture a document *names* should be the picture too.
//!
//! The engine cannot draw one — it has no pixels, no cell size and no worker
//! thread — so what it does is reserve the room and say what belongs there.
//! An image paragraph becomes [`ROWS`] lines of [`LineKind::Picture`], each
//! carrying its index within the block and the block's height, so a renderer
//! that can only see the rows currently on screen still knows the whole box
//! the picture is fitted into. The source travels in the line's one span, in
//! the same `link` field a markdown link's URL uses.
use super::{LineKind, MdLine, MdSpan, MdStyle};

/// How many rows a picture claims. Fixed rather than derived from the file:
/// the layout runs long before any worker has decoded anything, and a
/// document whose lines move once the pictures land would reflow under the
/// reader. Roughly a third of a comfortable window, and the picture is
/// letterboxed inside it, so a wide banner and a tall portrait both fit.
pub(crate) const ROWS: u16 = 12;

/// A never-drawn span carrying one picture's source and alt text through the
/// span stream — the same convention `tasklist::sentinel` uses for a checkbox.
pub(super) fn sentinel(src: String, alt: String) -> MdSpan {
    MdSpan {
        text: alt,
        style: MdStyle {
            marker: true,
            code: true, // never set together with `marker` by authored text
            ..MdStyle::default()
        },
        link: Some(src),
        src: None,
    }
}

/// Whether `s` is one of ours.
pub(super) fn is_sentinel(s: &MdSpan) -> bool {
    s.style.marker && s.style.code && s.link.is_some()
}

/// The lines a picture paragraph becomes, or `None` when the paragraph is not
/// one — an image with words beside it is prose that mentions a picture, and
/// it keeps its alt text inline rather than claiming a third of the window.
pub(super) fn lines(spans: &[MdSpan]) -> Option<Vec<MdLine>> {
    let [only] = spans else { return None };
    if !is_sentinel(only) {
        return None;
    }
    let n = rows_of(only.link.as_deref().unwrap_or_default());
    Some(
        (0..n)
            .map(|i| MdLine {
                spans: vec![only.clone()],
                kind: LineKind::Picture { i, n },
            })
            .collect(),
    )
}

/// A `#rows=N` on the source asks for N rows (2 to 40) instead of [`ROWS`]: a
/// mail knows its logo is 40 px tall before anything is decoded, and a box
/// sized from that never reflows the text when the picture lands (goal
/// 2026-10-20-mail-reader, item 14).
pub(crate) fn rows_of(src: &str) -> u16 {
    src.rsplit_once("#rows=")
        .and_then(|(_, n)| n.parse::<u16>().ok())
        .map_or(ROWS, |n| n.clamp(2, 40))
}

/// The source a picture line names, if it is one.
pub(crate) fn src_of(line: &MdLine) -> Option<&str> {
    matches!(line.kind, LineKind::Picture { .. })
        .then(|| line.spans.first()?.link.as_deref())
        .flatten()
}

#[cfg(test)]
mod rows_tests {
    #[test]
    fn a_rows_hint_sizes_the_box_and_is_kept_in_bounds() {
        let n = |md: &str| crate::md::render(md, 40).len();
        assert_eq!(n("![logo](cid:a#rows=3)"), 3);
        assert_eq!(n("![x](x.png)"), usize::from(super::ROWS));
        assert_eq!(super::rows_of("x#rows=900"), 40);
        assert_eq!(super::rows_of("x#rows=1"), 2);
    }
}
