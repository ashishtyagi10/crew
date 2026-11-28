//! The PANES cards' motion memory ([`crate::navpanemotion`]): what each card
//! said when last drawn, so a change can be told from a repaint, and the
//! frames a flash, fade or breath is owed.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::motion::MotionLevel;
use crate::navpanemotion::{Fx, BREATH_LEASE_MS, FLASH_MS};
use crate::panestate::EntryState;

/// What a card said last time it was drawn.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Memo {
    state: EntryState,
    words: String,
    pub(crate) fx: Fx,
}

/// The card's `state` and `words` at `now`, against what `memo` last saw.
pub(crate) fn observe(
    memo: Option<Memo>,
    state: EntryState,
    words: &str,
    now: u64,
    level: MotionLevel,
) -> Memo {
    let on = level != MotionLevel::Off;
    let Some(old) = memo else {
        return Memo {
            state,
            words: words.into(),
            fx: Fx {
                on,
                ..Fx::default()
            },
        };
    };
    let mut fx = Fx { on, ..old.fx };
    if state != old.state {
        fx.state_at = Some(now);
    }
    if words != old.words && !words.is_empty() {
        fx.words_at = Some(now);
    }
    Memo {
        state,
        words: words.into(),
        fx,
    }
}

thread_local! {
    static MEMOS: RefCell<HashMap<(u64, usize), Memo>> = RefCell::new(HashMap::new());
    static ENDS: Cell<u64> = const { Cell::new(0) };
    static LEASE: Cell<u64> = const { Cell::new(0) };
}

/// Observe every card of one `pane_rows` pass: `(born_ms, index, state,
/// words)` each; returns each one's [`Fx`] and forgets panes not listed.
pub(crate) fn observe_rows<'a>(
    rows: impl Iterator<Item = (u64, usize, EntryState, &'a str)>,
    now: u64,
) -> Vec<Fx> {
    let level = crate::motion::level();
    MEMOS.with_borrow_mut(|m| {
        let mut seen = HashMap::new();
        let fxs = rows
            .map(|(born, i, state, words)| {
                let memo = observe(m.remove(&(born, i)), state, words, now, level);
                let fx = memo.fx;
                if fx.on && state == EntryState::Working {
                    LEASE.set(now + BREATH_LEASE_MS);
                }
                for at in [fx.state_at, fx.words_at].into_iter().flatten() {
                    ENDS.set(ENDS.get().max(at + FLASH_MS));
                }
                seen.insert((born, i), memo);
                fx
            })
            .collect();
        *m = seen;
        fxs
    })
}

/// A flash or fade still has frames to draw at `now` (never at Off).
pub(crate) fn live(now: u64) -> bool {
    crate::motion::level() != MotionLevel::Off && now < ENDS.get()
}

/// A working card was drawn lately: the breath is owed ambient frames.
pub(crate) fn breathing(now: u64) -> bool {
    crate::motion::level() != MotionLevel::Off && now < LEASE.get()
}

#[cfg(test)]
#[path = "navpanememo_tests.rs"]
mod tests;
