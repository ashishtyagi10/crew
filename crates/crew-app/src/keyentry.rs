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
    /// Cleared when the user TYPES ([`Self::key`]'s `Char` arm); a paste
    /// ([`Self::paste`]) leaves it, because the browser flow can still land
    /// and is still the thing that will close this prompt.
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
    /// when there is a hint (a sign-in in flight, or a free key to point
    /// at), so an `ANTHROPIC_API_KEY` prompt must not reserve — and draw a
    /// blank — interior row. The renderer sizes the card from this.
    pub(crate) fn rows(&self) -> u16 {
        if self.hint().is_some() {
            ROWS_WAITING
        } else {
            ROWS_PLAIN
        }
    }

    /// The interior hint row's text (`keyhint`), if this prompt has one.
    fn hint(&self) -> Option<&'static str> {
        crate::keyhint::hint(&self.var, self.waiting)
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
}

#[path = "keycard.rs"]
mod keycard;

#[cfg(test)]
#[path = "keyentry_tests.rs"]
mod tests;
