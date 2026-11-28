//! The PANES cards' motion memory ([`crate::navpanemotion`]): what each card
//! said when last drawn, so a change can be told from a repaint, and the
//! frames a flash, fade or breath is owed.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::motion::MotionLevel;
use crate::navpanemotion::{Fx, BREATH_LEASE_MS, FLASH_MS};
use crate::navpanenter::ENTER_MS;
use crate::panestate::EntryState;

/// What a card said last time it was drawn.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Memo {
    state: EntryState,
    words: String,
    pub(crate) fx: Fx,
}

/// Line two with its numbers taken out: what a fade is allowed to react to.
///
/// A digit moving is a reading ticking, not new words. A Dash card's line is
/// `cpu 19% \u{b7} mem 72%`, re-read every second; compared whole, it faded
/// in from near-invisible (1.14:1 on a sheer window) once a second, forever.
/// `12s` becoming `13s` was already meant to be quiet; this makes any figure so.
fn gist(words: &str) -> String {
    words.chars().filter(|c| !c.is_ascii_digit()).collect()
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
    let words = gist(words);
    let Some(old) = memo else {
        return Memo {
            state,
            words,
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
    if words != old.words && !words.trim().is_empty() {
        fx.words_at = Some(now);
    }
    Memo { state, words, fx }
}

thread_local! {
    static MEMOS: RefCell<HashMap<(u64, usize), Memo>> = RefCell::new(HashMap::new());
    static ENDS: Cell<u64> = const { Cell::new(0) };
    static LEASE: Cell<u64> = const { Cell::new(0) };
    /// A cards pass has been drawn since the memory was last forgotten.
    static SEEDED: Cell<bool> = const { Cell::new(false) };
}

/// Observe every card of one `pane_rows` pass: `(born_ms, index, state,
/// words)` each; returns each one's [`Fx`] and forgets panes not listed.
/// Without `cards` (another section, a short panel, the rail) it forgets them
/// all and returns none: what changed out of sight is not news on return.
pub(crate) fn observe_rows<'a>(
    rows: impl Iterator<Item = (u64, usize, EntryState, &'a str)>,
    now: u64,
    cards: bool,
) -> Option<Vec<Fx>> {
    let level = crate::motion::level();
    if !cards {
        MEMOS.with_borrow_mut(HashMap::clear);
        SEEDED.set(false);
        return None;
    }
    let end = level.scale_ms(FLASH_MS);
    Some(MEMOS.with_borrow_mut(|m| {
        let rows: Vec<_> = rows.collect();
        // A created pane is pushed to the END and is the one card the last pass did not have.
        // `born_ms` alone can't say it: a tab switch and an un-hide re-stamp it, and every
        // card of the tab would replay the entrance (review, round 32).
        let last = rows.iter().map(|r| r.1).max();
        let unmatched = rows.iter().filter(|r| !m.contains_key(&(r.0, r.1))).count();
        let grew = rows.len() == m.len() + 1 && unmatched == 1;
        let mut seen = HashMap::new();
        let fxs = rows
            .into_iter()
            .map(|(born, i, state, words)| {
                let old = m.remove(&(born, i));
                let fresh = old.is_none()
                    && grew
                    && Some(i) == last
                    && SEEDED.get()
                    && crate::navpanenter::is_new(born, now);
                let mut memo = observe(old, state, words, now, level);
                if fresh && memo.fx.on {
                    memo.fx.born_at = Some(now);
                    ENDS.set(ENDS.get().max(now + level.scale_ms(ENTER_MS)));
                }
                let fx = memo.fx;
                if fx.on && state == EntryState::Working {
                    LEASE.set(now + BREATH_LEASE_MS);
                }
                for at in [fx.state_at, fx.words_at].into_iter().flatten() {
                    ENDS.set(ENDS.get().max(at + end));
                }
                seen.insert((born, i), memo);
                fx
            })
            .collect();
        *m = seen;
        SEEDED.set(true);
        fxs
    }))
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
