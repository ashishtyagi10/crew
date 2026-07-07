//! Type-ahead suggestions for the input bar: slash-command completion,
//! `cd` directory completion, and fish-style history autosuggestion. Returns the
//! ghost *suffix* to display after the typed text (and to insert on accept).
use std::path::Path;

/// A slash command shown in the command palette.
pub(crate) struct Cmd {
    pub name: &'static str,
    pub desc: &'static str,
}

/// One row in the input-bar palette: either a slash command, or a predefined
/// **value** for a command that offers a fixed set (e.g. `/theme` → the theme
/// names). Picking a value from the list beats remembering and typing it — the
/// "choose from a list" pattern, reusable by any closed-set command.
pub(crate) struct MenuItem {
    /// Text shown in the row (command name, or value).
    pub label: String,
    /// Dim hint after the label.
    pub desc: String,
    /// Input text set when this row is accepted with Tab (or run on Enter when
    /// `submit`).
    pub fill: String,
    /// Enter **runs** `fill` when true; when false Enter just inserts `fill` and
    /// keeps the palette open — a command expanding into its value picker.
    pub submit: bool,
}

/// The predefined `(value, description)` choices a command offers, or `None` for
/// a freeform / no-value command. **The single extension point** for the value
/// picker: give a command a closed set of values here and it gains an inline
/// picker for free (its rows run on Enter; unknown text still submits freeform).
pub(crate) fn options_for(cmd: &str) -> Option<Vec<(String, String)>> {
    match cmd {
        "/theme" => Some(
            crew_theme::ALL_THEMES
                .iter()
                .map(|t| (t.as_str().to_string(), t.describe().to_string()))
                .chain(std::iter::once((
                    "random".to_string(),
                    "rotates every 10 min".to_string(),
                )))
                .collect(),
        ),
        _ => None,
    }
}

/// The palette rows for the current input. Once a value-picker command has been
/// typed with a trailing space (`/theme …`), its value options are shown
/// (filtered by any partial value); otherwise the matching command names are
/// shown, and a value-picker command expands into its picker rather than running.
pub(crate) fn menu_items(text: &str) -> Vec<MenuItem> {
    if !text.starts_with('/') {
        return Vec::new();
    }
    if let Some(sp) = text.find(' ') {
        let cmd = &text[..sp];
        let arg = text[sp + 1..].trim_start().to_lowercase();
        let Some(opts) = options_for(cmd) else {
            return Vec::new(); // freeform arg (e.g. /run cargo …) → no picker
        };
        return opts
            .into_iter()
            .filter(|(v, _)| v.to_lowercase().starts_with(&arg))
            .map(|(v, desc)| MenuItem {
                fill: format!("{cmd} {v}"),
                label: v,
                desc,
                submit: true,
            })
            .collect();
    }
    matches(text)
        .into_iter()
        .map(|c| {
            let expands = options_for(c.name).is_some();
            MenuItem {
                label: c.name.to_string(),
                desc: c.desc.to_string(),
                fill: if expands {
                    format!("{} ", c.name)
                } else {
                    c.name.to_string()
                },
                submit: !expands,
            }
        })
        .collect()
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
        name: "/run",
        desc: "Run a command in a new pane (e.g. /run claude, /run codex)",
    },
    Cmd {
        name: "/diff",
        desc: "Review the working tree's git diff in a new pane",
    },
    Cmd {
        name: "/find",
        desc: "Search scrollback, highlighting matches (/find <text>)",
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
        name: "/clearall",
        desc: "Clear every pane's scrollback",
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
        name: "/closeall",
        desc: "Close every pane",
    },
    Cmd {
        name: "/pwd",
        desc: "Copy the working directory to the clipboard",
    },
    Cmd {
        name: "/about",
        desc: "Show the Crew version",
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
        name: "/font",
        desc: "Set the font size (/font <n>)",
    },
    Cmd {
        name: "/restart",
        desc: "Restart Crew — relaunch detached, applying an installed /update",
    },
    Cmd {
        name: "/theme",
        desc: "Switch theme — pick from the list",
    },
    Cmd {
        name: "/notify",
        desc: "Notification settings (/notify [on|off|add <text>|clear])",
    },
    Cmd {
        name: "/update",
        desc: "Update Crew to the latest release (left-nav progress; /restart applies)",
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
        name: "/goal",
        desc: "Plan a goal into a task graph and run it as a swarm (/goal <text>)",
    },
    Cmd {
        name: "/batch",
        desc: "Run a file of jobs (one per line) as a parallel swarm (/batch <file>)",
    },
    Cmd {
        name: "/md",
        desc: "view a markdown file (source | preview)",
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
pub(crate) fn is_subsequence(needle: &str, hay: &str) -> bool {
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
        // A value-picker command past its space ("/theme cr") ghosts the first
        // matching value's remainder, so Tab completes it like a command name.
        if let Some(sp) = text.find(' ') {
            let (cmd, arg) = (&text[..sp], &text[sp + 1..]);
            return options_for(cmd)?
                .into_iter()
                .map(|(v, _)| v)
                .find(|v| v.starts_with(arg) && v != arg)
                .map(|v| v[arg.len()..].to_string());
        }
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
