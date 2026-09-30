//! `sys:git` — the repository's history and state, read without a shell.
//!
//! "What changed in route.rs lately?", "who wrote this line?", "what is
//! uncommitted?" are what an agent asks git most, and the only way to ask was
//! `sys:run`: the one Irreversible built-in, so every `git log` waited on an
//! approval, was refused outright in read-only mode, and came back as a
//! shell's output. This runs `git` itself — no `sh`, no pager, no colour — for
//! five subcommands that only look, which is what lets `tier::sys_tier` call
//! it a read; `sysgitargs` is what keeps that true.
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crew_hive::childproc::no_console_window;

use super::runfit;
use super::toolclip::RUN_FIT;

/// A `log` over a large repository answers in a second or two; twenty is for
/// a cold `blame` of a long-lived file, and still well inside the hop.
const TIMEOUT: Duration = Duration::from_secs(20);

/// Bytes of stdout read. The agent is shown ~5,600 of them; a `log -p` over
/// all of history must not be held in memory to say how much was left out.
const READ_CAP: usize = 1 << 20;

/// `sys:git {"cmd", "args"?}` in the broker's working directory.
pub(crate) fn git(v: &serde_json::Value) -> Result<String, String> {
    git_in(Path::new("."), v)
}

/// [`git`] in `dir`: checked, run, and fitted to what the agent is shown. A
/// non-zero exit is an `Err` carrying git's stderr — a `log` of a revision
/// that does not exist is a failed call, not an empty answer.
pub(super) fn git_in(dir: &Path, v: &serde_json::Value) -> Result<String, String> {
    let (sub, argv) = super::sysgitargs::checked(v)?;
    let (out, cut) = run(dir, &argv)?;
    Ok(fitted(sub, &out, cut))
}

/// What the agent is shown. Status, log and blame keep their START — the
/// branch line, the newest commits, the top of the file; the end of a long log
/// is only older history. A diff or a commit is whole when it fits, else its
/// start and end as `runfit` keeps a long command's: the start is the `--stat`
/// header, which names every file, and the end a whole last hunk. Either way a
/// cut answer is saved whole first (`spill`), and its last line says where.
fn fitted(sub: &str, out: &str, cut: bool) -> String {
    if out.trim().is_empty() {
        return match sub {
            "diff" => "no unstaged changes \u{2014} args [\"--cached\"] shows staged ones, \
                       [\"HEAD\"] both"
                .into(),
            _ => format!("git {sub} printed nothing"),
        };
    }
    let spill = super::spill::saved("git", out);
    let mut text = match sub {
        "diff" | "show" => runfit::ends(out, RUN_FIT / 2, spill.room),
        _ => head(out, spill.room),
    };
    if cut {
        text.push_str("\n\u{2026} (git wrote over 1 MB; the rest was not read)");
    }
    spill.close(text)
}

/// `out`'s start within `room`, and how many lines were left.
fn head(out: &str, room: usize) -> String {
    if out.len() <= RUN_FIT {
        return out.to_string();
    }
    let kept = runfit::head_of(out, room - 200);
    let n = out[kept.len()..].lines().count();
    let nl = if kept.ends_with('\n') { "" } else { "\n" };
    format!("{kept}{nl}\u{2026} ({n} more lines \u{2014} ask for fewer: -n, a path after \"--\", or -L)")
}

/// Run git in `dir` under [`TIMEOUT`]: stdout and whether [`READ_CAP`] cut it,
/// or the start of stderr when git failed. No optional locks: `status` would
/// otherwise refresh the index and write it back, and contend with the user's
/// own git while doing it.
fn run(dir: &Path, argv: &[String]) -> Result<(String, bool), String> {
    let mut cmd = Command::new("git");
    no_console_window(&mut cmd);
    cmd.args(argv)
        .current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0");
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("git: {e}"))?;
    let (out, err) = (
        drain(child.stdout.take(), READ_CAP),
        drain(child.stderr.take(), 8_192),
    );
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let secs = TIMEOUT.as_secs();
                return Err(format!(
                    "git ran past {secs}s and was stopped \u{2014} narrow it: -n, a path, -L"
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(format!("git: {e}")),
        }
    };
    // Git is the pipes' only writer, so they close as it exits; the bound is
    // for a helper it started that kept one open.
    let grace = Duration::from_millis(500);
    let (stdout, cut) = out.recv_timeout(grace).unwrap_or_default();
    if status.success() {
        return Ok((stdout, cut));
    }
    let (stderr, _) = err.recv_timeout(grace).unwrap_or_default();
    match runfit::head_of(stderr.trim(), 1_200) {
        "" => Err(format!("git exited {}", status.code().unwrap_or(-1))),
        said => Err(said.trim_end().to_string()),
    }
}

/// Read `pipe` to its end on a thread — so git never blocks on a full one —
/// keeping the first `cap` bytes, and send them back with whether any were
/// dropped. Sent, not joined, so a pipe that never closes cannot hold the call.
pub(super) fn drain(
    pipe: Option<impl Read + Send + 'static>,
    cap: usize,
) -> mpsc::Receiver<(String, bool)> {
    let (tx, rx) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut kept = Vec::new();
            let _ = (&mut pipe).take(cap as u64).read_to_end(&mut kept);
            let rest = std::io::copy(&mut pipe, &mut std::io::sink()).unwrap_or(0);
            let _ = tx.send((String::from_utf8_lossy(&kept).into_owned(), rest > 0));
        });
    }
    rx
}

#[cfg(test)]
#[path = "sysgit_tests.rs"]
mod tests;
