//! `sys:edit` — changing part of a file without rewriting the whole of it.
//!
//! WHY: until now the only way an agent could change a file was
//! `sys:write_file`, which takes THE WHOLE FILE and overwrites it. To fix one
//! line of a 200-line source file the model had to reproduce 199 lines it had
//! no opinion about — slow, expensive, and the single most common way a model
//! silently deletes code, because a line it forgets to re-emit is simply gone.
//! Every agentic tool worth copying has this one (Claude Code's `Edit`,
//! Codex's `apply_patch`, Cline's `replace_in_file`) for exactly that reason.
//!
//! The contract is an EXACT, UNIQUE match, and both halves are load-bearing:
//!
//! - Exact, because a matcher that forgives whitespace is a matcher that
//!   sometimes edits a line you did not mean. The model can always read the
//!   file first; it cannot always tell which of two near-misses it hit.
//! - Unique, because `old` appearing twice means the edit has no single
//!   answer. Editing the first is a guess, and a guess that silently succeeds
//!   is worse than a failure that says what to do instead.
//!
//! So a miss is never a write. It is an error the model can act on, which is
//! why the errors here go to the trouble of saying WHY the match failed —
//! a near-miss on indentation is a different next move from no match at all.
use std::path::Path;

/// Replace the one occurrence of `old` in `path` with `new`.
pub(crate) fn edit(path: &str, old: &str, new: &str) -> Result<String, String> {
    if old.is_empty() {
        return Err("edit: `old` is empty \u{2014} to create a file use sys:write_file".into());
    }
    if old == new {
        return Err("edit: `old` and `new` are identical \u{2014} nothing to do".into());
    }
    let body = std::fs::read_to_string(path).map_err(|e| format!("edit {path}: {e}"))?;
    let edited = replace(&body, old, new).map_err(|e| format!("edit {path}: {e}"))?;
    std::fs::write(path, &edited).map_err(|e| format!("edit {path}: {e}"))?;
    Ok(report(path, &body, old, new))
}

/// The file with its one occurrence of `old` replaced, or why it could not be.
pub(crate) fn replace(body: &str, old: &str, new: &str) -> Result<String, String> {
    match body.matches(old).count() {
        1 => Ok(body.replacen(old, new, 1)),
        0 => Err(miss(body, old)),
        n => Err(format!(
            "`old` appears {n} times, so there is no single place to put `new` \u{2014} \
             include more of the surrounding lines to name just one"
        )),
    }
}

/// Why a match failed, in the terms the model's next move depends on.
fn miss(body: &str, old: &str) -> String {
    let flat = |s: &str| s.split_whitespace().collect::<String>();
    if !flat(old).is_empty() && body.lines().count() > 0 && flat(body).contains(&flat(old)) {
        return "`old` is not in the file, but the same text is there with different \
                whitespace \u{2014} read the file and copy the indentation exactly"
            .into();
    }
    if let Some(line) = old.lines().find(|l| !l.trim().is_empty()) {
        if body.contains(line.trim()) {
            return format!(
                "`old` is not in the file, though \u{201c}{}\u{201d} is \u{2014} the rest of \
                 `old` does not match what follows it",
                clip(line.trim())
            );
        }
    }
    "`old` is not in the file \u{2014} read it with sys:read_file and copy the text to replace"
        .into()
}

/// What changed, said in the units a reader checks: which line, and whether
/// the file grew or shrank. A byte count alone never told anyone whether the
/// right thing happened.
fn report(path: &str, before: &str, old: &str, new: &str) -> String {
    // Newlines BEFORE the match, not lines: a prefix ending in `\n` has
    // finished its last line, and the match starts on the one after it.
    let at = before[..before.find(old).unwrap_or(0)]
        .matches('\n')
        .count()
        + 1;
    let name = Path::new(path)
        .file_name()
        .map_or(path, |n| n.to_str().unwrap_or(path));
    let (was, now) = (old.lines().count(), new.lines().count());
    let delta = match now as isize - was as isize {
        0 => format!("{was} line{}", plural(was)),
        d if d > 0 => format!("{was} line{} \u{2192} {now}, +{d}", plural(was)),
        d => format!("{was} line{} \u{2192} {now}, {d}", plural(was)),
    };
    format!("edited {name} at line {at} ({delta})")
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// A fragment short enough to sit inside an error message.
fn clip(s: &str) -> String {
    const CAP: usize = 48;
    if s.chars().count() <= CAP {
        return s.into();
    }
    format!("{}\u{2026}", s.chars().take(CAP).collect::<String>())
}

#[cfg(test)]
#[path = "sysedit_tests.rs"]
mod tests;
