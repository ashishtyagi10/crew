//! Type-ahead suggestions for the input bar: slash-command completion,
//! `cd` directory completion, and fish-style history autosuggestion. Returns the
//! ghost *suffix* to display after the typed text (and to insert on accept).
use std::path::{Path, PathBuf};

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
        name: "/find",
        desc: "Search scrollback (/find <text>)",
    },
    Cmd {
        name: "/name",
        desc: "Rename the focused pane (/name <text>)",
    },
    Cmd {
        name: "/clear",
        desc: "Clear the focused pane's scrollback",
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

/// Completion suffix for a `cd <partial>` line: completes the final path
/// component to the first matching subdirectory of `base` (with a trailing `/`),
/// or `None`. The directory portion of `partial` is resolved against `base`.
pub(crate) fn dir_suggest(text: &str, base: &Path) -> Option<String> {
    let arg = text.strip_prefix("cd ")?;
    if arg.is_empty() || arg.ends_with('/') {
        return None; // nothing partial to complete
    }
    let (dir_part, leaf) = match arg.rfind('/') {
        Some(i) => (&arg[..=i], &arg[i + 1..]),
        None => ("", arg),
    };
    if leaf.is_empty() {
        return None;
    }
    let mut names: Vec<String> = std::fs::read_dir(expand(dir_part, base))
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(leaf) && n != leaf)
        .collect();
    names.sort();
    names
        .into_iter()
        .next()
        .map(|n| format!("{}/", &n[leaf.len()..]))
}

/// Resolve the directory portion of a `cd` argument to a path to list: `~/`
/// expands to `$HOME`, an absolute path is kept, otherwise it joins `base`.
fn expand(dir_part: &str, base: &Path) -> PathBuf {
    if dir_part.is_empty() {
        return base.to_path_buf();
    }
    if let Some(rest) = dir_part.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    let p = Path::new(dir_part);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
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
    fn dir_suggest_completes_subdir() {
        let base = std::env::temp_dir().join("crew_dirsuggest_test");
        std::fs::create_dir_all(base.join("alpha")).unwrap();
        std::fs::create_dir_all(base.join("beta")).unwrap();
        assert_eq!(dir_suggest("cd al", &base).as_deref(), Some("pha/"));
        assert_eq!(dir_suggest("cd be", &base).as_deref(), Some("ta/"));
        // no partial leaf, or a trailing slash → nothing to complete
        assert_eq!(dir_suggest("cd ", &base), None);
        assert_eq!(dir_suggest("cd alpha/", &base), None);
        // not a `cd` line, and a leaf that matches nothing
        assert_eq!(dir_suggest("ls al", &base), None);
        assert_eq!(dir_suggest("cd zzz", &base), None);
    }

    #[test]
    fn matches_filters_by_prefix() {
        let names: Vec<&str> = matches("/s").iter().map(|c| c.name).collect();
        assert!(names.contains(&"/settings") && names.contains(&"/shell"));
        assert!(!names.contains(&"/exit"));
        assert!(matches("ls").is_empty()); // non-slash → no palette
    }
}
