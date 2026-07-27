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

/// Height without the waiting hint: top border, input row, bottom border.
const ROWS_PLAIN: u16 = 3;
/// Height with it: the hint gets an interior row of its own.
const ROWS_WAITING: u16 = 4;

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
    ///
    /// Cleared when the user TYPES a character ([`Self::key`]'s `Char` arm):
    /// entering a key by hand means they are no longer waiting on the browser.
    /// A real paste does NOT clear it — Cmd+V and right-click-paste are routed
    /// to [`Self::paste`], which leaves the hint up on purpose, because the
    /// browser flow can still land and is still the thing that will close this
    /// prompt.
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

    /// Show (or stop showing) that a browser sign-in is in flight. Typing a
    /// character clears it again; pasting does not — see the field's doc.
    pub(crate) fn set_waiting(&mut self, waiting: bool) {
        self.waiting = waiting;
    }

    /// Drop anything typed and go back to showing the browser hint: what this
    /// prompt becomes while it is HIDDEN with its sign-in still in flight.
    ///
    /// Being hidden (the input bar took focus, another pane did, the help
    /// overlay opened) is not the user dismissing the prompt, so the flow —
    /// and the prompt that comes back with it — survives. The half-typed
    /// buffer does not: nothing on screen would be holding it, and the whole
    /// point of the masked field is that a secret never outlives the card
    /// showing it.
    pub(crate) fn forget_typing(&mut self) {
        self.buf.clear();
        self.waiting = true;
    }

    /// How tall this prompt's card is right now. The hint row only exists
    /// while a sign-in is in flight, so an `ANTHROPIC_API_KEY` prompt (which
    /// has no browser flow at all) must not reserve — and draw a blank —
    /// interior row for it. The renderer sizes the card from this, so the
    /// height and the drawn cells can never disagree.
    pub(crate) fn rows(&self) -> u16 {
        if self.waiting {
            ROWS_WAITING
        } else {
            ROWS_PLAIN
        }
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
                // Typed, not pasted (a paste never reaches here — see above):
                // the user is entering the key by hand, so the card should
                // stop claiming to be waiting on a browser.
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
    ///
    /// Deliberately leaves `waiting` alone, unlike [`Self::key`]'s `Char` arm:
    /// a paste is one gesture that may or may not be the user's final answer,
    /// and the browser flow behind the hint is still live until it lands or is
    /// dismissed.
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
            self.rows(),
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
            // ROW 2 IS LOAD-BEARING, not decoration: the hint text contains
            // almost every character of a typical key, so drawing it on row 1
            // would make the leak assertion (which scopes itself to row 1)
            // vacuous. A test pins it here.
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
