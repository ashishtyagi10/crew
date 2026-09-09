//! The crew pane's side of the typewriter (`chatreveal`): one [`CardReveal`]
//! per card still being typed out, checkpointed as deltas land and re-keyed
//! to the settled `Message` when the reply lands — plus the two predicates
//! that keep frames coming after the pane stops being busy (a card fading in,
//! a card still typing). Split from `chatflow` for the 200-line cap.
use crate::chat::ChatPane;
use crate::chatflow::stream_key;
use crate::chatreveal::{visible_len, CardReveal, CATCHUP_MS};

/// How long a settled card's state outlives its reveal: the reveal itself is
/// done within [`CATCHUP_MS`], and the state must still be found for the
/// card's fade-in window after that (it is what suppresses the re-fire).
const LINGER_MS: u64 = CATCHUP_MS + crate::chatcard::FADE_MS * 2;

impl ChatPane {
    /// A delta grew `key`'s provisional card `old` → `new` chars at `now`.
    pub(crate) fn note_delta(&mut self, key: &str, old: usize, new: usize, now: u64) {
        self.prune_reveals(now);
        let level = crate::motion::level();
        match self
            .reveals
            .iter_mut()
            .find(|r| r.key == key && r.ts.is_none())
        {
            Some(r) => r.state = r.state.arrived(now, old, new, level),
            None => self.reveals.push(CardReveal {
                key: key.to_string(),
                ts: None,
                state: crate::chatreveal::Reveal::default().arrived(now, old, new, level),
            }),
        }
    }

    /// `sender`'s reply settled as a `Message` stamped `ts` with `new` chars:
    /// its provisional card's reveal continues on the settled card from
    /// whatever was visible — no snap-back. Called BEFORE `settle_stream`
    /// drops the card, which is where the streamed length still is.
    pub(crate) fn note_settle(&mut self, sender: &str, ts: &str, new: usize, now: u64) {
        self.prune_reveals(now);
        let key = stream_key(sender);
        let old = self
            .streaming
            .iter()
            .find(|m| stream_key(&m.sender) == key)
            .map_or(0, |m| m.text.chars().count());
        if let Some(r) = self
            .reveals
            .iter_mut()
            .find(|r| r.key == key && r.ts.is_none())
        {
            r.state = r.state.settle(now, old, new, crate::motion::level());
            r.ts = Some(ts.to_string());
        }
    }

    /// Forget settled states past their linger window. Provisional states
    /// live as long as their card (see [`Self::drop_stream_reveals`]).
    fn prune_reveals(&mut self, now: u64) {
        self.reveals
            .retain(|r| r.ts.is_none() || now.saturating_sub(r.state.at_ms) < LINGER_MS);
    }

    /// The provisional cards are gone (turn over, broker gone): their reveal
    /// states go with them. Settled cards keep finishing.
    pub(crate) fn drop_stream_reveals(&mut self) {
        self.reveals.retain(|r| r.ts.is_some());
    }

    /// Show every card whole, as if motion were off — for tests that stream
    /// text and then assert on the rendered transcript.
    #[cfg(test)]
    pub(crate) fn reveal_all(&mut self) {
        self.reveals.clear();
    }

    /// Whether any card on screen still has characters to type out — the
    /// redraw-scheduling predicate (`panebusy::pane_animating`) that keeps
    /// the reveal ticking after the last delta, when the pane is idle.
    pub(crate) fn is_revealing(&self) -> bool {
        if self.reveals.is_empty() {
            return false; // the idle case, asked every poll tick
        }
        let (now, level) = (crate::chattime::unix_now_ms(), crate::motion::level());
        let settled = self.messages.len();
        self.visible_messages().iter().enumerate().any(|(i, m)| {
            crate::chatreveal::find(&self.reveals, m, i >= settled).is_some_and(|r| {
                let total = m.text.chars().count();
                visible_len(r, now, total, level) < total
            })
        })
    }

    /// Whether the newest message is still fading in — keeps redraw frames
    /// flowing for the fade's few hundred ms after a reply lands.
    pub(crate) fn is_fading(&self) -> bool {
        self.messages
            .last()
            .is_some_and(|m| crate::chatmsgs::fade_t(&m.ts, crate::chattime::unix_now_ms()) < 1.0)
    }
}

#[cfg(test)]
#[path = "chatrevealpane_tests.rs"]
mod tests;
