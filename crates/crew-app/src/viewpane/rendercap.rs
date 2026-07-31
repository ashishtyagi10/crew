//! Bounding how much of a file's TEXT any rung renders (Fix 5). Split out of
//! `lines.rs` to keep that file under the file-length budget, and so the
//! truncation logic itself has fast, precise unit tests that don't have to
//! run the (necessarily expensive, on a large input) real per-rung renderers
//! to exercise it.

/// Ceiling on how many SOURCE lines any rung will render. Reached from
/// `lines_for` on first render, on every `s` toggle, and on every distinct
/// `cols` value during a resize drag — for markdown, that runs the full
/// `md::render` over up to `load::MAX_VIEW_BYTES` (8 MB) each time, which is
/// roughly 250 MB allocated inside one winit-thread frame, freezing every
/// pane in the grid. 50 000 lines is generous for "the top of a huge file",
/// the stated use case (`load.rs`'s own "the 40 MB log is precisely the file
/// you want to look at the top of"), while keeping a worst-case render
/// bounded.
pub(crate) const MAX_RENDER_LINES: usize = 50_000;

/// `text` capped to at most [`MAX_RENDER_LINES`] source lines, plus the real
/// count when it was longer — so the caller can name the truncation in a
/// banner, the same way the byte cap already does. Cuts BEFORE the expensive
/// per-rung render runs (tokenizing, markdown layout, CSV column
/// measurement, ...), not after: truncating the RESULT would still pay the
/// full cost this fix exists to avoid.
pub(crate) fn cap_render_lines(text: &str) -> (&str, Option<usize>) {
    let total = text.split('\n').count();
    if total <= MAX_RENDER_LINES {
        return (text, None);
    }
    let mut seen = 0usize;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            seen += 1;
            if seen == MAX_RENDER_LINES {
                return (&text[..i], Some(total));
            }
        }
    }
    (text, None)
}

#[cfg(test)]
#[path = "rendercap_tests.rs"]
mod tests;
