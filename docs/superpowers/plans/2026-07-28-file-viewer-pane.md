# File Viewer Pane (Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One zoomed, read-only pane that opens any file where you already are — code, markdown, data, csv, diffs — with text extraction for PDF/Word and an `$EDITOR` handoff for editing.

**Architecture:** A new `viewpane/` module replaces `PaneContent::Markdown(MdPane)` with `PaneContent::View(ViewPane)`. Every rung of the format ladder produces the same `Vec<CardLine>` the chat card layout already uses, so `cells()` is one mapper over one uniform representation. **Detection and loading both run on a worker thread** — the pane opens in `LoadState::Loading` and a `poll_panes` drain swaps in the result — because a `stat` on a stalled mount or a `pdftotext` over 300 pages on the winit thread freezes every pane in the grid.

**Tech Stack:** Rust, `crew-app` (winit/wgpu GUI, binary crate), `crew-render` (`CellView`), `crew-theme`, the existing `md/` engine (pulldown-cmark), `open` crate (already vendored).

**Spec:** `docs/superpowers/specs/2026-07-28-file-viewer-pane-design.md`
**Goal doc:** `docs/superpowers/goals/2026-07-28-file-viewer-and-markdown-editing.md`

**Two deliberate refinements to the spec**, both tightening it:

1. **Detection runs on the worker thread, not the winit thread.** Classifying needs the head of the file and reading it is I/O. The spec read as detect-then-load; here `load::load_now` does both, and the pane opens knowing only its path.
2. **The file split is finer than the spec's five files** — `pane.rs` (model), `lines.rs` (rung → `CardLine`), `render.rs` (`CardLine` → `CellView`), `mdrung.rs`, `csv.rs` — to stay under the ~200-line ceiling this crate keeps.

## Global Constraints

- **NEVER run `cargo build --release` or `cargo clean`** — disk is tight on this machine. `cargo test` / `cargo clippy` (dev profile) only. `cargo clean --release` is the one permitted cleanup.
- **No new dependencies.** No `csv`, no `syntect`, no `tree-sitter`, no PDF crate. CSV is parsed by a small function in this module; extraction shells out to tools already on the machine.
- **No new colours and no new `Theme` fields.** Colour comes from `crew_theme::theme()` (`ink`, `text_muted`, `page_bg`, `ansi[..]`) **read at call time**, never cached in a `static`/`OnceLock` — a live `/theme` switch must repaint correctly.
- **Tests compare against `crew_theme::theme()`'s own slots**, never hardcoded RGB triples. The active theme is global mutable state and tests run in parallel.
- **Nothing blocking on the winit thread.** No `read_to_string`, no `Command`, no `metadata` on a path that could be a network mount, inside anything reachable from `keys.rs`, `render`, or `cells`. The one permitted exception is the single `is_file()` existence check in `open_view` (Task 8), which matches what `clickopen::open_path_token` already does.
- **Keep source files under ~200 lines.** When a file approaches it, move tests to a sibling `<name>_tests.rs` included via `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;` — the established pattern in this repo.
- **`cargo clippy --workspace --all-targets -- -D warnings` must be green**, with no `#[allow(...)]` added to make it so — **with one carve-out, covering two lint classes**: `dead_code` and `unused_imports`, when they arise because the item's or re-export's consumer is a task that has not landed yet. Tasks 1–7 build modules bottom-up, so each ships items and re-exports its caller acquires later. Silencing either with `#[allow]` is worse than the warning, because the allow outlives the gap and then hides the real thing. Every *other* warning class is a hard gate at every task, and clippy must be **fully** green from Task 8 onward, when the wiring lands — `unused_imports` on the `viewpane` re-exports in fact clears at Task 4, which is their first consumer. Verify with this while the gap is open — it prints every lint code that fired, so "only `dead_code`" is a fact rather than a hope:

```bash
cargo clippy -p crew-app --all-targets --message-format=json 2>/dev/null \
  | grep -o '"code":"[a-z_]*"' | sort | uniq -c | sort -rn
```

Grepping clippy's human-readable output for `dead_code` does **not** work: the lint name only appears in the note line, hyphenated as `dead-code`.
- **Never commit keys** — the crew repo is public.
- **Read-only.** No task in this plan writes to a viewed file. Editing is Phase 2 (markdown) or `$EDITOR` (everything else).
- `MAX_VIEW_BYTES = 8 * 1024 * 1024`, `SNIFF_BYTES = 8192`. Use these exact constants; tests reference them.

---

### Task 1: Format detection (pure)

The whole ladder hangs off one pure function. It takes bytes, never a filesystem, so it is exhaustively testable and can run on the worker thread in Task 2 without any rework.

**Files:**
- Create: `crates/crew-app/src/viewpane/detect.rs`
- Create: `crates/crew-app/src/viewpane/detect_tests.rs`
- Create: `crates/crew-app/src/viewpane/mod.rs` (module declarations only, this task)
- Modify: `crates/crew-app/src/main.rs` (add `mod viewpane;` beside the other `mod` lines)

**Interfaces:**
- Consumes: nothing.
- Produces: `Format`, `Extractor`, `Opaque`, `Probe`, `detect(path: &Path, head: &[u8], probe: Probe) -> Format`, `const SNIFF_BYTES: usize`. Tasks 2, 4, 5, 6 all match on `Format`.

- [ ] **Step 1: Create the module skeleton**

Create `crates/crew-app/src/viewpane/mod.rs`:

```rust
//! The file viewer pane: one zoomed, read-only pane over a ladder of formats.
//! `detect` classifies bytes, `load` fetches them off the winit thread, and
//! every rung renders down to the same `Vec<CardLine>` the chat cards use.
pub(crate) mod detect;
```

Add `mod viewpane;` to `crates/crew-app/src/main.rs` alongside the existing `mod` declarations, in alphabetical position.

- [ ] **Step 2: Write the failing tests**

Create `crates/crew-app/src/viewpane/detect_tests.rs`:

```rust
use super::*;
use std::path::Path;

fn all() -> Probe {
    Probe { textutil: true, pdftotext: true }
}

#[test]
fn extension_picks_the_rung() {
    assert!(matches!(
        detect(Path::new("a/b/main.rs"), b"fn main() {}", all()),
        Format::Code { lang: "rust" }
    ));
    assert!(matches!(
        detect(Path::new("README.md"), b"# hi", all()),
        Format::Markdown
    ));
    assert!(matches!(
        detect(Path::new("c.json"), b"{}", all()),
        Format::Data { lang: "json" }
    ));
    assert!(matches!(
        detect(Path::new("t.csv"), b"a,b", all()),
        Format::Csv { delim: ',' }
    ));
    assert!(matches!(
        detect(Path::new("t.tsv"), b"a\tb", all()),
        Format::Csv { delim: '\t' }
    ));
}

#[test]
fn a_nul_byte_outranks_the_extension() {
    // A .md file that is really a JPEG must not be handed to the markdown
    // engine — the binary verdict is the one sniff that beats the name.
    let jpeg = [0xFF, 0xD8, 0xFF, 0x00, 0x10];
    assert!(matches!(
        detect(Path::new("notes.md"), &jpeg, all()),
        Format::Opaque { why: Opaque::Binary }
    ));
}

#[test]
fn odd_text_in_a_markdown_file_is_still_markdown() {
    // Only the *binary* verdict outranks the extension. Weird prose does not.
    assert!(matches!(
        detect(Path::new("notes.md"), b"@@@ ---- ***", all()),
        Format::Markdown
    ));
}

#[test]
fn extensionless_files_are_sniffed() {
    assert!(matches!(
        detect(Path::new("run"), b"#!/bin/sh\necho hi\n", all()),
        Format::Code { lang: "shell" }
    ));
    assert!(matches!(
        detect(Path::new("changes"), b"diff --git a/x b/x\n", all()),
        Format::Diff
    ));
    assert!(matches!(
        detect(Path::new("hunk"), b"@@ -1,2 +1,3 @@\n", all()),
        Format::Diff
    ));
}

#[test]
fn invalid_utf8_without_a_nul_is_still_opaque() {
    // Latin-1 text has no NUL but cannot be rendered as UTF-8.
    assert!(matches!(
        detect(Path::new("notes.txt"), &[0xC3, 0x28, 0xA9], all()),
        Format::Opaque { why: Opaque::NotUtf8 }
    ));
}

#[test]
fn a_truncated_utf8_char_at_the_sniff_boundary_is_not_opaque() {
    // The head is a prefix of the file, so a multi-byte char can be cut in
    // half by SNIFF_BYTES. That must not condemn the whole file.
    let mut head = vec![b'a'; SNIFF_BYTES - 1];
    head.push(0xE2); // first byte of a 3-byte char, rest beyond the window
    assert!(!matches!(
        detect(Path::new("notes.txt"), &head, all()),
        Format::Opaque { .. }
    ));
}

#[test]
fn extract_rungs_pick_their_tool() {
    assert!(matches!(
        detect(Path::new("memo.docx"), b"PK\x03\x04", all()),
        Format::Extract { via: Extractor::TextUtil }
    ));
    assert!(matches!(
        detect(Path::new("paper.pdf"), b"%PDF-1.7", all()),
        Format::Extract { via: Extractor::PdfToText }
    ));
}

#[test]
fn a_missing_tool_degrades_the_rung_it_does_not_error() {
    let none = Probe { textutil: false, pdftotext: false };
    assert!(matches!(
        detect(Path::new("paper.pdf"), b"%PDF-1.7", none),
        Format::Opaque { why: Opaque::NoExtractor(Extractor::PdfToText) }
    ));
    assert!(matches!(
        detect(Path::new("memo.docx"), b"PK\x03\x04", none),
        Format::Opaque { why: Opaque::NoExtractor(Extractor::TextUtil) }
    ));
}

#[test]
fn an_unknown_extension_holding_text_reads_as_plain_code() {
    assert!(matches!(
        detect(Path::new("notes.wat"), b"hello there\n", all()),
        Format::Code { lang: "" }
    ));
}

#[test]
fn an_empty_file_is_not_opaque() {
    assert!(!matches!(
        detect(Path::new("empty.txt"), b"", all()),
        Format::Opaque { .. }
    ));
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p crew-app viewpane::detect`
Expected: FAIL — `could not find 'detect' in 'viewpane'` / unresolved imports.

- [ ] **Step 4: Implement `detect.rs`**

Create `crates/crew-app/src/viewpane/detect.rs`:

```rust
//! Classifying a file into a rung of the viewer's format ladder. Pure: it
//! takes the head of the file's bytes, never a filesystem, so the worker
//! thread in `load` can call it and the tests can cover every rung without
//! fixtures. The one rule worth stating out loud: the binary sniff outranks
//! the extension, and only for the binary verdict — a `.md` full of strange
//! prose is still markdown, a `.md` that is really a JPEG is not.
use std::path::Path;

/// How many leading bytes `detect` is given. The caller reads at most this
/// much, so a multi-byte char can be cut in half at the boundary — see
/// `looks_utf8`.
pub(crate) const SNIFF_BYTES: usize = 8192;

/// An external text-extraction tool. `TextUtil` ships with macOS; `PdfToText`
/// comes from poppler and is frequently absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Extractor {
    TextUtil,
    PdfToText,
}

impl Extractor {
    /// The binary name, used to probe `PATH` and to name what to install.
    pub(crate) fn bin(self) -> &'static str {
        match self {
            Extractor::TextUtil => "textutil",
            Extractor::PdfToText => "pdftotext",
        }
    }

    /// What the user is told to install when this tool is missing.
    pub(crate) fn install_hint(self) -> &'static str {
        match self {
            Extractor::TextUtil => "textutil (ships with macOS)",
            Extractor::PdfToText => "pdftotext — brew install poppler",
        }
    }
}

/// Why a file gets the metadata card instead of a rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opaque {
    Binary,
    NotUtf8,
    NoExtractor(Extractor),
}

/// Which tools are on `PATH`. Passed in rather than probed here so `detect`
/// stays pure and the "missing tool degrades a rung" rule is testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Probe {
    pub textutil: bool,
    pub pdftotext: bool,
}

impl Probe {
    fn has(self, e: Extractor) -> bool {
        match e {
            Extractor::TextUtil => self.textutil,
            Extractor::PdfToText => self.pdftotext,
        }
    }
}

/// One rung of the ladder. `lang` is the `md/syntax.rs` language tag, `""`
/// for "text, no keywords".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Code { lang: &'static str },
    Markdown,
    Data { lang: &'static str },
    Csv { delim: char },
    Diff,
    Extract { via: Extractor },
    Opaque { why: Opaque },
}

/// Extension → rung. Kept as a flat table because it is read once per open
/// and a `match` here is easier to extend than a lazy map.
fn by_extension(ext: &str) -> Option<Format> {
    let f = match ext {
        "rs" => Format::Code { lang: "rust" },
        "ts" | "tsx" | "js" | "jsx" => Format::Code { lang: "js" },
        "py" => Format::Code { lang: "python" },
        "go" => Format::Code { lang: "go" },
        "c" | "h" | "cpp" | "hpp" | "cc" => Format::Code { lang: "c" },
        "sh" | "bash" | "zsh" => Format::Code { lang: "shell" },
        "md" | "markdown" | "mdx" => Format::Markdown,
        "json" => Format::Data { lang: "json" },
        "yaml" | "yml" => Format::Data { lang: "yaml" },
        "toml" => Format::Data { lang: "toml" },
        "ini" | "conf" | "cfg" => Format::Data { lang: "" },
        "csv" => Format::Csv { delim: ',' },
        "tsv" => Format::Csv { delim: '\t' },
        "diff" | "patch" => Format::Diff,
        "docx" | "doc" | "rtf" | "odt" => Format::Extract { via: Extractor::TextUtil },
        "pdf" => Format::Extract { via: Extractor::PdfToText },
        _ => return None,
    };
    Some(f)
}

/// A NUL byte in the head. The cheapest reliable "this is not text" signal,
/// and the only one allowed to overrule the extension.
fn looks_binary(head: &[u8]) -> bool {
    head.contains(&0)
}

/// Whether `head` is valid UTF-8, tolerating a multi-byte char sliced by the
/// sniff window: an error in the final 3 bytes of a full-size head is the
/// boundary, not corruption.
fn looks_utf8(head: &[u8]) -> bool {
    match std::str::from_utf8(head) {
        Ok(_) => true,
        Err(e) => head.len() >= SNIFF_BYTES && e.valid_up_to() + 4 > head.len(),
    }
}

/// Content sniff for files whose extension told us nothing.
fn by_content(head: &[u8]) -> Format {
    let text = String::from_utf8_lossy(head);
    if text.starts_with("#!") {
        return Format::Code { lang: "shell" };
    }
    if text.starts_with("diff --git") || text.starts_with("@@ ") || text.starts_with("--- ") {
        return Format::Diff;
    }
    Format::Code { lang: "" }
}

/// Classify `path` given the first [`SNIFF_BYTES`] of its bytes and which
/// extractors exist. See the module comment for the precedence rule.
pub(crate) fn detect(path: &Path, head: &[u8], probe: Probe) -> Format {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let named = by_extension(&ext);

    // An extract rung is binary by nature, so the binary sniff must not fire
    // on it — resolve it first, degrading when its tool is absent.
    if let Some(Format::Extract { via }) = named {
        return if probe.has(via) {
            Format::Extract { via }
        } else {
            Format::Opaque { why: Opaque::NoExtractor(via) }
        };
    }
    if looks_binary(head) {
        return Format::Opaque { why: Opaque::Binary };
    }
    if !looks_utf8(head) {
        return Format::Opaque { why: Opaque::NotUtf8 };
    }
    named.unwrap_or_else(|| by_content(head))
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p crew-app viewpane::detect`
Expected: PASS, 10 tests.

- [ ] **Step 6: Lint**

Run: `cargo clippy -p crew-app --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-app/src/viewpane/ crates/crew-app/src/main.rs
git commit -m "feat(view): classify a file into a rung of the viewer ladder"
```

---

### Task 2: Off-thread load and extraction

Detection needs the head bytes, and reading bytes is I/O — so **detection runs on the worker too**. The winit thread never touches the file. This is a deliberate tightening of the spec, which read as detect-then-load.

**Files:**
- Create: `crates/crew-app/src/viewpane/load.rs`
- Create: `crates/crew-app/src/viewpane/load_tests.rs`
- Modify: `crates/crew-app/src/viewpane/mod.rs` (add `pub(crate) mod load;`)

**Interfaces:**
- Consumes: `detect::{detect, Format, Extractor, Probe, SNIFF_BYTES}`.
- Produces: `Loaded { text: String, truncated: Option<u64> }`, `LoadDone { format: Format, result: Result<Loaded, String> }`, `start(path: PathBuf) -> Receiver<LoadDone>`, `argv(e: Extractor, p: &Path) -> Vec<String>`, `probe() -> Probe`, `const MAX_VIEW_BYTES: u64`. Task 3 holds the `Receiver`; Tasks 4–6 render `Loaded.text`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/viewpane/load_tests.rs`:

```rust
use super::*;
use std::path::Path;

#[test]
fn textutil_argv_writes_to_stdout() {
    // -stdout is what keeps this a pipe read rather than a temp file we
    // would then have to clean up.
    assert_eq!(
        argv(Extractor::TextUtil, Path::new("/tmp/a b.docx")),
        vec!["-convert", "txt", "-stdout", "/tmp/a b.docx"]
    );
}

#[test]
fn pdftotext_argv_keeps_layout_and_writes_to_stdout() {
    // The trailing "-" is pdftotext's stdout sentinel; -layout preserves
    // columns, which is most of what makes an extract readable.
    assert_eq!(
        argv(Extractor::PdfToText, Path::new("/tmp/p.pdf")),
        vec!["-layout", "/tmp/p.pdf", "-"]
    );
}

#[test]
fn a_file_under_the_cap_is_not_truncated() {
    let dir = tempdir();
    let p = dir.join("small.txt");
    std::fs::write(&p, "hello\n").unwrap();
    let done = super::load_now(&p, Probe { textutil: false, pdftotext: false });
    let loaded = done.result.expect("small file loads");
    assert_eq!(loaded.text, "hello\n");
    assert_eq!(loaded.truncated, None, "nothing to announce");
}

#[test]
fn an_oversize_file_is_truncated_and_says_so() {
    // The cap applies to what is SHOWN, never to what may be opened — the
    // same call made for @file line ranges. A refusal here would make the
    // pane useless for exactly the files it is most wanted for.
    let dir = tempdir();
    let p = dir.join("big.log");
    let big = vec![b'x'; (MAX_VIEW_BYTES + 4096) as usize];
    std::fs::write(&p, &big).unwrap();
    let done = super::load_now(&p, Probe { textutil: false, pdftotext: false });
    let loaded = done.result.expect("an oversize file still loads");
    assert_eq!(loaded.text.len(), MAX_VIEW_BYTES as usize);
    assert_eq!(loaded.truncated, Some(MAX_VIEW_BYTES + 4096));
}

#[test]
fn a_missing_file_reports_why_and_names_itself() {
    let done = super::load_now(
        Path::new("/nonexistent/nope.txt"),
        Probe { textutil: false, pdftotext: false },
    );
    let err = done.result.expect_err("a missing file fails");
    assert!(err.contains("nope.txt"), "the message names the file: {err}");
}

#[test]
fn an_opaque_file_loads_no_text() {
    let dir = tempdir();
    let p = dir.join("blob.bin");
    std::fs::write(&p, [0u8, 1, 2, 3]).unwrap();
    let done = super::load_now(&p, Probe { textutil: false, pdftotext: false });
    assert!(matches!(done.format, Format::Opaque { .. }));
    assert_eq!(done.result.expect("opaque still succeeds").text, "");
}

#[test]
fn start_delivers_over_the_channel() {
    let dir = tempdir();
    let p = dir.join("chan.txt");
    std::fs::write(&p, "over the wire\n").unwrap();
    let rx = start(p);
    let done = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    assert_eq!(done.result.unwrap().text, "over the wire\n");
}

/// A unique temp dir for one test, created eagerly. No `tempfile` dep.
fn tempdir() -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-viewload-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app viewpane::load`
Expected: FAIL — `could not find 'load' in 'viewpane'`.

- [ ] **Step 3: Implement `load.rs`**

Create `crates/crew-app/src/viewpane/load.rs`:

```rust
//! Fetching a file for the viewer, always on a worker thread. Detection is
//! here too rather than at the call site, because classifying needs the head
//! of the file and reading it is I/O — doing that on the winit thread would
//! freeze every pane in the grid, agents included.
//!
//! Argv construction is pure and separately tested, the same split
//! `farpane/rclone.rs` makes, so CI covers the extractor commands on a
//! machine with neither tool installed.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use super::detect::{detect, Extractor, Format, Opaque, Probe, SNIFF_BYTES};

/// How much of a file the pane will show. Deliberately a cap on what is
/// DISPLAYED, not on what may be opened: the 40 MB log is precisely the file
/// you want to look at the top of.
pub(crate) const MAX_VIEW_BYTES: u64 = 8 * 1024 * 1024;

/// Loaded text plus, when the file was longer than the cap, its real size —
/// which the banner names so the truncation is never silent.
///
/// `Debug` is required, not decorative: the tests use `expect_err`, which
/// needs `T: Debug` on the `Ok` side.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub text: String,
    pub truncated: Option<u64>,
}

/// What the worker sends back: the rung it decided on, and the text or the
/// reason there is none.
pub(crate) struct LoadDone {
    pub format: Format,
    pub result: Result<Loaded, String>,
}

/// The extractor's argv, minus the binary name. Pure.
pub(crate) fn argv(e: Extractor, p: &Path) -> Vec<String> {
    let path = p.to_string_lossy().into_owned();
    match e {
        Extractor::TextUtil => vec!["-convert".into(), "txt".into(), "-stdout".into(), path],
        Extractor::PdfToText => vec!["-layout".into(), path, "-".into()],
    }
}

/// Which extractors are on `PATH`. Probed once per process — a tool does not
/// appear mid-session, and `which` is a fork we should not repeat per open.
pub(crate) fn probe() -> Probe {
    use std::sync::OnceLock;
    static PROBE: OnceLock<Probe> = OnceLock::new();
    *PROBE.get_or_init(|| Probe {
        textutil: on_path(Extractor::TextUtil.bin()),
        pdftotext: on_path(Extractor::PdfToText.bin()),
    })
}

fn on_path(bin: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|d| d.join(bin).is_file())
}

/// Read at most `MAX_VIEW_BYTES`, reporting the real size when longer.
fn read_capped(path: &Path) -> std::io::Result<(Vec<u8>, Option<u64>)> {
    let size = std::fs::metadata(path)?.len();
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    f.by_ref()
        .take(MAX_VIEW_BYTES)
        .read_to_end(&mut buf)?;
    let truncated = (size > MAX_VIEW_BYTES).then_some(size);
    Ok((buf, truncated))
}

/// Run an extractor and capture its stdout. A non-zero exit is reported with
/// the tool's own stderr — it knows why it failed and we do not.
fn extract(e: Extractor, path: &Path) -> Result<String, String> {
    let out = std::process::Command::new(e.bin())
        .args(argv(e, path))
        .output()
        .map_err(|err| format!("{}: {err}", e.bin()))?;
    if !out.status.success() {
        let tail = String::from_utf8_lossy(&out.stderr);
        let tail = tail.lines().last().unwrap_or("failed").to_string();
        return Err(format!("{}: {tail}", e.bin()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The whole job, synchronously — detection included. Called on the worker by
/// [`start`], and directly by tests.
pub(crate) fn load_now(path: &Path, probe: Probe) -> LoadDone {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    let (head, truncated) = match read_capped(path) {
        Ok(v) => v,
        Err(e) => {
            return LoadDone {
                format: Format::Opaque { why: Opaque::Binary },
                result: Err(format!("{name}: {e}")),
            }
        }
    };
    let sniff = &head[..head.len().min(SNIFF_BYTES)];
    let format = detect(path, sniff, probe);

    let result = match format {
        // The card is drawn from the path and the format alone; there is no
        // text to carry, and decoding these bytes would be a lie.
        Format::Opaque { .. } => Ok(Loaded { text: String::new(), truncated: None }),
        Format::Extract { via } => extract(via, path)
            .map(|text| Loaded { text, truncated: None })
            .map_err(|e| format!("{name}: {e}")),
        _ => Ok(Loaded {
            text: String::from_utf8_lossy(&head).into_owned(),
            truncated,
        }),
    };
    LoadDone { format, result }
}

/// Load `path` on a worker thread; the result arrives on the returned
/// channel. Dropping the receiver discards the result, which is what closing
/// the pane mid-load should do.
pub(crate) fn start(path: PathBuf) -> Receiver<LoadDone> {
    let (tx, rx) = mpsc::channel();
    let probe = probe();
    std::thread::spawn(move || {
        let _ = tx.send(load_now(&path, probe));
    });
    rx
}

#[cfg(test)]
#[path = "load_tests.rs"]
mod tests;
```

Add to `viewpane/mod.rs`:

```rust
pub(crate) mod load;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p crew-app viewpane::load`
Expected: PASS, 7 tests. They pass whether or not `pdftotext` is installed — nothing asserts on a real extraction.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/viewpane/
git commit -m "feat(view): load and extract off the winit thread"
```

---

### Task 3: The `ViewPane` model and its load state

**Files:**
- Create: `crates/crew-app/src/viewpane/pane.rs`
- Create: `crates/crew-app/src/viewpane/pane_tests.rs`
- Modify: `crates/crew-app/src/viewpane/mod.rs`

**Interfaces:**
- Consumes: `load::{start, LoadDone, Loaded}`, `detect::Format`.
- Produces: `ViewPane { path, state, scroll, raw }`, `LoadState`, `ViewPane::open(PathBuf) -> Self`, `ViewPane::poll(&mut self) -> bool`, `ViewPane::loading(&self) -> bool`, `ViewPane::reload(&mut self)`. Task 4 renders from `state`; Task 8 constructs via `open`; Task 10 calls `reload`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/viewpane/pane_tests.rs`:

```rust
use super::*;

#[test]
fn a_new_pane_is_loading_and_shows_no_content_yet() {
    let p = ViewPane::open(std::env::temp_dir().join("whatever.txt"));
    assert!(p.loading(), "the pane opens before the file is read");
}

#[test]
fn poll_swaps_loading_for_ready_and_reports_the_change() {
    let dir = std::env::temp_dir().join(format!("crew-viewpane-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("ready.txt");
    std::fs::write(&f, "content\n").unwrap();

    let mut p = ViewPane::open(f);
    // Spin until the worker lands; poll() is non-blocking by design.
    let mut changed = false;
    for _ in 0..500 {
        if p.poll() {
            changed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(changed, "poll reports the transition exactly once");
    assert!(!p.loading(), "the pane is no longer loading");
    assert!(!p.poll(), "a settled pane reports no further change");
}

#[test]
fn a_failed_load_lands_in_the_pane_not_a_status_line() {
    // The pane is already on screen by the time this fails, so reporting only
    // to a status line the user may never look at would lose the message.
    let mut p = ViewPane::open(std::path::PathBuf::from("/nonexistent/gone.txt"));
    for _ in 0..500 {
        if p.poll() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    match &p.state {
        LoadState::Failed(msg) => assert!(msg.contains("gone.txt"), "names the file: {msg}"),
        _ => panic!("a missing file must settle as Failed"),
    }
}

#[test]
fn reload_returns_the_pane_to_loading() {
    let mut p = ViewPane::open(std::env::temp_dir().join("x.txt"));
    p.state = LoadState::Failed("stale".into());
    p.reload();
    assert!(p.loading(), "reload re-arms the worker");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app viewpane::pane`
Expected: FAIL — unresolved `ViewPane`.

- [ ] **Step 3: Implement `pane.rs`**

Create `crates/crew-app/src/viewpane/pane.rs`:

```rust
//! The viewer pane's model: a path, a load state, and where it is scrolled.
//! Deliberately thin — rendering lives in `render`, key decoding in `keys`.
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use super::detect::Format;
use super::load::{self, Loaded};

/// Where a pane is between "you pressed the key" and "the bytes are here".
/// `Loading` holds the channel so `poll` can drain it without the app owning
/// a side table of in-flight loads.
pub(crate) enum LoadState {
    Loading { since_ms: u64, rx: Receiver<load::LoadDone> },
    Ready { format: Format, loaded: Loaded },
    Failed(String),
}

/// Wrapped lines for the last width this pane rendered at. Rebuilt on a width
/// change, a reload, or the `s` toggle — never per frame.
pub(crate) struct ViewCache {
    pub cols: u16,
    pub raw: bool,
    pub lines: Vec<crate::chatbody::CardLine>,
}

pub(crate) struct ViewPane {
    pub path: PathBuf,
    pub state: LoadState,
    /// Rows scrolled from the top, clamped to content by `clamp_scroll`.
    pub scroll: usize,
    /// `s`: show the text unrendered. The escape hatch for when the render is
    /// the thing being debugged.
    pub raw: bool,
    pub(crate) cache: RefCell<Option<ViewCache>>,
}

impl ViewPane {
    /// Open `path`: the worker starts immediately and the pane is on screen
    /// before a single byte has been read.
    pub(crate) fn open(path: PathBuf) -> Self {
        let rx = load::start(path.clone());
        Self {
            path,
            state: LoadState::Loading { since_ms: crate::anim::now_ms(), rx },
            scroll: 0,
            raw: false,
            cache: RefCell::new(None),
        }
    }

    pub(crate) fn loading(&self) -> bool {
        matches!(self.state, LoadState::Loading { .. })
    }

    /// Drain the worker channel. Returns `true` on the tick the state changed,
    /// which is what tells `poll_panes` to redraw.
    pub(crate) fn poll(&mut self) -> bool {
        let LoadState::Loading { rx, .. } = &self.state else {
            return false;
        };
        let Ok(done) = rx.try_recv() else {
            return false;
        };
        self.state = match done.result {
            Ok(loaded) => LoadState::Ready { format: done.format, loaded },
            Err(msg) => LoadState::Failed(msg),
        };
        self.cache.replace(None);
        self.scroll = 0;
        true
    }

    /// Re-read from disk, keeping the pane in place. Used by `r` and by the
    /// `$EDITOR` handoff when the editor exits.
    pub(crate) fn reload(&mut self) {
        let rx = load::start(self.path.clone());
        self.state = LoadState::Loading { since_ms: crate::anim::now_ms(), rx };
        self.cache.replace(None);
    }
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
```

Add to `viewpane/mod.rs`:

```rust
mod pane;
pub(crate) use pane::{LoadState, ViewCache, ViewPane};
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-app viewpane::pane`
Expected: PASS, 4 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/viewpane/
git commit -m "feat(view): the pane model, loading until the worker lands"
```

---

### Task 4: Rendering the text rungs

Every rung produces `Vec<CardLine>` — the same type the chat cards use — so `cells()` is one mapper and each rung is tested as data, not pixels.

**Files:**
- Create: `crates/crew-app/src/viewpane/lines.rs` (rung → `Vec<CardLine>`)
- Create: `crates/crew-app/src/viewpane/lines_tests.rs`
- Create: `crates/crew-app/src/viewpane/render.rs` (`CardLine` → `CellView`)
- Create: `crates/crew-app/src/viewpane/render_tests.rs`
- Modify: `crates/crew-app/src/viewpane/mod.rs`

**Interfaces:**
- Consumes: `LoadState`, `Format`, `Loaded`, `chatbody::{CardLine, plain}`, `chatwidth::fit_end`, `md::syntax`.
- Produces: `lines::for_state(state, raw, cols) -> Vec<CardLine>`, `lines::GUTTER_W`, `ViewPane::cells(cols, rows) -> Vec<CellView>`, `ViewPane::clamp_scroll(cols, rows)`, `ViewPane::lines_for(cols) -> Ref<'_, ViewCache>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/viewpane/lines_tests.rs`:

```rust
use super::*;
use crate::viewpane::detect::{Extractor, Format, Opaque};
use crate::viewpane::load::Loaded;

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

fn ready(format: Format, body: &str) -> LoadState {
    LoadState::Ready {
        format,
        loaded: Loaded { text: body.into(), truncated: None },
    }
}

#[test]
fn code_lines_carry_a_numbered_gutter() {
    let ls = for_state(&ready(Format::Code { lang: "rust" }, "fn a() {}\nfn b() {}\n"), false, 40);
    assert!(text(&ls[0]).starts_with("    1 "), "got {:?}", text(&ls[0]));
    assert!(text(&ls[1]).starts_with("    2 "));
}

#[test]
fn a_wrapped_row_does_not_reprint_its_line_number() {
    let long = "x".repeat(60);
    let ls = for_state(&ready(Format::Code { lang: "" }, &long), false, 20);
    assert!(text(&ls[0]).starts_with("    1 "));
    assert!(
        text(&ls[1]).starts_with("      "),
        "continuation gutter is blank, got {:?}",
        text(&ls[1])
    );
}

#[test]
fn truncation_is_announced_in_a_banner_row() {
    // A cap that bites silently reads as "this is the whole file", which is a
    // lie about the source.
    let state = LoadState::Ready {
        format: Format::Code { lang: "" },
        loaded: Loaded { text: "head\n".into(), truncated: Some(41_000_000) },
    };
    let ls = for_state(&state, false, 60);
    let banner = text(&ls[0]);
    assert!(banner.contains("8 MB"), "names what is shown: {banner}");
    assert!(banner.contains("39"), "names the real size in MB: {banner}");
    assert!(banner.contains("o "), "offers the escape: {banner}");
}

#[test]
fn an_extract_says_it_is_an_extract() {
    let ls = for_state(&ready(Format::Extract { via: Extractor::PdfToText }, "page one\n"), false, 60);
    let banner = text(&ls[0]);
    assert!(banner.contains("text extract"), "got {banner}");
    assert!(banner.contains("o "), "offers the OS app: {banner}");
}

#[test]
fn a_missing_extractor_names_what_to_install() {
    let state = ready(
        Format::Opaque { why: Opaque::NoExtractor(Extractor::PdfToText) },
        "",
    );
    let ls = for_state(&state, false, 60);
    let card: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(card.contains("poppler"), "names the install: {card}");
}

#[test]
fn a_loading_pane_draws_a_skeleton_not_an_empty_page() {
    let ls = for_state(
        &LoadState::Loading {
            since_ms: 0,
            rx: std::sync::mpsc::channel().1,
        },
        false,
        40,
    );
    assert!(!ls.is_empty(), "loading is visible");
}

#[test]
fn a_failure_is_drawn_in_the_pane() {
    let ls = for_state(&LoadState::Failed("gone.txt: not found".into()), false, 40);
    let card: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(card.contains("gone.txt"), "got {card}");
}

#[test]
fn raw_mode_shows_markdown_source_verbatim() {
    let ls = for_state(&ready(Format::Markdown, "# Heading\n"), true, 40);
    assert!(text(&ls[0]).contains("# Heading"), "raw keeps the hash");
}

#[test]
fn diff_ink_differs_between_added_and_removed() {
    let ls = for_state(&ready(Format::Diff, "+added\n-gone\n"), false, 40);
    let add = ls[0].iter().find(|c| c.c == 'a').unwrap().fg;
    let del = ls[1].iter().find(|c| c.c == 'g').unwrap().fg;
    assert_ne!(add, del, "a diff that colours both sides alike is not a diff");
}

#[test]
fn zero_width_never_panics() {
    let _ = for_state(&ready(Format::Code { lang: "" }, "x\n"), false, 0);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app viewpane::lines`
Expected: FAIL — `for_state` not found.

- [ ] **Step 3: Implement `lines.rs`**

Create `crates/crew-app/src/viewpane/lines.rs`. Colour is read at call time from `crew_theme::theme()`; the gutter is `text_muted`, body `ink`, banner `text_muted`, diff `+`/`−` from `ansi[2]`/`ansi[1]`.

```rust
//! Rung → `Vec<CardLine>`. Every format lands in the same representation the
//! chat cards use, so `render` is one mapper and each rung is tested as data.
use crate::chatbody::{plain, CardLine};
use crate::viewpane::detect::{Format, Opaque};
use crate::viewpane::load::{Loaded, MAX_VIEW_BYTES};
use crate::viewpane::LoadState;

/// Width of the line-number gutter, digits plus one space.
pub(crate) const GUTTER_W: usize = 6;

fn row(s: &str, fg: (u8, u8, u8), bold: bool) -> CardLine {
    s.chars().map(|c| plain(c, fg, bold)).collect()
}

/// Hard-wrap `text` at `w` display columns, tagging each row with its 1-based
/// source line (continuations repeat it so the gutter can blank them). Lifted
/// unchanged from `mdcache::wrap_source`, which the deleted source half used.
fn wrap(text: &str, w: usize) -> Vec<(usize, Vec<char>)> {
    let mut out = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let n = i + 1;
        let chars: Vec<char> = line.chars().collect();
        if w == 0 || chars.is_empty() {
            out.push((n, Vec::new()));
            continue;
        }
        let mut s = 0;
        while s < chars.len() {
            let e = crate::chatwidth::fit_end(&chars, s, w);
            out.push((n, chars[s..e].to_vec()));
            s = e;
        }
    }
    out
}

/// Numbered rows for the gutter rungs.
fn numbered(text: &str, cols: usize, ink: (u8, u8, u8), muted: (u8, u8, u8)) -> Vec<CardLine> {
    let w = cols.saturating_sub(GUTTER_W).max(1);
    let mut out = Vec::new();
    let mut last = 0usize;
    for (n, chars) in wrap(text, w) {
        let mut line: CardLine = if n == last {
            row(&" ".repeat(GUTTER_W), muted, false)
        } else {
            row(&format!("{n:>5} "), muted, false)
        };
        last = n;
        line.extend(chars.iter().map(|c| plain(*c, ink, false)));
        out.push(line);
    }
    out
}

/// `+`/`−` ink for diffs; everything else is body ink.
fn diff_lines(text: &str, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    let w = cols.saturating_sub(GUTTER_W).max(1);
    for (n, chars) in wrap(text, w) {
        let head = chars.first().copied().unwrap_or(' ');
        let fg = match head {
            '+' => t.ansi[2],
            '-' => t.ansi[1],
            '@' => t.ansi[6],
            _ => t.ink,
        };
        let mut line: CardLine = row(&format!("{n:>5} "), t.text_muted, false);
        line.extend(chars.iter().map(|c| plain(*c, fg, false)));
        out.push(line);
    }
    out
}

fn banner(msg: &str, cols: usize) -> CardLine {
    let t = crew_theme::theme();
    let mut s: String = msg.chars().take(cols.max(1)).collect();
    while s.chars().count() < cols {
        s.push(' ');
    }
    row(&s, t.text_muted, false)
}

fn mb(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}

/// The metadata card for a rung that cannot be rendered.
fn opaque_card(why: Opaque, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let head = match why {
        Opaque::Binary => "binary file — nothing to render".to_string(),
        Opaque::NotUtf8 => "not valid UTF-8 — nothing to render".to_string(),
        Opaque::NoExtractor(e) => format!("no extractor: install {}", e.install_hint()),
    };
    vec![
        row(&head, t.ink, true),
        Vec::new(),
        row("press  o  to open in the default app", t.text_muted, false),
    ]
    .into_iter()
    .map(|mut l| {
        l.truncate(cols.max(1));
        l
    })
    .collect()
}

/// Lines for the pane's current state at `cols` columns. `raw` shows text
/// unrendered (the `s` toggle); it only changes the `Markdown` rung, since
/// every other rung already shows the bytes as they are.
pub(crate) fn for_state(state: &LoadState, raw: bool, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    match state {
        LoadState::Loading { .. } => vec![banner("loading…", cols)],
        LoadState::Failed(msg) => vec![row(msg, t.ink, false)],
        LoadState::Ready { format, loaded } => ready_lines(*format, loaded, raw, cols),
    }
}

fn ready_lines(format: Format, loaded: &Loaded, raw: bool, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if let Some(real) = loaded.truncated {
        out.push(banner(
            &format!(
                "showing first {} MB of {} MB — press o to open externally",
                mb(MAX_VIEW_BYTES),
                mb(real)
            ),
            cols,
        ));
    }
    let body = match format {
        Format::Opaque { why } => opaque_card(why, cols),
        Format::Extract { via } => {
            out.push(banner(
                &format!("text extract via {} — press o to open the real file", via.bin()),
                cols,
            ));
            numbered(&loaded.text, cols, t.ink, t.text_muted)
        }
        Format::Diff => diff_lines(&loaded.text, cols),
        Format::Markdown if !raw => super::mdrung::lines(&loaded.text, cols),
        Format::Csv { delim } if !raw => super::csv::lines(&loaded.text, delim, cols),
        _ => numbered(&loaded.text, cols, t.ink, t.text_muted),
    };
    out.extend(body);
    out
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
```

**Execution order:** `lines.rs` calls `mdrung::lines` and `csv::lines`, so **Tasks 5 and 6 are implemented before this one** — they depend only on `chatbody` and `md`, not on anything here. Building them first means no temporary shim exists at any point. A rung that silently falls back to plain text is exactly the failure this ladder exists to prevent, and a shim is that failure with a comment on it.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-app viewpane::lines`
Expected: PASS, 10 tests.

- [ ] **Step 5: Write the `render.rs` tests**

Create `crates/crew-app/src/viewpane/render_tests.rs`:

```rust
use crate::viewpane::ViewPane;

fn pane_with(text: &str) -> ViewPane {
    use crate::viewpane::detect::Format;
    use crate::viewpane::load::Loaded;
    use crate::viewpane::LoadState;
    let mut p = ViewPane::open(std::env::temp_dir().join("r.txt"));
    p.state = LoadState::Ready {
        format: Format::Code { lang: "" },
        loaded: Loaded { text: text.into(), truncated: None },
    };
    p
}

#[test]
fn cells_fit_inside_the_grid() {
    let p = pane_with("one\ntwo\nthree\n");
    for c in p.cells(20, 3) {
        assert!(c.col < 20 && c.row < 3, "cell {:?},{:?} escaped", c.col, c.row);
    }
}

#[test]
fn a_zero_sized_grid_draws_nothing_and_does_not_panic() {
    assert!(pane_with("x\n").cells(0, 0).is_empty());
}

#[test]
fn clamp_scroll_pulls_a_wild_offset_back_to_the_last_page() {
    // window_top clamps the VIEW; the stored offset must be clamped too or
    // every later scroll-up tick is dead. (The Shift+End lesson.)
    let mut p = pane_with("a\nb\nc\nd\n");
    p.scroll = 9_000;
    p.clamp_scroll(20, 2);
    assert!(p.scroll <= 3, "offset clamped to content, got {}", p.scroll);
}

#[test]
fn the_cache_is_reused_across_frames_at_one_width() {
    let p = pane_with("a\nb\n");
    let _ = p.cells(30, 5);
    let before = p.cache.borrow().as_ref().map(|c| c.cols);
    let _ = p.cells(30, 5);
    assert_eq!(before, Some(30), "same width keeps the cache");
}
```

- [ ] **Step 6: Implement `render.rs`**

Create `crates/crew-app/src/viewpane/render.rs`:

```rust
//! `CardLine` → `CellView`, plus the width-keyed cache and the scroll clamp.
use std::cell::Ref;

use crew_render::CellView;

use crate::viewpane::lines;
use crate::viewpane::{LoadState, ViewCache, ViewPane};

impl ViewPane {
    /// Lines for `cols`, rebuilding the cache only on a width or mode change.
    pub(crate) fn lines_for(&self, cols: u16) -> Ref<'_, ViewCache> {
        let stale = self
            .cache
            .borrow()
            .as_ref()
            .is_none_or(|c| c.cols != cols || c.raw != self.raw);
        if stale {
            self.cache.replace(Some(ViewCache {
                cols,
                raw: self.raw,
                lines: lines::for_state(&self.state, self.raw, cols as usize),
            }));
        }
        Ref::map(self.cache.borrow(), |c| c.as_ref().expect("just filled"))
    }

    /// Cap the stored offset to the last full page — not merely the drawn
    /// view, or a big jump leaves later scroll ticks dead.
    pub(crate) fn clamp_scroll(&mut self, cols: u16, rows: u16) {
        if cols == 0 || rows == 0 {
            return;
        }
        let len = self.lines_for(cols).lines.len();
        self.scroll = self.scroll.min(len.saturating_sub(rows as usize));
    }

    pub(crate) fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols == 0 || rows == 0 {
            return Vec::new();
        }
        let page_bg = crew_theme::theme().page_bg;
        let cache = self.lines_for(cols);
        let top = self.scroll.min(cache.lines.len().saturating_sub(1));
        let mut out = Vec::new();
        for (r, line) in cache.lines.iter().skip(top).take(rows as usize).enumerate() {
            let mut col = 0u16;
            for cell in line {
                if col >= cols {
                    break;
                }
                out.push(CellView {
                    col,
                    row: r as u16,
                    c: cell.c,
                    fg: cell.fg,
                    bg: cell.bg.unwrap_or(page_bg),
                    bold: cell.bold,
                    italic: cell.italic,
                });
                col += crate::chatwidth::char_w(cell.c).max(1) as u16;
            }
        }
        out
    }

    /// True while the worker has not landed — read by `poll.rs`'s animation
    /// gate so the skeleton animates and, crucially, stops.
    pub(crate) fn animating(&self) -> bool {
        matches!(self.state, LoadState::Loading { .. })
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
```

- [ ] **Step 7: Run the tests**

Run: `cargo test -p crew-app viewpane::`
Expected: PASS, all tasks so far.

- [ ] **Step 8: Commit**

```bash
git add crates/crew-app/src/viewpane/
git commit -m "feat(view): render every rung through one CardLine mapper"
```

---

### Task 5: The markdown rung, at full width

The source half is deleted here. `mdcache::preview_lines` is the whole markdown renderer already; it moves and widens.

**Files:**
- Create: `crates/crew-app/src/viewpane/mdrung.rs`
- Create: `crates/crew-app/src/viewpane/mdrung_tests.rs`
- Modify: `crates/crew-app/src/viewpane/mod.rs` (add `pub(crate) mod mdrung;`)

**Interfaces:**
- Consumes: `md::render`, `chatmd::map_lines`, `crew_theme::theme()`.
- Produces: `mdrung::lines(text: &str, cols: usize) -> Vec<CardLine>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/viewpane/mdrung_tests.rs`:

```rust
use super::*;

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

#[test]
fn markdown_renders_rather_than_showing_its_markers() {
    // The whole point of deleting the source half: a heading reads as a
    // heading, not as "# heading".
    let ls = lines("# Title\n\nbody text\n", 40);
    let all: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(all.contains("Title"));
    assert!(!all.contains("# Title"), "markers are rendered away: {all}");
}

#[test]
fn a_heading_is_bold_and_the_body_is_not() {
    let ls = lines("# Title\n\nbody\n", 40);
    let head = ls.iter().find(|l| text(l).contains("Title")).unwrap();
    let body = ls.iter().find(|l| text(l).contains("body")).unwrap();
    assert!(head.iter().any(|c| c.bold), "heading is emphasised");
    assert!(!body.iter().any(|c| c.bold), "body is not");
}

#[test]
fn content_wraps_inside_the_pane_width() {
    let ls = lines(&format!("{}\n", "word ".repeat(60)), 30);
    for l in &ls {
        let w: usize = l.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
        assert!(w <= 30, "line of width {w} exceeds 30: {:?}", text(l));
    }
}

#[test]
fn a_link_keeps_its_url_on_the_cells() {
    // clickopen recovers the URL from the cell rather than re-parsing.
    let ls = lines("[crew](https://example.com)\n", 40);
    let has = ls.iter().flatten().any(|c| c.link.is_some());
    assert!(has, "link spans carry their target");
}

#[test]
fn zero_width_never_panics() {
    let _ = lines("# x\n", 0);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app viewpane::mdrung`
Expected: FAIL.

- [ ] **Step 3: Implement `mdrung.rs`**

```rust
//! The markdown rung: the render, full width, and nothing else. The
//! source|preview split the old `/md` pane drew is gone — showing markdown
//! source beside its render is a dev tool wearing a reading experience's
//! clothes, and `s` covers the times you genuinely need the bytes.
use crate::chatbody::CardLine;

/// Rendered markdown for `cols` columns.
///
/// `chatmd::map_lines` prepends an unconditional one-column indent to every
/// line (it is shared with the chat card layout), so content is wrapped one
/// column narrower — without that, every width-filling row loses its last
/// column when `cells` draws at `cols`.
pub(crate) fn lines(text: &str, cols: usize) -> Vec<CardLine> {
    let fg = crew_theme::theme().ink;
    let content_w = cols.saturating_sub(1);
    crate::chatmd::map_lines(crate::md::render(text, content_w), content_w, fg)
}

#[cfg(test)]
#[path = "mdrung_tests.rs"]
mod tests;
```

Add `pub(crate) mod mdrung;` to `viewpane/mod.rs`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-app viewpane::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/viewpane/
git commit -m "feat(view): markdown renders full width, source half deleted"
```

---

### Task 6: The CSV rung

`md/table.rs::lines` owns column widths, padding and the header rule but speaks markdown AST. This task parses rows and adapts them; it does not reimplement table layout.

**Files:**
- Create: `crates/crew-app/src/viewpane/csv.rs`
- Create: `crates/crew-app/src/viewpane/csv_tests.rs`
- Modify: `crates/crew-app/src/md/mod.rs` (re-export `table::lines` as `pub(crate)`)
- Modify: `crates/crew-app/src/viewpane/mod.rs` (add `pub(crate) mod csv;`)

**Interfaces:**
- Consumes: `md::table_lines(header, rows, cols) -> Vec<MdLine>`, `chatmd::map_lines`.
- Produces: `csv::lines(text: &str, delim: char, cols: usize) -> Vec<CardLine>`, `csv::parse(text: &str, delim: char) -> Vec<Vec<String>>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/viewpane/csv_tests.rs`:

```rust
use super::*;

#[test]
fn plain_rows_split_on_the_delimiter() {
    assert_eq!(
        parse("a,b\n1,2\n", ','),
        vec![vec!["a".to_string(), "b".into()], vec!["1".into(), "2".into()]]
    );
}

#[test]
fn a_quoted_field_keeps_its_delimiter() {
    assert_eq!(
        parse("name,note\n\"Smith, A\",ok\n", ','),
        vec![
            vec!["name".to_string(), "note".into()],
            vec!["Smith, A".into(), "ok".into()],
        ]
    );
}

#[test]
fn a_doubled_quote_is_one_literal_quote() {
    assert_eq!(parse("a\n\"say \"\"hi\"\"\"\n", ','), vec![
        vec!["a".to_string()],
        vec!["say \"hi\"".to_string()],
    ]);
}

#[test]
fn tabs_work_as_a_delimiter() {
    assert_eq!(parse("a\tb\n", '\t'), vec![vec!["a".to_string(), "b".into()]]);
}

#[test]
fn a_trailing_newline_does_not_make_an_empty_row() {
    assert_eq!(parse("a,b\n", ',').len(), 1);
}

#[test]
fn an_embedded_newline_inside_quotes_ends_the_row() {
    // Documented limitation, asserted so it is a decision and not a surprise:
    // a quoted field spanning lines splits. Rare in practice, and the
    // alternative is a streaming parser this pane does not need.
    assert_eq!(parse("\"two\nlines\"\n", ',').len(), 2);
}

#[test]
fn rendered_columns_align() {
    let ls = lines("name,n\nlong-value,1\nx,2\n", ',', 60);
    let texts: Vec<String> = ls.iter().map(|l| l.iter().map(|c| c.c).collect()).collect();
    assert!(texts.len() >= 3, "header, rule and rows: {texts:?}");
    let a = texts[2].find('1');
    let b = texts.last().unwrap().find('2');
    assert_eq!(a, b, "the second column starts at one column: {texts:?}");
}

#[test]
fn an_empty_file_renders_nothing_and_does_not_panic() {
    assert!(lines("", ',', 40).is_empty());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app viewpane::csv`
Expected: FAIL.

- [ ] **Step 3: Re-export the table layout**

In `crates/crew-app/src/md/mod.rs`, beside the existing `render`/`render_chat`:

```rust
/// Table layout for non-markdown callers (the viewer's CSV rung): column
/// widths, padded cells and the header rule, over spans the caller builds.
pub(crate) fn table_lines(
    header: &[Vec<MdSpan>],
    rows: &[Vec<Vec<MdSpan>>],
    cols: usize,
) -> Vec<MdLine> {
    table::lines(header, rows, cols)
}
```

- [ ] **Step 4: Implement `csv.rs`**

```rust
//! The CSV rung. Row parsing is here; column widths, padding and the header
//! rule are `md/table.rs`'s, reached through `md::table_lines` — this is a
//! CSV *adapter*, not a second table renderer.
//!
//! Limitation, deliberate: a quoted field containing a newline splits across
//! rows. Handling it needs a streaming parser, and a viewer that shows one
//! odd row imperfectly is a better trade than a dependency.
use crate::chatbody::CardLine;
use crate::md::{MdSpan, MdStyle};

/// Split `text` into rows of fields, honouring `"` quoting and `""` escapes.
pub(crate) fn parse(text: &str, delim: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            continue;
        }
        let mut fields = Vec::new();
        let mut cur = String::new();
        let mut quoted = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    cur.push('"');
                    chars.next();
                }
                '"' => quoted = !quoted,
                c if c == delim && !quoted => fields.push(std::mem::take(&mut cur)),
                c => cur.push(c),
            }
        }
        fields.push(cur);
        rows.push(fields);
    }
    rows
}

fn spans(field: &str) -> Vec<MdSpan> {
    vec![MdSpan { text: field.to_string(), style: MdStyle::default() }]
}

/// A column-aligned table for `cols` columns; the first row is the header.
pub(crate) fn lines(text: &str, delim: char, cols: usize) -> Vec<CardLine> {
    let rows = parse(text, delim);
    let Some((head, body)) = rows.split_first() else {
        return Vec::new();
    };
    let header: Vec<Vec<MdSpan>> = head.iter().map(|f| spans(f)).collect();
    let body: Vec<Vec<Vec<MdSpan>>> = body
        .iter()
        .map(|r| r.iter().map(|f| spans(f)).collect())
        .collect();
    let fg = crew_theme::theme().ink;
    let content_w = cols.saturating_sub(1);
    crate::chatmd::map_lines(
        crate::md::table_lines(&header, &body, content_w),
        content_w,
        fg,
    )
}

#[cfg(test)]
#[path = "csv_tests.rs"]
mod tests;
```

**Implementer note:** check `MdSpan`'s actual field names and `MdStyle`'s constructor in `crates/crew-app/src/md/mod.rs` before writing `spans` — if `MdStyle` has no `Default`, build it with every flag false. Do not add a `Default` impl for this.

Add `pub(crate) mod csv;` to `viewpane/mod.rs`.

- [ ] **Step 5: Run to verify pass**

Run: `cargo test -p crew-app viewpane::csv`
Expected: PASS, 8 tests.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/viewpane/ crates/crew-app/src/md/mod.rs
git commit -m "feat(view): csv as a table, reusing md's column layout"
```

---

### Task 7: Keys, scrolling and in-pane search

**Files:**
- Create: `crates/crew-app/src/viewpane/keys.rs`
- Create: `crates/crew-app/src/viewpane/keys_tests.rs`
- Create: `crates/crew-app/src/viewpane/search.rs`
- Create: `crates/crew-app/src/viewpane/search_tests.rs`
- Modify: `crates/crew-app/src/viewpane/mod.rs`, `crates/crew-app/src/viewpane/pane.rs` (add the `search` field)

**Interfaces:**
- Consumes: `ViewPane`.
- Produces: `ViewInput`, `ViewAction { Close, Status(String), Edit(PathBuf), OpenExternal(PathBuf), Reload }`, `view_key(logical: &Key, pressed: bool, ctrl: bool) -> ViewInput`, `ViewPane::on_key(&mut self, event: &KeyEvent, cols: u16, rows: u16, ctrl: bool) -> Option<ViewAction>`, `ViewPane::scroll_wheel(&mut self, cols: u16, rows: u16, lines: i32)`, `search::{Search, find_matches}`. Task 8 matches on `ViewAction` and calls `scroll_wheel`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/viewpane/keys_tests.rs`:

```rust
use super::*;
use winit::keyboard::{Key, NamedKey};

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

#[test]
fn releases_do_nothing() {
    assert_eq!(view_key(&Key::Named(NamedKey::Escape), false, false), ViewInput::Ignore);
}

#[test]
fn escape_closes() {
    assert_eq!(view_key(&Key::Named(NamedKey::Escape), true, false), ViewInput::Close);
}

#[test]
fn scroll_keys_map() {
    assert_eq!(view_key(&Key::Named(NamedKey::ArrowDown), true, false), ViewInput::Down);
    assert_eq!(view_key(&Key::Named(NamedKey::PageUp), true, false), ViewInput::PageUp);
    assert_eq!(view_key(&Key::Named(NamedKey::Home), true, false), ViewInput::Top);
    assert_eq!(view_key(&Key::Named(NamedKey::End), true, false), ViewInput::Bottom);
}

#[test]
fn letter_keys_map_and_ignore_case() {
    assert_eq!(view_key(&ch("e"), true, false), ViewInput::Edit);
    assert_eq!(view_key(&ch("O"), true, false), ViewInput::OpenExternal);
    assert_eq!(view_key(&ch("r"), true, false), ViewInput::Reload);
    assert_eq!(view_key(&ch("s"), true, false), ViewInput::ToggleRaw);
}

#[test]
fn a_ctrl_chord_never_reaches_a_letter_action() {
    // Defense in depth: Ctrl+R must not reload the file out from under a
    // chord the app handles globally.
    for k in ["e", "o", "r", "s"] {
        assert_eq!(view_key(&ch(k), true, true), ViewInput::Ignore);
    }
}

#[test]
fn toggling_raw_flips_the_flag_without_asking_the_app_to_do_anything() {
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.txt"));
    assert!(!p.raw);
    let act = apply(&mut p, ViewInput::ToggleRaw, 40, 10);
    assert!(p.raw, "s toggles");
    assert!(act.is_none(), "no host action needed");
}

#[test]
fn edit_and_open_carry_the_path() {
    let path = std::env::temp_dir().join("k.txt");
    let mut p = crate::viewpane::ViewPane::open(path.clone());
    match apply(&mut p, ViewInput::Edit, 40, 10) {
        Some(ViewAction::Edit(p2)) => assert_eq!(p2, path),
        _ => panic!("e produces Edit with the path"),
    }
    match apply(&mut p, ViewInput::OpenExternal, 40, 10) {
        Some(ViewAction::OpenExternal(p2)) => assert_eq!(p2, path),
        _ => panic!("o produces OpenExternal with the path"),
    }
}

#[test]
fn scrolling_up_at_the_top_stays_at_the_top() {
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.txt"));
    apply(&mut p, ViewInput::Up, 40, 10);
    assert_eq!(p.scroll, 0, "no underflow");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app viewpane::keys`
Expected: FAIL.

- [ ] **Step 3: Implement `keys.rs`**

Mirror `mdkeys.rs` exactly — a `view_key` classifier (winit's `KeyEvent` is `#[non_exhaustive]` and cannot be built in tests) and a pure `apply` beneath it:

```rust
//! Key reduction for the viewer, split into a pure seam the same way
//! `mdkeys`/`farpane::keys` are: `view_key` classifies a winit event into
//! `ViewInput`, and everything below is plain data the tests drive directly.
use std::path::PathBuf;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::viewpane::ViewPane;

/// A Page Up/Down jump, matching `mdkeys::PAGE`.
const PAGE: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewInput {
    Close,
    Up,
    Down,
    PageUp,
    PageDown,
    Top,
    Bottom,
    Edit,
    OpenExternal,
    Reload,
    ToggleRaw,
    Ignore,
}

/// What the viewer asks the host app to do after a key press.
pub(crate) enum ViewAction {
    Close,
    Status(String),
    /// `e` — open `$EDITOR` on this path in a terminal pane.
    Edit(PathBuf),
    /// `o` — hand this path to the OS default application.
    OpenExternal(PathBuf),
    Reload,
}

pub(crate) fn view_key(logical: &Key, pressed: bool, ctrl: bool) -> ViewInput {
    if !pressed {
        return ViewInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::Escape) => ViewInput::Close,
        Key::Named(NamedKey::ArrowUp) => ViewInput::Up,
        Key::Named(NamedKey::ArrowDown) => ViewInput::Down,
        Key::Named(NamedKey::PageUp) => ViewInput::PageUp,
        Key::Named(NamedKey::PageDown) => ViewInput::PageDown,
        Key::Named(NamedKey::Home) => ViewInput::Top,
        Key::Named(NamedKey::End) => ViewInput::Bottom,
        Key::Character(s) if !ctrl => match s.to_ascii_lowercase().as_str() {
            "e" => ViewInput::Edit,
            "o" => ViewInput::OpenExternal,
            "r" => ViewInput::Reload,
            "s" => ViewInput::ToggleRaw,
            _ => ViewInput::Ignore,
        },
        _ => ViewInput::Ignore,
    }
}

/// Apply a classified press, returning an action when the host must act.
pub(crate) fn apply(p: &mut ViewPane, input: ViewInput, cols: u16, rows: u16) -> Option<ViewAction> {
    let scroll = |p: &mut ViewPane, d: i32| {
        p.scroll = p.scroll.saturating_add_signed(d as isize);
        p.clamp_scroll(cols, rows);
    };
    match input {
        ViewInput::Close => return Some(ViewAction::Close),
        ViewInput::Up => scroll(p, -1),
        ViewInput::Down => scroll(p, 1),
        ViewInput::PageUp => scroll(p, -PAGE),
        ViewInput::PageDown => scroll(p, PAGE),
        ViewInput::Top => p.scroll = 0,
        ViewInput::Bottom => scroll(p, i32::MAX / 2),
        ViewInput::Edit => return Some(ViewAction::Edit(p.path.clone())),
        ViewInput::OpenExternal => return Some(ViewAction::OpenExternal(p.path.clone())),
        ViewInput::Reload => return Some(ViewAction::Reload),
        ViewInput::ToggleRaw => {
            p.raw = !p.raw;
            p.cache.replace(None);
        }
        ViewInput::Ignore => {}
    }
    None
}

impl ViewPane {
    pub(crate) fn on_key(
        &mut self,
        event: &KeyEvent,
        cols: u16,
        rows: u16,
        ctrl: bool,
    ) -> Option<ViewAction> {
        let input = view_key(&event.logical_key, event.state.is_pressed(), ctrl);
        apply(self, input, cols, rows)
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-app viewpane::keys`
Expected: PASS, 8 tests.

- [ ] **Step 5: Write the failing search tests**

Create `crates/crew-app/src/viewpane/search_tests.rs`:

```rust
use super::*;

#[test]
fn matches_are_line_indexes_in_order() {
    let lines = ["alpha", "beta", "alpha again"];
    assert_eq!(find_matches(&lines, "alpha"), vec![0, 2]);
}

#[test]
fn matching_ignores_case() {
    let lines = ["Alpha", "BETA"];
    assert_eq!(find_matches(&lines, "beta"), vec![1]);
}

#[test]
fn an_empty_needle_matches_nothing() {
    // Otherwise every line "matches" and n/N walks the whole file uselessly.
    let lines = ["a", "b"];
    assert!(find_matches(&lines, "").is_empty());
}

#[test]
fn next_wraps_at_the_end_and_prev_wraps_at_the_start() {
    let mut s = Search::new("x".into(), vec![2, 7]);
    assert_eq!(s.next(), Some(2));
    assert_eq!(s.next(), Some(7));
    assert_eq!(s.next(), Some(2), "wraps forward");
    assert_eq!(s.prev(), Some(7), "wraps backward");
}

#[test]
fn a_search_with_no_hits_reports_none() {
    let mut s = Search::new("zzz".into(), Vec::new());
    assert_eq!(s.next(), None);
    assert_eq!(s.prev(), None);
}
```

- [ ] **Step 6: Implement `search.rs`**

```rust
//! In-pane search: a pure matcher over rendered line text plus a cursor that
//! wraps. Kept out of `keys` so `n`/`N` behaviour is testable without a pane.

/// Indexes of the lines containing `needle`, case-insensitively. An empty
/// needle matches nothing — matching everything would make `n` walk the whole
/// file for no reason.
pub(crate) fn find_matches(lines: &[impl AsRef<str>], needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let needle = needle.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.as_ref().to_lowercase().contains(&needle))
        .map(|(i, _)| i)
        .collect()
}

/// A live search: the needle, its hits, and where `n`/`N` last landed.
pub(crate) struct Search {
    pub needle: String,
    pub hits: Vec<usize>,
    /// Index into `hits`, `None` before the first `n`.
    at: Option<usize>,
    /// True while the user is still typing the needle.
    pub typing: bool,
}

impl Search {
    pub(crate) fn new(needle: String, hits: Vec<usize>) -> Self {
        Self { needle, hits, at: None, typing: false }
    }

    /// The next hit's line, wrapping at the end.
    pub(crate) fn next(&mut self) -> Option<usize> {
        if self.hits.is_empty() {
            return None;
        }
        let i = match self.at {
            None => 0,
            Some(i) => (i + 1) % self.hits.len(),
        };
        self.at = Some(i);
        Some(self.hits[i])
    }

    /// The previous hit's line, wrapping at the start.
    pub(crate) fn prev(&mut self) -> Option<usize> {
        if self.hits.is_empty() {
            return None;
        }
        let i = match self.at {
            None => self.hits.len() - 1,
            Some(0) => self.hits.len() - 1,
            Some(i) => i - 1,
        };
        self.at = Some(i);
        Some(self.hits[i])
    }
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
```

Add `pub search: Option<search::Search>` to `ViewPane` (initialised `None` in `open`, cleared by `reload`), and `pub(crate) mod search;` to `viewpane/mod.rs`.

- [ ] **Step 7: Wire `/`, `n`, `N` and typing into `keys.rs`**

Add `Slash`, `NextHit`, `PrevHit`, `Char(char)`, `Backspace` to `ViewInput`, classified in `view_key`. In `apply`:

- `Slash` → `p.search = Some(Search::new(String::new(), Vec::new()))` with `typing = true`.
- While `p.search` is `Some` and `typing`, `Char`/`Backspace` edit the needle and recompute hits against the current `lines_for(cols)` text, and `Close` cancels the search rather than the pane (**one Esc cancels the search, a second closes the pane** — otherwise typing a needle and changing your mind kills the pane).
- Enter sets `typing = false` and jumps to the first hit.
- `NextHit`/`PrevHit` set `p.scroll` to the returned line, then `clamp_scroll`.

Add a test for the two-stage Esc:

```rust
#[test]
fn escape_cancels_a_live_search_before_it_closes_the_pane() {
    let mut p = crate::viewpane::ViewPane::open(std::env::temp_dir().join("k.txt"));
    apply(&mut p, ViewInput::Slash, 40, 10);
    assert!(apply(&mut p, ViewInput::Close, 40, 10).is_none(), "first Esc cancels the search");
    assert!(p.search.is_none());
    assert!(
        matches!(apply(&mut p, ViewInput::Close, 40, 10), Some(ViewAction::Close)),
        "second Esc closes the pane"
    );
}
```

- [ ] **Step 8: Add `scroll_wheel`**

`scroll.rs` needs this in Task 8:

```rust
impl ViewPane {
    /// Mouse-wheel scrolling. Positive `lines` scrolls down, matching
    /// `MdPane::scroll_wheel`'s sign convention so `scroll.rs` is unchanged
    /// apart from the variant name.
    pub(crate) fn scroll_wheel(&mut self, cols: u16, rows: u16, lines: i32) {
        self.scroll = self.scroll.saturating_add_signed(lines as isize);
        self.clamp_scroll(cols, rows);
    }
}
```

**Implementer note:** confirm the sign convention by reading `MdPane::scroll_wheel` before deleting it in Task 8. Getting it backwards is invisible to every unit test and instantly obvious in the app.

- [ ] **Step 9: Run, lint, commit**

Run: `cargo test -p crew-app viewpane::`
Expected: PASS.

```bash
cargo clippy -p crew-app --all-targets -- -D warnings
git add crates/crew-app/src/viewpane/
git commit -m "feat(view): keys, wheel scrolling and in-pane search"
```

---

### Task 8: Wire the pane in and retire `PaneContent::Markdown`

The largest task, and the one that must land in a single commit — the enum variant cannot be half-replaced.

**Files:**
- Modify: `crates/crew-app/src/pane.rs:39` (variant), `:114`, `:134`
- Modify: `crates/crew-app/src/keys.rs:188` (routing + action match)
- Modify: `crates/crew-app/src/scroll.rs:62`
- Modify: `crates/crew-app/src/clickopen.rs:99`
- Modify: `crates/crew-app/src/clipboard.rs:90`
- Modify: `crates/crew-app/src/poll.rs:193` (per-pane poll) and `:25` (animation gate)
- Modify: `crates/crew-app/src/windowtitle.rs:23`
- Modify: `crates/crew-app/src/askbar.rs:205` (+ its test)
- Modify: `crates/crew-app/src/spawnmd.rs` → becomes `open_view`
- Modify: `crates/crew-app/src/cmddefs.rs:139` (`/md` description, add `/view`)
- Delete: `mdpane.rs`, `mdpane_view.rs`, `mdpane_tests.rs`, `mdkeys.rs`, `mdcache.rs` and their test siblings
- Test: `crates/crew-app/src/viewpane/open_tests.rs`

**Interfaces:**
- Consumes: everything from Tasks 1–7.
- Produces: `CrewApp::open_view(&mut self, path: &str)`. Tasks 9–11 all call it.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/viewpane/open_tests.rs`:

```rust
use crate::app::CrewApp;
use crate::pane::PaneContent;

#[test]
fn open_view_pushes_a_zoomed_focused_pane() {
    let dir = std::env::temp_dir();
    let f = dir.join("open-view-test.txt");
    std::fs::write(&f, "hi\n").unwrap();
    let mut app = CrewApp::default();
    app.cwd = dir;
    app.open_view("open-view-test.txt");
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::View(_)));
    assert!(app.zoomed, "the viewer opens zoomed");
}

#[test]
fn a_missing_file_opens_no_pane_and_says_why() {
    let mut app = CrewApp::default();
    app.cwd = std::env::temp_dir();
    app.open_view("definitely-not-here.txt");
    assert!(app.panes.is_empty(), "no empty pane for a missing file");
    assert!(
        app.status().contains("definitely-not-here.txt"),
        "the status names the file: {}",
        app.status()
    );
}

#[test]
fn an_empty_argument_is_a_usage_hint() {
    let mut app = CrewApp::default();
    app.open_view("");
    assert!(app.panes.is_empty());
    assert!(app.status().contains("/view"), "got {}", app.status());
}

#[test]
fn a_loading_pane_keeps_the_animation_gate_open_and_a_settled_one_does_not() {
    // wants_animation_frame IS the "an idle crew never repaints" invariant.
    // A skeleton that is not registered will not animate; one that never
    // deregisters burns the GPU forever.
    let dir = std::env::temp_dir();
    let f = dir.join("anim-gate-test.txt");
    std::fs::write(&f, "hi\n").unwrap();
    let mut app = CrewApp::default();
    app.cwd = dir;
    app.open_view("anim-gate-test.txt");
    assert!(
        app.wants_animation_frame(crate::anim::now_ms()),
        "a loading pane animates"
    );
    for _ in 0..500 {
        if let PaneContent::View(v) = &mut app.panes[0].content {
            if v.poll() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        !app.wants_animation_frame(crate::anim::now_ms()),
        "a settled pane stops asking for frames"
    );
}
```

**Implementer note:** `CrewApp::status()` may not exist as a getter — check `status.rs` and use whatever field the existing tests read (`app.status_text`, `app.log.last()`, …). Do not add a getter purely for the test if one is already reachable.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app viewpane::open`
Expected: FAIL.

- [ ] **Step 3: Swap the enum variant**

In `pane.rs`, replace `Markdown(MdPane)` with `View(crate::viewpane::ViewPane)` and update the two `match` arms at `:114` (title text — use the file name) and `:134` (`cells`, now `v.cells(self.grid.cols, self.grid.rows)`).

- [ ] **Step 4: Rewrite `spawnmd.rs` as `open_view`**

Replace the body of `spawn_md_pane` with:

```rust
impl CrewApp {
    /// Open `path` in the viewer, zoomed and focused. An empty path is a
    /// usage hint; a path that does not resolve to a file reports why in the
    /// status bar rather than opening an empty pane.
    ///
    /// The `is_file` check is the one filesystem call this makes on the winit
    /// thread — the same one `clickopen::open_path_token` already makes. Every
    /// byte after it is read on a worker (`viewpane::load`).
    pub(crate) fn open_view(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            self.set_status("usage: /view <path>");
            return;
        }
        let resolved = crate::pathexpand::expand_path(&self.cwd, path);
        if !resolved.is_file() {
            self.set_status(format!("view: not a file: {path}"));
            return;
        }
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            content: PaneContent::View(ViewPane::open(resolved)),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        self.focus_new_pane();
        self.zoomed = true;
        self.redraw();
    }
}
```

Rename the file to `crates/crew-app/src/openview.rs` and update `main.rs`. Point the `/md` command handler and a new `/view` handler at it.

- [ ] **Step 5: Route keys and actions**

In `keys.rs:188`, replace the `Markdown` arm with:

```rust
PaneContent::View(v) => {
    view_action = v.on_key(event, pane.grid.cols, pane.grid.rows, mstate.control_key())
}
```

and the action match with:

```rust
if let Some(action) = view_action {
    use crate::viewpane::ViewAction;
    match action {
        ViewAction::Close => self.close_pane(focused),
        ViewAction::Status(msg) => self.set_status(msg),
        ViewAction::Reload => {
            if let Some(PaneContent::View(v)) = self.panes.get_mut(focused).map(|p| &mut p.content) {
                v.reload();
            }
        }
        ViewAction::OpenExternal(p) => {
            let _ = open::that(&p);
            self.set_status(format!("opening {}", p.display()));
        }
        ViewAction::Edit(p) => self.edit_in_pane(&p.to_string_lossy()),
    }
}
```

- [ ] **Step 6: Poll and the animation gate**

In `poll.rs`, add the per-tick drain beside the other pane drains:

```rust
PaneContent::View(v) => {
    if v.poll() {
        any_changed = true;
    }
}
```

and add to `wants_animation_frame` (`poll.rs:25`), inside the existing `self.panes.iter().any(...)` closure's `match`:

```rust
PaneContent::View(v) => v.animating(),
```

- [ ] **Step 7: Update the remaining call sites**

`scroll.rs:62` → `PaneContent::View(v) => v.scroll_wheel(cols, rows, lines)` (add a small `scroll_wheel` to `keys.rs` that adjusts `scroll` and clamps). `clipboard.rs:90` → add `View(_)` to the no-op arm. `windowtitle.rs:23` → `View(_)`. `askbar.rs:205` → assert on `View(_)`. `clickopen.rs:99` → the markdown link hit-test becomes a `View` arm reading the `link` field off the cell at `(row, col)` via `lines_for`.

- [ ] **Step 8: Delete the old pane**

```bash
git rm crates/crew-app/src/mdpane.rs crates/crew-app/src/mdpane_view.rs \
       crates/crew-app/src/mdpane_tests.rs crates/crew-app/src/mdkeys.rs \
       crates/crew-app/src/mdcache.rs
```

Remove their `mod` lines from `main.rs`. Any `mdcache`/`mdkeys` test file that exists alongside goes too. **Do not leave `MdPane` behind as a "just in case"** — two half-viewers is the rot this task exists to prevent.

- [ ] **Step 9: Update `cmddefs.rs`**

```rust
    Cmd {
        name: "/view",
        desc: "view any file — code, markdown, data, csv, diffs (/view <path>)",
    },
    Cmd {
        name: "/md",
        desc: "view a file (alias for /view)",
    },
```

- [ ] **Step 10: Run the whole suite**

Run: `cargo test -p crew-app`
Expected: PASS. Fix every reference the compiler flags — the ten call sites above are the complete list, so a reference outside them means something was missed in an earlier task.

- [ ] **Step 11: Lint and commit**

```bash
cargo clippy --workspace --all-targets -- -D warnings
git add -A
git commit -m "feat(view): one viewer pane replaces the markdown split"
```

---

### Task 9: `/far` F3/F4 and Cmd+click

Retires the two lies: F3 "View" and F4 "Edit" both call the OS today.

**Files:**
- Modify: `crates/crew-app/src/farpane/keys.rs:20-29` (`FarAction`), `:126-127`, `:385` (`open_selected`)
- Modify: `crates/crew-app/src/faraction.rs:19`
- Modify: `crates/crew-app/src/clickopen.rs:113` (`open_path_token`)
- Test: `crates/crew-app/src/faraction.rs` tests, `crates/crew-app/src/farpane/keys_tests.rs`

**Interfaces:**
- Consumes: `CrewApp::open_view`, `CrewApp::edit_in_pane`.
- Produces: `FarAction::View(PathBuf)`, `FarAction::Edit(PathBuf)`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/crew-app/src/faraction.rs`'s test module:

```rust
#[test]
fn view_opens_the_viewer_rather_than_handing_the_file_to_the_os() {
    let f = std::env::temp_dir().join("far-view-test.txt");
    std::fs::write(&f, "x\n").unwrap();
    let mut app = far_pane_app();
    app.apply_far_action(FarAction::View(f), 0);
    assert!(
        app.panes
            .iter()
            .any(|p| matches!(p.content, PaneContent::View(_))),
        "F3 stays inside crew"
    );
}

#[test]
fn edit_spawns_a_terminal_pane_not_an_os_app() {
    let f = std::env::temp_dir().join("far-edit-test.txt");
    std::fs::write(&f, "x\n").unwrap();
    let mut app = far_pane_app();
    let before = app.panes.len();
    app.apply_far_action(FarAction::Edit(f), 0);
    assert!(app.panes.len() > before, "F4 opens $EDITOR in a pane");
}
```

And in `farpane/keys_tests.rs`, a test that F3 on a selected file yields `FarAction::View` and F4 yields `FarAction::Edit`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app faraction`
Expected: FAIL — no `FarAction::View`.

- [ ] **Step 3: Split the action**

In `farpane/keys.rs`:

```rust
    /// Open a file with the OS default application (a downloaded remote temp
    /// file, and directories).
    Open(PathBuf),
    /// F3 — show the file in the viewer pane, inside crew.
    View(PathBuf),
    /// F4 — open `$EDITOR` on the file in a terminal pane.
    Edit(PathBuf),
```

Split `open_selected` into `view_selected` (F3 → `View`) and `edit_selected` (F4 → `Edit`), keeping the directory and remote-download branches exactly as they are — a remote file still downloads first and then views.

In `faraction.rs`:

```rust
            FarAction::View(path) => self.open_view(&path.to_string_lossy()),
            FarAction::Edit(path) => self.edit_in_pane(&path.to_string_lossy()),
```

- [ ] **Step 4: Point Cmd+click at the viewer**

In `clickopen.rs`, `open_path_token`: replace `self.edit_in_pane(tok)` with `self.open_view(tok)`. Update the doc comment on `resolve_click` (`clickopen.rs:33-37`) — it currently says a file "opens in `$EDITOR`", which becomes wrong. `e` is one key away inside the viewer.

- [ ] **Step 5: Make agent-cited paths in chat clickable**

Today the `Chat` arm of `resolve_click` only follows markdown link URLs, so a path an agent writes in a `/smith` reply is dead text. Extend that arm: when `chatview::link_at` finds no URL, fall through to `token_at` on the clicked row and then `open_path_token` — the same resolution terminal panes get.

Resolution is **lazy, at click time**. Do not pre-scan reply text for paths or `stat` candidates during layout: that would put a filesystem call on the winit thread in the render path, for every line of every transcript.

```rust
#[test]
fn a_path_an_agent_wrote_opens_the_viewer() {
    // The file the agent just changed should be one click away, without
    // leaving the transcript.
    let dir = std::env::temp_dir();
    let f = dir.join("agent-cited.rs");
    std::fs::write(&f, "fn main() {}\n").unwrap();
    // Build a chat pane whose last reply body contains "agent-cited.rs",
    // click that token, and assert a View pane opened.
}
```

**Implementer note:** finish that test against the chat pane's real constructor — `chat_tests.rs` has the existing helpers for building a pane with a reply in it. Reuse them rather than hand-rolling a `ChatPane`.

- [ ] **Step 6: Run and commit**

```bash
cargo test -p crew-app
git add -A
git commit -m "feat(far): F3 views inside crew, F4 edits in a pane"
```

---

### Task 10: The `$EDITOR` handoff reloads the viewer

Without this, `e` silently leaves stale content on screen and reads as a bug.

**Files:**
- Modify: `crates/crew-app/src/viewpane/pane.rs` (add `editor_born: Option<u64>`)
- Modify: `crates/crew-app/src/keys.rs` (the `ViewAction::Edit` arm records the spawned pane)
- Modify: `crates/crew-app/src/poll.rs` (detect the editor pane ending)
- Test: `crates/crew-app/src/viewpane/reload_tests.rs`

**Interfaces:**
- Consumes: `ViewPane::reload`, `TermPane.cmd`, `CrewApp::edit_in_pane`.
- Produces: `CrewApp::reload_views_after_edit(&mut self) -> bool`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_viewer_reloads_when_its_editor_pane_goes_away() {
    let dir = std::env::temp_dir();
    let f = dir.join("reload-after-edit.txt");
    std::fs::write(&f, "before\n").unwrap();
    let mut app = CrewApp::default();
    app.cwd = dir;
    app.open_view("reload-after-edit.txt");
    settle(&mut app);

    // Pretend an editor pane was spawned for it and has since exited: a
    // born_ms no live terminal pane carries.
    if let PaneContent::View(v) = &mut app.panes[0].content {
        v.editor_born = Some(1);
    }
    std::fs::write(&f, "after\n").unwrap();
    assert!(app.reload_views_after_edit(), "the exit triggers a reload");
    settle(&mut app);
    match &app.panes[0].content {
        PaneContent::View(v) => match &v.state {
            crate::viewpane::LoadState::Ready { loaded, .. } => {
                assert_eq!(loaded.text, "after\n", "the viewer shows the edited file")
            }
            _ => panic!("settled"),
        },
        _ => panic!("still a viewer"),
    }
}
```

Write `settle(&mut app)` as a helper that spins `poll()` on every `View` pane until it reports no change, with a 5s ceiling.

- [ ] **Step 2: Run to verify failure, then implement**

Add `pub editor_born: Option<u64>` to `ViewPane` (initialised `None` in `open`). **Identify the editor pane by its `born_ms`, not its index** — pane indices shift the moment any pane closes, and a stale index reloads the wrong viewer or panics.

In the `ViewAction::Edit` arm of `keys.rs`, call `edit_in_pane`, then record the new pane's `born_ms` on the viewer:

```rust
ViewAction::Edit(p) => {
    self.edit_in_pane(&p.to_string_lossy());
    let born = self.panes.last().map(|p| p.born_ms);
    if let Some(PaneContent::View(v)) = self.panes.get_mut(focused).map(|p| &mut p.content) {
        v.editor_born = born;
    }
}
```

Add to `poll.rs`, called once per tick from `poll_panes`:

```rust
    /// A viewer whose `$EDITOR` pane has ended re-reads the file. Without this
    /// the handoff leaves stale content on screen, which reads as a bug in the
    /// viewer rather than as a missing step.
    ///
    /// The editor pane is found by `born_ms` rather than by index: closing any
    /// pane shifts every index after it, and this runs a tick later by
    /// definition. Returns whether anything was reloaded, so the caller can
    /// fold it into the tick's `any_changed`.
    pub(crate) fn reload_views_after_edit(&mut self) -> bool {
        // Which editor panes are still alive and still running something.
        let live: Vec<u64> = self
            .panes
            .iter()
            .filter(|p| match &p.content {
                PaneContent::Terminal(t) => t.cmd.is_some(),
                _ => false,
            })
            .map(|p| p.born_ms)
            .collect();

        let mut reloaded = false;
        for pane in &mut self.panes {
            let PaneContent::View(v) = &mut pane.content else {
                continue;
            };
            let Some(born) = v.editor_born else {
                continue;
            };
            if live.contains(&born) {
                continue;
            }
            v.editor_born = None;
            v.reload();
            reloaded = true;
        }
        reloaded
    }
```

**Implementer note:** `TermPane.cmd` is refreshed roughly once a second by `procname`, so the reload lands within about a second of the editor exiting rather than instantly. That is the right trade — polling `procname` faster to shave it would cost every pane, every tick, for a reload nobody is watching the clock on.

- [ ] **Step 3: Run, lint, commit**

```bash
cargo test -p crew-app
git commit -am "feat(view): the editor handoff reloads what it edited"
```

---

### Task 11: Session persistence (droppable)

The pane works without this. If time or review pressure says cut, cut this one.

**Files:**
- Modify: `crates/crew-app/src/sessionsave.rs:17-25` (`SavedPane`), `:78` (`valid`)
- Modify: `crates/crew-app/src/sessionrestore.rs`
- Test: `crates/crew-app/src/sessionsave_tests.rs`

**Interfaces:**
- Consumes: `CrewApp::open_view`.
- Produces: `SavedPane { kind: "view", dir: Some(<full path>) }`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_viewer_is_saved_with_its_path() {
    // `kind` is an open string precisely so an older build skips this rather
    // than failing the whole load.
    let sp = SavedPane { kind: "view".into(), dir: Some("/tmp/x.rs".into()), min: false, remote: false };
    assert!(sp.valid_with(|p| p == std::path::Path::new("/tmp/x.rs")));
}

#[test]
fn a_viewer_on_a_deleted_file_is_dropped_not_restored_empty() {
    let sp = SavedPane { kind: "view".into(), dir: Some("/tmp/gone.rs".into()), min: false, remote: false };
    assert!(!sp.valid_with(|_| false));
}
```

**Implementer note:** `valid()` currently calls the filesystem directly. Either add a `valid_with(exists: impl Fn(&Path) -> bool)` seam and have `valid()` delegate, or write these tests against real temp files. Prefer the seam — the existing `valid()` tests will tell you which way the file already leans.

- [ ] **Step 2: Implement**

Add the `"view"` arm to `valid()` requiring a non-empty `dir` that exists as a **file**; add the save arm producing `kind: "view"` with the full path in `dir`; add the restore arm calling `open_view`.

- [ ] **Step 3: Run, lint, commit**

```bash
cargo test -p crew-app
cargo clippy --workspace --all-targets -- -D warnings
git commit -am "feat(view): restore open viewers with the session"
```

---

## Final verification

- [ ] `cargo test -p crew-app` — whole suite green.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — clean, no new `#[allow]`.
- [ ] `cargo fmt --check` — clean (the pre-commit hook runs this anyway).
- [ ] **Live app**, via the `verify` skill (`.claude/skills/verify`): `/view` a `.rs`, a `.md`, a `.csv`, a `.pdf`, and a binary; check Esc restores the prior focus and zoom, `e` opens `$EDITOR` and the viewer reflects the edit on exit, `o` opens the OS app, and `/far` F3 no longer leaves crew.
- [ ] **Idle check:** with a settled viewer open and `Motion` at its default, confirm the app is not repainting every frame — the 0.9.0 invariant.
