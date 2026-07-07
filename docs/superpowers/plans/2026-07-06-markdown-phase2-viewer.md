# Markdown Phase 2 — `/md` Viewer Pane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/md <path>` opens a full-width/height pane showing a markdown file as side-by-side source | preview, with Tab side-switching, independent scrolling, `r` reload, and Cmd/Ctrl+click links.

**Architecture:** New `PaneContent::Markdown(MdPane)` renders hand-built `CellView`s: left half raw source with line numbers, `│` divider, right half `md::render` (CommonMark semantics — NOT `render_chat`) mapped to cells with the same style rules chat uses. Spawn sets `self.zoomed = true` (full width/height per the user's ask). Spec §4: docs/superpowers/specs/2026-07-06-markdown-renderer-design.md.

**Tech Stack:** Everything already on main: `md::render`, `chatmd`-style mapping, `chatwidth`, FarPane's two-panel pattern as reference (farpane/mod.rs:87-99), `cursor_rowcol` + `open_modifier` from phase 1.

## Global Constraints

- Source files ≤200 lines (test files exempt). New pane code goes in `crates/crew-app/src/mdpane.rs` (+ `mdpane_view.rs` if needed), NOT into pane.rs/spawn.rs beyond the minimal arms.
- No new dependencies. TDD every task (failing test first, observed). `cargo fmt` before commit (pre-commit hook).
- The viewer consumes `md::render` (CommonMark: soft breaks join). Style mapping must match chat's (share code with `chatmd` where practical rather than duplicating the style table — extract a reusable span→CardCell mapper if that keeps files ≤200 and chat behavior byte-identical, pinned by the existing chat tests).
- Unreadable/missing/non-UTF-8 file → status-bar error, no pane spawned (`spawn_batch_pane` precedent, spawn.rs:189-215).
- Never touch `crates/crew-app/src/suggest.rs` uncommitted working-tree changes beyond ADDING the `/md` command entry (that file has unrelated uncommitted modifications — stage ONLY your hunk via `git add -p` is NOT available non-interactively; instead: the unrelated changes are in `options_for`/theme tests; your change adds a `Cmd` entry to `COMMANDS` — commit the whole file is FORBIDDEN. Resolution: the controller pre-commits or stashes the unrelated changes before this task runs; implementers must verify `git diff --cached` contains only their hunks and report BLOCKED if suggest.rs shows foreign changes staged.)

---

### Task 1: `MdPane` model + side-by-side cells

**Files:** Create `crates/crew-app/src/mdpane.rs` (+ `mdpane_tests.rs`); register `mod mdpane;`.

**Interfaces (Produces):**
```rust
pub(crate) struct MdPane {
    pub path: std::path::PathBuf,
    pub source: String,
    pub active: Side,            // enum Side { Source, Preview } — Task 3 consumes
    pub scroll_src: usize,       // lines scrolled up from top, clamped
    pub scroll_prev: usize,
}
impl MdPane {
    pub(crate) fn new(path: PathBuf, source: String) -> Self;
    pub(crate) fn cells(&self, cols: u16, rows: u16) -> Vec<crew_render::CellView>;
    pub(crate) fn link_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<String>; // Task 4 consumes
}
```
Layout inside `cells`: left width = `(cols.saturating_sub(1)) / 2`; divider `│` column (muted); right = remainder. Left: source lines hard-wrapped at left width, 4-col right-aligned line numbers (muted) + space, scrolled by `scroll_src`. Right: `md::render(source, right_width)` mapped to styled cells (bold/italic/bg/link per chat's style table), scrolled by `scroll_prev`. Active side marked by a brighter divider or header tint — keep minimal: the divider column uses `ink` for the active side's edge, `text_muted` otherwise is overkill; simplest: no visual marker in Task 1, Task 3 adds a one-cell `▸` indicator at the top of the active side. `link_at`: only meaningful on the preview side (col > divider): map to preview-local col, walk the rendered line's cells by display width (reuse `chatplace::cell_at_col` if visibility allows, else a local copy of its 10 lines with a comment naming the original).

TDD: tests for split geometry (divider at expected col; left shows numbered source; right shows styled preview — find a bold cell from `**x**`), scroll clamping (scroll beyond end → last page), zero/tiny cols no panic, link_at hit/miss on the preview side.
Commit: `feat(crew): MdPane — side-by-side markdown source|preview cells`

### Task 2: pane variant + `/md` command + zoomed spawn

**Files:** Modify `pane.rs` (variant + 3 match arms: `title_text` → file name + `" · md"`, `cells`, key routing stub that ignores keys for now), `spawn.rs` or new `spawnmd.rs` (`spawn_md_pane(&mut self, path: &str)`: resolve vs `self.cwd`, `read_to_string` with status error, push pane, `self.zoomed = true`, focus), `dispatch.rs` (`/md <path>` arm via strip_prefix), `suggest.rs` `COMMANDS` entry `{ name: "/md", desc: "view a markdown file (source | preview)" }` (see Global Constraints re: staging).

TDD: dispatch test (`/md README.md` spawns a Markdown pane and sets zoomed), missing-file test (no pane, status set), suggest completion test (`/m` completes `/md`).
Commit: `feat(crew): /md opens a zoomed markdown viewer pane`

### Task 3: keys — Tab switch, scroll, reload

**Files:** `mdpane.rs` (+ `on_key(&mut self, event) -> bool` handled in pane key routing), wheel routing (grep how scroll_at_cursor dispatches per pane type; Markdown pane scrolls the side under the cursor's half).

Tab flips `active`; Up/Down/PageUp/PageDown scroll active side (clamped); `r` re-reads the file (path kept; read error → status, keep old content); active-side `▸` indicator. Esc/close parity with other panes (check how Chat handles Close and mirror).
TDD: key tests per behavior; reload test with a temp file rewritten between calls.
Commit: `feat(crew): markdown viewer keys — tab switch, scroll, reload`

### Task 4: viewer link clicks

**Files:** `clickopen.rs` — extend the chat fallback branch: `PaneContent::Markdown(m)` → `cursor_rowcol` → `m.link_at(...)` → `open::that` + status.

TDD: same geometry-independent style as chat's link tests (render cells, locate link glyph, resolve). Include a wide-glyph-before-link case on the preview side.
Commit: `feat(crew): cmd/ctrl+click opens links in the markdown viewer`

### Task 5 (controller, inline): workspace green → merge flow → rebuild/install → live smoke

`cargo test --workspace`; finishing-a-development-branch; `cargo build --release`; install; GUI smoke `/md README.md` via harness or user.

## Self-Review Notes
- Spec §4 fully covered (split, zoomed, Tab, scroll, reload, links, error paths). Reuses phase-1 primitives; no engine changes needed. suggest.rs staging hazard called out with controller resolution.
