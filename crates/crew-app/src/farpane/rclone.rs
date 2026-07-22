//! `rclone` command construction and `lsjson` parsing for remote Far panels.
//! Everything here is pure — argv vectors and JSON→`Entry` mapping — so it is
//! unit-tested without a network or an installed `rclone`. The worker that
//! actually runs these argv lives alongside in `run` (Task 3).
use std::sync::mpsc::{self, Receiver};

use serde::Deserialize;

use super::list;
use super::location::Location;
use super::Entry;

pub(crate) fn argv_lsjson(loc: &Location) -> Vec<String> {
    vec!["lsjson".into(), loc.rclone_addr()]
}

pub(crate) fn argv_listremotes() -> Vec<String> {
    vec!["listremotes".into()]
}

pub(crate) fn argv_mkdir(loc: &Location) -> Vec<String> {
    vec!["mkdir".into(), loc.rclone_addr()]
}

/// `deletefile` for a single file (Drive trash on by default); `purge` removes
/// a directory and its contents.
pub(crate) fn argv_delete(loc: &Location, is_dir: bool) -> Vec<String> {
    let verb = if is_dir { "purge" } else { "deletefile" };
    vec![verb.into(), loc.rclone_addr()]
}

pub(crate) fn argv_copy(src: &Location, dst: &Location, is_dir: bool) -> Vec<String> {
    let verb = if is_dir { "copy" } else { "copyto" };
    vec![verb.into(), src.rclone_addr(), dst.rclone_addr()]
}

pub(crate) fn argv_move(src: &Location, dst: &Location, is_dir: bool) -> Vec<String> {
    let verb = if is_dir { "move" } else { "moveto" };
    vec![verb.into(), src.rclone_addr(), dst.rclone_addr()]
}

/// One row of `rclone lsjson` output (only the fields we use).
#[derive(Deserialize)]
struct LsRow {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Size")]
    size: i64,
    #[serde(rename = "IsDir")]
    is_dir: bool,
}

/// Parse `rclone lsjson` output for `loc` into a sorted `Entry` list, with the
/// synthetic `..` row prepended when `loc` has a parent.
pub(crate) fn parse_lsjson(json: &str, loc: &Location) -> Result<Vec<Entry>, String> {
    let rows: Vec<LsRow> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if loc.has_parent() {
        out.push(Entry {
            name: "..".into(),
            is_dir: true,
            is_parent: true,
            size: 0,
        });
    }
    let mut items: Vec<Entry> = rows
        .into_iter()
        .map(|r| Entry {
            name: r.name,
            is_dir: r.is_dir,
            is_parent: false,
            // Drive dirs report Size -1; normalise to 0 like the local reader.
            size: if r.is_dir || r.size < 0 {
                0
            } else {
                r.size as u64
            },
        })
        .collect();
    list::sort_entries(&mut items);
    out.extend(items);
    Ok(out)
}

/// What a finished `rclone` run reports: exit code (None = failed to spawn or
/// killed), full stdout, and the last non-empty stderr line for status.
pub(crate) struct RcloneDone {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr_tail: String,
}

/// Run `rclone <argv...>` on a worker thread.
pub(crate) fn run(argv: Vec<String>) -> Receiver<RcloneDone> {
    run_with("rclone", argv)
}

/// Run `program <argv...>` on a worker thread; the result arrives on the
/// returned channel. Split from `run` so tests can drive a stub binary.
pub(crate) fn run_with(program: &str, argv: Vec<String>) -> Receiver<RcloneDone> {
    let (tx, rx) = mpsc::channel();
    let program = program.to_string();
    std::thread::spawn(move || {
        let done = match std::process::Command::new(&program).args(&argv).output() {
            Ok(out) => RcloneDone {
                code: out.status.code(),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr_tail: tail_line(&out.stderr).unwrap_or_default(),
            },
            Err(e) => RcloneDone {
                code: None,
                stdout: String::new(),
                stderr_tail: format!("failed to start rclone: {e}"),
            },
        };
        let _ = tx.send(done);
    });
    rx
}

fn tail_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

/// Whether the `rclone` binary resolves on `$PATH`.
pub(crate) fn available() -> bool {
    std::process::Command::new("rclone")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "rclone_tests.rs"]
mod tests;
