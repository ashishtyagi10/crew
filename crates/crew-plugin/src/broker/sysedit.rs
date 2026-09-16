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

/// One replacement to make.
pub(crate) struct Swap<'a> {
    pub old: &'a str,
    pub new: &'a str,
}

/// Replace the one occurrence of `old` in `path` with `new`.
pub(crate) fn edit(path: &str, old: &str, new: &str) -> Result<String, String> {
    edit_all(path, &[Swap { old, new }])
}

/// Make every `swaps` replacement in `path`, or none of them.
///
/// WHY a batch at all: a change touching five places in one file was five tool
/// calls, each with its own round trip and its own chance for the model to
/// lose track of what it had already done. Claude Code grew `MultiEdit` for
/// exactly this.
///
/// WHY all-or-nothing: a partial edit is the worst outcome available. Half a
/// rename does not compile, and the model sent to fix it is working from a
/// file matching neither what it read nor what it meant to write. So every
/// swap is applied to a BUFFER and only a complete set reaches the disk.
///
/// Applied in order, each against the result of the last: that is what lets
/// one swap rewrite a line a later swap then matches.
pub(crate) fn edit_all(path: &str, swaps: &[Swap]) -> Result<String, String> {
    if swaps.is_empty() {
        return Err("edit: no edits \u{2014} pass {\"old\": …, \"new\": …}".into());
    }
    for (i, s) in swaps.iter().enumerate() {
        let why = if s.old.is_empty() {
            "`old` is empty \u{2014} to create a file use sys:write_file"
        } else if s.old == s.new {
            "`old` and `new` are identical \u{2014} nothing to do"
        } else {
            continue;
        };
        return Err(format!("edit: {}", numbered(swaps.len(), i, why)));
    }
    let body =
        std::fs::read_to_string(path).map_err(|e| super::syspath::with_hint("edit", path, e))?;
    let mut edited = body.clone();
    for (i, s) in swaps.iter().enumerate() {
        edited = replace(&edited, s.old, s.new)
            .map_err(|e| format!("edit {path}: {}", numbered(swaps.len(), i, &e)))?;
    }
    std::fs::write(path, &edited).map_err(|e| format!("edit {path}: {e}"))?;
    Ok(report_all(path, &body, swaps))
}

/// A failure, saying WHICH edit failed and that the file is untouched — but
/// only when there was more than one, since "edit 1 of 1" is noise.
fn numbered(total: usize, i: usize, why: &str) -> String {
    match total {
        1 => why.to_string(),
        n => format!("edit {} of {n}: {why} \u{2014} nothing was written", i + 1),
    }
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
fn report_all(path: &str, before: &str, swaps: &[Swap]) -> String {
    let [one] = swaps else {
        let lines: isize = swaps
            .iter()
            .map(|s| s.new.lines().count() as isize - s.old.lines().count() as isize)
            .sum();
        let delta = match lines {
            0 => String::new(),
            d if d > 0 => format!(", +{d} lines"),
            d => format!(", {d} lines"),
        };
        return format!("edited {} in {} places{delta}", name(path), swaps.len());
    };
    report(path, before, one.old, one.new)
}

fn report(path: &str, before: &str, old: &str, new: &str) -> String {
    // Newlines BEFORE the match, not lines: a prefix ending in `\n` has
    // finished its last line, and the match starts on the one after it.
    let at = before[..before.find(old).unwrap_or(0)]
        .matches('\n')
        .count()
        + 1;
    let name = name(path);
    let (was, now) = (old.lines().count(), new.lines().count());
    let delta = match now as isize - was as isize {
        0 => format!("{was} line{}", plural(was)),
        d if d > 0 => format!("{was} line{} \u{2192} {now}, +{d}", plural(was)),
        d => format!("{was} line{} \u{2192} {now}, {d}", plural(was)),
    };
    format!("edited {name} at line {at} ({delta})")
}

/// The file's own name: the path is the caller's, the name is the reader's.
fn name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .map_or(path, |n| n.to_str().unwrap_or(path))
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
