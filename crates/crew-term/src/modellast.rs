//! The viewport's last line with anything on it — what a pane is saying right
//! now, without building a cell for every position on the screen.
//!
//! [`TermCore::cells`] is the only other way to read the grid's text, and it
//! resolves a colour, a background and a decoration per cell and allocates a
//! `RenderCell` for each survivor. A caller that wants one line of text (the
//! minimized strip's thumbnail, once a frame, for a pane nobody is looking at)
//! would pay for the whole screen to get it. This walks the same display in
//! one pass and keeps a single `String`.
use super::*;

impl TermCore {
    /// The bottom-most viewport row that is not blank, trimmed, or `None` on
    /// an empty screen.
    ///
    /// The *bottom-most*, not the newest: a program that redraws its own
    /// footer (an agent CLI, a progress bar) has its live interface down
    /// there, and that is what the pane looks like from across the room.
    pub(crate) fn last_line(&self) -> Option<String> {
        let content = self.term.renderable_content();
        // Scrolled into history, viewport lines are negative; the same
        // `display_offset` correction `cells` makes maps them back.
        let off = content.display_offset as i32;
        let mut row = i32::MIN;
        let mut buf = String::new();
        let mut last: Option<String> = None;
        let keep = |buf: &mut String, last: &mut Option<String>| {
            let text = buf.trim_end();
            if !text.trim().is_empty() {
                *last = Some(text.to_string());
            }
            buf.clear();
        };
        for ind in content.display_iter {
            let line = ind.point.line.0 + off;
            if line < 0 {
                continue;
            }
            if line != row {
                keep(&mut buf, &mut last);
                row = line;
            }
            // The blank alacritty parks behind a full-width character is not a
            // column of text (see `modelcells`), and `\0` is not a glyph.
            if ind.c != '\0'
                && !ind
                    .flags
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                buf.push(ind.c);
            }
        }
        keep(&mut buf, &mut last);
        last
    }
}

#[cfg(test)]
#[path = "modellast_tests.rs"]
mod modellast_tests;
