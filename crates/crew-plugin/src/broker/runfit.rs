//! What a `sys:run` result looks like to the agent: fitted to what it is
//! shown, and read back as failed when the command failed.
//!
//! A build or a test run prints its progress first and its verdict last, and
//! a compiler writes to stderr, which comes after stdout in the result. Both
//! engines then keep the first ~6,000 chars of a tool result, so `cargo
//! build` reached the agent as `exit 101` and a screen of `Compiling` lines
//! (crew's own graph has 563 packages), and the `error[E0308]` that was the
//! point of running it was cut. Fitting here, before either clip, keeps the
//! exit line, the start of stdout, a line saying how much was left out, and
//! the END of each stream — the tail, which is what Claude Code's Bash tool
//! keeps of a long output.
//!
//! Only the unix `sysrun` calls [`fit`] (Windows has no `sys:run`; `sys:git`
//! uses [`ends`] everywhere), and the Windows job builds with `-D warnings`,
//! so the `sys:run` half is allowed to go unused there.
#![cfg_attr(not(unix), allow(dead_code))]

#[cfg(test)]
#[path = "runfit_tests.rs"]
mod tests;

// These run a real `sh`, which Windows does not have.
#[cfg(all(test, unix))]
#[path = "runflag_tests.rs"]
mod flag_tests;

use super::toolclip::RUN_FIT;

/// Bytes of stdout kept from its START: enough for what ran (`running 512
/// tests`, a script's banner) without spending the budget on progress.
const HEAD: usize = 1_200;

/// Bytes held back from [`RUN_FIT`] for the lines this file writes itself:
/// the exit line, `--- stderr ---`, two gap lines and the 64 KB note come to
/// about 210 at their longest.
const FRAME: usize = 300;

/// Said when a pipe gave more than `systools::CAP` and the rest was dropped
/// unread — a different loss from the fitting's, which is counted in lines.
const TRUNCATED: &str = "\n\u{2026} (output truncated at 64 KB)";

/// The result as `sys:run` has always written it, uncut.
pub(super) fn whole(code: i32, stdout: &str, stderr: &str, truncated: bool) -> String {
    let mut text = format!("exit {code}\n{stdout}");
    if !stderr.is_empty() {
        text.push_str(&format!("\n--- stderr ---\n{stderr}"));
    }
    if truncated {
        text.push_str(TRUNCATED);
    }
    text
}

/// [`whole`], or — when that is over [`RUN_FIT`] — the exit line, stdout's
/// first [`HEAD`] bytes, and the end of each stream, each gap marked with how
/// many lines it holds. Cuts fall on line ends. A result that already fits
/// comes back byte-identical, so a short command reads exactly as before.
pub(super) fn fit(code: i32, stdout: &str, stderr: &str, truncated: bool) -> String {
    let all = whole(code, stdout, stderr, truncated);
    if all.len() <= RUN_FIT {
        return all;
    }
    let head = head_of(stdout, HEAD);
    let rest = &stdout[head.len()..];
    let (out_room, err_room) = share(rest.len(), stderr.len(), RUN_FIT - FRAME - head.len());
    let mut text = format!("exit {code}\n{head}");
    push_end(&mut text, rest, out_room);
    if !stderr.is_empty() {
        text.push_str("\n--- stderr ---\n");
        push_end(&mut text, stderr, err_room);
    }
    if truncated {
        text.push_str(TRUNCATED);
    }
    text
}

/// Whether a `sys:run` result is a command that ran and failed: its first
/// line is `exit N` with N not 0 (-1 is a signal). `sysrun` returns such a
/// run as `Ok`, because the output IS the answer; this is what keeps the
/// card from reading ✓ and the provider from being told `is_error: false`
/// about a build that did not build — the way Codex reports a non-zero exit.
/// A timeout is already an `Err`.
pub(super) fn failed(text: &str) -> bool {
    text.lines()
        .next()
        .and_then(|l| l.strip_prefix("exit "))
        .and_then(|n| n.trim().parse::<i32>().ok())
        .is_some_and(|n| n != 0)
}

/// How the tail budget splits between the rest of stdout and stderr.
///
/// Stderr first, because that is where a compiler writes its errors; but
/// stdout keeps up to a third, because that is where a test runner writes
/// its failures and `test result: FAILED` — `cargo test` puts its compile
/// progress on stderr and its verdict on stdout. Room either one does not
/// need goes to the other.
fn share(out_need: usize, err_need: usize, room: usize) -> (usize, usize) {
    let out_floor = out_need.min(room / 3);
    let err = err_need.min(room - out_floor);
    (out_need.min(room - err), err)
}

/// `s`'s end within `room` bytes, after a line saying how much was left out
/// when anything was.
fn push_end(text: &mut String, s: &str, room: usize) {
    let end = tail_of(s, room);
    let cut = &s[..s.len() - end.len()];
    if !cut.is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let n = cut.lines().count();
        let s = if n == 1 { "" } else { "s" };
        text.push_str(&format!(
            "\u{2026} ({n} line{s} of output cut \u{2014} the end is below)\n"
        ));
    }
    text.push_str(end);
}

/// One stream fitted the way [`fit`] fits stdout, for a result with no exit
/// line or stderr of its own: whole within [`RUN_FIT`], else its first `head`
/// bytes and its end with the gap counted. `sys:git` keeps a long diff so —
/// its start is the `--stat` naming every file, its end a whole last hunk.
pub(super) fn ends(s: &str, head: usize) -> String {
    if s.len() <= RUN_FIT {
        return s.to_string();
    }
    let head = head_of(s, head.min(RUN_FIT - FRAME));
    let mut text = head.to_string();
    push_end(&mut text, &s[head.len()..], RUN_FIT - FRAME - head.len());
    text
}

/// The longest start of `s` within `max` bytes that ends a line — or, when
/// the first line alone is longer, as much of it as fits.
pub(super) fn head_of(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    match s.as_bytes()[..max].iter().rposition(|&b| b == b'\n') {
        Some(i) => &s[..=i],
        None => {
            &s[..(0..=max)
                .rev()
                .find(|&i| s.is_char_boundary(i))
                .unwrap_or(0)]
        }
    }
}

/// The longest end of `s` within `max` bytes that starts a line — or, when
/// the last line alone is longer, as much of it as fits.
fn tail_of(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let from = s.len() - max;
    // A line starts at `from` when the byte before it ends one.
    match s.as_bytes()[from - 1..].iter().position(|&b| b == b'\n') {
        Some(i) if from + i < s.len() => &s[from + i..],
        _ => {
            &s[(from..=s.len())
                .find(|&i| s.is_char_boundary(i))
                .unwrap_or(s.len())..]
        }
    }
}
