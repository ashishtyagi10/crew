//! Whether the cursor blinks, and the switch that darkens it.
//!
//! alacritty keeps the blink a program asked for (DECSCUSR 1/3/5 blink, 2/4/6
//! stand still, DECSET 12 toggles it for the current shape) on the cursor's
//! style, and leaves the clock to the window — it only raises
//! `CursorBlinkingChange`, which crew never needed to hear, because the app
//! reads the style at frame time instead. So the terminal answers one
//! question (does it blink?) and takes one instruction (lit or dark this
//! frame); when, and for which pane, is the app's call (`cursorblink` there).
use super::*;

impl TermCore {
    /// Whether the program wants a blinking cursor — the shape it draws is
    /// [`crate::cursor::shape_for`]'s business, the timing the app's.
    pub(crate) fn cursor_blinks(&self) -> bool {
        self.term.cursor_style().blinking
    }

    /// Light or darken the focused cursor for the frames that follow.
    pub(crate) fn set_cursor_lit(&mut self, lit: bool) {
        self.cursor_lit = lit;
    }
}

impl crate::PtyTerm {
    /// [`TermCore::cursor_blinks`].
    pub fn cursor_blinks(&self) -> bool {
        self.core.cursor_blinks()
    }

    /// [`TermCore::set_cursor_lit`].
    pub fn set_cursor_lit(&mut self, lit: bool) {
        self.core.set_cursor_lit(lit);
    }
}

impl HeadlessTerm {
    /// [`TermCore::cursor_blinks`].
    pub fn cursor_blinks(&self) -> bool {
        self.core.cursor_blinks()
    }

    /// [`TermCore::set_cursor_lit`].
    pub fn set_cursor_lit(&mut self, lit: bool) {
        self.core.set_cursor_lit(lit);
    }
}

#[cfg(test)]
#[path = "cursorblink_tests.rs"]
mod tests;
