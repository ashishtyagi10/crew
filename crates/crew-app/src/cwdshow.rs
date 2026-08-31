//! How a working directory READS: `~`-abbreviated for the input bar's legend,
//! clipped from the left so the tail survives, and with Windows' verbatim
//! `\\?\` prefix unwrapped.
//!
//! Split from [`crate::cwd`] for the line cap, along the line between what a
//! path IS — resolving, canonicalising, deciding whether it is a place to
//! work — and what it looks like once shown.
#[cfg(test)]
#[path = "cwdshow_tests.rs"]
mod tests;

use std::path::{Path, PathBuf};

/// Strip Windows' extended-length `\\?\` prefix from a canonicalized path.
///
/// `std::fs::canonicalize` returns *verbatim* paths on Windows —
/// `C:\Users\me\code` comes back as `\\?\C:\Users\me\code`. Crew canonicalizes
/// both the saved start directory and every `cd` target, so that form ended up
/// in the input-bar legend, in each pane's spawn directory, and in the
/// PowerShell prompt: four extra characters of noise on a path that was
/// already being shown in full, because the verbatim prefix also stops
/// [`abbreviate`] from ever recognising home.
///
/// Only the plain-drive form is unwrapped. `\\?\UNC\server\share` is left
/// exactly as it is: that prefix is load-bearing for network paths, and
/// rewriting it is how you break someone's mapped drive.
pub(crate) fn strip_verbatim(path: &str) -> &str {
    let Some(rest) = path.strip_prefix(r"\\?\") else {
        return path;
    };
    let mut chars = rest.chars();
    let is_drive = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.next() == Some(':')
        && chars.next() == Some('\\');
    if is_drive {
        rest
    } else {
        path
    }
}

/// [`strip_verbatim`] applied to an owned path.
pub(crate) fn simplified(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    match strip_verbatim(&s) {
        stripped if stripped.len() == s.len() => path.clone(),
        stripped => PathBuf::from(stripped),
    }
}

/// `~`-abbreviated display string for `path`, e.g. `~/code/crew`.
///
/// Windows needed two fixes here, and together they are why the legend read as
/// a "long name": the home directory came from `$HOME`, which Windows does not
/// set, and the separator was hardcoded to `/`. So `C:\Users\me\code` never
/// abbreviated and the bar showed the whole absolute path.
pub(crate) fn display(path: &Path) -> String {
    let s = path.to_string_lossy();
    match dirs::home_dir() {
        Some(home) => abbreviate(&s, &home.to_string_lossy(), std::path::MAIN_SEPARATOR),
        None => s.into_owned(),
    }
}

/// The abbreviation itself, with home and the separator passed in.
///
/// Parameterised so the **Windows** behaviour is testable from any machine.
/// A test that reads `MAIN_SEPARATOR` proves nothing about Windows when it
/// runs on macOS, where that constant is already `/` — the hardcoded `/` this
/// replaces would have passed such a test every time while leaving every
/// Windows path unabbreviated.
pub(crate) fn abbreviate(path: &str, home: &str, sep: char) -> String {
    if home.is_empty() {
        return path.to_string();
    }
    if path == home {
        return "~".to_string();
    }
    // Trim a trailing separator first: a home of `C:\` would otherwise be
    // matched as the prefix `C:\\`, which no path carries.
    let base = home.trim_end_matches(sep);
    match path.strip_prefix(&format!("{base}{sep}")) {
        Some(rest) => format!("~{sep}{rest}"),
        None => path.to_string(),
    }
}

/// Truncate `s` from the left to at most `max` columns, keeping the tail (the
/// most specific path component) behind a leading `…`. Used so a deep cwd legend
/// shows the current directory rather than being clipped at the root.
pub(crate) fn fit_legend(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max || max == 0 {
        return s.to_string();
    }
    let tail: String = s.chars().skip(n - max.saturating_sub(1)).collect();
    format!("…{tail}")
}
