# Google Drive browsing in `/far` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a `/far` panel be re-rooted into Google Drive (via `rclone`) so the dual-pane file manager can list, open/edit, copy, move, delete, and mkdir against Drive exactly like the local disk, with the other panel still on local disk or another remote.

**Architecture:** A new `Location { backend: Backend, path: String }` replaces `Panel.cwd: PathBuf`; `Backend::Local` keeps today's synchronous `std::fs` code, `Backend::Rclone { remote }` shells out to `rclone` on worker threads and lands results through a generalized poll loop (the exact worker + `mpsc` + per-tick-poll pattern already used by `ask.rs`/`run.rs`). rclone owns Google OAuth, so crew writes no auth code.

**Tech Stack:** Rust, winit (single-threaded render loop — never block it), ratatui rendering, `rclone` CLI (external, JSON via `lsjson`), `serde`/`serde_json` (already a workspace dep).

## Global Constraints

- **Never block the winit thread.** Every `rclone` invocation runs on a spawned worker thread; results arrive over `std::sync::mpsc` and are drained in a per-tick poll. This mirrors `crates/crew-app/src/farpane/run.rs::start` and `ask.rs`.
- **rclone is the only Drive dependency.** No native OAuth. rclone-missing and no-remotes are handled as friendly status/overlay messages, never panics.
- **Local-only behavior must not regress.** `Backend::Local` panels keep the existing synchronous `std::fs` code paths and all existing `farpane` tests must stay green.
- **Drive trash by default.** Deletes use `rclone delete`/`purge` (rclone defaults `--drive-use-trash=true`), matching far's local delete-to-trash.
- **v1 scope for the bottom command line:** the Far command line (typed shell commands, Tab completion, `!` AI ask) stays **local-panel-only**. When the active panel is remote, the command line is inert (see Task 4). This is an explicit v1 limitation, not an omission.
- **Serde derive:** the workspace already depends on `serde` + `serde_json`; add them to `crates/crew-app/Cargo.toml` only if not already present (check first).

## File Structure

- Create `crates/crew-app/src/farpane/location.rs` — `Backend`, `Location`, address/nav helpers (pure).
- Create `crates/crew-app/src/farpane/location_tests.rs`.
- Create `crates/crew-app/src/farpane/rclone.rs` — argv builders, `lsjson` parsing, the generic worker runner.
- Create `crates/crew-app/src/farpane/rclone_tests.rs`.
- Create `crates/crew-app/src/farpane/remote.rs` — `PendingOp` state machine + `poll_ops`, drive-select overlay state, remote op dispatch.
- Create `crates/crew-app/src/farpane/remote_tests.rs`.
- Modify `crates/crew-app/src/farpane/mod.rs` — `Panel.loc`, generalized reload/poll, new fields, module decls.
- Modify `crates/crew-app/src/farpane/list.rs` — extract shared `sort_entries`.
- Modify `crates/crew-app/src/farpane/render.rs` — render `loc`, loading indicator, drive-select overlay.
- Modify `crates/crew-app/src/farpane/keys.rs` — Alt+F1/F2, remote-aware navigate/ops, overlay key handling.
- Modify `crates/crew-app/src/farpane/fileops.rs` — remote branch in copy/move/delete/mkdir.
- Modify `crates/crew-app/src/poll.rs` — call `poll_ops`, route its `FarAction`.
- Modify `crates/crew-app/src/keys.rs` (app) — no change expected beyond existing `FarAction` handling; verify Open path.
- Modify `crates/crew-app/src/sessionrestore.rs` + `sessionsave.rs` — persist/restore remote panel location (Task 12).

---

## Phase A — rclone backbone (pure, no UI, no network in tests)

### Task 1: `Location` / `Backend` types

**Files:**
- Create: `crates/crew-app/src/farpane/location.rs`
- Create: `crates/crew-app/src/farpane/location_tests.rs`
- Modify: `crates/crew-app/src/farpane/mod.rs` (add `mod location;` and re-exports)

**Interfaces:**
- Produces:
  - `enum Backend { Local, Rclone { remote: String } }` (derive `Clone, PartialEq, Eq`)
  - `struct Location { backend: Backend, path: String }` (derive `Clone, PartialEq, Eq`)
  - `Location::local(p: &std::path::Path) -> Location`
  - `Location::is_remote(&self) -> bool`
  - `Location::local_path(&self) -> Option<std::path::PathBuf>` — `Some` for Local, `None` for remote
  - `Location::rclone_addr(&self) -> String` — `"gdrive:sub/dir"` for remote; the plain path for local
  - `Location::display(&self) -> String` — what the panel legend shows
  - `Location::child(&self, name: &str) -> Location` — descend into `name`
  - `Location::parent(&self) -> Option<Location>` — ascend; `None` at a root
  - `Location::has_parent(&self) -> bool`

- [ ] **Step 1: Write the failing test**

Create `crates/crew-app/src/farpane/location_tests.rs`:

```rust
use super::location::{Backend, Location};
use std::path::Path;

#[test]
fn local_round_trips_a_path() {
    let loc = Location::local(Path::new("/home/x/proj"));
    assert!(!loc.is_remote());
    assert_eq!(loc.local_path().unwrap(), Path::new("/home/x/proj"));
    assert_eq!(loc.rclone_addr(), "/home/x/proj");
}

#[test]
fn remote_addr_is_remote_colon_path() {
    let root = Location {
        backend: Backend::Rclone { remote: "gdrive".into() },
        path: String::new(),
    };
    assert!(root.is_remote());
    assert_eq!(root.rclone_addr(), "gdrive:");
    assert!(root.local_path().is_none());
    let photos = root.child("Photos");
    assert_eq!(photos.rclone_addr(), "gdrive:Photos");
    assert_eq!(photos.child("2024").rclone_addr(), "gdrive:Photos/2024");
}

#[test]
fn remote_parent_ascends_and_stops_at_root() {
    let deep = Location {
        backend: Backend::Rclone { remote: "gdrive".into() },
        path: "Photos/2024".into(),
    };
    assert!(deep.has_parent());
    let up = deep.parent().unwrap();
    assert_eq!(up.rclone_addr(), "gdrive:Photos");
    let root = up.parent().unwrap();
    assert_eq!(root.rclone_addr(), "gdrive:");
    assert!(!root.has_parent());
    assert!(root.parent().is_none());
}

#[test]
fn local_parent_matches_path_parent() {
    let loc = Location::local(Path::new("/a/b"));
    assert!(loc.has_parent());
    assert_eq!(loc.parent().unwrap().local_path().unwrap(), Path::new("/a"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-app --lib farpane::location`
Expected: FAIL — `location` module does not exist / unresolved import.

- [ ] **Step 3: Write the implementation**

Create `crates/crew-app/src/farpane/location.rs`:

```rust
//! Where a Far panel is rooted: the local filesystem, or an `rclone` remote
//! (e.g. Google Drive). Local panels keep a real `PathBuf`; remote panels
//! carry the remote name plus a `/`-joined sub-path. `rclone_addr` produces
//! the `remote:sub/path` string every `rclone` subcommand takes.
use std::path::{Path, PathBuf};

/// The storage backend a panel is browsing.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Backend {
    Local,
    Rclone { remote: String },
}

/// A resolved location: a backend plus a path within it. For `Local` the path
/// is an absolute filesystem path; for `Rclone` it is the remote-relative
/// sub-path (empty at the remote root).
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Location {
    pub backend: Backend,
    pub path: String,
}

impl Location {
    pub(crate) fn local(p: &Path) -> Self {
        Self {
            backend: Backend::Local,
            path: p.to_string_lossy().into_owned(),
        }
    }

    pub(crate) fn is_remote(&self) -> bool {
        matches!(self.backend, Backend::Rclone { .. })
    }

    pub(crate) fn local_path(&self) -> Option<PathBuf> {
        match self.backend {
            Backend::Local => Some(PathBuf::from(&self.path)),
            Backend::Rclone { .. } => None,
        }
    }

    /// The `rclone`-addressable string: `remote:sub/path` for a remote, or the
    /// plain filesystem path for local.
    pub(crate) fn rclone_addr(&self) -> String {
        match &self.backend {
            Backend::Local => self.path.clone(),
            Backend::Rclone { remote } => format!("{remote}:{}", self.path),
        }
    }

    /// What the panel legend shows.
    pub(crate) fn display(&self) -> String {
        match &self.backend {
            Backend::Local => self.path.clone(),
            Backend::Rclone { .. } => self.rclone_addr(),
        }
    }

    /// Descend into `name`.
    pub(crate) fn child(&self, name: &str) -> Self {
        match &self.backend {
            Backend::Local => Self::local(&PathBuf::from(&self.path).join(name)),
            Backend::Rclone { remote } => {
                let path = if self.path.is_empty() {
                    name.to_string()
                } else {
                    format!("{}/{name}", self.path)
                };
                Self {
                    backend: Backend::Rclone { remote: remote.clone() },
                    path,
                }
            }
        }
    }

    pub(crate) fn has_parent(&self) -> bool {
        match &self.backend {
            Backend::Local => PathBuf::from(&self.path).parent().is_some(),
            Backend::Rclone { .. } => !self.path.is_empty(),
        }
    }

    /// Ascend one level; `None` at a root (filesystem root or remote root).
    pub(crate) fn parent(&self) -> Option<Self> {
        match &self.backend {
            Backend::Local => PathBuf::from(&self.path)
                .parent()
                .map(|p| Self::local(p)),
            Backend::Rclone { remote } => {
                if self.path.is_empty() {
                    return None;
                }
                let parent = match self.path.rsplit_once('/') {
                    Some((head, _)) => head.to_string(),
                    None => String::new(),
                };
                Some(Self {
                    backend: Backend::Rclone { remote: remote.clone() },
                    path: parent,
                })
            }
        }
    }
}
```

In `crates/crew-app/src/farpane/mod.rs`, add to the module list (near line 17-25):

```rust
mod location;
```

And add the test-module hook at the bottom of `location.rs`:

```rust
#[cfg(test)]
#[path = "location_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app --lib farpane::location`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/farpane/location.rs crates/crew-app/src/farpane/location_tests.rs crates/crew-app/src/farpane/mod.rs
git commit -m "feat(far): Location/Backend types for local + rclone panels"
```

---

### Task 2: Shared entry sort + `rclone` argv builders + `lsjson` parsing

**Files:**
- Create: `crates/crew-app/src/farpane/rclone.rs`
- Create: `crates/crew-app/src/farpane/rclone_tests.rs`
- Modify: `crates/crew-app/src/farpane/list.rs` (extract `sort_entries`)
- Modify: `crates/crew-app/src/farpane/mod.rs` (add `mod rclone;`)

**Interfaces:**
- Consumes: `Entry` (`mod.rs`), `Location` (Task 1).
- Produces:
  - `list::sort_entries(items: &mut Vec<Entry>)` — the folders-first / size-desc / name sort, extracted so remote listings match local ordering.
  - `rclone::argv_lsjson(loc: &Location) -> Vec<String>`
  - `rclone::argv_listremotes() -> Vec<String>`
  - `rclone::argv_mkdir(loc: &Location) -> Vec<String>`
  - `rclone::argv_delete(loc: &Location, is_dir: bool) -> Vec<String>`
  - `rclone::argv_copy(src: &Location, dst: &Location, is_dir: bool) -> Vec<String>`
  - `rclone::argv_move(src: &Location, dst: &Location, is_dir: bool) -> Vec<String>`
  - `rclone::parse_lsjson(json: &str, loc: &Location) -> Result<Vec<Entry>, String>` — includes the leading `..` row when `loc.has_parent()`, sorted via `list::sort_entries`.

- [ ] **Step 1: Extract the shared sort (refactor, keep list tests green)**

In `crates/crew-app/src/farpane/list.rs`, replace the inline `items.sort_by(...)` block (lines ~39-44) with a call, and add the function:

```rust
/// Sort a listing folders-first, then files largest-first, name as tiebreak
/// (case-insensitive). Shared by the local reader and remote `lsjson`
/// parsing so both panels order identically.
pub(crate) fn sort_entries(items: &mut [Entry]) {
    items.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| b.size.cmp(&a.size))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}
```

Then in `read_dir`, replace the sort call with `sort_entries(&mut items);`.

Run: `cargo test -p crew-app --lib farpane::list`
Expected: PASS (existing 2 tests unchanged).

- [ ] **Step 2: Write the failing test**

Create `crates/crew-app/src/farpane/rclone_tests.rs`:

```rust
use super::location::{Backend, Location};
use super::rclone;

fn gdrive(path: &str) -> Location {
    Location {
        backend: Backend::Rclone { remote: "gdrive".into() },
        path: path.into(),
    }
}

#[test]
fn lsjson_argv_targets_the_address() {
    assert_eq!(
        rclone::argv_lsjson(&gdrive("Photos")),
        vec!["lsjson", "gdrive:Photos"]
    );
}

#[test]
fn mkdir_delete_move_copy_argv() {
    let a = gdrive("Photos/a.txt");
    let b = gdrive("Backup/a.txt");
    assert_eq!(rclone::argv_mkdir(&gdrive("New")), vec!["mkdir", "gdrive:New"]);
    // file delete uses `deletefile`; dir delete uses `purge`
    assert_eq!(rclone::argv_delete(&a, false), vec!["deletefile", "gdrive:Photos/a.txt"]);
    assert_eq!(rclone::argv_delete(&gdrive("Photos"), true), vec!["purge", "gdrive:Photos"]);
    // file copy/move use the *to variants; dirs use plain copy/move
    assert_eq!(rclone::argv_copy(&a, &b, false), vec!["copyto", "gdrive:Photos/a.txt", "gdrive:Backup/a.txt"]);
    assert_eq!(rclone::argv_move(&a, &b, false), vec!["moveto", "gdrive:Photos/a.txt", "gdrive:Backup/a.txt"]);
    let da = gdrive("Photos/sub");
    let db = gdrive("Backup/sub");
    assert_eq!(rclone::argv_copy(&da, &db, true), vec!["copy", "gdrive:Photos/sub", "gdrive:Backup/sub"]);
}

#[test]
fn parse_lsjson_maps_fields_and_sorts_with_parent_row() {
    // rclone lsjson emits an array of {Name, Size, IsDir, ...}
    let json = r#"[
        {"Name":"small.txt","Size":1,"IsDir":false},
        {"Name":"zdir","Size":-1,"IsDir":true},
        {"Name":"big.txt","Size":500,"IsDir":false},
        {"Name":"adir","Size":-1,"IsDir":true}
    ]"#;
    let loc = Location { backend: Backend::Rclone { remote: "gdrive".into() }, path: "Photos".into() };
    let entries = rclone::parse_lsjson(json, &loc).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["..", "adir", "zdir", "big.txt", "small.txt"]);
    assert!(entries[0].is_parent);
    assert_eq!(entries[3].size, 500);
    assert_eq!(entries[1].size, 0, "directories carry no size");
}

#[test]
fn parse_lsjson_at_root_has_no_parent_row() {
    let loc = Location { backend: Backend::Rclone { remote: "gdrive".into() }, path: String::new() };
    let entries = rclone::parse_lsjson("[]", &loc).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn parse_lsjson_rejects_garbage() {
    let loc = Location { backend: Backend::Rclone { remote: "gdrive".into() }, path: String::new() };
    assert!(rclone::parse_lsjson("not json", &loc).is_err());
}
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test -p crew-app --lib farpane::rclone`
Expected: FAIL — `rclone` module missing.

- [ ] **Step 4: Write the implementation**

Create `crates/crew-app/src/farpane/rclone.rs` (runner added in Task 3; this step is argv + parse only):

```rust
//! `rclone` command construction and `lsjson` parsing for remote Far panels.
//! Everything here is pure — argv vectors and JSON→`Entry` mapping — so it is
//! unit-tested without a network or an installed `rclone`. The worker that
//! actually runs these argv lives alongside in `run` (Task 3).
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
        out.push(Entry { name: "..".into(), is_dir: true, is_parent: true, size: 0 });
    }
    let mut items: Vec<Entry> = rows
        .into_iter()
        .map(|r| Entry {
            name: r.name,
            is_dir: r.is_dir,
            is_parent: false,
            // Drive dirs report Size -1; normalise to 0 like the local reader.
            size: if r.is_dir || r.size < 0 { 0 } else { r.size as u64 },
        })
        .collect();
    list::sort_entries(&mut items);
    out.extend(items);
    Ok(out)
}

#[cfg(test)]
#[path = "rclone_tests.rs"]
mod tests;
```

Add `mod rclone;` to `crates/crew-app/src/farpane/mod.rs`. Confirm `serde` (with `derive`) and `serde_json` are in `crates/crew-app/Cargo.toml`; add them if missing:

```bash
grep -E '^serde|^serde_json' crates/crew-app/Cargo.toml
# if absent, add under [dependencies]:
#   serde = { workspace = true, features = ["derive"] }
#   serde_json = { workspace = true }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p crew-app --lib farpane::rclone`
Expected: PASS (6 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/rclone.rs crates/crew-app/src/farpane/rclone_tests.rs crates/crew-app/src/farpane/list.rs crates/crew-app/src/farpane/mod.rs crates/crew-app/Cargo.toml
git commit -m "feat(far): rclone argv builders + lsjson parsing"
```

---

### Task 3: Generic `rclone` worker runner

**Files:**
- Modify: `crates/crew-app/src/farpane/rclone.rs` (add `run`, `RcloneDone`)
- Modify: `crates/crew-app/src/farpane/rclone_tests.rs` (add runner test)

**Interfaces:**
- Produces:
  - `struct RcloneDone { code: Option<i32>, stdout: String, stderr_tail: String }`
  - `rclone::run(argv: Vec<String>) -> std::sync::mpsc::Receiver<RcloneDone>` — spawns a worker that runs `rclone <argv...>`, captures stdout, and the last non-empty stderr line. Dropping the receiver discards the result. Modeled on `run.rs::start`.
  - `rclone::available() -> bool` — cheap check that the `rclone` binary resolves on `$PATH` (used by the drive-select overlay, Task 6).

- [ ] **Step 1: Write the failing test**

The runner spawns a program; to test without depending on rclone, add an internal seam: `run_with(program, argv)` and have `run` call it with `"rclone"`. Test `run_with` against `/bin/echo`.

Add to `crates/crew-app/src/farpane/rclone_tests.rs`:

```rust
use std::time::Duration;

#[test]
fn runner_captures_stdout_and_exit_code() {
    let rx = rclone::run_with("/bin/echo", vec!["hello".into()]);
    let done = rx.recv_timeout(Duration::from_secs(5)).expect("result");
    assert_eq!(done.code, Some(0));
    assert_eq!(done.stdout.trim(), "hello");
}

#[test]
fn runner_reports_stderr_tail_and_nonzero() {
    let rx = rclone::run_with("/bin/sh", vec!["-c".into(), "echo boom >&2; exit 2".into()]);
    let done = rx.recv_timeout(Duration::from_secs(5)).expect("result");
    assert_eq!(done.code, Some(2));
    assert_eq!(done.stderr_tail, "boom");
}

#[test]
fn runner_missing_binary_is_a_none_code() {
    let rx = rclone::run_with("/definitely/not/here", vec![]);
    let done = rx.recv_timeout(Duration::from_secs(5)).expect("result");
    assert_eq!(done.code, None);
    assert!(!done.stderr_tail.is_empty());
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-app --lib farpane::rclone::tests::runner`
Expected: FAIL — `run_with` not found.

- [ ] **Step 3: Write the implementation**

Add to `crates/crew-app/src/farpane/rclone.rs`:

```rust
use std::sync::mpsc::{self, Receiver};

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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app --lib farpane::rclone`
Expected: PASS (existing + 3 new runner tests).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/farpane/rclone.rs crates/crew-app/src/farpane/rclone_tests.rs
git commit -m "feat(far): off-thread rclone runner + availability probe"
```

---

## Phase B — Panel location plumbing (behavior-preserving refactor)

### Task 4: Replace `Panel.cwd: PathBuf` with `Panel.loc: Location`

This is the wide refactor. Goal: identical local behavior, all existing `farpane` + `app` tests green. No remote behavior yet — remote panels can be *constructed* but navigation/ops land in later tasks.

**Files:**
- Modify: `crates/crew-app/src/farpane/mod.rs`
- Modify: `crates/crew-app/src/farpane/render.rs`
- Modify: `crates/crew-app/src/farpane/keys.rs`
- Modify: `crates/crew-app/src/farpane/fileops.rs`
- Modify: `crates/crew-app/src/farpane/run.rs`
- Modify: `crates/crew-app/src/sessionrestore.rs`

**Interfaces:**
- Consumes: `Location` (Task 1).
- Produces:
  - `Panel.loc: Location` (replaces `cwd: PathBuf`).
  - `Panel::reload(&mut self)` stays synchronous for `Backend::Local`; for remote it is a **no-op stub** in this task (real async reload is Task 5) — document that.
  - `FarPane::active_loc(&self) -> Location` (replaces the internal uses of `active_cwd`). `active_cwd(&self) -> PathBuf` is kept but returns the local path or, for a remote panel, `std::env::temp_dir()` as an inert fallback (command line is local-only per Global Constraints).

- [ ] **Step 1: Change the `Panel` struct and constructor**

In `mod.rs`, replace `pub cwd: PathBuf` with:

```rust
pub loc: super::farpane::location::Location, // adjust path: `location::Location`
```

(Use `use location::Location;` at the top of `mod.rs`.) Update `Panel::new`:

```rust
fn new(cwd: PathBuf) -> Self {
    let loc = Location::local(&cwd);
    let entries = list::read_dir(&cwd);
    Self { loc, entries, sel: 0 }
}

/// Re-read the current location. Local reads synchronously; remote reload is
/// driven asynchronously via `remote.rs` (Task 5) and is a no-op here.
fn reload(&mut self) {
    if let Some(path) = self.loc.local_path() {
        self.entries = list::read_dir(&path);
    }
    self.sel = self.sel.min(self.entries.len().saturating_sub(1));
}
```

- [ ] **Step 2: Update every `.cwd` reader**

Search and update each site:

Run: `grep -rn "\.cwd" crates/crew-app/src/farpane/`

- `keys.rs::activate` — `panel.cwd.push(name)` becomes:
  ```rust
  let child = panel.loc.child(&name);
  panel.loc = child;
  panel.sel = 0;
  panel.reload();
  ```
  and the file `Open` branch `Some(FarAction::Open(panel.cwd.join(name)))` becomes, for now (local only; remote open is Task 10):
  ```rust
  match panel.loc.local_path() {
      Some(dir) => Some(FarAction::Open(dir.join(name))),
      None => None, // remote open handled in Task 10
  }
  ```
- `keys.rs::ascend` — replace `cwd.parent()` walk with:
  ```rust
  if let Some(parent) = panel.loc.parent() {
      panel.loc = parent;
      panel.sel = 0;
      panel.reload();
  }
  ```
- `keys.rs::open_selected` — same local-path guard as `activate`'s Open branch.
- `fileops.rs::transfer_paths`, `delete`, `make_dir` — these are local-only in this task. Guard each with `loc.local_path()`; when the active (or other) panel is remote, return `FarAction::Status("remote copy/move/delete lands in a later task".into())` as a temporary stub replaced in Tasks 7-9. (Keeps the refactor green without half-implementing remote ops.) Concretely, `transfer_paths` becomes:
  ```rust
  let src_dir = src_panel.loc.local_path()?;
  let dst_dir = p.panel(p.other_side()).loc.local_path()?;
  // ... src = src_dir.join(&name); dst = dst_dir.join(&name);
  ```
  (When either is remote, `?` returns `None` → the caller's "nothing to copy" path; acceptable interim behavior.)
- `run.rs::active_cwd`/`change_dir` and `mod.rs::active_cwd` — see Step 3.
- `render.rs` — `panel.cwd` in `legend(&panel.cwd, ...)` and `command_bar(&p.active_cwd(), ...)`. Change `legend` to take a `&str` display string: `legend(&panel.loc.display(), ...)` and adjust `legend`'s signature from `cwd: &std::path::Path` to `display: &str` (use `display` directly instead of `cwd.to_string_lossy()`; drop the `file_name` logic — show the full `display`). In `command_bar`, replace the `cwd: &std::path::Path` param with a `folder: &str` computed by the caller as `p.active_panel_folder()` (a small helper returning `loc.display()`'s last segment or the whole display).

- [ ] **Step 3: Update `active_cwd` and add `active_loc`**

In `mod.rs`:

```rust
/// The active panel's location.
pub(crate) fn active_loc(&self) -> Location {
    self.panel(self.active).loc.clone()
}

/// The active panel's directory as a local path — the working dir for the
/// bottom command line, which is LOCAL-ONLY in v1. A remote active panel
/// yields the temp dir as an inert fallback (the command line is disabled
/// for remote panels in `run.rs`).
pub(crate) fn active_cwd(&self) -> PathBuf {
    self.active_loc().local_path().unwrap_or_else(std::env::temp_dir)
}
```

In `run.rs::run_cmdline`, gate the command line on a local active panel:

```rust
if p.active_loc().is_remote() {
    return FarAction::Status("command line is local-only — switch this panel to local".into());
}
```

Place this near the top of `run_cmdline` (after taking `cmd`, before `cd`/exec). Likewise short-circuit `submit_ask` with the same check.

- [ ] **Step 4: Update sessionrestore**

`sessionrestore.rs:47` calls `f.active_cwd().to_string_lossy()`. Keep it working: this now returns temp_dir for remote panels, which is wrong to persist. For this task, guard it so **only local** far panes are saved with their real dir (remote persistence is Task 12):

```rust
PaneContent::Far(f) => {
    let loc = f.active_loc();
    loc.local_path().map(|p| SavedPane::far(p.to_string_lossy().into_owned()))
}
```

(If the surrounding code expects `Some(...)`, adjust to the `Option` it already handles — the shell arm two lines up already produces `Option`.)

- [ ] **Step 5: Build and run the full farpane + app test suites**

Run: `cargo test -p crew-app --lib farpane`
Run: `cargo test -p crew-app --lib app`
Expected: PASS — no behavioral change for local panels. Fix any compile error surfaced by the `.cwd` → `.loc` migration until green.

- [ ] **Step 6: Commit**

```bash
git add -A crates/crew-app/src/farpane crates/crew-app/src/sessionrestore.rs
git commit -m "refactor(far): panels carry a Location instead of a raw cwd"
```

---

## Phase C — remote listing + drive select

### Task 5: Async remote listing + generalized `poll_ops`

**Files:**
- Create: `crates/crew-app/src/farpane/remote.rs`
- Create: `crates/crew-app/src/farpane/remote_tests.rs`
- Modify: `crates/crew-app/src/farpane/mod.rs` (fields, `poll_ops`, `is_busy`)
- Modify: `crates/crew-app/src/farpane/keys.rs` (remote navigate kicks off a listing)
- Modify: `crates/crew-app/src/farpane/render.rs` (loading indicator)
- Modify: `crates/crew-app/src/poll.rs` (call `poll_ops`)

**Interfaces:**
- Produces:
  - `enum PendingKind { List { side: Side, loc: Location }, Mkdir, Delete, Transfer, Download { .. } }` (start with `List`; later tasks extend).
  - `struct PendingOp { kind: PendingKind, rx: Receiver<rclone::RcloneDone>, note: String }`
  - `FarPane.pending: Option<PendingOp>` — one in-flight remote op at a time (keep it simple; reject a second with a "busy" status).
  - `FarPane::begin_list(&mut self, side: Side)` — spawn `rclone lsjson` for that panel's `loc`.
  - `FarPane::poll_ops(&mut self) -> Option<FarAction>` — drain a finished op; for `List`, parse + install entries + clear loading; returns a `FarAction::Status` or, in later tasks, `FarAction::Open`.
  - `Panel.loading: bool` — true while a remote listing is in flight for that side.

- [ ] **Step 1: Write the failing test (parse-install path, no network)**

The network is untestable in CI, so test the *result-absorption* logic by injecting a synthetic `RcloneDone`. Add a seam `FarPane::absorb_list(side, loc, done)` that `poll_ops` calls, and test it directly.

Create `crates/crew-app/src/farpane/remote_tests.rs`:

```rust
use super::location::{Backend, Location};
use super::rclone::RcloneDone;
use super::{FarPane, Side};

fn remote_pane() -> FarPane {
    let mut f = FarPane::new(std::env::temp_dir());
    f.left.loc = Location { backend: Backend::Rclone { remote: "gdrive".into() }, path: String::new() };
    f.left.loading = true;
    f
}

#[test]
fn absorb_list_installs_sorted_entries() {
    let mut f = remote_pane();
    let loc = f.left.loc.clone();
    let done = RcloneDone {
        code: Some(0),
        stdout: r#"[{"Name":"b.txt","Size":2,"IsDir":false},{"Name":"adir","Size":-1,"IsDir":true}]"#.into(),
        stderr_tail: String::new(),
    };
    let status = f.absorb_list(Side::Left, loc, done);
    assert!(!f.left.loading);
    let names: Vec<&str> = f.left.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["adir", "b.txt"]); // remote root: no ".." row
    assert!(status.contains("gdrive:"));
}

#[test]
fn absorb_list_surfaces_rclone_error() {
    let mut f = remote_pane();
    let loc = f.left.loc.clone();
    let done = RcloneDone { code: Some(1), stdout: String::new(), stderr_tail: "auth failed".into() };
    let status = f.absorb_list(Side::Left, loc, done);
    assert!(!f.left.loading);
    assert!(status.contains("auth failed"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-app --lib farpane::remote`
Expected: FAIL — `remote` module / `loading` field / `absorb_list` missing.

- [ ] **Step 3: Implement `remote.rs` and wire fields**

Add to `Panel` (in `mod.rs`): `pub loading: bool,` (init `false` in `Panel::new`).
Add to `FarPane`: `pub(crate) pending: Option<remote::PendingOp>,` (init `None` in `FarPane::new`). Add `mod remote;`.

Create `crates/crew-app/src/farpane/remote.rs`:

```rust
//! Remote (rclone) operations for Far panels: spawn an `rclone` worker, track
//! the single in-flight op in `FarPane::pending`, and land its result in the
//! per-tick `poll_ops` — the same worker + mpsc + poll shape as `ask.rs`, so
//! the winit thread never blocks on the network.
use std::sync::mpsc::Receiver;

use super::keys::FarAction;
use super::location::Location;
use super::rclone::{self, RcloneDone};
use super::{FarPane, Side};

/// Which remote op a `PendingOp` represents (extended in Tasks 7-10).
pub(crate) enum PendingKind {
    List { side: Side, loc: Location },
}

pub(crate) struct PendingOp {
    pub kind: PendingKind,
    pub rx: Receiver<RcloneDone>,
    /// Short label shown while it runs (e.g. "listing gdrive:Photos").
    pub note: String,
}

impl FarPane {
    /// Kick off an `rclone lsjson` for `side`'s current remote location.
    pub(crate) fn begin_list(&mut self, side: Side) -> FarAction {
        if self.pending.is_some() {
            return FarAction::Status("rclone busy — wait for it".into());
        }
        let loc = self.panel(side).loc.clone();
        if !loc.is_remote() {
            return FarAction::Status("not a remote panel".into());
        }
        self.panel_mut(side).loading = true;
        let note = format!("listing {}", loc.rclone_addr());
        let rx = rclone::run(rclone::argv_lsjson(&loc));
        self.pending = Some(PendingOp { kind: PendingKind::List { side, loc }, rx, note: note.clone() });
        FarAction::Status(note)
    }

    /// Drain a finished remote op this tick, if any.
    pub fn poll_ops(&mut self) -> Option<FarAction> {
        let pending = self.pending.as_ref()?;
        let done = pending.rx.try_recv().ok()?;
        let kind = self.pending.take().map(|p| p.kind)?;
        match kind {
            PendingKind::List { side, loc } => Some(FarAction::Status(self.absorb_list(side, loc, done))),
        }
    }

    /// Install a finished listing (or surface its error). Split out for tests.
    pub(crate) fn absorb_list(&mut self, side: Side, loc: Location, done: RcloneDone) -> String {
        self.panel_mut(side).loading = false;
        if done.code != Some(0) {
            return format!("rclone: {}", if done.stderr_tail.is_empty() { "listing failed".into() } else { done.stderr_tail });
        }
        match rclone::parse_lsjson(&done.stdout, &loc) {
            Ok(entries) => {
                let panel = self.panel_mut(side);
                panel.entries = entries;
                panel.sel = 0;
                format!("{} — {} items", loc.rclone_addr(), self.panel(side).entries.len())
            }
            Err(e) => format!("rclone: bad listing: {e}"),
        }
    }

    /// Whether a remote op is in flight (feeds the busy sweep / spinner).
    pub(crate) fn ops_busy(&self) -> bool {
        self.pending.is_some()
    }
}
```

Extend `FarPane::is_busy` (mod.rs) to include `|| self.ops_busy()`.

- [ ] **Step 4: Trigger listings on remote navigation**

In `keys.rs`, `activate` and `ascend` currently call `panel.reload()`. For a remote panel, `reload()` is a no-op (Task 4), so after setting the new `loc`, call `begin_list`. Because `activate`/`ascend` borrow `panel` mutably, restructure to set `loc` + `sel`, drop the borrow, then:

```rust
let side = p.active;
if p.panel(side).loc.is_remote() {
    return Some(p.begin_list(side));
}
```

(For local panels the synchronous `reload()` already ran — keep that path unchanged.)

- [ ] **Step 5: Render the loading state + poll wiring**

In `render.rs::panel`, when `panel.loading` is true, render a single centered dim `listing…` line instead of (or above) the entry list; keep the bordered box + legend. Minimal version: if `panel.loading && panel.entries.is_empty()`, push one `ListItem` reading `⟳ listing…`.

In `crates/crew-app/src/poll.rs` (the `PaneContent::Far(f)` arm, ~line 91), add after the existing `poll_ask` block:

```rust
if let Some(action) = f.poll_ops() {
    if let crate::farpane::FarAction::Status(msg) = action {
        far_statuses.push(msg);
    } else {
        far_ops_actions.push(action); // Open handled in Task 10
    }
    changed = true;
}
```

For now `far_ops_actions` can be a local `Vec<FarAction>` drained right after the pane loop the same way `far_statuses` is turned into `set_status`; route non-Status variants through the same `match` used in the app `keys.rs` (`FarAction::Open` → `open::that`). If that routing helper doesn't exist yet, defer the Open arm to Task 10 and only handle Status here.

- [ ] **Step 6: Run tests**

Run: `cargo test -p crew-app --lib farpane`
Expected: PASS (existing + 2 new `remote` tests).

- [ ] **Step 7: Commit**

```bash
git add -A crates/crew-app/src/farpane crates/crew-app/src/poll.rs
git commit -m "feat(far): async remote listing via rclone lsjson + poll_ops"
```

---

### Task 6: Drive-select overlay (Alt+F1 / Alt+F2)

**Files:**
- Modify: `crates/crew-app/src/farpane/remote.rs` (overlay state + logic)
- Modify: `crates/crew-app/src/farpane/mod.rs` (overlay field)
- Modify: `crates/crew-app/src/farpane/keys.rs` (Alt+F1/F2, overlay key handling)
- Modify: `crates/crew-app/src/farpane/render.rs` (overlay rendering)
- Modify: `crates/crew-app/src/farpane/remote_tests.rs`

**Interfaces:**
- Produces:
  - `struct DriveSelect { side: Side, options: Vec<DriveOption>, sel: usize }`
  - `enum DriveOption { Local, Remote(String) }`
  - `FarPane.drive_select: Option<DriveSelect>`
  - `FarPane::open_drive_select(&mut self, side: Side) -> FarAction` — probes `rclone::available()`, spawns `listremotes`, and opens the overlay in a loading state; the result lands via a new `PendingKind::Remotes`.
  - `FarPane::choose_drive(&mut self) -> Option<FarAction>` — apply the highlighted option: re-root the panel (Local → its previous local dir or cwd; Remote → `remote:` root + `begin_list`).

- [ ] **Step 1: Write the failing test**

Add to `remote_tests.rs`:

```rust
use super::rclone::RcloneDone;

#[test]
fn absorb_remotes_populates_the_overlay() {
    let mut f = FarPane::new(std::env::temp_dir());
    f.drive_select = Some(super::remote::DriveSelect::loading(Side::Left));
    let done = RcloneDone { code: Some(0), stdout: "gdrive:\ndropbox:\n".into(), stderr_tail: String::new() };
    f.absorb_remotes(done);
    let ds = f.drive_select.as_ref().unwrap();
    // Local + two remotes
    assert_eq!(ds.options.len(), 3);
}

#[test]
fn choose_remote_reroots_and_lists() {
    let mut f = FarPane::new(std::env::temp_dir());
    f.drive_select = Some(super::remote::DriveSelect {
        side: Side::Left,
        options: vec![super::remote::DriveOption::Remote("gdrive".into())],
        sel: 0,
    });
    let _ = f.choose_drive();
    assert!(f.left.loc.is_remote());
    assert_eq!(f.left.loc.rclone_addr(), "gdrive:");
    assert!(f.pending.is_some(), "re-rooting kicks off a listing");
    assert!(f.drive_select.is_none(), "overlay closes on choose");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app --lib farpane::remote`
Expected: FAIL — overlay types/methods missing.

- [ ] **Step 3: Implement overlay state + logic**

In `remote.rs` add:

```rust
pub(crate) enum DriveOption {
    Local,
    Remote(String), // remote name without the trailing ':'
}

pub(crate) struct DriveSelect {
    pub side: Side,
    pub options: Vec<DriveOption>,
    pub sel: usize,
}

impl DriveSelect {
    /// The overlay shown while `listremotes` is still running.
    pub(crate) fn loading(side: Side) -> Self {
        Self { side, options: Vec::new(), sel: 0 }
    }
}
```

Extend `PendingKind` with `Remotes` and handle it in `poll_ops`:

```rust
PendingKind::Remotes => Some(FarAction::Status(self.absorb_remotes(done))),
```

Add methods:

```rust
impl FarPane {
    pub(crate) fn open_drive_select(&mut self, side: Side) -> FarAction {
        if !rclone::available() {
            return FarAction::Status("rclone not found — install it and run `rclone config`".into());
        }
        if self.pending.is_some() {
            return FarAction::Status("rclone busy — wait for it".into());
        }
        self.drive_select = Some(DriveSelect::loading(side));
        let rx = rclone::run(rclone::argv_listremotes());
        self.pending = Some(PendingOp { kind: PendingKind::Remotes, rx, note: "listing remotes".into() });
        FarAction::Status("choose a drive…".into())
    }

    pub(crate) fn absorb_remotes(&mut self, done: RcloneDone) -> String {
        let Some(ds) = self.drive_select.as_mut() else { return String::new() };
        if done.code != Some(0) {
            self.drive_select = None;
            return format!("rclone: {}", if done.stderr_tail.is_empty() { "listremotes failed".into() } else { done.stderr_tail });
        }
        let mut options = vec![DriveOption::Local];
        for line in done.stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
            options.push(DriveOption::Remote(line.trim_end_matches(':').to_string()));
        }
        ds.options = options;
        ds.sel = 0;
        "choose a drive — Enter to open, Esc to cancel".into()
    }

    pub(crate) fn choose_drive(&mut self) -> Option<FarAction> {
        let ds = self.drive_select.take()?;
        let option = ds.options.into_iter().nth(ds.sel)?;
        let side = ds.side;
        match option {
            DriveOption::Local => {
                // Re-root to the process cwd (or temp as a last resort).
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
                let panel = self.panel_mut(side);
                panel.loc = Location::local(&cwd);
                panel.sel = 0;
                panel.reload();
                Some(FarAction::Status(format!("local — {}", cwd.display())))
            }
            DriveOption::Remote(remote) => {
                let panel = self.panel_mut(side);
                panel.loc = Location { backend: super::location::Backend::Rclone { remote }, path: String::new() };
                panel.sel = 0;
                panel.entries.clear();
                Some(self.begin_list(side))
            }
        }
    }
}
```

Add `pub(crate) drive_select: Option<remote::DriveSelect>,` to `FarPane` (init `None`).

- [ ] **Step 4: Keys — open + navigate the overlay**

In `keys.rs::reduce`, before the main match, if `p.drive_select.is_some()` handle overlay keys (Up/Down move `sel`, Enter → `choose_drive`, Esc → `p.drive_select = None`) and return. Add Alt+F1/Alt+F2 detection. winit exposes modifiers on the event stream, not `KeyEvent`; the pane's `on_key` currently takes only `&KeyEvent`. Extend the signature to `on_key(&mut self, key: &KeyEvent, alt: bool)` and thread `alt` from the app `keys.rs` call site (`f.on_key(event, mstate.alt_key())` — verify the modifiers accessor name in `crate::keys`/`mstate`). Then:

```rust
if alt {
    match &key.logical_key {
        Key::Named(NamedKey::F1) => return Some(p.open_drive_select(Side::Left)),
        Key::Named(NamedKey::F2) => return Some(p.open_drive_select(Side::Right)),
        _ => {}
    }
}
```

- [ ] **Step 5: Render the overlay**

In `render.rs::render`, after panels, if `p.drive_select.is_some()` draw a small centered box listing `Local disk` + each remote, highlighting `sel`; show `listing remotes…` when `options` is empty. Model the box on the existing `Block::bordered()` usage.

- [ ] **Step 6: Run tests**

Run: `cargo test -p crew-app --lib farpane`
Expected: PASS (existing + 2 new overlay tests).

- [ ] **Step 7: Commit**

```bash
git add -A crates/crew-app/src/farpane
git commit -m "feat(far): FAR-style drive-select overlay (Alt+F1/F2) via rclone listremotes"
```

---

## Phase D — remote file operations

### Task 7: Remote delete (F8)

**Files:**
- Modify: `crates/crew-app/src/farpane/fileops.rs`, `remote.rs`, `remote_tests.rs`

**Interfaces:**
- Produces: `PendingKind::Simple { refresh: Side, verb: &'static str }` (a generic "run, then re-list the affected side" op reused by delete/mkdir); `FarPane::begin_simple(argv, refresh, verb, note)`.

- [ ] **Step 1: Failing test** — add to `remote_tests.rs`:

```rust
#[test]
fn absorb_simple_success_triggers_relist() {
    let mut f = remote_pane(); // left = gdrive root
    let status = f.absorb_simple(Side::Left, "deleted", RcloneDone { code: Some(0), stdout: String::new(), stderr_tail: String::new() });
    assert!(status.contains("deleted"));
    assert!(f.pending.is_some(), "a successful mutation re-lists the panel");
}

#[test]
fn absorb_simple_failure_surfaces_stderr_no_relist() {
    let mut f = remote_pane();
    let status = f.absorb_simple(Side::Left, "deleted", RcloneDone { code: Some(1), stdout: String::new(), stderr_tail: "permission denied".into() });
    assert!(status.contains("permission denied"));
    assert!(f.pending.is_none());
}
```

- [ ] **Step 2: Run — FAIL.** `cargo test -p crew-app --lib farpane::remote::tests::absorb_simple`

- [ ] **Step 3: Implement.** Add `Simple { refresh: Side, verb: &'static str }` to `PendingKind`, handle in `poll_ops` (`self.absorb_simple(refresh, verb, done)`), and:

```rust
pub(crate) fn begin_simple(&mut self, argv: Vec<String>, refresh: Side, verb: &'static str, note: String) -> FarAction {
    if self.pending.is_some() { return FarAction::Status("rclone busy — wait for it".into()); }
    let rx = rclone::run(argv);
    self.pending = Some(PendingOp { kind: PendingKind::Simple { refresh, verb }, rx, note: note.clone() });
    FarAction::Status(note)
}

pub(crate) fn absorb_simple(&mut self, refresh: Side, verb: &'static str, done: RcloneDone) -> String {
    if done.code != Some(0) {
        return format!("rclone: {} failed: {}", verb, if done.stderr_tail.is_empty() { "error".into() } else { done.stderr_tail });
    }
    // Re-list the affected panel to reflect the change.
    let _ = self.begin_list(refresh);
    format!("{verb} ✓")
}
```

In `fileops.rs::delete`, branch on remote at the top:

```rust
if p.panel(p.active).loc.is_remote() {
    let panel = p.panel(p.active);
    let Some(entry) = panel.entries.get(panel.sel) else { return FarAction::Status("nothing to delete".into()); };
    if entry.is_parent { return FarAction::Status("can't delete the ‘..’ entry".into()); }
    let target = panel.loc.child(&entry.name);
    let is_dir = entry.is_dir;
    let side = p.active;
    return p.begin_simple(super::rclone::argv_delete(&target, is_dir), side, "deleted", format!("deleting {}", target.rclone_addr()));
}
```

- [ ] **Step 4: Run — PASS.** `cargo test -p crew-app --lib farpane`
- [ ] **Step 5: Commit.** `git commit -am "feat(far): remote delete via rclone deletefile/purge"`

---

### Task 8: Remote mkdir (F7)

**Files:** `fileops.rs`, `remote_tests.rs`

- [ ] **Step 1: Failing test** — add:

```rust
#[test]
fn remote_mkdir_starts_a_simple_op() {
    let mut f = remote_pane(); // left active, remote
    f.active = Side::Left;
    let action = super::fileops::make_dir(&mut f, "New");
    assert!(matches!(action, super::keys::FarAction::Status(_)));
    assert!(f.pending.is_some());
}
```

- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement.** In `fileops.rs::make_dir`, branch on remote:

```rust
if p.panel(p.active).loc.is_remote() {
    let target = p.panel(p.active).loc.child(name);
    let side = p.active;
    return p.begin_simple(super::rclone::argv_mkdir(&target), side, "created folder", format!("mkdir {}", target.rclone_addr()));
}
```

- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit.** `git commit -am "feat(far): remote mkdir via rclone mkdir"`

---

### Task 9: Remote copy/move (F5 / F6)

**Files:** `fileops.rs`, `remote.rs`, `remote_tests.rs`

**Interfaces:**
- Produces: `PendingKind::Transfer { refresh_both: bool }`; `FarPane::begin_transfer(argv, verb, note)` that re-lists both panels on success (so a remote↔local transfer refreshes the destination even if it is the other side). For local↔local, keep the existing synchronous path untouched.

- [ ] **Step 1: Failing test** — verifies argv selection across combos and that a transfer with a remote endpoint goes async:

```rust
#[test]
fn copy_local_to_remote_is_async() {
    let mut f = FarPane::new(std::env::temp_dir()); // left local
    f.right.loc = Location { backend: Backend::Rclone { remote: "gdrive".into() }, path: String::new() };
    f.active = Side::Left;
    // put a fake selected file in the local panel
    f.left.entries = vec![super::Entry { name: "a.txt".into(), is_dir: false, is_parent: false, size: 1 }];
    f.left.sel = 0;
    let action = super::fileops::copy(&mut f);
    assert!(matches!(action, super::keys::FarAction::Status(_)));
    assert!(f.pending.is_some(), "a transfer touching a remote runs on rclone");
}
```

- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement.** At the top of `fileops.rs::copy` and `rename_move`, detect whether either endpoint is remote:

```rust
let src_panel = p.panel(p.active);
let Some(entry) = src_panel.entries.get(src_panel.sel) else { return FarAction::Status("nothing to copy".into()); };
if entry.is_parent { return FarAction::Status("can't copy the ‘..’ entry".into()); }
let name = entry.name.clone();
let is_dir = entry.is_dir;
let src = src_panel.loc.child(&name);
let dst = p.panel(p.other_side()).loc.child(&name);
if src.is_remote() || dst.is_remote() {
    let argv = super::rclone::argv_copy(&src, &dst, is_dir); // argv_move in rename_move
    let note = format!("copying {} → {}", src.rclone_addr(), dst.rclone_addr());
    return p.begin_transfer(argv, "copied", note);
}
// else: existing local std::fs path (unchanged)
```

Add `begin_transfer`/`Transfer` handling in `remote.rs`, absorbing like `absorb_simple` but re-listing **both** sides on success:

```rust
pub(crate) fn absorb_transfer(&mut self, verb: &'static str, done: RcloneDone) -> String {
    if done.code != Some(0) {
        return format!("rclone: {verb} failed: {}", if done.stderr_tail.is_empty() { "error".into() } else { done.stderr_tail });
    }
    // Re-list whichever sides are remote; local sides reload synchronously.
    for side in [Side::Left, Side::Right] {
        if self.panel(side).loc.is_remote() { let _ = self.begin_list(side); break; }
        else { self.panel_mut(side).reload(); }
    }
    format!("{verb} ✓")
}
```

(Note: only one remote list can be pending at a time; if both panels are remote, re-list the active side and leave the other stale until focused — acceptable v1.)

- [ ] **Step 4: Run — PASS.** `cargo test -p crew-app --lib farpane`
- [ ] **Step 5: Commit.** `git commit -am "feat(far): remote copy/move via rclone copyto/moveto (all local/remote combos)"`

---

## Phase E — remote open with auto-upload on save

### Task 10: Download-and-open a remote file (F3 / F4 / Enter)

**Files:** `keys.rs`, `remote.rs`, `mod.rs`, `poll.rs`, `remote_tests.rs`

**Interfaces:**
- Produces:
  - `PendingKind::Download { remote: Location, temp: PathBuf }`
  - `FarPane::begin_download(&mut self, entry_name: &str) -> FarAction` — computes the remote file `Location`, a temp dest `~/…/far-drive/<name>`, spawns `rclone copyto remote:file temp`.
  - `poll_ops` for `Download` returns `FarAction::Open(temp)` on success and **registers a watch** (Task 11) mapping `temp → remote`.

- [ ] **Step 1: Failing test** — the absorb path returns an Open and registers a watch:

```rust
#[test]
fn absorb_download_opens_temp_and_registers_watch() {
    let mut f = remote_pane();
    let remote = f.left.loc.child("notes.txt");
    let temp = std::env::temp_dir().join("far-drive-test-notes.txt");
    std::fs::write(&temp, b"hi").unwrap();
    let action = f.absorb_download(remote.clone(), temp.clone(), RcloneDone { code: Some(0), stdout: String::new(), stderr_tail: String::new() });
    assert!(matches!(action, super::keys::FarAction::Open(ref p) if p == &temp));
    assert_eq!(f.watches.len(), 1);
    let _ = std::fs::remove_file(&temp);
}
```

- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement.** Add `pub(crate) watches: Vec<remote::Watch>` to `FarPane` (init empty). Define in `remote.rs`:

```rust
use std::path::PathBuf;
use std::time::SystemTime;

/// A downloaded remote file being watched for local edits to push back.
pub(crate) struct Watch {
    pub temp: PathBuf,
    pub remote: Location,
    pub mtime: Option<SystemTime>,
}
```

Implement `begin_download` (spawn `rclone copyto remote temp`, `PendingKind::Download { remote, temp }`) and:

```rust
pub(crate) fn absorb_download(&mut self, remote: Location, temp: PathBuf, done: RcloneDone) -> FarAction {
    if done.code != Some(0) {
        return FarAction::Status(format!("rclone: download failed: {}", if done.stderr_tail.is_empty() { "error".into() } else { done.stderr_tail }));
    }
    let mtime = std::fs::metadata(&temp).and_then(|m| m.modified()).ok();
    self.watches.push(Watch { temp: temp.clone(), remote, mtime });
    FarAction::Open(temp)
}
```

In `keys.rs`, `activate`'s file branch and `open_selected`: when the panel is remote and the entry is a file, `return Some(p.begin_download(&name));`.

In `poll.rs`, the `poll_ops` result may now be `FarAction::Open` — route it through the same handling as the keys-path Open (call `open::that`). Factor the app `keys.rs` FarAction match into a `fn apply_far_action(&mut self, action: FarAction, focused: usize)` and call it from both the keys path and the poll path.

- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit.** `git commit -am "feat(far): download+open remote files (F3/F4/Enter)"`

---

### Task 11: Auto-upload watched temp files on change

**Files:** `remote.rs`, `mod.rs`, `poll.rs`, `remote_tests.rs`

**Interfaces:**
- Produces: `FarPane::poll_watches(&mut self) -> Option<FarAction>` — each tick, stat every `Watch`; if a temp file's mtime advanced past the recorded one and no op is pending, spawn `rclone copyto temp remote` and update the recorded mtime. Returns a `↑ syncing` status.

- [ ] **Step 1: Failing test:**

```rust
#[test]
fn changed_watch_triggers_upload() {
    let mut f = remote_pane();
    let temp = std::env::temp_dir().join("far-drive-watch-test.txt");
    std::fs::write(&temp, b"one").unwrap();
    let old = std::fs::metadata(&temp).unwrap().modified().unwrap();
    f.watches.push(super::remote::Watch { temp: temp.clone(), remote: f.left.loc.child("w.txt"), mtime: Some(old) });
    // Simulate an edit with a strictly newer mtime.
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(&temp, b"two").unwrap();
    let action = f.poll_watches();
    assert!(matches!(action, Some(super::keys::FarAction::Status(ref m)) if m.contains("syncing")));
    assert!(f.pending.is_some(), "an edit pushes the file back to the remote");
    let _ = std::fs::remove_file(&temp);
}
```

- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement `poll_watches`:**

```rust
pub fn poll_watches(&mut self) -> Option<FarAction> {
    if self.pending.is_some() { return None; } // one rclone op at a time
    for i in 0..self.watches.len() {
        let current = std::fs::metadata(&self.watches[i].temp).and_then(|m| m.modified()).ok();
        let changed = match (current, self.watches[i].mtime) {
            (Some(c), Some(prev)) => c > prev,
            (Some(_), None) => true,
            _ => false,
        };
        if changed {
            self.watches[i].mtime = current;
            let w = &self.watches[i];
            let argv = rclone::argv_copy(&Location::local(&w.temp), &w.remote, false);
            let note = format!("↑ syncing {}", w.remote.rclone_addr());
            let remote_side = if self.left.loc.is_remote() { Side::Left } else { Side::Right };
            return Some(self.begin_simple(argv, remote_side, "synced", note));
        }
    }
    None
}
```

Wire `poll_watches` into `poll.rs` alongside `poll_ops`, routing its `FarAction` the same way. Add `|| !self.watches.is_empty()` is **not** wanted for `is_busy` (watching shouldn't keep the app hot); instead rely on the existing periodic tick. If the far pane only polls while `is_busy()`, add a lightweight "has watches" condition to the poll gate so edits are noticed — verify how often `poll.rs` runs the Far arm (it runs each tick in the poll loop; confirm the loop ticks regularly even when idle, else gate on `!watches.is_empty()`).

- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit.** `git commit -am "feat(far): auto-upload edited remote files via mtime watch"`

---

## Phase F — persistence & manual verification

### Task 12: Session persistence for remote panels

**Files:** `crates/crew-app/src/sessionsave.rs`, `sessionrestore.rs`, tests.

- [ ] **Step 1:** Inspect `SavedPane::far` (currently stores a single dir string). Extend it to store the active panel's `rclone_addr` plus a `remote: bool` flag (or a small `{ backend, path }` shape). Add a failing test asserting a remote far pane round-trips through save/load with its `remote:path`.
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3:** Implement: on save, persist `active_loc()` fully (local path or `remote:path`); on restore, reconstruct the `Location` and, for remote, call `begin_list`. Update the `sessionrestore.rs` Task-4 guard that currently drops remote panes.
- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit.** `git commit -am "feat(far): persist and restore remote panel locations"`

---

### Task 13: Manual end-to-end verification

Not CI-testable (needs live Google credentials). Use the repo `verify` skill (`.claude/skills/verify`) against a real configured remote.

- [ ] **Step 1:** On the dev machine, ensure `rclone config` has a Google Drive remote (e.g. `gdrive`). Confirm `rclone lsjson gdrive:` returns JSON in a terminal.
- [ ] **Step 2:** Build and launch the live app via the `verify` skill. `/far`, then Alt+F1 → pick `gdrive` on the left panel; confirm the listing loads with a `listing…` flash then entries.
- [ ] **Step 3:** Navigate into a folder and back (`..`); F8 delete a throwaway file (verify it lands in Drive trash); F7 mkdir; F5 copy a small file local→drive and drive→local; F3 open a text file, edit it, save, confirm the `↑ syncing` flash and that Drive reflects the edit.
- [ ] **Step 4:** Note any rough edges as follow-up issues; do **not** expand scope here.
- [ ] **Step 5:** No commit (verification only) unless fixes were needed.

---

## Self-Review

**Spec coverage:**
- rclone-only backend, no OAuth → Global Constraints + Task 2/3. ✓
- Pluggable `Location`/`Backend` → Task 1, 4. ✓
- rclone adapter (all ops) → Task 2 argv + Task 7-11 wiring. ✓
- Async worker+poll, never block winit → Task 3 runner, Task 5 `poll_ops`, poll.rs wiring. ✓
- FAR-style drive select (Alt+F1/F2), rclone-missing/no-remotes states → Task 6. ✓
- Full two-way ops (copy/move/delete/mkdir) → Tasks 7-9. ✓
- Open/edit remote with auto-upload on save (mtime poll, no watch crate) → Tasks 10-11. ✓
- Error handling in status line → each absorb_* returns a status string. ✓
- Testing: pure unit (argv, parse, sort), injected-result state machine (absorb_*), manual GUI verify → Tasks 1-3, 5-12 tests + Task 13. ✓
- Known gotcha (Google-native docs export) → accepted as rclone default; no task needed (documented in spec). ✓
- v1 command-line-local-only limitation → Global Constraints + Task 4 Step 3. ✓

**Placeholder scan:** No "TBD"/"add error handling" left vague — every absorb path returns a concrete status; every step shows code. Task 12/13 are intentionally lighter (persistence detail depends on the current `SavedPane` shape, which the implementer inspects in Step 1; manual verify has no code).

**Type consistency:** `Location`, `Backend`, `PendingKind`, `PendingOp`, `RcloneDone`, `DriveSelect`, `DriveOption`, `Watch` names are used identically across tasks. `begin_list`/`absorb_list`, `begin_simple`/`absorb_simple`, `begin_transfer`/`absorb_transfer`, `begin_download`/`absorb_download` pair up consistently. `argv_*` names match between Task 2 defs and Task 7-11 uses.

**Known integration risks to confirm during implementation (call out, don't hide):**
1. `on_key` signature gains an `alt: bool` param (Task 6) — the app `keys.rs` call site and any tests calling `on_key` must pass it. Grep `\.on_key(` in farpane tests.
2. The Far poll arm in `poll.rs` must run every tick even when the pane is idle for the mtime watch to fire (Task 11) — verify the poll cadence; gate on `!watches.is_empty()` if needed.
3. Only one `pending` rclone op at a time — deliberate simplification; a second is rejected with "rclone busy". If this feels too coarse in manual testing, a per-op queue is a follow-up, not v1.
