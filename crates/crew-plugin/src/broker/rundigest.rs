//! A cut build's errors, every one by where it is, at the top, and as many
//! as fit quoted by the line of code each points at.
//!
//! WHY: [`super::runfit`] keeps a long run's tail, so a build with fifty
//! errors reached the agent as its last two. On the 2026-10-22/23 probes the
//! compile-error task's `cargo test` ran 54 times, cut every time (312 lines
//! gone), and the agent fixed the two it could see each round and ran it
//! again; no run passed. The whole output is saved to a file the result names,
//! and no agent read it. One line naming every error's place lets one round
//! fix them all, now that a round's edits all run. The places alone sent the
//! agent to read each file first (34 of 57 lookup-only rounds were into files
//! a run had just named) or to retype the line from memory (32 edits): the
//! line rustc quoted, with what it said of it, can be copied as `old`, as
//! Aider hands a lint's errors back with their code.

/// Bytes the line may take before it says how many more it left out.
const MAX: usize = 400;

/// Bytes the quoted lines may take, after the line of places: the 25 of the
/// compile task's build, out of [`super::toolclip::RUN_FIT`]'s 5,600 (a build
/// that does not compile has no stdout head, and its tail is these errors).
const QUOTED: usize = 2_600;

/// One error: where, rustc's word on it, and the line of code it points at.
struct Error<'a> {
    path: &'a str,
    line: &'a str,
    why: &'a str,
    code: Option<&'a str>,
    label: Option<&'a str>,
}

/// `N errors at path:line,line · path:line …`, from every error a compiler
/// wrote in `stdout` and `stderr` (rustc's `error…` then ` --> path:line:col`;
/// a note's or help's arrow is not an error's), then each one rustc quoted,
/// `line│ code — what it said` under its path. Empty when there are fewer than two,
/// which the tail shows anyway.
pub(super) fn digest(stdout: &str, stderr: &str) -> String {
    let errors = read(stderr.lines().chain(stdout.lines()));
    if errors.len() < 2 {
        return String::new();
    }
    let mut files: Vec<(&str, Vec<&str>)> = Vec::new();
    for e in &errors {
        match files.iter_mut().find(|(p, _)| *p == e.path) {
            Some((_, lines)) => lines.push(e.line),
            None => files.push((e.path, vec![e.line])),
        }
    }
    let mut out = format!("{} errors at", errors.len());
    let mut left = 0;
    for (k, (path, lines)) in files.iter().enumerate() {
        let one = format!(
            "{} {path}:{}",
            if k == 0 { "" } else { " \u{b7}" },
            lines.join(",")
        );
        if out.len() + one.len() > MAX {
            left = files.len() - k;
            break;
        }
        out.push_str(&one);
    }
    if left > 0 {
        out.push_str(&format!(
            " \u{b7} and {left} more file{}",
            if left == 1 { "" } else { "s" }
        ));
    }
    out + "\n" + &quoted(&errors)
}

/// The errors rustc quoted a line for, under their file, one each, as far
/// as [`QUOTED`] goes.
fn quoted(errors: &[Error]) -> String {
    let (mut out, mut file, mut left) = (String::new(), "", 0);
    for e in errors {
        let Some(code) = e.code else { continue };
        let code = super::route::clip(code.trim(), 100);
        let why = super::route::clip(e.label.unwrap_or(e.why), 80);
        let head = if e.path == file {
            String::new()
        } else {
            format!("{}\n", e.path)
        };
        let row = format!("{head}  {}\u{2502} {code} \u{2014} {why}\n", e.line);
        if left > 0 || out.len() + row.len() > QUOTED {
            left += 1;
            continue;
        }
        (out, file) = (out + &row, e.path);
    }
    match left {
        0 => out,
        n => out + &format!("({n} more not quoted)\n"),
    }
}

/// Every error in a compiler's output, read the way rustc writes one: its
/// `error…: why`, its ` --> path:line:col`, the code row numbered `line`,
/// and the `^^^ label` under it.
fn read<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<Error<'a>> {
    let (mut errors, mut head): (Vec<Error>, Option<&str>) = (Vec::new(), None);
    let mut open = false;
    for line in lines {
        let t = line.trim_start();
        if t.starts_with("error") || t.starts_with("warning") || t.starts_with("note") {
            let why = t.split_once(": ").map_or(t, |(_, w)| w);
            (head, open) = (t.starts_with("error").then_some(why), false);
            continue;
        }
        if let Some(at) = t.strip_prefix("--> ") {
            let mut parts = at.trim().rsplitn(3, ':');
            let (_col, n, path) = (parts.next(), parts.next(), parts.next());
            open = false;
            if let (Some(why), Some(line), Some(path)) = (head.take(), n, path) {
                let (code, label) = (None, None);
                errors.push(Error {
                    path,
                    line,
                    why,
                    code,
                    label,
                });
                open = true;
            }
            continue;
        }
        let Some(e) = errors.last_mut().filter(|_| open) else {
            continue;
        };
        match t.split_once('|') {
            Some((n, code)) if n.trim() == e.line && e.code.is_none() => {
                e.code = Some(code.strip_prefix(' ').unwrap_or(code));
            }
            Some((n, marks)) if n.trim().is_empty() && e.code.is_some() && e.label.is_none() => {
                let said = marks.rsplit('^').next().filter(|_| marks.contains('^'));
                e.label = said.map(str::trim).filter(|s| !s.is_empty());
            }
            _ => {}
        }
    }
    errors
}

/// Whether a `sys:run` result is a command that ran and failed: its first
/// line is `exit N` with N not 0 (-1 is a signal). `sysrun` returns such a
/// run as `Ok`, because the output IS the answer; this is what keeps the
/// card from reading ✓ and the provider from being told `is_error: false`
/// about a build that did not build — the way Codex reports a non-zero exit.
/// A timeout is already an `Err`.
pub(crate) fn failed(text: &str) -> bool {
    text.lines()
        .next()
        .and_then(|l| l.strip_prefix("exit "))
        .and_then(|n| n.trim().parse::<i32>().ok())
        .is_some_and(|n| n != 0)
}

#[cfg(test)]
#[path = "rundigest_tests.rs"]
mod tests;
