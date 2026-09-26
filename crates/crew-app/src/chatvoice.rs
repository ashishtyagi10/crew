//! What ink a card's body speaks in, and the text it says it with.
use crate::chatcard::{is_system_voice, is_tool_card, TOOL_PREFIX};
use crate::chatlayout::Message;
use std::borrow::Cow;

/// The prefix the broker stamps on a failure it reports for an agent — the
/// fan's `[error] opencode: timed out after 180s`, a relay hop that errored.
pub(crate) const ERROR_PREFIX: &str = "[error] ";

/// The body's ink and text. Agents speak in ink; the system voice — and the
/// machine talking on an agent's behalf — stays muted.
///
/// The `[tool] ` and `[error] ` markers are MACHINERY (the broker's card
/// kind, carried in the text because that is the only field that crosses
/// the wire), stripped in the ONE place both the counting and the drawing
/// pass read, so they agree on where it wraps. A tool card's gutter and ink
/// already say what it is; an error said so in a bracketed word, the last
/// one in the pane after the chrome's brackets became drawn marks. It now
/// leads with the `✗` a failed tool call wears, in the same removed ink.
pub(crate) fn body_voice(m: &Message) -> ((u8, u8, u8), Cow<'_, str>) {
    let muted = crew_theme::theme().text_muted;
    if is_tool_card(m) {
        return (muted, Cow::Borrowed(&m.text[TOOL_PREFIX.len()..]));
    }
    if let Some(rest) = m.text.strip_prefix(ERROR_PREFIX) {
        let fg = crate::chatink::token_fg(crate::md::syntax::Token::Removed);
        return (fg, Cow::Owned(format!("\u{2717} {rest}")));
    }
    match is_system_voice(&m.sender) {
        true => (muted, Cow::Borrowed(&m.text)),
        false => (crew_theme::theme().ink, Cow::Borrowed(&m.text)),
    }
}

#[cfg(test)]
#[path = "chatvoice_tests.rs"]
mod tests;
