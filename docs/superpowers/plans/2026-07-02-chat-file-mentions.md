# Chat `@file` Mentions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Typing `@` mid-message in the crew chat composer opens a fuzzy file popup; accepted mentions expand to file contents in the outgoing message.

**Architecture:** All app-side in `crates/crew-app`; the broker/protocol is untouched. New pure modules `fileindex` (bounded walkdir scan) and `chatmention` (token detection, fuzzy filter, popup reducer, send-time expansion) plug into `ChatPane::on_key`; the popup renders via the existing `cmdmenu::menu_card` fieldset card as an overlay scene in `render.rs`.

**Tech Stack:** Rust, winit-driven cell UI, `walkdir` (workspace dep, first in-tree use).

Spec: `docs/superpowers/specs/2026-07-02-chat-file-mentions-design.md`

## Global Constraints

- Message-leading `@` stays the agent selector; only tokens after the first are file mentions.
- Scan bounds: depth ≤ 8, ≤ 2000 files, skip hidden + `target`/`node_modules`/`.git` (winit thread must not stall).
- Expansion caps: skip files > 64 KiB or non-UTF-8 with a one-line note; never block sending.
- Every panel is a fieldset card (legend on the top border) — reuse `cmdmenu::menu_card`.
- Pre-commit runs `cargo fmt` + `cargo check`; run `cargo test -p crew-app` per task.
- Comment style: module docs with `//!`, behavior-explaining doc comments like the surrounding files.

---

### Task 1: Bounded file index (`fileindex.rs`)

**Files:**
- Modify: `crates/crew-app/Cargo.toml` (add `walkdir = { workspace = true }` to `[dependencies]`)
- Create: `crates/crew-app/src/fileindex.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod fileindex;` alongside the other mods)

**Interfaces:**
- Produces: `pub(crate) fn scan(root: &Path) -> Vec<String>` — sorted relative paths, `/`-separated.

- [ ] **Step 1: Write the failing test** (bottom of the new `fileindex.rs`, module skeleton + tests only)

```rust
//! Bounded file listing for chat `@file` mentions: a walkdir scan of the app
//! cwd, capped in depth and count so the winit thread never stalls, skipping
//! hidden entries and heavyweight build dirs.
use std::path::Path;

/// Most files collected per scan; fuzzy filtering still works over a
/// truncated set, and the cap bounds the main-thread stall.
pub(crate) const MAX_FILES: usize = 2000;
const MAX_DEPTH: usize = 8;
/// Directories that are never worth mentioning and often huge.
const SKIP_DIRS: [&str; 3] = ["target", "node_modules", ".git"];

pub(crate) fn scan(root: &Path) -> Vec<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a throwaway tree under the OS temp dir; unique per test run.
    fn fixture(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("crew-fileindex-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("target/debug")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("README.md"), "hi").unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("src/.hidden"), "x").unwrap();
        std::fs::write(dir.join("target/debug/junk"), "x").unwrap();
        std::fs::write(dir.join(".git/config"), "x").unwrap();
        dir
    }

    #[test]
    fn scan_lists_files_relative_and_sorted() {
        let dir = fixture("basic");
        let files = scan(&dir);
        assert_eq!(files, vec!["README.md".to_string(), "src/main.rs".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_skips_hidden_and_build_dirs() {
        let dir = fixture("skips");
        let files = scan(&dir);
        assert!(!files.iter().any(|f| f.contains(".git") || f.contains("target") || f.contains(".hidden")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_of_missing_dir_is_empty() {
        assert!(scan(Path::new("/nonexistent/definitely-not-here")).is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app fileindex`
Expected: FAIL (panics at `todo!()`), after adding `mod fileindex;` to `main.rs` and the dep to `Cargo.toml`.

- [ ] **Step 3: Implement `scan`**

```rust
/// List files under `root` as sorted, `/`-separated relative paths. Bounded
/// (depth, count, skip list) — see the module doc; errors are skipped.
pub(crate) fn scan(root: &Path) -> Vec<String> {
    let walker = walkdir::WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && !(e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()))
        });
    let mut files = Vec::new();
    for entry in walker.flatten() {
        if files.len() >= MAX_FILES {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(root) {
            files.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    files
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-app fileindex`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/Cargo.toml crates/crew-app/src/fileindex.rs crates/crew-app/src/main.rs Cargo.lock
git commit -m "feat(crew-app): bounded file index for chat @file mentions"
```

---

### Task 2: Mention detection, fuzzy filter, accept (`chatmention.rs`)

**Files:**
- Create: `crates/crew-app/src/chatmention.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod chatmention;`)
- Modify: `crates/crew-app/src/suggest.rs:240` (`fn is_subsequence` → `pub(crate) fn is_subsequence`)

**Interfaces:**
- Produces: `pub(crate) fn pending_mention(input: &str) -> Option<&str>`, `pub(crate) fn filter(files: &[String], query: &str) -> Vec<String>`, `pub(crate) fn accept(input: &str, path: &str) -> String`.
- Consumes: `crate::suggest::is_subsequence(needle, hay)`.

- [ ] **Step 1: Write the failing tests** (new `chatmention.rs`)

```rust
//! `@file` mentions in the crew composer: detect the trailing `@token` being
//! typed (the message-leading token stays the `@agent` selector), fuzzy-filter
//! the file index against it, and splice the accepted path back into the
//! input. Pure string-in/string-out, like `chatcomplete`.

/// The query of a file mention being typed: the trailing token starts with
/// `@` and is not the leading token (`hey @sr` → `Some("sr")`).
pub(crate) fn pending_mention(input: &str) -> Option<&str> {
    todo!()
}

/// Files matching `query`, best first: filename-prefix, then path-substring,
/// then path-subsequence matches; ties break shorter-path-first. Capped.
pub(crate) fn filter(files: &[String], query: &str) -> Vec<String> {
    todo!()
}

/// Replace the trailing `@query` token with `@path ` (trailing space ends the
/// mention so the popup closes).
pub(crate) fn accept(input: &str, path: &str) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn pending_mention_is_the_trailing_at_token() {
        assert_eq!(pending_mention("hey @sr"), Some("sr"));
        assert_eq!(pending_mention("hey @"), Some(""));
        assert_eq!(pending_mention("@coder fix @src/ma"), Some("src/ma"));
    }

    #[test]
    fn leading_token_is_the_agent_selector_not_a_mention() {
        assert_eq!(pending_mention("@coder"), None);
        assert_eq!(pending_mention("@pl"), None);
    }

    #[test]
    fn plain_text_and_ended_tokens_are_no_mention() {
        assert_eq!(pending_mention("hello"), None);
        assert_eq!(pending_mention("hey @src/main.rs "), None);
        assert_eq!(pending_mention("mail a@b"), Some("b")); // '@' mid-word: last token isn't '@'-led
        assert_eq!(pending_mention(""), None);
    }

    #[test]
    fn filter_ranks_name_prefix_over_substring_over_subsequence() {
        let f = files(&["docs/main-notes.md", "src/main.rs", "crates/app/mod.rs"]);
        let got = filter(&f, "main");
        assert_eq!(got[0], "src/main.rs"); // filename prefix
        assert_eq!(got[1], "docs/main-notes.md"); // path substring
        let got = filter(&f, "camod");
        assert_eq!(got, vec!["crates/app/mod.rs".to_string()]); // subsequence
    }

    #[test]
    fn filter_empty_query_lists_everything_and_misses_are_dropped() {
        let f = files(&["a.rs", "b.rs"]);
        assert_eq!(filter(&f, "").len(), 2);
        assert!(filter(&f, "zzz").is_empty());
    }

    #[test]
    fn accept_replaces_the_trailing_token() {
        assert_eq!(accept("hey @sr", "src/main.rs"), "hey @src/main.rs ");
        assert_eq!(accept("look at @", "a.txt"), "look at @a.txt ");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatmention`
Expected: FAIL at `todo!()` (after `mod chatmention;` in `main.rs`).

- [ ] **Step 3: Implement the three functions**

```rust
pub(crate) fn pending_mention(input: &str) -> Option<&str> {
    // rfind gives the LAST whitespace: everything after it is the trailing
    // token. No whitespace at all → the leading token → agent selector.
    let (i, c) = input.char_indices().rev().find(|(_, c)| c.is_whitespace())?;
    input[i + c.len_utf8()..].strip_prefix('@')
}

/// Cap on returned matches: the popup shows 10 and scrolls; beyond ~50 the
/// tail is noise.
const MAX_MATCHES: usize = 50;

pub(crate) fn filter(files: &[String], query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    let mut scored: Vec<(u8, &String)> = files
        .iter()
        .filter_map(|f| rank(f, &q).map(|r| (r, f)))
        .collect();
    scored.sort_by(|(ra, fa), (rb, fb)| (ra, fa.len(), fa).cmp(&(rb, fb.len(), fb)));
    scored.truncate(MAX_MATCHES);
    scored.into_iter().map(|(_, f)| f.clone()).collect()
}

/// Match quality of `path` against lowercased `q`: filename prefix beats
/// path substring beats path subsequence; `None` for no match.
fn rank(path: &str, q: &str) -> Option<u8> {
    let low = path.to_lowercase();
    let name = low.rsplit('/').next().unwrap_or(&low);
    if name.starts_with(q) {
        Some(0)
    } else if low.contains(q) {
        Some(1)
    } else if crate::suggest::is_subsequence(q, &low) {
        Some(2)
    } else {
        None
    }
}

pub(crate) fn accept(input: &str, path: &str) -> String {
    let cut = input
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    format!("{}@{path} ", &input[..cut])
}
```

Also in `suggest.rs`, change `fn is_subsequence` to `pub(crate) fn is_subsequence` (line ~240).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app chatmention && cargo test -p crew-app suggest`
Expected: all pass (suggest untouched behaviorally).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/chatmention.rs crates/crew-app/src/main.rs crates/crew-app/src/suggest.rs
git commit -m "feat(crew-app): @file mention detection, fuzzy filter, accept"
```

---

### Task 3: Send-time expansion (`chatmention::expand`)

**Files:**
- Modify: `crates/crew-app/src/chatmention.rs`

**Interfaces:**
- Produces: `pub(crate) fn expand(text: &str, cwd: &Path) -> String`, `pub(crate) const MAX_FILE_BYTES: usize`.

- [ ] **Step 1: Write the failing tests** (append to `chatmention.rs` tests)

```rust
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("crew-mention-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn expand_appends_mentioned_file_contents() {
        let dir = tmp("expand");
        std::fs::write(dir.join("note.txt"), "hello world").unwrap();
        let out = expand("summarize @note.txt please", &dir);
        assert!(out.starts_with("summarize @note.txt please"));
        assert!(out.contains("--- file: note.txt ---\nhello world\n--- end file ---"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_skips_oversize_binary_and_missing() {
        let dir = tmp("caps");
        std::fs::write(dir.join("big.txt"), vec![b'a'; MAX_FILE_BYTES + 1]).unwrap();
        std::fs::write(dir.join("bin.dat"), [0u8, 159, 146, 150]).unwrap();
        let out = expand("see @big.txt @bin.dat @gone.txt", &dir);
        assert!(out.contains("--- file: big.txt skipped: too large ---"));
        assert!(out.contains("--- file: bin.dat skipped: binary ---"));
        assert!(!out.contains("gone.txt ---")); // unresolvable token left alone
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_ignores_the_leading_selector_and_dedups() {
        let dir = tmp("lead");
        std::fs::write(dir.join("a.txt"), "A").unwrap();
        // Leading token is the @agent selector even if it happens to be a path.
        let out = expand("@a.txt do it", &dir);
        assert_eq!(out, "@a.txt do it");
        let out = expand("x @a.txt and @a.txt", &dir);
        assert_eq!(out.matches("--- file: a.txt ---").count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatmention`
Expected: FAIL — `expand`/`MAX_FILE_BYTES` not found.

- [ ] **Step 3: Implement `expand`**

```rust
use std::path::Path;

/// Largest file inlined into a message; bigger mentions become a skip note
/// instead of blowing up the agents' context.
pub(crate) const MAX_FILE_BYTES: usize = 64 * 1024;

/// Expand `@path` mentions in an outgoing message: every non-leading token
/// that resolves to a file under `cwd` gets its contents appended as a
/// `--- file: … ---` block. The tokens stay in place; unresolvable ones are
/// left alone (they may be genuine prose). Never blocks sending.
pub(crate) fn expand(text: &str, cwd: &Path) -> String {
    let mut out = text.to_string();
    let mut seen: Vec<&str> = Vec::new();
    for (i, tok) in text.split_whitespace().enumerate() {
        // Token 0 is the @agent selector position, never a file mention.
        if i == 0 {
            continue;
        }
        let Some(rel) = tok.strip_prefix('@') else { continue };
        if rel.is_empty() || seen.contains(&rel) {
            continue;
        }
        let path = cwd.join(rel);
        if !path.is_file() {
            continue;
        }
        seen.push(rel);
        out.push_str(&attachment(rel, &path));
    }
    out
}

/// One mention's appended block: contents, or a one-line skip note.
fn attachment(rel: &str, path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) if bytes.len() > MAX_FILE_BYTES => {
            format!("\n\n--- file: {rel} skipped: too large ---")
        }
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => format!("\n\n--- file: {rel} ---\n{s}\n--- end file ---"),
            Err(_) => format!("\n\n--- file: {rel} skipped: binary ---"),
        },
        Err(e) => format!("\n\n--- file: {rel} skipped: {e} ---"),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app chatmention`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/chatmention.rs
git commit -m "feat(crew-app): expand @file mentions into outgoing chat messages"
```

---

### Task 4: Arrow keys in the chat key seam

**Files:**
- Modify: `crates/crew-app/src/chatkeys.rs`

**Interfaces:**
- Produces: `ChatInput::Up`, `ChatInput::Down` variants (classified from ArrowUp/ArrowDown).

- [ ] **Step 1: Write the failing test** (in `chatkeys.rs` tests)

```rust
    #[test]
    fn arrows_classify_for_popup_navigation() {
        assert_eq!(chat_key(&Key::Named(NamedKey::ArrowUp), true), ChatInput::Up);
        assert_eq!(chat_key(&Key::Named(NamedKey::ArrowDown), true), ChatInput::Down);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app chatkeys`
Expected: compile FAIL — no `Up`/`Down` variants.

- [ ] **Step 3: Add the variants and arms**

In the `ChatInput` enum add (with doc comment):

```rust
    /// Arrow keys — navigate the @file mention popup when it is open.
    Up,
    Down,
```

In `chat_key`'s match add:

```rust
        Key::Named(NamedKey::ArrowUp) => ChatInput::Up,
        Key::Named(NamedKey::ArrowDown) => ChatInput::Down,
```

Note: `chat.rs`'s match on `chat_key` must not break — Task 5 handles the new variants; for THIS commit to compile, add them to the early-return arm in `chat.rs:160`: `ChatInput::Ignore | ChatInput::Up | ChatInput::Down => return None,`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app chatkeys`
Expected: all pass; `cargo check` clean.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/chatkeys.rs crates/crew-app/src/chat.rs
git commit -m "feat(crew-app): classify arrow keys for the chat mention popup"
```

---

### Task 5: Popup state machine and ChatPane wiring

**Files:**
- Modify: `crates/crew-app/src/chatmention.rs` (add `MentionState`, `popup_key`, `after_edit`)
- Modify: `crates/crew-app/src/chat.rs` (field `mention`, `on_key(&mut self, key, cwd: &Path)`)
- Modify: `crates/crew-app/src/keys.rs:145` (pass `&self.cwd`)

**Interfaces:**
- Consumes: `ChatInput` (Task 4), `filter`/`pending_mention`/`accept` (Task 2), `fileindex::scan` (Task 1), `expand` (Task 3).
- Produces: `pub(crate) struct MentionState { pub files: Vec<String>, pub matches: Vec<String>, pub sel: usize }`; `pub(crate) enum MentionKey { Consumed, Forward }`; `pub(crate) fn popup_key(mention: &mut Option<MentionState>, input: &mut String, key: &ChatInput) -> MentionKey`; `pub(crate) fn after_edit(mention: &mut Option<MentionState>, input: &str, scan: impl FnOnce() -> Vec<String>)`; `ChatPane.mention: Option<MentionState>` (pub(crate)); `ChatPane::on_key(&mut self, key: &KeyEvent, cwd: &Path)`.

- [ ] **Step 1: Write the failing tests** (append to `chatmention.rs` tests)

```rust
    use crate::chatkeys::ChatInput;

    fn open(matches: &[&str]) -> Option<MentionState> {
        Some(MentionState {
            files: files(matches),
            matches: files(matches),
            sel: 0,
        })
    }

    #[test]
    fn popup_navigates_accepts_and_closes() {
        let mut m = open(&["a.rs", "b.rs"]);
        let mut input = "see @".to_string();
        assert!(matches!(popup_key(&mut m, &mut input, &ChatInput::Down), MentionKey::Consumed));
        assert_eq!(m.as_ref().unwrap().sel, 1);
        assert!(matches!(popup_key(&mut m, &mut input, &ChatInput::Enter), MentionKey::Consumed));
        assert_eq!(input, "see @b.rs ");
        assert!(m.is_none()); // accept closes

        let mut m = open(&["a.rs"]);
        assert!(matches!(popup_key(&mut m, &mut input, &ChatInput::Close), MentionKey::Consumed));
        assert!(m.is_none()); // Esc closes the popup, not the pane
    }

    #[test]
    fn popup_forwards_when_closed_and_on_edits() {
        let mut m: Option<MentionState> = None;
        let mut input = String::new();
        assert!(matches!(popup_key(&mut m, &mut input, &ChatInput::Enter), MentionKey::Forward));
        let mut m = open(&["a.rs"]);
        assert!(matches!(popup_key(&mut m, &mut input, &ChatInput::Char('x')), MentionKey::Forward));
    }

    #[test]
    fn after_edit_opens_refilters_and_closes() {
        let mut m: Option<MentionState> = None;
        // Typing "@" after a word opens the popup with the scanned files.
        after_edit(&mut m, "see @", || files(&["a.rs", "b.md"]));
        assert_eq!(m.as_ref().unwrap().matches.len(), 2);
        // Narrowing the query refilters WITHOUT rescanning (scan would panic).
        after_edit(&mut m, "see @a", || unreachable!("no rescan while open"));
        assert_eq!(m.as_ref().unwrap().matches, vec!["a.rs".to_string()]);
        // No match → closed; token ended → stays closed.
        after_edit(&mut m, "see @zzz", || unreachable!());
        assert!(m.is_none());
        after_edit(&mut m, "see @a.rs ", || files(&["a.rs"]));
        assert!(m.is_none());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatmention`
Expected: compile FAIL — `MentionState` etc. not found.

- [ ] **Step 3: Implement the reducer in `chatmention.rs`**

```rust
use crate::chatkeys::ChatInput;

/// The open mention popup: the scanned index (kept while typing narrows the
/// query), the current matches, and the selected row.
pub(crate) struct MentionState {
    pub files: Vec<String>,
    pub matches: Vec<String>,
    pub sel: usize,
}

/// Whether the popup consumed a key (navigation/accept/close) or the
/// composer should handle it normally.
pub(crate) enum MentionKey {
    Consumed,
    Forward,
}

/// Popup-first key routing: while open, arrows move, Tab/Enter accept the
/// selection into `input`, Escape closes the popup (not the pane).
pub(crate) fn popup_key(
    mention: &mut Option<MentionState>,
    input: &mut String,
    key: &ChatInput,
) -> MentionKey {
    let Some(m) = mention else {
        return MentionKey::Forward;
    };
    match key {
        ChatInput::Up => m.sel = m.sel.saturating_sub(1),
        ChatInput::Down => m.sel = (m.sel + 1).min(m.matches.len().saturating_sub(1)),
        ChatInput::Complete | ChatInput::Enter => {
            if let Some(path) = m.matches.get(m.sel) {
                *input = accept(input, path);
            }
            *mention = None;
        }
        ChatInput::Close => *mention = None,
        _ => return MentionKey::Forward,
    }
    MentionKey::Consumed
}

/// Sync the popup to the input after an edit: open it (scanning once) when a
/// mention is being typed, refilter while it is, close it when the token
/// ends or nothing matches.
pub(crate) fn after_edit(
    mention: &mut Option<MentionState>,
    input: &str,
    scan: impl FnOnce() -> Vec<String>,
) {
    let Some(q) = pending_mention(input) else {
        *mention = None;
        return;
    };
    let m = mention.get_or_insert_with(|| MentionState {
        files: scan(),
        matches: Vec::new(),
        sel: 0,
    });
    m.matches = filter(&m.files, q);
    m.sel = m.sel.min(m.matches.len().saturating_sub(1));
    if m.matches.is_empty() {
        *mention = None;
    }
}
```

- [ ] **Step 4: Wire `ChatPane`**

In `chat.rs`, add the field (after `unread`, with the same doc style) and init in `new()`:

```rust
    /// The @file mention popup while one is being typed (see `chatmention`).
    pub(crate) mention: Option<crate::chatmention::MentionState>,
```

Replace `on_key` with:

```rust
    /// Handle a winit key event. Returns [`ChatAction::Close`] when the user asks
    /// to close the pane (Escape) — mirroring the Far/Settings panes. While the
    /// @file popup is open it gets keys first (Escape then closes the popup, not
    /// the pane). `cwd` roots mention scanning and expansion.
    pub fn on_key(&mut self, key: &KeyEvent, cwd: &std::path::Path) -> Option<ChatAction> {
        let k = chat_key(&key.logical_key, key.state.is_pressed());
        if matches!(
            crate::chatmention::popup_key(&mut self.mention, &mut self.input, &k),
            crate::chatmention::MentionKey::Consumed
        ) {
            return None;
        }
        let (ch, enter, backspace) = match k {
            ChatInput::Close => return Some(ChatAction::Close),
            ChatInput::Ignore | ChatInput::Up | ChatInput::Down => return None,
            ChatInput::Complete => {
                if let Some(done) = crate::chatcomplete::complete(&self.input, &self.agents) {
                    self.input = done;
                }
                return None;
            }
            ChatInput::Char(c) => (Some(c), false, false),
            ChatInput::Enter => (None, true, false),
            ChatInput::Backspace => (None, false, true),
        };
        if let Some(text) = input_reduce(&mut self.input, ch, enter, backspace) {
            self.scroll = 0; // sending snaps back to the live bottom
            if crate::chatexport::intercept(self, &text) {
                return None; // answered locally (e.g. /export)
            }
            if !text.is_empty() {
                let cmd = PluginCommand::Send {
                    channel: self.channel.clone(),
                    text: crate::chatmention::expand(&text, cwd),
                };
                match self.plugin.send(&cmd) {
                    Ok(()) => self.awaiting = true, // wait for the reply
                    Err(e) => eprintln!("crew-app: plugin send error: {e}"),
                }
            }
        } else {
            // A Char/Backspace edit: sync the mention popup to the new input.
            crate::chatmention::after_edit(&mut self.mention, &self.input, || {
                crate::fileindex::scan(cwd)
            });
        }
        None
    }
```

In `keys.rs` change the dispatch (line ~145):

```rust
                PaneContent::Chat(c) => chat_action = c.on_key(event, &self.cwd),
```

- [ ] **Step 5: Run the full crate tests**

Run: `cargo test -p crew-app`
Expected: all pass (fix any `on_key` callers the compiler flags — `chat_tests.rs` builds panes but cannot construct `KeyEvent`, so no test-call sites exist).

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/chatmention.rs crates/crew-app/src/chat.rs crates/crew-app/src/keys.rs
git commit -m "feat(crew-app): @file mention popup state and ChatPane wiring"
```

---

### Task 6: Render the popup card

**Files:**
- Modify: `crates/crew-app/src/cmdmenu.rs:33` (`menu_card` gains a leading `title: &str` param)
- Modify: `crates/crew-app/src/render.rs` (existing call site ~line 176 + new mention block)

**Interfaces:**
- Consumes: `ChatPane.mention`, `Pane.rect`, `chatinput::composer_rows`, `suggest::MenuItem`.
- Produces: `pub fn menu_card(title: &str, matches: &[MenuItem], sel: usize, cols: u16, rows: u16) -> Vec<CellView>`.

- [ ] **Step 1: Write the failing test** (in `cmdmenu.rs` tests)

```rust
    #[test]
    fn card_legend_is_the_given_title() {
        let matches = crate::suggest::menu_items("/s");
        let cells = menu_card("files", &matches, 0, 40, menu_rows(matches.len()));
        // The legend on the top border spells the title.
        let row0: String = {
            let mut cs: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
            cs.sort_by_key(|c| c.col);
            cs.iter().map(|c| c.c).collect()
        };
        assert!(row0.contains("files"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app cmdmenu`
Expected: compile FAIL — `menu_card` takes 4 args.

- [ ] **Step 3: Add the param**

In `cmdmenu.rs`: `pub fn menu_card(title: &str, matches: &[MenuItem], sel: usize, cols: u16, rows: u16)` and pass `title` to `titled_card` where `"commands"` was hardcoded. Update the existing tests' calls to `menu_card("commands", …)` and the `render.rs:176` call site to `crate::cmdmenu::menu_card("commands", &matches, self.input.menu_sel, ic, mr)`.

- [ ] **Step 4: Add the mention popup scene in `render.rs`**

After the command-menu block (before the final `scenes` return in `build_frame`):

```rust
        // @file mention popup: a "files" fieldset card sitting above the focused
        // crew pane's composer while a mention is being typed. Overlay scene, so
        // the overlay pass backs it with an opaque page background.
        if !self.input.focused {
            if let Some(pane) = self.panes.get(self.focused) {
                if let crate::pane::PaneContent::Chat(c) = &pane.content {
                    if let Some(m) = &c.mention {
                        if !m.matches.is_empty() {
                            let items: Vec<crate::suggest::MenuItem> = m
                                .matches
                                .iter()
                                .map(|p| crate::suggest::MenuItem {
                                    label: format!("@{p}"),
                                    desc: String::new(),
                                    fill: String::new(),
                                    submit: false,
                                })
                                .collect();
                            let r = pane.rect;
                            let cols = (r.w / cw).floor() as u16;
                            let mr = crate::cmdmenu::menu_rows(items.len());
                            let comp = f32::from(crate::chatinput::composer_rows(
                                (r.h / ch).floor() as u16,
                            )) * ch;
                            let mh = f32::from(mr) * ch;
                            let my = (r.y + r.h - comp - mh).max(0.0);
                            scenes.push(PaneScene {
                                cells: crate::cmdmenu::menu_card("files", &items, m.sel, cols, mr),
                                x: r.x,
                                y: my,
                                w: r.w,
                                h: mh,
                                focused: false,
                                bordered: false,
                                overlay: true,
                            });
                        }
                    }
                }
            }
        }
```

(`MenuItem` is `pub(crate)` with public `label`/`desc`/`fill`/`submit` fields — `fill`/`submit` are unused by `menu_card`, so empty/false is fine.)

- [ ] **Step 5: Run tests + check**

Run: `cargo test -p crew-app && cargo check`
Expected: all pass, no warnings about the new block.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/cmdmenu.rs crates/crew-app/src/render.rs
git commit -m "feat(crew-app): render the @file mention popup as a files card"
```

---

### Task 7: Tint mid-message mentions in the composer

**Files:**
- Modify: `crates/crew-app/src/chatmention.rs` (add `spans`)
- Modify: `crates/crew-app/src/chatinput.rs` (`prompt_cells` colors span chars accent)

**Interfaces:**
- Produces: `pub(crate) fn spans(input: &str) -> Vec<(usize, usize)>` — half-open char-index ranges of every non-leading `@token` (length > 1).

- [ ] **Step 1: Write the failing test** (in `chatmention.rs` tests)

```rust
    #[test]
    fn spans_cover_non_leading_at_tokens() {
        assert_eq!(spans("hey @a.rs now"), vec![(4, 9)]);
        assert_eq!(spans("@coder fix @src/x.rs"), vec![(11, 20)]); // leading selector excluded
        assert!(spans("plain text").is_empty());
        assert!(spans("hey @").is_empty()); // bare '@' is not a mention yet
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app chatmention`
Expected: FAIL — `spans` not found.

- [ ] **Step 3: Implement `spans` and wire the composer**

```rust
/// Half-open char-index ranges of every non-leading `@token` in the input —
/// the composer tints them so mentions read as chips while typing.
pub(crate) fn spans(input: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        if start > 0 && chars[start] == '@' && i - start > 1 {
            spans.push((start, i));
        }
    }
    spans
}
```

In `chatinput.rs::prompt_cells`, tint span chars accent (the leading agent mention keeps its roster color):

```rust
    let spans = crate::chatmention::spans(input);
    let in_span = |i: usize| spans.iter().any(|&(s, e)| i >= s && i < e);
    let styled = input.chars().enumerate().map(|(i, c)| {
        let (fg, bold) = if i < mention {
            (m_color, true)
        } else if in_span(i) {
            (accent, false)
        } else {
            (t.ink, false)
        };
        (c, (fg, bold))
    });
```

(Replace the existing `styled` binding; `accent` is already in scope.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: all pass (including `chatinput_tests.rs` — if a test asserts the old uniform ink styling on inputs containing mid-line `@`, update it to expect the accent tint).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/chatmention.rs crates/crew-app/src/chatinput.rs
git commit -m "feat(crew-app): tint @file mentions in the crew composer"
```

---

### Task 8: Full verification

- [ ] **Step 1: Workspace tests + lints**

Run: `cargo fmt --all && cargo test --workspace && cargo clippy -p crew-app -- -D warnings`
Expected: clean. Fix anything flagged.

- [ ] **Step 2: End-to-end check**

Launch the app (`cargo run -p crew-app` or the project's run skill), open `/crew`, type `explain @` — popup appears listing repo files; arrows move; Enter inserts `@path `; sending a message with a mention reaches the broker expanded (verify with the mock/echo plugin via `CREW_CHAT_PLUGIN` if API keys are unavailable — the echo plugin reflects the sent text, so the expansion is directly visible).

- [ ] **Step 3: Commit any fixes**

```bash
git add -A && git commit -m "fix(crew-app): @file mention polish from end-to-end verification"
```
