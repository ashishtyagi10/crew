//! The masked provider-key prompt. Opens when a model row is accepted that the
//! active stack can't serve for want of a key (`Route::needs_key`), so the
//! answer to "this needs ANTHROPIC_API_KEY" is a field rather than a trip to a
//! shell rc file and a restart.
//!
//! Modal by construction: while it is open every key belongs to it (see
//! [`KeyEntry::key`]), so nothing leaks to the pane underneath while a secret
//! is half-typed.
//!
//! The buffer is NEVER rendered in plaintext, logged, exported or written
//! anywhere but the credential store.
use crew_render::CellView;

use crate::chatkeys::ChatInput;

/// Total height: top border, one input row, bottom border.
pub(crate) const ROWS: u16 = 3;

/// What one key did to the prompt.
pub(crate) enum KeyOutcome {
    /// Handled; the prompt stays open. Every key that isn't Enter or Escape
    /// lands here, including ones the prompt ignores.
    Consumed,
    /// Escape: discard the buffer and close.
    Cancelled,
    /// Enter on a non-blank buffer: the trimmed key.
    Submit(String),
}

pub(crate) struct KeyEntry {
    /// The variable being supplied, e.g. `ANTHROPIC_API_KEY`. Shown; the
    /// buffer is not.
    pub(crate) var: String,
    buf: String,
}

impl KeyEntry {
    pub(crate) fn new(var: String) -> Self {
        Self {
            var,
            buf: String::new(),
        }
    }

    /// Route one key. Enter submits a non-blank buffer, Escape cancels,
    /// Backspace deletes, printable characters append (a paste arrives as a
    /// run of `Char`s). EVERYTHING else is swallowed rather than forwarded —
    /// this prompt is modal.
    pub(crate) fn key(&mut self, k: &ChatInput) -> KeyOutcome {
        match k {
            ChatInput::Char(c) => {
                self.buf.push(*c);
                KeyOutcome::Consumed
            }
            ChatInput::Backspace => {
                self.buf.pop();
                KeyOutcome::Consumed
            }
            ChatInput::Close => KeyOutcome::Cancelled,
            ChatInput::Enter => {
                // Pasted keys commonly carry a trailing newline or space.
                let v = self.buf.trim().to_string();
                if v.is_empty() {
                    KeyOutcome::Consumed
                } else {
                    KeyOutcome::Submit(v)
                }
            }
            _ => KeyOutcome::Consumed,
        }
    }

    /// The prompt as a fieldset card — a bordered box with the variable named
    /// in the legend, matching every other panel on the canvas rather than
    /// floating above it. The interior is one row of mask glyphs, one per
    /// typed character, clipped to the card's width.
    pub(crate) fn card(&self, cols: u16) -> Vec<CellView> {
        let t = crew_theme::theme();
        // Uppercase, not "paste {var}": the var name is already all-caps, and
        // a lowercase word here would put ordinary letters on screen that can
        // coincidentally match a secret's own characters (e.g. "paste" itself
        // contains p/a/s/t/e), defeating the point of masking. All-caps
        // keeps the instruction readable without that collision.
        let title = format!("PASTE {}", self.var);
        let mut cells = crate::boxdraw::titled_card(
            cols,
            ROWS,
            &title,
            t.border_normal,
            t.legend_off,
            t.page_bg,
        );
        if cells.is_empty() {
            return cells;
        }
        let inner = cols.saturating_sub(2) as usize;
        for i in 0..self.buf.chars().count().min(inner) {
            cells.push(CellView {
                col: 1 + i as u16,
                row: 1,
                c: '•',
                fg: t.ink,
                bg: t.page_bg,
                bold: false,
                italic: false,
            });
        }
        cells
    }
}

#[cfg(test)]
#[path = "keyentry_tests.rs"]
mod tests;
