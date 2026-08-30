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
pub(super) const ROWS: u16 = 12;

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
    Some(
        (0..ROWS)
            .map(|i| MdLine {
                spans: vec![only.clone()],
                kind: LineKind::Picture { i, n: ROWS },
            })
            .collect(),
    )
}

/// The source a picture line names, if it is one.
pub(crate) fn src_of(line: &MdLine) -> Option<&str> {
    matches!(line.kind, LineKind::Picture { .. })
        .then(|| line.spans.first()?.link.as_deref())
        .flatten()
}
