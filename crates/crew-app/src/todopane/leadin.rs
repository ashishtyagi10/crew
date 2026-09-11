//! The word someone puts IN FRONT of a date: `pay rent due friday`,
//! `ship build by fri 5pm`, `dentist on aug 15`, `standup at 9am`.
//!
//! [`super::duedate`] parses the date itself, and none of these words is
//! part of any date form — so the parser never ate them and they stayed
//! behind in the title, which then read `pay rent due`. Nobody typed that.
//! The preposition belongs to the date it introduces: it is tinted with the
//! fragment as you type and leaves the title with it on save.
//!
//! Only a run sitting IMMEDIATELY before the fragment counts, which is what
//! keeps the ordinary uses of these words safe — `turn on the lights
//! tomorrow` has prose between the `on` and the date, so it keeps both.

/// The lead-in words, lowercased. Each one reads as "…and here comes when".
const WORDS: [&str; 5] = ["due", "by", "on", "at", "before"];

/// Char index the highlighted fragment starts at: the date window opening
/// at token `s`, widened back over any lead-in words in front of it.
pub(crate) fn start(toks: &[(usize, usize, String)], s: usize) -> usize {
    let mut i = s;
    while i > 0 && WORDS.contains(&toks[i - 1].2.as_str()) {
        i -= 1;
    }
    toks[i].0
}

#[cfg(test)]
#[path = "leadin_tests.rs"]
mod tests;
