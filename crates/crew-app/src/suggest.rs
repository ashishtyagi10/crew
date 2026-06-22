//! Type-ahead suggestions for the input bar: slash-command completion and
//! fish-style history autosuggestion. Returns the ghost *suffix* to display
//! after the typed text (and to insert when the user accepts it).

/// Slash commands offered for completion (kept in sync with `run_slash_command`).
pub(crate) const SLASH_COMMANDS: &[&str] = &["/settings", "/shell", "/exit", "/update"];

/// Suggested completion suffix for `text`, or `None` if nothing completes it.
/// Slash input completes against the command list; everything else against the
/// most recent matching `history` entry.
pub(crate) fn suggest(text: &str, history: &[String]) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    if text.starts_with('/') {
        return SLASH_COMMANDS
            .iter()
            .find(|cmd| cmd.starts_with(text) && **cmd != text)
            .map(|cmd| cmd[text.len()..].to_string());
    }
    history
        .iter()
        .rev()
        .find(|past| past.starts_with(text) && past.as_str() != text)
        .map(|past| past[text.len()..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_no_suggestion() {
        assert_eq!(suggest("", &[]), None);
    }

    #[test]
    fn slash_prefix_completes_command() {
        assert_eq!(suggest("/se", &[]).as_deref(), Some("ttings"));
        assert_eq!(suggest("/sh", &[]).as_deref(), Some("ell"));
    }

    #[test]
    fn exact_command_offers_nothing() {
        assert_eq!(suggest("/exit", &[]), None);
    }

    #[test]
    fn unknown_slash_has_no_suggestion() {
        assert_eq!(suggest("/zzz", &[]), None);
    }

    #[test]
    fn history_autosuggests_most_recent_match() {
        let hist = vec!["git status".to_string(), "git push".to_string()];
        // most recent ("git push") wins for the shared "git " prefix
        assert_eq!(suggest("git ", &hist).as_deref(), Some("push"));
        assert_eq!(suggest("git s", &hist).as_deref(), Some("tatus"));
    }

    #[test]
    fn history_no_match_is_none() {
        let hist = vec!["ls -la".to_string()];
        assert_eq!(suggest("cargo", &hist), None);
    }
}
