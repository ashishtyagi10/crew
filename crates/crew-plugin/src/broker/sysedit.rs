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
use super::editpick::Shift;

/// One replacement to make.
pub(crate) struct Swap<'a> {
    pub old: &'a str,
    pub new: &'a str,
    /// Every occurrence (a rename), as Claude Code's `replace_all`.
    pub all: bool,
    /// A line inside the one to change, when `old` is in the file more than
    /// once, numbered as the file read before the call (`super::editpick`).
    pub line: Option<usize>,
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
    let mut landed = super::editshow::Landed::default();
    let (mut shift, mut first) = (Shift::default(), Vec::new());
    for (i, s) in swaps.iter().enumerate() {
        let ats = place(&edited, s, &shift)
            .map_err(|e| format!("edit {path}: {}", numbered(swaps.len(), i, &e)))?;
        landed.swap(&edited, ats[0], s.old, s.new);
        shift.note(&edited, &ats, s.old, s.new);
        edited = splice(&edited, &ats, s.old, s.new);
        if i == 0 {
            first = ats;
        }
    }
    std::fs::write(path, &edited).map_err(|e| format!("edit {path}: {e}"))?;
    // The report line, then the edited lines as they now read, so the edit
    // can be checked without spending a round on sys:read_file.
    let report = super::editreport::report_all(path, &body, swaps, &first);
    match landed.show(&edited) {
        shown if shown.is_empty() => Ok(report),
        shown => Ok(format!("{report}\n{shown}")),
    }
}

/// A failure, saying WHICH edit failed and that the file is untouched — but
/// only when there was more than one, since "edit 1 of 1" is noise.
fn numbered(total: usize, i: usize, why: &str) -> String {
    match total {
        1 => why.to_string(),
        n => format!("edit {} of {n}: {why} \u{2014} nothing was written", i + 1),
    }
}

/// Where in `body` the swap goes: its one `old`, the one its `line` names,
/// or every one when `all` — byte offsets, never empty — or why nowhere.
pub(super) fn place(body: &str, s: &Swap, shift: &Shift) -> Result<Vec<usize>, String> {
    let ats: Vec<usize> = body.match_indices(s.old).map(|(at, _)| at).collect();
    match (ats.len(), s.line) {
        (0, _) => Err(miss(body, s.old)),
        (1, _) => Ok(ats),
        _ if s.all => Ok(ats),
        (_, Some(line)) => {
            super::editpick::choose(body, s.old, &ats, shift.now(line)).map(|at| vec![at])
        }
        _ => Err(super::editpick::ambiguous(body, &ats)),
    }
}

/// `body` with `new` in place of the `old` at each of `ats`.
fn splice(body: &str, ats: &[usize], old: &str, new: &str) -> String {
    let mut out = String::with_capacity(body.len() + new.len() * ats.len());
    let mut from = 0;
    for &at in ats {
        out.push_str(&body[from..at]);
        out.push_str(new);
        from = at + old.len();
    }
    out + &body[from..]
}

/// Why a match failed, in the terms the model's next move depends on.
fn miss(body: &str, old: &str) -> String {
    if super::editrows::carries_numbers(old) {
        return "`old` carries the line numbers from a read or edit result (`41\u{2502} `) \
                \u{2014} they are not in the file; leave them out and copy only the text"
            .into();
    }
    let flat = |s: &str| s.split_whitespace().collect::<String>();
    if !flat(old).is_empty() && body.lines().count() > 0 && flat(body).contains(&flat(old)) {
        let lines = super::editnear::spaced(body, old).unwrap_or_default();
        return format!(
            "`old` is not in the file, but the same text is there with different \
             whitespace \u{2014} copy it as it is, indentation and all (without the \
             numbers):\n{lines}"
        );
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
    match super::editnear::nearest(body, old) {
        Some(rows) => format!(
            "`old` is not in the file \u{2014} the nearest is line {}; copy its text \
             (without the number) or read the file:\n{rows}",
            rows.split('\u{2502}').next().unwrap_or_default().trim()
        ),
        None => "`old` is not in the file \u{2014} read it with sys:read_file and copy the \
                 text to replace"
            .into(),
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
