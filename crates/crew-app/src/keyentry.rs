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

/// Total height: top border, input row, waiting-hint row, bottom border.
pub(crate) const ROWS: u16 = 4;

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
    /// A browser sign-in is in flight for this prompt (OpenRouter only).
    /// Cleared the moment the user types, since that means they are pasting
    /// instead of waiting on the browser.
    waiting: bool,
}

impl KeyEntry {
    pub(crate) fn new(var: String) -> Self {
        Self {
            var,
            buf: String::new(),
            waiting: false,
        }
    }

    /// Show that a browser sign-in is in flight. Cleared as soon as the user
    /// types, since that means they are pasting instead.
    pub(crate) fn set_waiting(&mut self, waiting: bool) {
        self.waiting = waiting;
    }

    /// Route one key. Enter submits a non-blank buffer, Escape cancels,
    /// Backspace deletes, printable characters append. A paste never reaches
    /// this method at all: on this app, Cmd+V / right-click-paste is handled
    /// entirely in `clipboard.rs`, well before any pane sees a `ChatInput`, so
    /// it cannot arrive here as a run of `Char`s. `clipboard.rs` checks for an
    /// open prompt and routes a paste straight to [`Self::paste`] instead.
    /// EVERYTHING else is swallowed rather than forwarded — this prompt is
    /// modal.
    pub(crate) fn key(&mut self, k: &ChatInput) -> KeyOutcome {
        match k {
            ChatInput::Char(c) => {
                self.buf.push(*c);
                self.waiting = false;
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

    /// Append pasted `text` to the buffer, the entry point `clipboard.rs`
    /// uses instead of routing a paste into the composer while this prompt is
    /// open. Strips newlines — a copied key commonly carries a trailing one,
    /// and the buffer is trimmed again on submit regardless.
    pub(crate) fn paste(&mut self, text: &str) {
        self.buf.push_str(&text.replace(['\n', '\r'], ""));
    }

    /// The prompt as a fieldset card — a bordered box with the variable named
    /// in the legend, matching every other panel on the canvas rather than
    /// floating above it. The interior is one row of mask glyphs, one per
    /// typed character, clipped to the card's width.
    pub(crate) fn card(&self, cols: u16) -> Vec<CellView> {
        let t = crew_theme::theme();
        // Lowercase, matching every other composer-overlay legend
        // (`commands`, `attach`, `models`, `files`) — uppercase is the
        // sidebar's idiom, not this canvas's. The legend text itself is
        // never a leak risk: the secret is drawn on its own interior row
        // (row 1) and the leak test scopes its assertion to that row alone,
        // so a coincidental letter overlap with the legend can't hide a
        // real leak.
        //
        // `fit_legend` keeps the tail (the variable name) over the head (the
        // word "paste") when the card is too narrow for both, the same
        // treatment `inputbar_render.rs` gives the cwd legend.
        let title = crate::cwd::fit_legend(
            &format!("paste {}", self.var),
            cols.saturating_sub(6) as usize,
        );
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
        if self.waiting {
            let hint = "waiting for browser · or paste the key";
            for (i, ch) in hint.chars().take(inner).enumerate() {
                cells.push(CellView {
                    col: 1 + i as u16,
                    row: 2,
                    c: ch,
                    fg: t.text_muted,
                    bg: t.page_bg,
                    bold: false,
                    italic: false,
                });
            }
        }
        cells
    }
}

#[cfg(test)]
#[path = "keyentry_tests.rs"]
mod tests;
