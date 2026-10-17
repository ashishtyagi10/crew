//! The code line's word (`chatphase` picks it) and how it decodes
//! out of the glyphs — a scramble, the letters locking in left to right
//! behind a bright head, the word held, then dissolving back to glyphs.
//!
//! Split from [`crate::chatcodeline`], which draws it, for the line cap.
use crate::charrain::hash;

/// The word's cycle at full motion: all glyphs, then the letters locking in
/// one by one behind a bright head, then the word held, then dissolving.
pub(crate) const SCRAMBLE_MS: u64 = 500;
pub(crate) const LOCK_MS: u64 = 70;
pub(crate) const HOLD_MS: u64 = 2_400;
pub(crate) const DISSOLVE_MS: u64 = 450;

/// What cell `i` of an `n`-letter word shows `at` ms into the cycle.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Slot {
    Glyph,
    Head,
    Letter,
}

/// Cell `i` of an `n`-letter word `at` ms into its cycle (`ms` scales each
/// phase for the Motion level), `seed` ordering the dissolve.
pub(crate) fn slot(i: u64, n: u64, at: u64, seed: u64, ms: impl Fn(u64) -> u64) -> Slot {
    let (scramble, lock, hold) = (ms(SCRAMBLE_MS), ms(LOCK_MS).max(1), ms(HOLD_MS));
    let locked = scramble + n * lock;
    match at {
        a if a < scramble => Slot::Glyph,
        a if a < locked => match (a - scramble) / lock {
            k if i < k => Slot::Letter,
            k if i == k => Slot::Head,
            _ => Slot::Glyph,
        },
        a if a < locked + hold => Slot::Letter,
        // Letters fall back to glyphs in a scattered order, not a wipe.
        a if a - locked - hold < hash(seed, i) % ms(DISSOLVE_MS).max(1) => Slot::Letter,
        _ => Slot::Glyph,
    }
}

/// The whole cycle for an `n`-letter word, each phase scaled by `ms`.
pub(crate) fn cycle(n: u64, ms: impl Fn(u64) -> u64) -> u64 {
    (ms(SCRAMBLE_MS) + n * ms(LOCK_MS) + ms(HOLD_MS) + ms(DISSOLVE_MS)).max(1)
}

#[cfg(test)]
#[path = "chatcodeword_tests.rs"]
mod tests;
