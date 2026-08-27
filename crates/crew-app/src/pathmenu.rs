//! The command bar's file picker: rows for `/view `, `/md `, `/dump ` and
//! `/batch `.
//!
//! Every command with a closed set of values already opens a picker — type
//! `/theme ` and the palettes are listed, with the current one marked. The
//! commands whose argument is a *path* got nothing: a ghosted completion of
//! the single alphabetically-first match, and no way to see what else was
//! there. Which is the wrong way round, because a path is the argument you
//! are least likely to be able to type from memory.
//!
//! One `read_dir` of the directory the partial names — not a walk. That is
//! the same read [`crate::pathcomplete`] already does on every keystroke, and
//! the reason it is bounded matters more here than usual: this runs on the
//! winit thread, where a stall freezes every pane in the grid.
//!
//! Directories do not submit. Accepting one fills `<cmd> dir/` and leaves the
//! bar open so the next read lists what is inside it, which is how you walk
//! into a tree with the same key you use to pick out of it.
use std::path::Path;

use crate::suggest::MenuItem;

/// Most rows one listing offers. A directory of ten thousand generated files
/// is a real thing to point crew at, and the menu is a list you read.
const MAX_ROWS: usize = 200;

/// Picker rows for `text` (a `<path-command> <partial>` line) resolved
/// against `base`, or `None` when `text` is not one of those commands.
///
/// Hidden entries appear only once the partial says so — a leading `.` —
/// which is the rule every shell's completion follows, and the reason a
/// listing of a home directory is readable at all.
pub(crate) fn rows(text: &str, base: &Path) -> Option<Vec<MenuItem>> {
    let cmd = crate::pathcomplete::PATH_COMMANDS
        .iter()
        .find(|c| text.strip_prefix(**c).is_some_and(|r| r.starts_with(' ')))?;
    let arg = text[cmd.len() + 1..].trim_start();
    let (dir_part, leaf) = match arg.rfind('/') {
        Some(i) => (&arg[..=i], &arg[i + 1..]),
        None => ("", arg),
    };
    let dir = crate::pathcomplete::expand(dir_part, base);
    let want_hidden = leaf.starts_with('.');
    let lower = leaf.to_lowercase();
    let mut found: Vec<(bool, String)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') && !want_hidden {
                return None;
            }
            if !name.to_lowercase().starts_with(&lower) {
                return None;
            }
            Some((e.path().is_dir(), name))
        })
        .take(MAX_ROWS)
        .collect();
    // Directories first, then by name: you are usually navigating before you
    // are choosing, and a folder buried among its own files is a folder you
    // scroll past.
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Some(
        found
            .into_iter()
            .map(|(is_dir, name)| {
                let path = format!("{dir_part}{name}{}", if is_dir { "/" } else { "" });
                MenuItem {
                    fill: format!("{cmd} {path}"),
                    label: path,
                    desc: if is_dir {
                        "folder".into()
                    } else {
                        String::new()
                    },
                    // A folder is a step, not an answer: filling it and
                    // leaving the bar open lists what is inside on the very
                    // next read.
                    submit: !is_dir,
                    ..Default::default()
                }
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "pathmenu_tests.rs"]
mod tests;
