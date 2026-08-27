//! Type-ahead suggestions for the input bar: slash-command completion,
//! `cd` directory completion, and fish-style history autosuggestion. Returns the
//! ghost *suffix* to display after the typed text (and to insert on accept).
//! The palette's command table lives in `cmddefs`.
use std::path::Path;

pub(crate) use crate::cmddefs::{Cmd, COMMANDS};
use crate::suggestvalues::{expands, options_for};

/// One row in the input-bar palette: either a slash command, or a predefined
/// **value** for a command that offers a fixed set (e.g. `/theme` → the theme
/// names). Picking a value from the list beats remembering and typing it — the
/// "choose from a list" pattern, reusable by any closed-set command.
#[derive(Default)]
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
    /// A section title, not a choice: never selected, drawn dim without the
    /// selection marker. The picker groups rows by provider with these.
    pub header: bool,
    /// A choice the active stack can't currently serve (`Route::unserveable`)
    /// — renders muted like a header, but keeps its desc column and its
    /// selectability, unlike one. Always `false` outside the model picker.
    pub dim: bool,
    /// The provider key this row is blocked on (`Route::needs_key`). Accepting
    /// such a row prompts for the key instead of choosing a model that cannot
    /// run. `None` everywhere outside the model picker.
    pub needs: Option<String>,
    /// Character indices of `label` that the query matched, marked in the row
    /// so a fuzzy hit explains itself. Empty when nothing matched (an empty
    /// query, or a row that is not a search result).
    pub hit: Vec<usize>,
    /// Colours this row stands for, drawn between the label and the
    /// description ([`crate::swatch`]). Empty for a value that is not a
    /// colour, which is most of them.
    pub swatch: Vec<crate::swatch::Chip>,
    /// The chord that runs this command, shown right-aligned
    /// ([`crate::cmdkeys`]). `None` for rows that are not commands.
    pub key: Option<&'static str>,
    /// Optional label tint. The command palette leaves it `None` (accent);
    /// `/todo`'s tag popup colors each row in its project's own color
    /// (`crew_theme::tag_color`), matching the chips it completes into.
    pub color: Option<(u8, u8, u8)>,
}

/// The first row a selection may land on — headers are titles, not choices.
/// Falls back to 0 when every row is a header (nothing to select).
pub(crate) fn first_selectable(items: &[MenuItem]) -> usize {
    items.iter().position(|i| !i.header).unwrap_or(0)
}

/// Move the selection one row down (`down`) or up, wrapping, skipping header
/// rows. Returns `sel` unchanged when no row is selectable.
pub(crate) fn step_sel(items: &[MenuItem], sel: usize, down: bool) -> usize {
    if items.is_empty() || items.iter().all(|i| i.header) {
        return sel;
    }
    let n = items.len();
    let mut i = sel;
    for _ in 0..n {
        i = if down { (i + 1) % n } else { (i + n - 1) % n };
        if !items[i].header {
            return i;
        }
    }
    sel
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
        let items = opts
            .into_iter()
            .filter(|(v, _)| v.is_empty() || v.to_lowercase().starts_with(&arg))
            .map(|(v, desc)| match v.is_empty() {
                // An empty value is a HEADING rather than a choice: never
                // selected, no swatch, nothing to fill in.
                true => MenuItem {
                    label: desc,
                    header: true,
                    ..Default::default()
                },
                false => MenuItem {
                    fill: format!("{cmd} {v}"),
                    swatch: crate::swatch::for_value(cmd, &v),
                    label: v,
                    desc,
                    submit: true,
                    ..Default::default()
                },
            })
            .collect::<Vec<_>>();
        // A heading with nothing under it is a lie about where you are in the
        // list — the same rule the `/keys` overlay's filter follows.
        let mut out: Vec<MenuItem> = Vec::with_capacity(items.len());
        for item in items {
            if out.last().is_some_and(|prev: &MenuItem| prev.header) && item.header {
                out.pop();
            }
            out.push(item);
        }
        if out.last().is_some_and(|i| i.header) {
            out.pop();
        }
        return out;
    }
    matches(text)
        .into_iter()
        .map(|c| {
            let exp = expands(c.name);
            MenuItem {
                label: c.name.to_string(),
                desc: c.desc.to_string(),
                fill: if exp {
                    format!("{} ", c.name)
                } else {
                    c.name.to_string()
                },
                submit: !exp,
                hit: hit_positions(c.name, &text[1..].to_lowercase()),
                key: crate::cmdkeys::key_for(c.name),
                ..Default::default()
            }
        })
        .collect()
}

/// Commands matching `text` for the palette: a prefix match ranks first, then a
/// fuzzy subsequence match (so `/dmp` still finds `/dump`). Empty unless `text`
/// begins with `/`.
///
/// Ties are broken by what the user has actually run (see
/// [`crate::cmdrecents`]) and only then by the order they are declared in —
/// which means something to whoever last edited `cmddefs` and nothing at all
/// to the person typing. Recency reorders **within** a match-quality band and
/// never across one: a prefix match still beats a fuzzy match, always, so
/// `/de` can never float something that does not begin with `de` above
/// something that does. A learned list that can reorder the *kind* of match is
/// a list you can no longer aim at.
pub(crate) fn matches(text: &str) -> Vec<&'static Cmd> {
    matches_with(text, &crate::cmdrecents::now())
}

/// [`matches`] against an explicit recents list — the seam the tests use, so
/// they never have to reach for the process-wide one.
pub(crate) fn matches_with(text: &str, recents: &[String]) -> Vec<&'static Cmd> {
    if !text.starts_with('/') {
        return Vec::new();
    }
    let q = text[1..].to_lowercase();
    let mut scored: Vec<(u8, usize, usize, &'static Cmd)> = COMMANDS
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            rank(&c.name[1..], &q).map(|r| (r, crate::cmdrecents::rank_of(recents, c.name), i, c))
        })
        .collect();
    scored.sort_by_key(|(r, recent, i, _)| (*r, *recent, *i));
    scored.into_iter().map(|(_, _, _, c)| c).collect()
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

/// Which characters of `label` the query `q` matched, as character indices
/// into `label` — a prefix run for a prefix match, the greedy leftmost
/// subsequence otherwise. The palette marks these, which is what makes a fuzzy
/// hit (`/dmp` finding `/dump`) explain itself instead of looking like a bug.
///
/// The slash is index 0 of the label and is never part of `q`, so the search
/// starts past it.
pub(crate) fn hit_positions(label: &str, q: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut chars = label.chars().enumerate().skip(1);
    for want in q.chars() {
        loop {
            match chars.next() {
                Some((i, c)) if c.eq_ignore_ascii_case(&want) => {
                    out.push(i);
                    break;
                }
                Some(_) => continue,
                None => return out,
            }
        }
    }
    out
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

/// The palette command closest to `typo`, when one is close enough to be
/// worth naming. Longest shared prefix, at least three characters — enough to
/// catch `/setings` and `/reslore` without inventing a suggestion for
/// something the user never meant.
pub(crate) fn closest_command(typo: &str) -> Option<&'static str> {
    let typo = typo.split_whitespace().next().unwrap_or("").to_lowercase();
    let shared = |a: &str, b: &str| a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    crate::cmddefs::COMMANDS
        .iter()
        .map(|c| (c.name, shared(&typo, c.name.trim_start_matches('/'))))
        .filter(|(_, n)| *n >= 3)
        .max_by_key(|(_, n)| *n)
        .map(|(name, _)| name)
}

#[cfg(test)]
mod closest_tests {
    #[test]
    fn near_misses_get_a_suggestion_and_nonsense_does_not() {
        assert_eq!(super::closest_command("setings"), Some("/settings"));
        assert_eq!(super::closest_command("clearal"), Some("/clearall"));
        // Two characters is not enough to guess from.
        assert_eq!(super::closest_command("zx"), None);
        assert_eq!(super::closest_command("wobble"), None);
        // An argument must not confuse the match.
        assert_eq!(super::closest_command("thme dark"), None);
    }
}
