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
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_the_font_arg() {
        assert_eq!(parse("/font random"), Some("random".to_string()));
        assert_eq!(parse("/font"), Some(String::new()));
        assert_eq!(parse("  /font 18  "), Some("18".to_string()));
    }

    #[test]
    fn parse_rejects_foreign_text() {
        assert_eq!(parse("/fontx"), None);
        assert_eq!(parse("/ font"), None);
        assert_eq!(parse("hello /font"), None);
    }
}
