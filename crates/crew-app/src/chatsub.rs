//! Subagent sections: one card per agent that worked on a turn, collapsed
//! until you ask for it.
//!
//! A fan-out sends the same task to the whole roster and eleven replies come
//! back, each of them a full answer. Rendered as ordinary reply cards they
//! were one wall of text: the eleventh agent's first line sat a screen and a
//! half below the first agent's, nothing said how much more there was, and
//! the summary line that closes the turn — the line you actually want — had
//! scrolled past before the fan finished.
//!
//! So a marked card ([`crew_plugin::metatag::SUB`], stamped by the broker
//! that fanned the work out) renders like a tool card: its header, its first
//! body line, and ` … +N` for the rest. The header keeps the agent's badge
//! and its `├`/`└` tree connector, so the turn reads as a list of who
//! answered; a click opens the one you want, exactly as [`crate::chatfold`]
//! opens any other folded card, and the open state lives on the message so
//! it survives the card settling.
//!
//! The agent's thought and its tool block are NOT hidden with it. They seat
//! above the header and are already one collapsed row each
//! (`chatthoughtview`, `chattoolview`), so they say what the agent did
//! without saying it at length — and each opens on its own click.
use crate::chatlayout::Message;

/// Whether this card is one subagent's section rather than an answer to the
/// user.
///
/// The broker decides, in `meta` — never the app by guessing at the sender.
/// `planner → user` is the shape of a fan reply AND the shape of the only
/// reply in an ordinary one-agent turn, and folding that one by default
/// would collapse the answer the user is waiting for.
pub(crate) fn is_section(m: &Message) -> bool {
    crew_plugin::metatag::has(&m.meta, crew_plugin::metatag::SUB)
}

/// The `meta` a LIVE card opens with: the mark when the broker said this
/// stream is one subagent's ([`crew_plugin::metatag::SUB`] on the `Delta`),
/// nothing when it did not. The live card is written with the same tag the
/// settled card will carry, so one predicate folds both and the section does
/// not pop open as the turn settles.
pub(crate) fn live_meta(sub: bool) -> String {
    match sub {
        true => crew_plugin::metatag::SUB.to_string(),
        false => String::new(),
    }
}

#[cfg(test)]
#[path = "chatsub_tests.rs"]
mod tests;
