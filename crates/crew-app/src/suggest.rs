//! Type-ahead suggestions for the input bar: slash-command completion,
//! `cd` directory completion, and fish-style history autosuggestion. Returns the
//! ghost *suffix* to display after the typed text (and to insert on accept).
use std::path::Path;

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
        name: "/crew",
        desc: "Open the multi-agent pane (claude/codex/opencode relay)",
    },
    Cmd {
        name: "/claude",
        desc: "Open Claude Code in a new pane",
    },
    Cmd {
        name: "/codex",
        desc: "Open Codex in a new pane",
    },
    Cmd {
        name: "/opencode",
        desc: "Open opencode in a new pane",
    },
    Cmd {
        name: "/run",
        desc: "Run a command in a new pane (/run <cmd>)",
    },
    Cmd {
        name: "/find",
        desc: "Search scrollback, highlighting matches (/find <text>)",
    },
    Cmd {
        name: "/edit",
        desc: "Open a file in $EDITOR in a new pane (/edit <file>)",
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
        name: "/clearlog",
        desc: "Clear the live activity log in the sidebar",
    },
    Cmd {
        name: "/only",
        desc: "Close all panes except the focused one",
    },
    Cmd {
        name: "/copy",
        desc: "Copy the focused pane's full scrollback to the clipboard",
    },
    Cmd {
        name: "/dump",
        desc: "Save scrollback to a file (/dump [file])",
    },
    Cmd {
        name: "/open",
        desc: "Open a URL/path, or the last URL on screen (/open [target])",
    },
    Cmd {
        name: "/font",
        desc: "Set the font size (/font <n>)",
    },
    Cmd {
        name: "/reload",
        desc: "Reload config.toml from disk and apply it",
    },
    Cmd {
        name: "/update",
        desc: "Update Crew to the latest release",
    },
    Cmd {
        name: "/broadcast",
        desc: "Toggle synchronized input to all panes (Cmd+S)",
    },
    Cmd {
        name: "/zoom",
        desc: "Toggle zoom of the focused pane (Cmd+Z)",
    },
    Cmd {
        name: "/sidebar",
        desc: "Toggle the stats sidebar (Cmd+G)",
    },
    Cmd {
        name: "/keys",
        desc: "Show keyboard shortcuts",
    },
    Cmd {
        name: "/far",
        desc: "Open a dual-pane file manager",
    },
    Cmd {
        name: "/exit",
        desc: "Quit Crew",
    },
];

/// Commands matching `text` for the palette: a prefix match ranks first, then a
/// fuzzy subsequence match (so `/dmp` still finds `/dump`). Empty unless `text`
/// begins with `/`; original list order breaks ties.
pub(crate) fn matches(text: &str) -> Vec<&'static Cmd> {
    if !text.starts_with('/') {
        return Vec::new();
    }
    let q = text[1..].to_lowercase();
    let mut scored: Vec<(u8, usize, &'static Cmd)> = COMMANDS
        .iter()
        .enumerate()
        .filter_map(|(i, c)| rank(&c.name[1..], &q).map(|r| (r, i, c)))
        .collect();
    scored.sort_by_key(|(r, i, _)| (*r, *i));
    scored.into_iter().map(|(_, _, c)| c).collect()
}

/// Match quality of `name` (sans slash) against lowercased query `q`: `0` for a
/// prefix match, `1` for a fuzzy subsequence match, `None` for no match.
fn rank(name: &str, q: &str) -> Option<u8> {
    let name = name.to_lowercase();
    if name.starts_with(q) {
        Some(0)
    } else if is_subsequence(q, &name) {
        Some(1)
    } else {
        None
    }
}

/// Whether every char of `needle` appears in `hay`, in order (not necessarily
/// contiguous).
fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut chars = hay.chars();
    needle.chars().all(|c| chars.any(|h| h == c))
}

/// Suggested completion suffix for `text`, or `None` if nothing completes it.
/// Slash input completes against the command list; everything else against the
/// most recent matching `history` entry. When several commands share the prefix
/// (e.g. `/co` → `/copy`, `/codex`), the **shortest** one is ghosted — it's the
/// nearest completion, and a longer sibling is reached by typing one more char.
pub(crate) fn suggest(text: &str, history: &[String]) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    if text.starts_with('/') {
        return COMMANDS
            .iter()
            .map(|c| c.name)
            .filter(|name| name.starts_with(text) && *name != text)
            .min_by_key(|name| name.len())
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
/// or `None`. Delegates to [`crate::pathcomplete`] (directories only).
pub(crate) fn dir_suggest(text: &str, base: &Path) -> Option<String> {
    let arg = text.strip_prefix("cd ")?;
    crate::pathcomplete::complete_path(arg, base, false)
}

#[cfg(test)]
#[path = "suggest_tests.rs"]
mod tests;
