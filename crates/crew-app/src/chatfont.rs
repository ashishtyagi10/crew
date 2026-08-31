//! `/font` typed in a chat pane's composer: recognized here so it can be
//! routed to the app's input-bar font path (`set_font_cmd`) instead of being
//! sent to the broker as swarm text — where it silently did nothing.

/// The argument of a `/font` composer submission (`""` for the bare command),
/// or `None` when `text` isn't a `/font` command at all.
pub(crate) fn parse(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed == "/font" {
        return Some(String::new());
    }
    trimmed
        .strip_prefix("/font ")
        .map(|arg| arg.trim().to_string())
}

#[cfg(test)]
#[path = "chatfont_tests.rs"]
mod tests;
