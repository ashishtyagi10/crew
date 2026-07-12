# Far Command Bar: Smart Completion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Far pane's command bar completes like a good shell (Tab on commands/paths) and remembers like fish (persisted history, Up/Down recall, accept-with-Right ghost text).

**Architecture:** Two new pure/testable modules — `farpane/complete.rs` (caret-token detection, ranked candidates, token replacement, `$PATH` binary scan) and `farpane/cmdhist.rs` (a `CmdHistory` type: persisted history + Up/Down browse + ghost lookup) — both taking explicit parameters and touching only the one directory/file they need, no globals. `FarPane` gains four fields (`history`, `complete`, `bins`, `bins_scan_started`) that `keys.rs` wires into Tab/Up/Down/Right/End/Esc, `run.rs` feeds on every executed command, and `render.rs` paints as dim ghost text after the cursor.

**Tech Stack:** Rust, `dirs` (existing workspace dep, same crate `crate::history` already uses), `ratatui` (existing `render.rs` widgets), `winit::keyboard` (existing key types), `std::sync::{Arc, OnceLock}` + `std::thread` for the background `$PATH` scan.

## Global Constraints

- Never block the winit thread unboundedly: a completion reads at most ONE directory (the one the token resolves into); the `$PATH` binary list is scanned once per session on a background thread and cached (empty until ready — completion degrades gracefully, no waiting).
- Tab stays contextual, matching the existing `typing` flag in `farpane/keys.rs`: bar empty → Tab switches panels (unchanged); bar non-empty → Tab completes/cycles.
- The engine is pure: all functions take `(text, cwd, binaries)` as parameters and return data — unit-testable against tempdirs, no globals.
- History file lives beside the existing chat-input history (same `dirs` base), named `far-history`, newline-delimited, deduped-adjacent, capped at 500 entries (oldest dropped), loaded once per pane.
- Ghost text renders in `text_muted` after the caret and is never part of `cmdline` until accepted; all rendered text follows the existing width helpers.
- Zero `cargo check` warnings; rustfmt clean.

---

### Task 1: `farpane/complete.rs` — pure completion engine

**Files:**
- Create: `crates/crew-app/src/farpane/complete.rs` (implementation + inline `#[cfg(test)] mod tests`, mirroring `farpane/run.rs`'s and `farpane/list.rs`'s in-file test convention)
- Modify: `crates/crew-app/src/farpane/mod.rs` (add `mod complete;` to the existing `mod` list)

**Interfaces:**
- Produces (consumed by Task 4 `keys.rs`, Task 5 `render.rs`):
  - `pub(crate) enum TokenKind { Command, Path }` (`Debug, Clone, Copy, PartialEq, Eq`)
  - `pub(crate) fn caret_token(text: &str) -> (TokenKind, &str)`
  - `pub(crate) fn candidates(text: &str, cwd: &Path, binaries: &[String]) -> Vec<String>`
  - `pub(crate) fn apply(text: &str, candidate: &str) -> String`
  - `pub(crate) struct CycleState { pub(crate) candidates: Vec<String>, pub(crate) i: usize, pub(crate) prefix: String }`
  - `pub(crate) fn scan_path_binaries(path_var: &str) -> Vec<String>`
- Consumes: `crate::pathexpand::expand_path(base: &Path, arg: &str) -> PathBuf` (existing).

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/farpane/complete.rs` with only this content (the implementation lands in Step 3 above the test module, but write the file with the tests first so they fail to compile):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(key: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("crew_far_complete_{key}"));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("src")).unwrap();
        std::fs::create_dir_all(base.join("srcdocs")).unwrap();
        std::fs::write(base.join("src/main.rs"), b"x").unwrap();
        std::fs::write(base.join("src/Main.txt"), b"x").unwrap();
        std::fs::write(base.join("readme.md"), b"x").unwrap();
        base
    }

    #[test]
    fn caret_token_splits_command_and_path_words() {
        assert_eq!(caret_token("ls"), (TokenKind::Command, "ls"));
        assert_eq!(caret_token("ls src/fa"), (TokenKind::Path, "src/fa"));
    }

    #[test]
    fn caret_token_trailing_space_starts_an_empty_token() {
        assert_eq!(caret_token("ls "), (TokenKind::Path, ""));
    }

    #[test]
    fn cd_argument_is_always_a_path_token() {
        assert_eq!(caret_token("cd sr"), (TokenKind::Path, "sr"));
    }

    #[test]
    fn path_candidates_list_the_tokens_parent_dir_with_dir_slash_suffix() {
        let base = fixture("pathlist");
        let cands = candidates("ls ", &base, &[]);
        assert!(cands.contains(&"src/".to_string()), "{cands:?}");
        assert!(cands.contains(&"srcdocs/".to_string()), "{cands:?}");
        assert!(cands.contains(&"readme.md".to_string()), "{cands:?}");
    }

    #[test]
    fn path_candidates_prefix_match_case_sensitive_then_insensitive() {
        let base = fixture("pathcase");
        // "M" matches "Main.txt" case-sensitively first, "main.rs" only
        // case-insensitively — case-sensitive matches must come first.
        let cands = candidates("ls src/M", &base, &[]);
        assert_eq!(
            cands,
            vec!["src/Main.txt".to_string(), "src/main.rs".to_string()]
        );
    }

    #[test]
    fn a_unique_prefix_yields_a_single_candidate() {
        let base = fixture("unique");
        let cands = candidates("cat read", &base, &[]);
        assert_eq!(cands, vec!["readme.md".to_string()]);
    }

    #[test]
    fn command_candidates_are_builtins_plus_binaries_prefix_matched() {
        let base = fixture("cmdlist");
        let bins = vec!["cargo".to_string(), "cat".to_string(), "ls".to_string()];
        let cands = candidates("ca", &base, &bins);
        assert_eq!(cands, vec!["cargo".to_string(), "cat".to_string()]);
    }

    #[test]
    fn cd_argument_completes_directories_in_context() {
        let base = fixture("cdpath");
        let cands = candidates("cd sr", &base, &[]);
        assert_eq!(cands, vec!["src/".to_string(), "srcdocs/".to_string()]);
    }

    #[test]
    fn apply_replaces_only_the_caret_token() {
        assert_eq!(apply("ls src/fa", "src/farpane/"), "ls src/farpane/");
        assert_eq!(apply("ca", "cargo"), "cargo");
        assert_eq!(apply("ls ", "readme.md"), "ls readme.md");
    }

    #[cfg(unix)]
    #[test]
    fn scan_path_binaries_finds_executables_only_sorted_and_deduped() {
        let base = std::env::temp_dir().join("crew_far_complete_pathscan");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let exe = base.join("mytool");
        std::fs::write(&exe, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(base.join("notes.txt"), b"x").unwrap();
        // Duplicate PATH entry — the scan must dedupe across dirs too.
        let path_var = format!("{}:{}", base.display(), base.display());
        assert_eq!(scan_path_binaries(&path_var), vec!["mytool".to_string()]);
    }
}
```

Note: `std::fs::Permissions::from_mode` needs `std::os::unix::fs::PermissionsExt` in scope for that one test — add `use std::os::unix::fs::PermissionsExt;` inside the `#[cfg(unix)]` test function body (not at module top, so it stays unused-import-free on non-unix builds).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::complete::`
Expected: compile FAIL — `TokenKind`, `caret_token`, `candidates`, `apply`, `scan_path_binaries` not found in this scope.

- [ ] **Step 3: Implement**

Insert this ABOVE the `#[cfg(test)] mod tests { ... }` block already in the file:

```rust
//! Pure completion engine for the Far command bar: which token the caret
//! sits in (`caret_token`), the ranked candidate list for that token
//! (`candidates`), and applying a chosen candidate back into the command
//! line (`apply`). Everything here takes `(text, cwd, binaries)` as
//! parameters and returns data — no globals, no I/O beyond the single
//! bounded `read_dir` a `Path`-kind lookup needs — so it's unit-testable
//! against tempdirs without touching a real `FarPane`.
use std::collections::HashSet;
use std::path::Path;

/// Which token the caret sits in — completion always assumes the caret is at
/// end-of-line (the command bar is append/pop only today).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// The first whitespace-separated word: builtins + `$PATH` binaries.
    Command,
    /// Any later word (including `cd`'s argument): directory entries.
    Path,
}

/// A builtin the command bar understands directly (not a `$PATH` binary).
const BUILTINS: [&str; 1] = ["cd"];

/// Which token the caret sits in and its text so far. The first
/// whitespace-separated word is `Command`; every later word — including
/// `cd`'s argument — is `Path`.
pub(crate) fn caret_token(text: &str) -> (TokenKind, &str) {
    let token_start = text.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
    let token = &text[token_start..];
    let is_first_word = text[..token_start].trim().is_empty();
    let kind = if is_first_word {
        TokenKind::Command
    } else {
        TokenKind::Path
    };
    (kind, token)
}

/// Ranked candidates for the caret token in `text`: full replacement
/// strings for that token, ready for [`apply`]. Case-sensitive prefix
/// matches come first, then case-insensitive ones, each group in the input
/// order (`binaries` is expected pre-sorted, as produced by
/// [`scan_path_binaries`]; directory listings are sorted here).
pub(crate) fn candidates(text: &str, cwd: &Path, binaries: &[String]) -> Vec<String> {
    let (kind, token) = caret_token(text);
    match kind {
        TokenKind::Command => command_candidates(token, binaries),
        TokenKind::Path => path_candidates(token, cwd),
    }
}

fn command_candidates(prefix: &str, binaries: &[String]) -> Vec<String> {
    let pool: Vec<&str> = BUILTINS
        .iter()
        .copied()
        .chain(binaries.iter().map(String::as_str))
        .collect();
    rank_prefix(prefix, pool.into_iter())
}

/// Path-kind candidates: split `token` at its last `/` into the directory
/// part (kept literal — `~`/`$VAR` stay unexpanded in the returned string)
/// and the name prefix to match; read that one directory (expanded via
/// `pathexpand` against `cwd`) once, prefix-match its entries, and suffix
/// directories with `/`.
fn path_candidates(token: &str, cwd: &Path) -> Vec<String> {
    let (dir_part, name_prefix) = match token.rfind('/') {
        Some(i) => (&token[..=i], &token[i + 1..]),
        None => ("", token),
    };
    let dir = if dir_part.is_empty() {
        cwd.to_path_buf()
    } else {
        crate::pathexpand::expand_path(cwd, dir_part)
    };
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut entries: Vec<(String, bool)> = read
        .filter_map(|e| e.ok())
        .map(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (e.file_name().to_string_lossy().into_owned(), is_dir)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    rank_prefix(name_prefix, names.into_iter())
        .into_iter()
        .map(|name| {
            let is_dir = entries.iter().any(|(n, d)| *n == name && *d);
            format!("{dir_part}{name}{}", if is_dir { "/" } else { "" })
        })
        .collect()
}

/// Case-sensitive prefix matches first, then case-insensitive matches not
/// already included — each group in `items`'s given order, deduped.
fn rank_prefix<'a>(prefix: &str, items: impl Iterator<Item = &'a str>) -> Vec<String> {
    let items: Vec<&str> = items.collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for s in &items {
        if s.starts_with(prefix) && seen.insert(*s) {
            out.push(s.to_string());
        }
    }
    if !prefix.is_empty() {
        let lower = prefix.to_lowercase();
        for s in &items {
            if !seen.contains(s) && s.to_lowercase().starts_with(&lower) {
                seen.insert(*s);
                out.push(s.to_string());
            }
        }
    }
    out
}

/// The new full command line after replacing the caret token with
/// `candidate` — reuses the same end-of-line token boundary as
/// [`caret_token`], so `candidate` must be the token's full replacement
/// text (a bare command name, or a path candidate from [`candidates`],
/// which already includes the token's directory part).
pub(crate) fn apply(text: &str, candidate: &str) -> String {
    let token_start = text.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
    format!("{}{}", &text[..token_start], candidate)
}

/// An in-progress Tab-completion cycle on `FarPane::complete`: the ranked
/// candidates, which one is currently applied, and the pre-cycle command
/// line text (`prefix`) so Esc can restore it.
pub(crate) struct CycleState {
    pub(crate) candidates: Vec<String>,
    pub(crate) i: usize,
    pub(crate) prefix: String,
}

/// Read each directory in `path_var` (a `:`-joined `$PATH`-style string)
/// once, collecting executable file names; sorted and deduped. Missing or
/// unreadable directories are skipped, not fatal — this is the background
/// scan `FarPane`'s first Command-kind Tab kicks off on its own thread.
pub(crate) fn scan_path_binaries(path_var: &str) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for dir in std::env::split_paths(path_var) {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.filter_map(|e| e.ok()) {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            #[cfg(unix)]
            let executable =
                std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o111 != 0;
            #[cfg(not(unix))]
            let executable = true;
            if executable {
                names.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.into_iter().collect()
}
```

Then in `crates/crew-app/src/farpane/mod.rs`, change:

```rust
mod fileops;
mod icons;
mod keys;
mod list;
mod render;
mod run;
```

to:

```rust
mod complete;
mod fileops;
mod icons;
mod keys;
mod list;
mod render;
mod run;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::complete::`
Expected: PASS (10 tests: 9 on all platforms + 1 `#[cfg(unix)]`).

- [ ] **Step 5: Format + check clean**

Run: `cargo fmt -p crew-app` then `cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/complete.rs crates/crew-app/src/farpane/mod.rs
git commit -m "feat(crew): farpane complete.rs — pure command/path completion engine"
```

---

### Task 2: `farpane/cmdhist.rs` — persisted history + ghost lookup

**Files:**
- Create: `crates/crew-app/src/farpane/cmdhist.rs` (implementation + inline `#[cfg(test)] mod tests`)
- Modify: `crates/crew-app/src/farpane/mod.rs` (add `mod cmdhist;` to the `mod` list, alphabetically before `mod complete;`)

**Interfaces:**
- Produces (consumed by Task 3 `mod.rs`, Task 4 `keys.rs`/`run.rs`, Task 5 `render.rs`):
  - `pub(crate) struct CmdHistory` (fields private)
  - `impl CmdHistory { pub(crate) fn load() -> Self; pub(crate) fn push(&mut self, cmd: &str); pub(crate) fn prev(&mut self, current: &str) -> Option<&str>; pub(crate) fn next(&mut self, current: &str) -> Option<&str>; pub(crate) fn ghost(&self, prefix: &str) -> Option<&str>; #[cfg(test)] pub(crate) fn from_entries(entries: Vec<String>) -> Self }`
  - `#[cfg(test)] pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()>`
- Consumes: `dirs::config_dir()` (same base as `crate::history::path()` in `crates/crew-app/src/history.rs`, which does `dirs::config_dir().map(|d| d.join("crew").join("history"))` — this module mirrors that exactly with `far-history` as the filename).

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/farpane/cmdhist.rs` with only this content:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Point `$HOME` at a fresh tempdir for the duration of `f`, then
    /// restore it. Callers must hold `test_guard()` first.
    fn with_tmp_home<T>(f: impl FnOnce() -> T) -> T {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());
        let out = f();
        match prev {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        out
    }

    #[test]
    fn load_is_empty_when_no_file_exists() {
        let _g = test_guard();
        with_tmp_home(|| {
            assert!(CmdHistory::load().entries.is_empty());
        });
    }

    #[test]
    fn push_persists_and_reloads() {
        let _g = test_guard();
        with_tmp_home(|| {
            let mut h = CmdHistory::load();
            h.push("ls");
            h.push("cargo test");
            let reloaded = CmdHistory::load();
            assert_eq!(
                reloaded.entries,
                vec!["ls".to_string(), "cargo test".to_string()]
            );
        });
    }

    #[test]
    fn push_skips_blank_and_adjacent_duplicate() {
        let _g = test_guard();
        with_tmp_home(|| {
            let mut h = CmdHistory::load();
            h.push("ls");
            h.push("ls"); // adjacent dupe, skipped
            h.push(""); // blank, skipped
            h.push("pwd");
            h.push("ls"); // not adjacent (pwd in between) — kept
            assert_eq!(
                h.entries,
                vec!["ls".to_string(), "pwd".to_string(), "ls".to_string()]
            );
        });
    }

    #[test]
    fn push_caps_at_max_dropping_oldest() {
        let _g = test_guard();
        with_tmp_home(|| {
            let mut h = CmdHistory::load();
            for i in 0..MAX + 10 {
                h.push(&format!("cmd{i}"));
            }
            assert_eq!(h.entries.len(), MAX);
            assert_eq!(h.entries.first().unwrap(), "cmd10"); // oldest 10 dropped
            assert_eq!(h.entries.last().unwrap(), &format!("cmd{}", MAX + 9));
        });
    }

    #[test]
    fn prev_next_cycle_and_restore_typed_text() {
        let mut h =
            CmdHistory::from_entries(vec!["ls".into(), "pwd".into(), "cargo test".into()]);
        assert_eq!(h.prev("half-typed"), Some("cargo test")); // newest first
        assert_eq!(h.prev("half-typed"), Some("pwd"));
        assert_eq!(h.prev("half-typed"), Some("ls")); // oldest
        assert_eq!(h.prev("half-typed"), Some("ls")); // stays at oldest
        assert_eq!(h.next("ls"), Some("pwd"));
        assert_eq!(h.next("pwd"), Some("cargo test"));
        assert_eq!(h.next("cargo test"), Some("half-typed")); // restored
        assert_eq!(h.next("anything"), None); // not browsing anymore
    }

    #[test]
    fn prev_with_no_history_returns_none() {
        let mut h = CmdHistory::from_entries(vec![]);
        assert_eq!(h.prev("typed"), None);
    }

    #[test]
    fn ghost_matches_the_newest_extending_entry() {
        let h = CmdHistory::from_entries(vec![
            "cargo build".into(),
            "cargo check".into(),
            "cargo test".into(),
        ]);
        assert_eq!(h.ghost("cargo"), Some("cargo test")); // newest wins
        assert_eq!(h.ghost("cargo test"), None); // no STRICT extension
        assert_eq!(h.ghost("zz"), None); // no match
    }

    #[test]
    fn ghost_is_none_on_an_empty_bar() {
        let h = CmdHistory::from_entries(vec!["cargo test".into()]);
        assert_eq!(h.ghost(""), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::cmdhist::`
Expected: compile FAIL — `CmdHistory`, `MAX`, `test_guard` not found in this scope.

- [ ] **Step 3: Implement**

Insert this ABOVE the `#[cfg(test)] mod tests { ... }` block:

```rust
//! Command history for the Far command bar: persisted beside the existing
//! chat-input history (same `dirs` base as `crate::history`, a sibling file
//! named `far-history`), newline-delimited, deduped against the immediately
//! preceding entry, capped at 500 entries (oldest dropped), loaded once per
//! pane. Also serves fish-style ghost-text: the newest entry that strictly
//! extends the text currently being typed.
use std::path::PathBuf;

/// Keep at most this many recent commands on disk.
const MAX: usize = 500;

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("far-history"))
}

/// Non-empty lines, oldest first (mirrors `crate::history::deserialize`).
fn deserialize(s: &str) -> Vec<String> {
    s.lines().filter(|l| !l.is_empty()).map(str::to_string).collect()
}

/// The Far command bar's persisted history. `cursor` tracks an in-progress
/// Up/Down browse: `None` means the bar shows live typed text; `Some(i)`
/// means it shows `entries[i]`. `stash` holds the text that was being typed
/// when browsing started, restored once Down passes the newest entry.
pub(crate) struct CmdHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    stash: String,
}

impl CmdHistory {
    /// Load the persisted history (empty if the file is missing/unreadable).
    pub(crate) fn load() -> Self {
        let entries = path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| deserialize(&s))
            .unwrap_or_default();
        Self {
            entries,
            cursor: None,
            stash: String::new(),
        }
    }

    /// Build a history directly from `entries` (oldest first) — for tests
    /// that need known content without touching the filesystem.
    #[cfg(test)]
    pub(crate) fn from_entries(entries: Vec<String>) -> Self {
        Self {
            entries,
            cursor: None,
            stash: String::new(),
        }
    }

    /// Record a run command: skip blanks and immediate repeats, cap at
    /// `MAX` (oldest dropped), persist, and end any active browse.
    pub(crate) fn push(&mut self, cmd: &str) {
        self.cursor = None;
        self.stash.clear();
        if cmd.is_empty() || self.entries.last().map(String::as_str) == Some(cmd) {
            return;
        }
        self.entries.push(cmd.to_string());
        if self.entries.len() > MAX {
            let drop = self.entries.len() - MAX;
            self.entries.drain(..drop);
        }
        self.save();
    }

    fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, self.entries.join("\n"));
    }

    /// Up: recall the previous (older) entry, stashing `current` the first
    /// time this is called since the last edit/push. `None` with no history.
    pub(crate) fn prev(&mut self, current: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        let i = match self.cursor {
            None => {
                self.stash = current.to_string();
                self.entries.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.cursor = Some(i);
        Some(&self.entries[i])
    }

    /// Down: recall the next (newer) entry, or restore the stashed typed
    /// text once past the newest. `None` when not currently browsing.
    pub(crate) fn next(&mut self, _current: &str) -> Option<&str> {
        let i = self.cursor?;
        if i + 1 < self.entries.len() {
            self.cursor = Some(i + 1);
            Some(&self.entries[i + 1])
        } else {
            self.cursor = None;
            Some(self.stash.as_str())
        }
    }

    /// The newest entry that strictly extends `prefix` (`None` for an empty
    /// prefix — no ghost on an empty bar — or no match).
    pub(crate) fn ghost(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() {
            return None;
        }
        self.entries
            .iter()
            .rev()
            .find(|e| e.starts_with(prefix) && e.len() > prefix.len())
            .map(String::as_str)
    }
}

/// Serialises tests that mutate `$HOME` to point at a tempdir — several
/// tests below load/save real history files and would race under the
/// default parallel test runner. Mirrors `crate::palette::test_guard`.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
```

Then in `crates/crew-app/src/farpane/mod.rs`, change:

```rust
mod complete;
mod fileops;
mod icons;
mod keys;
mod list;
mod render;
mod run;
```

to:

```rust
mod cmdhist;
mod complete;
mod fileops;
mod icons;
mod keys;
mod list;
mod render;
mod run;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::cmdhist::`
Expected: PASS (8 tests).

- [ ] **Step 5: Format + check clean**

Run: `cargo fmt -p crew-app` then `cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/cmdhist.rs crates/crew-app/src/farpane/mod.rs
git commit -m "feat(crew): farpane cmdhist.rs — persisted far-history + ghost-text lookup"
```

---

### Task 3: Wire `history`/`complete`/`bins` state onto `FarPane`

**Files:**
- Modify: `crates/crew-app/src/farpane/mod.rs` (struct fields + `new()`)
- Test: `crates/crew-app/src/farpane/mod_tests.rs` (append)

**Interfaces:**
- Consumes: `complete::CycleState` (Task 1), `cmdhist::CmdHistory` (Task 2).
- Produces (consumed by Task 4 `keys.rs`/`run.rs`, Task 5 `render.rs`):
  - `FarPane.history: cmdhist::CmdHistory`
  - `FarPane.complete: Option<complete::CycleState>`
  - `FarPane.bins: std::sync::Arc<std::sync::OnceLock<Vec<String>>>`
  - `FarPane.bins_scan_started: bool`

- [ ] **Step 1: Write the failing test**

Append to `crates/crew-app/src/farpane/mod_tests.rs` (after the existing tests, before the final closing brace):

```rust
#[test]
fn new_pane_starts_with_empty_completion_and_scan_state() {
    let (_b, p) = fixture("newstate");
    assert!(p.complete.is_none());
    assert!(p.bins.get().is_none());
    assert!(!p.bins_scan_started);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app new_pane_starts_with_empty_completion_and_scan_state`
Expected: compile FAIL — no field `complete`/`bins`/`bins_scan_started` on `FarPane`.

- [ ] **Step 3: Implement**

In `crates/crew-app/src/farpane/mod.rs`, change the `FarPane` struct from:

```rust
pub struct FarPane {
    pub(crate) left: Panel,
    pub(crate) right: Panel,
    pub(crate) active: Side,
    /// Active text prompt (F7 make-folder), captured before any nav key.
    pub(crate) prompt: Option<Prompt>,
    /// The classic Far command line at the bottom: typed text runs (Enter) as a
    /// command in the active panel's directory. Empty when nothing is typed.
    pub(crate) cmdline: String,
    /// A command started from the command line that is still running on its
    /// worker thread: `(command text, result channel)`.
    pub(crate) running: Option<(String, std::sync::mpsc::Receiver<run::CmdDone>)>,
}
```

to:

```rust
pub struct FarPane {
    pub(crate) left: Panel,
    pub(crate) right: Panel,
    pub(crate) active: Side,
    /// Active text prompt (F7 make-folder), captured before any nav key.
    pub(crate) prompt: Option<Prompt>,
    /// The classic Far command line at the bottom: typed text runs (Enter) as a
    /// command in the active panel's directory. Empty when nothing is typed.
    pub(crate) cmdline: String,
    /// A command started from the command line that is still running on its
    /// worker thread: `(command text, result channel)`.
    pub(crate) running: Option<(String, std::sync::mpsc::Receiver<run::CmdDone>)>,
    /// Persisted command-line history (`far-history`) + Up/Down browse state
    /// and fish-style ghost-text lookups.
    pub(crate) history: cmdhist::CmdHistory,
    /// An in-progress Tab-completion cycle, if any — invalidated by any
    /// edit to `cmdline` (typing, Backspace, running a command).
    pub(crate) complete: Option<complete::CycleState>,
    /// Cached `$PATH` binaries for Command-kind Tab completion, filled by a
    /// background scan kicked off by the first Tab that needs it.
    pub(crate) bins: std::sync::Arc<std::sync::OnceLock<Vec<String>>>,
    /// Whether the `$PATH` scan thread has already been spawned — guards
    /// against spawning one per keystroke before the first scan lands.
    pub(crate) bins_scan_started: bool,
}
```

And change `FarPane::new` from:

```rust
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            left: Panel::new(cwd.clone()),
            right: Panel::new(cwd),
            active: Side::Left,
            prompt: None,
            cmdline: String::new(),
            running: None,
        }
    }
```

to:

```rust
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            left: Panel::new(cwd.clone()),
            right: Panel::new(cwd),
            active: Side::Left,
            prompt: None,
            cmdline: String::new(),
            running: None,
            history: cmdhist::CmdHistory::load(),
            complete: None,
            bins: std::sync::Arc::new(std::sync::OnceLock::new()),
            bins_scan_started: false,
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-app farpane::`
Expected: PASS — this filter matches the whole `farpane::` module tree, so it now includes Task 1's `complete::` tests (10) and Task 2's `cmdhist::` tests (8) too: 37 original + 10 + 8 + 1 new = 56 tests.

- [ ] **Step 5: Format + check clean**

Run: `cargo fmt -p crew-app` then `cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/mod.rs crates/crew-app/src/farpane/mod_tests.rs
git commit -m "feat(crew): FarPane gains history/completion/bins state"
```

---

### Task 4: Wire keys — Tab/Up/Down/Right/End/Esc + history push on run

**Files:**
- Modify: `crates/crew-app/src/farpane/keys.rs` (full-file replacement — see below)
- Modify: `crates/crew-app/src/farpane/run.rs` (`run_cmdline` pushes history + clears `complete`)
- Test: `crates/crew-app/src/farpane/mod_tests.rs` (append)

**Interfaces:**
- Consumes: `complete::{caret_token, candidates, apply, CycleState, TokenKind, scan_path_binaries}` (Task 1); `cmdhist::{CmdHistory, test_guard}` (Task 2); `FarPane.{history,complete,bins,bins_scan_started}` (Task 3).
- Produces (new `pub(crate)` functions in `keys.rs`, callable directly from tests the same way `move_sel`/`ascend`/`activate` already are — `winit::event::KeyEvent` is `#[non_exhaustive]` and can't be constructed in tests, so `reduce`'s own dispatch stays untested directly, matching the file's existing convention):
  - `pub(crate) fn escape_cmdline(p: &mut FarPane) -> Option<FarAction>`
  - `pub(crate) fn tab_complete(p: &mut FarPane)`
  - `pub(crate) fn history_prev(p: &mut FarPane)`
  - `pub(crate) fn history_next(p: &mut FarPane)`
  - `pub(crate) fn accept_ghost(p: &mut FarPane)`

- [ ] **Step 1: Write the failing tests**

First, extend the `use` block at the top of `crates/crew-app/src/farpane/mod_tests.rs` from:

```rust
use super::keys::{activate, ascend, move_sel};
use super::run::run_cmdline;
use super::{FarAction, FarPane, Side};
```

to:

```rust
use super::cmdhist::CmdHistory;
use super::keys::{
    accept_ghost, activate, ascend, escape_cmdline, history_next, history_prev, move_sel,
    tab_complete,
};
use super::run::run_cmdline;
use super::{FarAction, FarPane, Side};
```

Then append these tests (after the Task 3 test, before the closing brace):

```rust
#[test]
fn tab_completes_a_unique_path_candidate_with_trailing_space() {
    let (base, mut p) = fixture("tabunique");
    std::fs::write(base.join("readme.md"), b"x").unwrap();
    p.cmdline = "cat read".into();
    tab_complete(&mut p);
    assert_eq!(p.cmdline, "cat readme.md ");
    assert!(p.complete.is_none(), "single match doesn't start a cycle");
}

#[test]
fn tab_completes_a_directory_without_a_trailing_space() {
    let (_b, mut p) = fixture("tabdir");
    p.cmdline = "cd su".into();
    tab_complete(&mut p);
    assert_eq!(p.cmdline, "cd sub/");
}

#[test]
fn tab_with_multiple_candidates_starts_a_cycle_and_wraps() {
    let (base, mut p) = fixture("tabcycle");
    std::fs::write(base.join("apple.txt"), b"x").unwrap();
    std::fs::write(base.join("avocado.txt"), b"x").unwrap();
    p.cmdline = "cat a".into();
    tab_complete(&mut p);
    let first = p.cmdline.clone();
    assert!(p.complete.is_some(), "multiple matches start a cycle");
    tab_complete(&mut p);
    let second = p.cmdline.clone();
    assert_ne!(first, second, "second Tab advances the cycle");
    tab_complete(&mut p);
    assert_eq!(p.cmdline, first, "third Tab wraps back to the first candidate");
}

#[test]
fn escape_during_a_cycle_restores_the_pre_cycle_text() {
    let (base, mut p) = fixture("tabesc");
    std::fs::write(base.join("apple.txt"), b"x").unwrap();
    std::fs::write(base.join("avocado.txt"), b"x").unwrap();
    p.cmdline = "cat a".into();
    tab_complete(&mut p);
    assert!(p.complete.is_some());
    let action = escape_cmdline(&mut p);
    assert!(action.is_none(), "Esc during a cycle doesn't close the pane");
    assert_eq!(p.cmdline, "cat a");
    assert!(p.complete.is_none());
}

#[test]
fn escape_on_a_typed_bar_clears_it_without_closing() {
    let (_b, mut p) = fixture("tabescclear");
    p.cmdline = "ls".into();
    assert!(escape_cmdline(&mut p).is_none());
    assert!(p.cmdline.is_empty());
}

#[test]
fn escape_on_an_empty_bar_closes_the_pane() {
    let (_b, mut p) = fixture("tabescclose");
    assert!(matches!(escape_cmdline(&mut p), Some(FarAction::Close)));
}

#[test]
fn history_prev_and_next_cycle_through_the_bar() {
    let (_b, mut p) = fixture("histcycle");
    p.history = CmdHistory::from_entries(vec!["ls".into(), "cargo test".into()]);
    p.cmdline = "half".into();
    history_prev(&mut p);
    assert_eq!(p.cmdline, "cargo test");
    history_prev(&mut p);
    assert_eq!(p.cmdline, "ls");
    history_next(&mut p);
    assert_eq!(p.cmdline, "cargo test");
    history_next(&mut p);
    assert_eq!(p.cmdline, "half", "Down past the newest restores the typed text");
}

#[test]
fn accept_ghost_fills_in_the_matching_history_entry() {
    let (_b, mut p) = fixture("ghostaccept");
    p.history = CmdHistory::from_entries(vec!["cargo build".into()]);
    p.cmdline = "cargo".into();
    accept_ghost(&mut p);
    assert_eq!(p.cmdline, "cargo build");
}

#[test]
fn accept_ghost_is_a_no_op_without_a_match() {
    let (_b, mut p) = fixture("ghostnoop");
    p.history = CmdHistory::from_entries(vec!["cargo build".into()]);
    p.cmdline = "zz".into();
    accept_ghost(&mut p);
    assert_eq!(p.cmdline, "zz");
}

/// Point `$HOME` at a fresh tempdir for the duration of `f`, then restore it
/// — callers must hold `super::cmdhist::test_guard()` first. `run_cmdline`
/// persists history via the real `dirs`-based path, so any test exercising
/// it needs this isolation (mirrors `cmdhist.rs`'s own test helper — kept
/// separate since it's a different file/module).
fn with_tmp_home<T>(f: impl FnOnce() -> T) -> T {
    let dir = tempfile::tempdir().unwrap();
    let prev = std::env::var_os("HOME");
    std::env::set_var("HOME", dir.path());
    let out = f();
    match prev {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }
    out
}

#[test]
fn run_cmdline_pushes_the_command_into_history() {
    let _g = super::cmdhist::test_guard();
    with_tmp_home(|| {
        let (base, mut p) = fixture("histpush");
        p.right.cwd = base.join("sub");
        p.active = Side::Right;
        p.cmdline = "touch made-here".into();
        run_cmdline(&mut p);
        assert_eq!(p.history.prev(""), Some("touch made-here"));
    });
}

#[test]
fn cd_is_pushed_into_history_too() {
    let _g = super::cmdhist::test_guard();
    with_tmp_home(|| {
        let (_b, mut p) = fixture("histpushcd");
        p.cmdline = "cd sub".into();
        run_cmdline(&mut p);
        assert_eq!(p.history.prev(""), Some("cd sub"));
    });
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::`
Expected: compile FAIL — `tab_complete`, `escape_cmdline`, `history_prev`, `history_next`, `accept_ghost`, `super::cmdhist::test_guard` not found (`keys.rs` doesn't export them yet, `p.history.prev("")` still works after Task 3 but the new helpers don't exist).

- [ ] **Step 3: Implement**

Replace the ENTIRE contents of `crates/crew-app/src/farpane/keys.rs` with:

```rust
//! Key reduction for the Far pane: panel switching, cursor movement, descending
//! into directories / opening files, the classic function-key actions
//! (copy/move/delete/make-folder/view/edit/help), Tab completion + Up/Down
//! history + Right/End ghost-text acceptance on the command line, and closing
//! the pane.
use std::path::PathBuf;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::fileops::{copy, delete, make_dir, rename_move};
use super::run::run_cmdline;
use super::{FarPane, Prompt};

/// A page jump (Page Up / Page Down) moves the cursor this many rows.
const PAGE: i32 = 10;

/// Outcome of a key press the host app must act on. Filesystem mutations happen
/// in-place on the pane; these are the effects that need the wider app.
pub enum FarAction {
    /// Tear the pane down (Esc on an empty command line / F10).
    Close,
    /// Open the keyboard-shortcuts overlay (F1).
    Help,
    /// Open a file with the OS default application (F3 / F4 / Enter on a file).
    Open(PathBuf),
    /// Show a transient status message (operation result or error).
    Status(String),
}

pub(crate) fn reduce(p: &mut FarPane, key: &KeyEvent) -> Option<FarAction> {
    if !key.state.is_pressed() {
        return None;
    }
    // A live text prompt (F7 make-folder) swallows every key until it's
    // confirmed (Enter) or cancelled (Esc).
    if p.prompt.is_some() {
        return prompt_key(p, key);
    }
    let typing = !p.cmdline.is_empty();
    match &key.logical_key {
        // F10 always quits. Esc cancels a Tab-cycle, else clears a typed
        // command, else quits.
        Key::Named(NamedKey::F10) => return Some(FarAction::Close),
        Key::Named(NamedKey::Escape) => return escape_cmdline(p),
        Key::Named(NamedKey::F1) => return Some(FarAction::Help),
        // Tab stays contextual: an empty bar switches panels (unchanged); a
        // typed bar completes/cycles the caret token.
        Key::Named(NamedKey::Tab) => {
            if typing {
                tab_complete(p);
            } else {
                p.active = p.other_side();
            }
        }
        Key::Named(NamedKey::ArrowDown) => {
            if typing {
                history_next(p);
            } else {
                move_sel(p, 1);
            }
        }
        Key::Named(NamedKey::ArrowUp) => {
            if typing {
                history_prev(p);
            } else {
                move_sel(p, -1);
            }
        }
        Key::Named(NamedKey::ArrowRight) => {
            if typing {
                accept_ghost(p);
            }
        }
        Key::Named(NamedKey::PageDown) => move_sel(p, PAGE),
        Key::Named(NamedKey::PageUp) => move_sel(p, -PAGE),
        Key::Named(NamedKey::Home) => set_sel(p, 0),
        Key::Named(NamedKey::End) => {
            if typing {
                accept_ghost(p);
            } else {
                set_sel(p, usize::MAX);
            }
        }
        // Enter runs a typed command; with an empty command line it activates
        // the selected entry (descend / open), preserving the old behaviour.
        Key::Named(NamedKey::Enter) => {
            if typing {
                return Some(run_cmdline(p));
            }
            return activate(p);
        }
        // Backspace edits the command line while typing, else ascends.
        Key::Named(NamedKey::Backspace) => {
            if typing {
                p.cmdline.pop();
                p.complete = None;
            } else {
                ascend(p);
            }
        }
        // F3 View / F4 Edit both open the selected file with the OS default app.
        Key::Named(NamedKey::F3) | Key::Named(NamedKey::F4) => return open_selected(p),
        Key::Named(NamedKey::F5) => return Some(copy(p)),
        Key::Named(NamedKey::F6) => return Some(rename_move(p)),
        Key::Named(NamedKey::F7) => p.prompt = Some(Prompt::mkdir()),
        Key::Named(NamedKey::F8) => return Some(delete(p)),
        // Printable input builds up the command line (classic Far behaviour).
        Key::Named(NamedKey::Space) => {
            p.cmdline.push(' ');
            p.complete = None;
        }
        Key::Character(s) => {
            p.cmdline.push_str(s.as_str());
            p.complete = None;
        }
        _ => {}
    }
    None
}

/// Esc on the command line: cancel an active Tab-cycle (restoring the
/// pre-cycle text) if one is running; else clear a typed command; else ask
/// the app to close the pane.
pub(crate) fn escape_cmdline(p: &mut FarPane) -> Option<FarAction> {
    if let Some(state) = p.complete.take() {
        p.cmdline = state.prefix;
        return None;
    }
    if !p.cmdline.is_empty() {
        p.cmdline.clear();
        return None;
    }
    Some(FarAction::Close)
}

/// Tab while the command line has text: cycle an existing candidate list, or
/// build a fresh one from the caret token. A single candidate applies
/// immediately (trailing space unless it's a directory, so deeper completion
/// chains); more than one starts a cycle on the first candidate, and another
/// Tab advances it (wrapping).
pub(crate) fn tab_complete(p: &mut FarPane) {
    if let Some(state) = &mut p.complete {
        state.i = (state.i + 1) % state.candidates.len();
        let candidate = state.candidates[state.i].clone();
        p.cmdline = super::complete::apply(&state.prefix, &candidate);
        return;
    }
    let (kind, _token) = super::complete::caret_token(&p.cmdline);
    let binaries = command_binaries(p, kind);
    let candidates = super::complete::candidates(&p.cmdline, &p.active_cwd(), &binaries);
    if candidates.is_empty() {
        return;
    }
    if candidates.len() == 1 {
        p.cmdline = super::complete::apply(&p.cmdline, &candidates[0]);
        if !p.cmdline.ends_with('/') {
            p.cmdline.push(' ');
        }
        return;
    }
    let prefix = p.cmdline.clone();
    p.cmdline = super::complete::apply(&prefix, &candidates[0]);
    p.complete = Some(super::complete::CycleState {
        candidates,
        i: 0,
        prefix,
    });
}

/// Command-kind completion needs the cached `$PATH` binaries; kick off the
/// background scan on first use (returns builtins-only until it lands, never
/// blocking this thread). Path-kind completion needs no binaries at all.
fn command_binaries(p: &mut FarPane, kind: super::complete::TokenKind) -> Vec<String> {
    if kind != super::complete::TokenKind::Command {
        return Vec::new();
    }
    if let Some(bins) = p.bins.get() {
        return bins.clone();
    }
    if !p.bins_scan_started {
        p.bins_scan_started = true;
        let slot = p.bins.clone();
        std::thread::spawn(move || {
            let path_var = std::env::var("PATH").unwrap_or_default();
            let bins = super::complete::scan_path_binaries(&path_var);
            let _ = slot.set(bins);
        });
    }
    Vec::new()
}

/// Up while typing: recall the previous (older) history entry into the
/// command line, stashing the currently-typed text so Down can restore it.
pub(crate) fn history_prev(p: &mut FarPane) {
    if let Some(s) = p.history.prev(&p.cmdline) {
        p.cmdline = s.to_string();
    }
    p.complete = None;
}

/// Down while typing: recall the next (newer) history entry, or restore the
/// text that was being typed once past the newest entry.
pub(crate) fn history_next(p: &mut FarPane) {
    if let Some(s) = p.history.next(&p.cmdline) {
        p.cmdline = s.to_string();
    }
    p.complete = None;
}

/// Right/End while typing: accept the visible ghost-text history suggestion
/// into the command line, if one is showing.
pub(crate) fn accept_ghost(p: &mut FarPane) {
    if let Some(g) = p.history.ghost(&p.cmdline) {
        p.cmdline = g.to_string();
    }
    p.complete = None;
}

/// Handle a key while the make-folder prompt is open.
fn prompt_key(p: &mut FarPane, key: &KeyEvent) -> Option<FarAction> {
    match &key.logical_key {
        Key::Named(NamedKey::Escape) => {
            p.prompt = None;
            None
        }
        Key::Named(NamedKey::Enter) => {
            let name = p.prompt.take().map(|pr| pr.input).unwrap_or_default();
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(make_dir(p, name))
        }
        Key::Named(NamedKey::Backspace) => {
            if let Some(pr) = p.prompt.as_mut() {
                pr.input.pop();
            }
            None
        }
        Key::Named(NamedKey::Space) => {
            if let Some(pr) = p.prompt.as_mut() {
                pr.input.push(' ');
            }
            None
        }
        Key::Character(s) => {
            if let Some(pr) = p.prompt.as_mut() {
                pr.input.push_str(s.as_str());
            }
            None
        }
        _ => None,
    }
}

/// Move the active panel's cursor by `delta`, clamped to the list.
pub(crate) fn move_sel(p: &mut FarPane, delta: i32) {
    let panel = p.active_panel_mut();
    let n = panel.entries.len();
    if n == 0 {
        return;
    }
    panel.sel = (panel.sel as i32 + delta).clamp(0, n as i32 - 1) as usize;
}

fn set_sel(p: &mut FarPane, idx: usize) {
    let panel = p.active_panel_mut();
    let n = panel.entries.len();
    if n > 0 {
        panel.sel = idx.min(n - 1);
    }
}

/// Enter the selected directory (or `..`), or ask the app to open a file.
pub(crate) fn activate(p: &mut FarPane) -> Option<FarAction> {
    let panel = p.active_panel_mut();
    let entry = panel.entries.get(panel.sel)?;
    let (is_parent, is_dir, name) = (entry.is_parent, entry.is_dir, entry.name.clone());
    if is_parent {
        ascend(p);
        None
    } else if is_dir {
        panel.cwd.push(name);
        panel.sel = 0;
        panel.reload();
        None
    } else {
        Some(FarAction::Open(panel.cwd.join(name)))
    }
}

/// F3/F4: open the selected file with the OS default app (directories ignored).
fn open_selected(p: &FarPane) -> Option<FarAction> {
    let panel = p.panel(p.active);
    let entry = panel.entries.get(panel.sel)?;
    if entry.is_parent || entry.is_dir {
        return None;
    }
    Some(FarAction::Open(panel.cwd.join(&entry.name)))
}

/// Move the active panel up to its parent directory.
pub(crate) fn ascend(p: &mut FarPane) {
    let panel = p.active_panel_mut();
    if let Some(parent) = panel.cwd.parent().map(PathBuf::from) {
        panel.cwd = parent;
        panel.sel = 0;
        panel.reload();
    }
}
```

Then in `crates/crew-app/src/farpane/run.rs`, change `run_cmdline` from:

```rust
pub(crate) fn run_cmdline(p: &mut FarPane) -> FarAction {
    let cwd = p.active_cwd();
    let cmd = std::mem::take(&mut p.cmdline);
    let cmd = cmd.trim().to_string();
    if cmd.is_empty() {
        return FarAction::Status("nothing to run".into());
    }
    if let Some(target) = cd_target(&cmd) {
        return change_dir(p, &cwd, target);
    }
    if let Some((running, _)) = &p.running {
        return FarAction::Status(format!("still running ‘{running}’ — wait for it"));
    }
    let rx = start(&crate::spawn::default_shell(), &cmd, &cwd);
    let status = format!("running ‘{cmd}’ in {}…", cwd.display());
    p.running = Some((cmd, rx));
    FarAction::Status(status)
}
```

to:

```rust
pub(crate) fn run_cmdline(p: &mut FarPane) -> FarAction {
    let cwd = p.active_cwd();
    let cmd = std::mem::take(&mut p.cmdline);
    let cmd = cmd.trim().to_string();
    p.complete = None;
    if cmd.is_empty() {
        return FarAction::Status("nothing to run".into());
    }
    if let Some(target) = cd_target(&cmd) {
        p.history.push(&cmd);
        return change_dir(p, &cwd, target);
    }
    if let Some((running, _)) = &p.running {
        return FarAction::Status(format!("still running ‘{running}’ — wait for it"));
    }
    p.history.push(&cmd);
    let rx = start(&crate::spawn::default_shell(), &cmd, &cwd);
    let status = format!("running ‘{cmd}’ in {}…", cwd.display());
    p.running = Some((cmd, rx));
    FarAction::Status(status)
}
```

(Only truly-dispatched commands are pushed — the "still running, wait for it" bail-out does NOT push, matching normal shell history semantics.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::`
Expected: PASS — all farpane tests (67 tests: 56 from before + 11 new).

- [ ] **Step 5: Format + check clean**

Run: `cargo fmt -p crew-app` then `cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/keys.rs crates/crew-app/src/farpane/run.rs crates/crew-app/src/farpane/mod_tests.rs
git commit -m "feat(crew): far cmdbar — Tab completion, Up/Down history, Right/End ghost accept"
```

---

### Task 5: Render ghost text after the cursor

**Files:**
- Modify: `crates/crew-app/src/farpane/render.rs` (`render()` computes the ghost suffix; `command_bar` gains a `ghost` parameter and renders it)
- Test: `crates/crew-app/src/farpane/render_tests.rs` (append)

**Interfaces:**
- Consumes: `FarPane.{history,complete}` (Task 3); `cmdhist::CmdHistory::ghost`/`from_entries` (Task 2); `complete::CycleState` (Task 1).
- Produces: nothing new outside `render.rs` (rendering is a leaf).

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/farpane/render_tests.rs` (after the existing tests, before the closing brace):

```rust
#[test]
fn ghost_text_renders_dim_after_the_cursor() {
    use crate::farpane::cmdhist::CmdHistory;
    let mut pane = fixture_pane("ghost");
    pane.cmdline = "ba".into();
    pane.history = CmdHistory::from_entries(vec!["bazqux".into()]);
    let cells = render(&pane, 80, 24);
    let cmd_row = 22; // rows(24) - cmdline row(1) - function bar row(1)
    let mut row: Vec<(u16, char, (u8, u8, u8))> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c, c.fg))
        .collect();
    row.sort_unstable_by_key(|(col, _, _)| *col);
    let line: String = row.iter().map(|(_, c, _)| *c).collect();
    assert!(line.contains("ba▏zqux"), "ghost text missing: {line:?}");
    let dim = crew_theme::theme().text_muted;
    let z_cell = row
        .iter()
        .find(|(_, c, _)| *c == 'z')
        .expect("ghost cell rendered");
    assert_eq!(z_cell.2, dim, "ghost text must render in text_muted");
}

#[test]
fn ghost_text_absent_when_history_does_not_extend_the_typed_text() {
    use crate::farpane::cmdhist::CmdHistory;
    let mut pane = fixture_pane("noghost");
    pane.cmdline = "yy".into();
    pane.history = CmdHistory::from_entries(vec!["bazqux".into()]);
    let cells = render(&pane, 80, 24);
    let cmd_row = 22;
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.into_iter().map(|(_, c)| c).collect();
    assert!(!line.contains('z'), "no history entry should match 'yy': {line:?}");
}

#[test]
fn ghost_hidden_while_a_tab_cycle_is_active() {
    use crate::farpane::cmdhist::CmdHistory;
    use crate::farpane::complete::CycleState;
    let mut pane = fixture_pane("ghostcycle");
    pane.cmdline = "ba".into();
    pane.history = CmdHistory::from_entries(vec!["bazqux".into()]);
    pane.complete = Some(CycleState {
        candidates: vec!["ba".into()],
        i: 0,
        prefix: "ba".into(),
    });
    let cells = render(&pane, 80, 24);
    let cmd_row = 22;
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.into_iter().map(|(_, c)| c).collect();
    assert!(
        !line.contains('z'),
        "ghost must be suppressed during a completion cycle: {line:?}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::render::`
Expected: the file COMPILES (`pane.history`/`pane.complete` already exist from Task 3, and these tests only call the existing `render()` signature) but `ghost_text_renders_dim_after_the_cursor` and `ghost_hidden_while_a_tab_cycle_is_active` FAIL ON ASSERTION — `render()` doesn't compute/pass a ghost yet, so `line.contains("ba▏zqux")` is `false`. `ghost_text_absent_when_history_does_not_extend_the_typed_text` PASSES already (there's nothing to suppress), which is fine — it stays green through Step 4 too.

- [ ] **Step 3: Implement**

In `crates/crew-app/src/farpane/render.rs`, change the tail of `render()` from:

```rust
    scroll_thumb(&mut buf, larea, &p.left, p.active == Side::Left);
    scroll_thumb(&mut buf, rarea, &p.right, p.active == Side::Right);
    let running = p.running.as_ref().map(|(cmd, _)| cmd.as_str());
    command_bar(&mut buf, split[1], &p.active_cwd(), &p.cmdline, running);
```

to:

```rust
    scroll_thumb(&mut buf, larea, &p.left, p.active == Side::Left);
    scroll_thumb(&mut buf, rarea, &p.right, p.active == Side::Right);
    // A Tab-cycle already shows its candidate in `cmdline` directly; the
    // ghost suggestion would be confusing layered on top of it, so it's
    // suppressed while a cycle is active.
    let ghost = if p.complete.is_none() {
        p.history
            .ghost(&p.cmdline)
            .map(|full| full[p.cmdline.len()..].to_string())
    } else {
        None
    };
    let running = p.running.as_ref().map(|(cmd, _)| cmd.as_str());
    command_bar(
        &mut buf,
        split[1],
        &p.active_cwd(),
        &p.cmdline,
        ghost.as_deref(),
        running,
    );
```

And change `command_bar` from:

```rust
fn command_bar(
    buf: &mut Buffer,
    area: Rect,
    cwd: &std::path::Path,
    cmdline: &str,
    running: Option<&str>,
) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let folder = cwd
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    let mut spans = vec![
        Span::styled(format!("{folder} "), Style::new().fg(dim).bg(bg)),
        Span::styled("$ ", Style::new().fg(accent_color()).bg(bg)),
        Span::styled(format!("{cmdline}▏"), Style::new().fg(ink).bg(bg)),
    ];
    if let Some(cmd) = running {
        spans.push(Span::styled(
            format!("  \u{27f3} {cmd}"),
            Style::new().fg(dim).bg(bg),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bg))
        .render(area, buf);
}
```

to:

```rust
fn command_bar(
    buf: &mut Buffer,
    area: Rect,
    cwd: &std::path::Path,
    cmdline: &str,
    ghost: Option<&str>,
    running: Option<&str>,
) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let folder = cwd
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    let mut spans = vec![
        Span::styled(format!("{folder} "), Style::new().fg(dim).bg(bg)),
        Span::styled("$ ", Style::new().fg(accent_color()).bg(bg)),
        Span::styled(format!("{cmdline}▏"), Style::new().fg(ink).bg(bg)),
    ];
    if let Some(g) = ghost {
        spans.push(Span::styled(g.to_string(), Style::new().fg(dim).bg(bg)));
    }
    if let Some(cmd) = running {
        spans.push(Span::styled(
            format!("  \u{27f3} {cmd}"),
            Style::new().fg(dim).bg(bg),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bg))
        .render(area, buf);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::render::`
Expected: PASS (17 tests: 14 existing + 3 new).

- [ ] **Step 5: Full-suite run + format + check clean**

Run: `cargo test -p crew-app` → PASS (all suites). Then `cargo fmt -p crew-app` and `cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/render.rs crates/crew-app/src/farpane/render_tests.rs
git commit -m "feat(crew): far cmdbar — render fish-style ghost text in text_muted"
```

---

## Self-Review Notes

**Spec coverage mapping** (`docs/superpowers/specs/2026-07-11-far-cmdbar-complete-design.md`):
- §1 Completion engine (`TokenKind`, `caret_token`, `candidates`, `apply`, `CycleState`) → Task 1 (engine) + Task 4 (`tab_complete`/`escape_cmdline` wiring, single-candidate immediate-apply-with-space-except-dirs rule, cycle-invalidated-by-edit via `p.complete = None` on every Character/Space/Backspace/history/run).
- §2 `$PATH` binary cache (`bins: Arc<OnceLock<Vec<String>>>`, one background scan, builtins-only fallback) → Task 3 (fields) + Task 4 (`command_binaries` spawn-once guard via `bins_scan_started`, `scan_path_binaries` from Task 1).
- §3 History + ghost (`CmdHistory::{load,push,prev,next,ghost}`, `run_cmdline` pushes every command including `cd`, Up/Down restore-typed-text, Right/End accept) → Task 2 (type) + Task 3 (field) + Task 4 (`history_prev`/`history_next`/`accept_ghost`, `run.rs` push).
- §4 Rendering (ghost in `text_muted` after the caret, never in `cmdline` until accepted, cycling shown via `cmdline` directly) → Task 5.
- §5 Testing bullets: command-vs-path split ✓ (Task 1 `caret_token_*`), `cd` arg is Path ✓ (Task 1 + Task 4), prefix ranking ✓ (Task 1 case-sensitive-then-insensitive tests), dir slash suffix ✓ (Task 1 `path_candidates_list_the_tokens_parent_dir_with_dir_slash_suffix`), one-candidate immediate apply ✓ (Task 4 `tab_completes_a_unique_path_candidate_with_trailing_space`), `apply()` mid-command replacement ✓ (Task 1 `apply_replaces_only_the_caret_token`); history push/dedupe/cap ✓ (Task 2), prev/next restore-typed-text ✓ (Task 2 + Task 4 glue), ghost empty-bar rule ✓ (Task 2 `ghost_is_none_on_an_empty_bar`); Tab contextual (empty→panel, typing→complete) ✓ (Task 4, though the `typing` boolean gate inside `reduce` itself is untestable without a constructible `KeyEvent` — same limitation the file already has for Backspace/Enter), Esc-during-cycle restores ✓ (Task 4 `escape_during_a_cycle_restores_the_pre_cycle_text`), Right accepts ghost ✓ (Task 4 `accept_ghost_fills_in_the_matching_history_entry`); ghost cells use `text_muted` and never enter `cmdline` ✓ (Task 5, and `accept_ghost`/render only ever write the FULL entry string into `cmdline` on explicit accept, never a partial).
- Global constraints: one `read_dir` per completion ✓ (`path_candidates` calls `std::fs::read_dir` exactly once); never block winit unboundedly ✓ (`$PATH` scan is a spawned thread, not inline); `far-history` filename/cap 500/dirs-base-mirrors-`crate::history` ✓ (Task 2, verified against `crates/crew-app/src/history.rs`'s own `dirs::config_dir().join("crew").join("history")` pattern); engine pure with `(text, cwd, binaries)` params ✓ (Task 1 has no globals, no `FarPane` reference).

**Type-consistency check:**
- `TokenKind` (Task 1, `Debug+Clone+Copy+PartialEq+Eq`) is compared with `!=` in Task 4's `command_binaries` — requires `PartialEq`, present.
- `CycleState { candidates: Vec<String>, i: usize, prefix: String }` (Task 1, all fields `pub(crate)`) is constructed identically in Task 4 (`tab_complete`) and Task 5's test (`ghost_hidden_while_a_tab_cycle_is_active`) — same three fields, same names, both reachable since the struct and its fields are `pub(crate)`.
- `CmdHistory::{prev,next}(&mut self, current_or_underscore: &str) -> Option<&str>` — Task 4's `history_prev`/`history_next` call sites pass `&p.cmdline` matching the `&str` parameter; `next`'s parameter is intentionally unused (`_current`) since restoring the stash doesn't need it — documented in Task 2's implementation so Task 4 isn't surprised by the leading underscore.
- `CmdHistory::ghost(&self, prefix: &str) -> Option<&str>` returns the **full matching entry**, not a suffix — both Task 4's `accept_ghost` (`p.cmdline = g.to_string()`) and Task 5's render code (`full[p.cmdline.len()..]`) treat the return value this way consistently.
- `FarPane.bins: Arc<OnceLock<Vec<String>>>` — Task 3 initializes with `Arc::new(OnceLock::new())`; Task 4 clones the `Arc` (not the inner `Vec`) into the scan thread and calls `.set()` on the clone, so the pane's copy observes the write through the shared `OnceLock`.
- `complete::candidates`/`caret_token`/`apply` signatures match the spec text verbatim (`(text: &str, cwd: &Path, binaries: &[String]) -> Vec<String>`, `(text: &str) -> (TokenKind, &str)`, `(text: &str, candidate: &str) -> String`) and Task 4 calls them with exactly those argument shapes.

**Resolved ambiguities** (spec left these implicit; decisions made so the plan has no "TBD"):
1. **Trailing-space rule for single-candidate apply.** Spec says "trailing space for commands, none for dirs." Generalized to: no trailing space iff the applied `cmdline` now ends with `/` — this covers Command-kind (never ends with `/`, always gets a space) and Path-kind uniformly (dirs get `/` + no space; files get no `/` + a space), avoiding a kind-specific branch.
2. **`cd`'s argument is "always Path."** Verified this needs no special-casing: the general rule (word 1 = Command, word 2+ = Path) already makes any argument after `cd` a Path token, since `cd` is just an ordinary first word.
3. **Up/Down gating.** "Same guard as other cmdline keys" is read as the existing `typing = !p.cmdline.is_empty()` boolean already used for Backspace/Escape: an EMPTY bar keeps the old panel-cursor Up/Down/PageUp/PageDown/Home behavior unchanged; only a non-empty bar routes Up/Down to history. (Recalling history onto a fully empty bar is out of scope, consistent with the spec's own "Tab: bar empty → switches panels" symmetry.)
4. **Ghost suppressed during an active Tab-cycle.** Not stated explicitly; decided because `cmdline` already shows the cycling candidate, and a second suggestion source layered on top would be confusing. Implemented as `if p.complete.is_none()` before computing the ghost in `render()`.
5. **`CmdHistory` internal `stash` field.** The spec's code sketch shows only `{ entries, cursor }`, but restoring "what was being typed" across a `prev`/`next` sequence needs somewhere to hold it; added a private `stash: String` field (not part of any public signature, so it doesn't violate the sketch's public surface).
6. **Test isolation for `dirs`-based paths.** Verified empirically (`dirs::config_dir()` re-reads `$HOME` on every call, no caching) that overriding `$HOME` per-test is safe; added a `test_guard()` mutex (mirroring `crate::palette::test_guard`/`crate::app::theme_test_guard`, an existing repo convention for serializing global-state tests) so Task 2's and Task 4's `$HOME`-mutating tests can't race under the parallel test runner. Task 4's `run_cmdline`-driven history-push tests specifically need this because `push()` unconditionally calls `save()` against the real path — without the tempdir override those tests would write into the developer's actual `~/Library/Application Support/crew/far-history`.
7. **`reduce()`'s own dispatch stays untested.** `winit::event::KeyEvent` is `#[non_exhaustive]`, so — matching the file's pre-existing convention (none of `move_sel`/`ascend`/`activate`/`run_cmdline` are tested through `reduce`, only directly) — all new behavior is implemented as `pub(crate)` functions callable straight from tests, and `reduce` itself is a thin, deliberately-untested dispatch table.
