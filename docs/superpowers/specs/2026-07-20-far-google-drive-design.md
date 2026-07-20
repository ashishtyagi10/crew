# Google Drive browsing in `/far` — Design

**Date:** 2026-07-20
**Status:** Approved, pre-implementation

## Goal

Let a `/far` panel be re-rooted into Google Drive so the dual-pane file
manager can browse and operate on Drive exactly like the local disk —
list, view/edit, copy, move, delete, mkdir — with the other panel still
on local disk (or another remote). Full two-way file operations in v1.

## Non-goals (v1)

- Mounting Drive as a local filesystem.
- Any multi-account UI beyond enumerating `rclone` remotes.
- Native OAuth — explicitly delegated to rclone (see below).
- Special-casing Google-native docs export formats (accept rclone's
  default export behavior; see Known gotcha).

## Key decision: rclone is the only Drive dependency

crew shells out to [`rclone`](https://rclone.org) on a worker thread and
reuses far's existing shell-run machinery. **rclone owns the Google OAuth
consent + token storage**, so crew writes zero OAuth code (none exists in
the codebase today — every provider is API-key only). The cost is that
users install rclone and run `rclone config` once to authorize Drive.
This was chosen over native Rust OAuth (much larger, ships a Google client
secret) and over a Drive MCP server (surfaces to the `/crew` agent relay,
not into the `/far` panel UI).

## Architecture

### 1. Pluggable panel location

Today a `Panel` is hardwired to a local `cwd: PathBuf` listed via
`std::fs::read_dir` (`crates/crew-app/src/farpane/list.rs`,
`farpane/mod.rs`). Generalize the panel's location:

```rust
enum Backend {
    Local,
    Rclone { remote: String },   // e.g. "gdrive"
}

struct Location {
    backend: Backend,
    path: String,   // absolute local path, or sub-path within the remote
}
```

All list/copy/move/delete/mkdir operations dispatch on `backend`.
**Local-only operations keep the existing `std::fs` code path** — no
rclone dependency for users who never touch Drive, and no perf regression
for local browsing. rclone is invoked only when a panel is remote or an
operation crosses local↔remote.

### 2. rclone adapter — `farpane/rclone.rs`

Thin functions that build and run rclone argv, one per operation:

| Operation      | rclone command                                  |
|----------------|-------------------------------------------------|
| list           | `rclone lsjson remote:path` → parse into `Entry` |
| list remotes   | `rclone listremotes`                            |
| copy file/dir  | `rclone copyto` (file) / `rclone copy` (dir)    |
| move file/dir  | `rclone moveto` (file) / `rclone move` (dir)    |
| delete         | `rclone delete` (file) / `rclone purge` (dir) — Drive trash on by default |
| mkdir          | `rclone mkdir remote:path`                      |
| download       | `rclone copyto remote:path <temp>`              |
| upload         | `rclone copyto <temp> remote:path`              |

The **command-builders are pure functions** (Location + args → exact
argv). This is the primary unit-tested surface. `lsjson` output
(`Name`, `Size`, `ModTime`, `IsDir`, `MimeType`, `Path`) maps directly
onto the existing `Entry` fields.

### 3. Async model — never block the winit thread

Every rclone call is network I/O and MUST run off the winit render thread
(all work is synchronous on that thread; blocking it freezes every pane).
Reuse the **exact worker-thread + `std::sync::mpsc` + per-tick poll
pattern already in `farpane/ask.rs`** (`AskState` / `FarPane::poll_ask`).

Generalize it to a `PendingOp` per panel:

- While a listing/copy/etc. is in flight, the panel shows a
  `loading gdrive:…` spinner and ignores conflicting input.
- On completion the worker sends its `Result` over the channel; the next
  tick's poll swaps in results (or an error) and refreshes the panel.

### 4. Entering Drive — FAR-style drive select

Alt+F1 (left panel) / Alt+F2 (right panel) opens a modal list overlay:
**"Local disk" + each entry from `rclone listremotes`**. Arrow + Enter
re-roots that panel to the chosen backend at its root (e.g. `gdrive:`).

`listremotes` runs on the worker (fast, but still a subprocess).
Graceful states in the popup:

- rclone binary not found → `rclone not found — install it and run \`rclone config\``
- zero remotes configured → `no rclone remotes configured`

### 5. Operations against Drive

- **F5 copy / F6 move** resolve src and dst to rclone-addressable strings
  whenever either side is remote — covering local→drive, drive→local, and
  drive→drive. Pure local→local keeps the current `std::fs` implementation.
  The op runs on the worker; both panels refresh on completion.
- **F8 delete** → `rclone delete` / `purge` (Drive trash by default),
  behind the same confirm dialog as today.
- **F7 mkdir** → `rclone mkdir`.

### 6. Open / view / edit a remote file (auto-upload on save)

F3-view / F4-edit / Enter-open on a remote entry:

1. Worker downloads to `$TMP/far-drive/<name>` via `rclone copyto`.
2. Open it (viewer / editor / OS default) as with a local file.
3. A lightweight **mtime poll** — folded into the existing tick loop, no
   new filesystem-watch crate — watches the temp file. On a detected
   change, a worker runs `rclone copyto` back to the origin `remote:path`,
   showing an `↑ syncing` indicator.
4. On upload failure, surface the error and keep the temp copy so no edit
   is lost.

### 7. Error handling

All worker operations return `Result`. Failures render in the panel's
status/message line and are non-fatal. rclone-missing is detected at
drive-select time. Network stalls rely on rclone's own timeouts; we
surface the tail of rclone's stderr on failure.

## Testing

- **Unit (bulk of the logic):**
  - Command-builder argv correctness for every operation.
  - `lsjson` JSON fixture → `Entry` parsing.
  - Copy/move src↔dst resolution across all four local/remote combinations.
- **State machine:** drive the `PendingOp` poll loop with an injected fake
  source returning canned results — no network required.
- **Manual:** GUI verify harness (repo `.claude/skills/verify`) for one
  end-to-end pass once a real test remote is configured via `rclone
  config`. Cannot run in CI (needs live credentials) — explicitly manual.

## Known gotcha (documented, not solved in v1)

Google-native docs (Docs/Sheets/Slides) are not byte files. `rclone
lsjson` lists them, and download *exports* them (default docx/xlsx/pptx).
v1 accepts rclone's default export behavior; choosing export formats is
out of scope.

## Reused code patterns

- Async-into-UI: `farpane/ask.rs` (`AskState`, worker + `mpsc`) and
  `FarPane::poll_ask` in `farpane/mod.rs`.
- Shell execution on a worker: `farpane/run.rs`.
- Panel / `Entry` model and listing: `farpane/mod.rs`, `farpane/list.rs`.
- File ops + confirm dialogs + function-key bar: `farpane/fileops.rs`,
  `farpane/keys.rs`.
