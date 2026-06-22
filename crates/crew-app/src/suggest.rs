//! Type-ahead suggestions for the input bar: slash-command completion and
//! fish-style history autosuggestion. Returns the ghost *suffix* to display
//! after the typed text (and to insert when the user accepts it).

/// A slash command shown in the command palette.
pub(crate) struct Cmd {
    pub name: &'static str,
    pub desc: &'static str,
}

/// Known slash commands (kept in sync with `run_slash_command`).
pub(crate) const COMMANDS: &[Cmd] = &[
    Cmd {
        name: "/settings",
        desc: "Open settings",
    },
    Cmd {
        name: "/shell",
        desc: "New shell pane",
    },
    Cmd {
        name: "/update",
        desc: "Update Crew (git pull)",
    },
    Cmd {
        name: "/keys",
        desc: "Show keyboard shortcuts",
    },
    Cmd {
        name: "/exit",
        desc: "Quit Crew",
    },
];

/// Commands whose name starts with `text` (empty unless `text` begins with `/`).
pub(crate) fn matches(text: &str) -> Vec<&'static Cmd> {
    if !text.starts_with('/') {
        return Vec::new();
    }
    COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(text))
        .collect()
}

/// Suggested completion suffix for `text`, or `None` if nothing completes it.
/// Slash input completes against the command list; everything else against the
/// most recent matching `history` entry.
pub(crate) fn suggest(text: &str, history: &[String]) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    if text.starts_with('/') {
        return COMMANDS
            .iter()
            .map(|c| c.name)
            .find(|name| name.starts_with(text) && *name != text)
            .map(|name| name[text.len()..].to_string());
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

    #[test]
    fn matches_filters_by_prefix() {
        let names: Vec<&str> = matches("/s").iter().map(|c| c.name).collect();
        assert!(names.contains(&"/settings") && names.contains(&"/shell"));
        assert!(!names.contains(&"/exit"));
        assert!(matches("ls").is_empty()); // non-slash → no palette
    }
}
