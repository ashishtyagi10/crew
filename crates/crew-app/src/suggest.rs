//! Type-ahead suggestions for the input bar: slash-command completion,
//! `cd` directory completion, and fish-style history autosuggestion. Returns the
//! ghost *suffix* to display after the typed text (and to insert on accept).
//! The palette's command table lives in `cmddefs`.
use std::path::Path;

pub(crate) use crate::cmddefs::{Cmd, COMMANDS};

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
        // Rotation modes lead the list (the two `random-*` first), then the
        // fixed themes. The bare `random` alias is intentionally omitted — it
        // still parses, but random-dark is the canonical name.
        "/theme" => Some(
            vec![
                (
                    "random-dark".to_string(),
                    "rotates dark themes every 10 min".to_string(),
                ),
                (
                    "random-light".to_string(),
                    "rotates light themes every 10 min".to_string(),
                ),
                (
                    "auto".to_string(),
                    "light by day, dark by night — follows the OS".to_string(),
                ),
            ]
            .into_iter()
            .chain(
                crew_theme::ALL_THEMES
                    .iter()
                    .map(|t| (t.as_str().to_string(), t.describe().to_string())),
            )
            .collect(),
        ),
        "/crt" => Some(vec![
            ("on".to_string(), "force the CRT tube look on".to_string()),
            ("off".to_string(), "force the CRT tube look off".to_string()),
            (
                "auto".to_string(),
                "follow the theme (on for crt-* themes)".to_string(),
            ),
        ]),
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
