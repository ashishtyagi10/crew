# Agent Grid Wiring — Implementation Plan (Plan 2 of 4)

> *Historical record: this plan predates the Crew pivot and targets editor crates that have since been removed.*

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Wire the `legacy-core::grid` engine (Plan 1) into the live app so spawned agents render as a near-square grid that fills the canvas, with the 7th+ agent demoting to a minimized thumbnail strip; empty canvas when no agents.

**Architecture:** Replace the `LayoutNode` split-tree as the *main panel surface* with `GridLayout` + `compute_grid_layout`. Terminals get **stable ids** (a monotonic counter) so closing one never reindexes the others. The render path computes grid rects each frame, paints full tiles with the existing `render_terminal`, paints minimized tiles as compact thumbnails, and records every tile's rect in `cached_panel_rects` for mouse hit-testing. File panels/`LayoutNode` are NOT deleted (that is Plan 4) — they are simply no longer rendered in the main path during this plan; the explorer returns as a toggle in Plan 3.

**Tech Stack:** Rust, ratatui/crossterm, the `legacy-core::grid` module from Plan 1. Verification is via a PTY harness driving the real `target/debug/legacy` (no unit tests — `App` is not unit-constructable).

## Global Constraints

- HARD **200-line maximum per `.rs` file**, no exceptions. If an edit would push a file over, split it.
- This plan must keep the app **compiling and runnable** at every task boundary.
- `focused_terminal: Option<usize>` holds a **stable terminal id** after Task 1 (NOT a Vec index). `None` = nothing focused (empty canvas or no agent focused).
- The agent grid uses `legacy_core::{GridLayout, compute_grid_layout, GridRects, MINIMIZED_STRIP_HEIGHT, MAX_FULL_TILES}` exactly.
- Do not touch the `legacy-core::grid` module (Plan 1, frozen) except by calling its public API.
- Verification harness lives at `/tmp/legacy_grid_verify.py` (created in Task 3) and drives the built binary in a PTY at 120×40.

---

### Task 1: Stable terminal ids

Give each `TerminalSession` a stable id so closing a terminal never shifts the others. Keep the existing tree/render working (no visible change) — pure refactor.

**Files:**
- Modify: `crates/legacy-ui/src/components/embedded_terminal/session.rs` (add `pub id` field; set in `spawn`)
- Modify: `crates/legacy-ui/src/app/state.rs` (add `next_terminal_id: usize`)
- Modify: `crates/legacy-ui/src/app/lifecycle.rs` (init `next_terminal_id: 0`)
- Modify: `crates/legacy-ui/src/app/terminals.rs` (assign ids; add lookup helpers; drop index-reindexing)
- Modify: `crates/legacy-ui/src/app/render/panels.rs`, `crates/legacy-ui/src/app/keys/fullscreen.rs`, `crates/legacy-ui/src/app/mouse/hit_test.rs`, `crates/legacy-ui/src/app/dispatch/control.rs` (replace `terminals.get*(tid)` index access with id lookups)

**Interfaces:**
- Produces on `TerminalSession`: `pub id: usize` (set from the spawn caller).
- Produces on `App`: `pub(crate) fn terminal_by_id(&self, id: usize) -> Option<&TerminalSession>` and `pub(crate) fn terminal_by_id_mut(&mut self, id: usize) -> Option<&mut TerminalSession>` (linear scan over `self.terminals` matching `.id`).
- Changes meaning: `PanelLeaf::Terminal(usize)` and `focused_terminal: Option<usize>` now carry a stable id.

- [ ] **Step 1: Add `id` to `TerminalSession`**

In `session.rs`, add `pub id: usize,` to the struct. Change `spawn`'s signature to accept it as the first param: `pub fn spawn(id: usize, cmd: &str, args: &[&str], cwd: &std::path::Path, rows: u16, cols: u16, waker: Option<OutputWaker>) -> anyhow::Result<Self>` and set `id` in the returned `Self`.

- [ ] **Step 2: Add the counter + helpers on `App`**

In `state.rs` add field `pub(super) next_terminal_id: usize,`. In `lifecycle.rs` initialize `next_terminal_id: 0,`. In `terminals.rs` add the two lookup helpers (linear scan) and update `spawn_embedded_terminal` to allocate an id:

```rust
pub(crate) fn terminal_by_id(&self, id: usize) -> Option<&TerminalSession> {
    self.terminals.iter().find(|t| t.id == id)
}
pub(crate) fn terminal_by_id_mut(&mut self, id: usize) -> Option<&mut TerminalSession> {
    self.terminals.iter_mut().find(|t| t.id == id)
}
```

In `spawn_embedded_terminal`: replace `let terminal_id = self.terminals.len();` with `let terminal_id = self.next_terminal_id; self.next_terminal_id += 1;`, and pass `terminal_id` as the new first arg to `TerminalSession::spawn`.

In `close_terminal`: remove the `adjust_terminal_ids` call and the `id > terminal_id` shifting branch; remove the terminal by id (`self.terminals.retain(|t| t.id != terminal_id);`) and set `focused_terminal = None` if it matched. Keep `self.layout.remove_terminal(terminal_id)` (the tree still stores ids). Remove the now-unused `adjust_terminal_ids` usage (leave the `LayoutNode` method in legacy-core; just stop calling it).

- [ ] **Step 3: Replace index access with id lookups**

In `render/panels.rs`, `keys/fullscreen.rs`, `mouse/hit_test.rs`, `dispatch/control.rs`: replace every `self.terminals.get(*tid)` / `get_mut(*tid)` (where `tid` came from a `PanelLeaf::Terminal` or `focused_terminal`) with `self.terminal_by_id(*tid)` / `self.terminal_by_id_mut(*tid)`. The `cycle_focus` logic in `terminals.rs` already compares `PanelLeaf::Terminal(tid)` by value — that still works with stable ids.

- [ ] **Step 4: Build + behavior check**

Run: `cargo build && cargo clippy -p legacy-ui -- -W clippy::all`
Expected: compiles; no new warnings.

Manual PTY check (the app must still behave as before): launch the binary in a PTY with a temp `$HOME`, open two shells (`/shell`⏎ twice), close the first (focus it, `Ctrl-W`), confirm the second still renders and accepts input. (This is the case that the old reindexing made fragile.) Capture the final frame; confirm a live `bash` prompt is present.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "refactor(ui): stable terminal ids (no reindex on close)"
```

---

### Task 2: Maintain `GridLayout` on the App

Add the grid state and keep it in sync with terminal lifecycle. Not yet rendered — purely state.

**Files:**
- Modify: `crates/legacy-ui/src/app/state.rs` (add `grid: GridLayout`)
- Modify: `crates/legacy-ui/src/app/lifecycle.rs` (init `grid: legacy_core::GridLayout::new()`)
- Modify: `crates/legacy-ui/src/app/terminals.rs` (`add` on spawn, `remove` on close, `touch` on focus + on output)

**Interfaces:**
- Produces on `App`: `pub(super) grid: legacy_core::GridLayout`.
- Consumes: `GridLayout::{add, remove, touch, full, minimized}` from Plan 1.

- [ ] **Step 1: Add + init the field**

`state.rs`: `pub(super) grid: legacy_core::GridLayout,`. `lifecycle.rs`: `grid: legacy_core::GridLayout::new(),`.

- [ ] **Step 2: Wire lifecycle**

In `terminals.rs`:
- `spawn_embedded_terminal`, after pushing the session and setting `focused_terminal = Some(terminal_id)`: `self.grid.add(terminal_id);`
- `close_terminal`: `self.grid.remove(terminal_id);`
- `cycle_focus`, in the `PanelLeaf::Terminal(tid)` arm where focus moves to a terminal: `self.grid.touch(tid);`
- `poll_terminals`: when a terminal produced output, also `self.grid.touch(<its id>)` is NOT wanted for every byte (it would thrash LRU). Instead, only `touch` on focus change. Leave `poll_terminals` ordering alone. (Rationale: LRU is by *focus* recency, which is the user-meaningful "active" — output alone shouldn't reorder tiles under the user.)

- [ ] **Step 3: Build**

Run: `cargo build && cargo clippy -p legacy-ui -- -W clippy::all`
Expected: compiles, no new warnings, no behavior change yet.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(ui): maintain GridLayout across terminal lifecycle"
```

---

### Task 3: Render the agent grid as the canvas

Replace the main panel surface with the grid of full tiles. Empty canvas when no agents.

**Files:**
- Modify: `crates/legacy-ui/src/app/render/mod.rs` (compute grid rects instead of `LayoutNode::compute_rects`; feed a new renderer)
- Modify: `crates/legacy-ui/src/app/render/panels.rs` (new `render_agent_grid` that paints full tiles + records rects)
- Create: `/tmp/legacy_grid_verify.py` (PTY verification harness)

**Interfaces:**
- Produces on `App`: `pub(super) fn render_agent_grid(&mut self, frame: &mut Frame, area: Rect)` — computes `compute_grid_layout(area, &self.grid)`, paints each full tile with `render_terminal`, resizes each tile's PTY to its inner size, records `(PanelLeaf::Terminal(id), rect)` into `self.cached_panel_rects` for every full tile (minimized handled in Task 4).
- Consumes: `legacy_core::{compute_grid_layout, GridRects}`.

- [ ] **Step 1: Add `render_agent_grid`**

In `panels.rs`, add a method that replaces the old leaf loop for the main surface:

```rust
pub(super) fn render_agent_grid(&mut self, frame: &mut Frame, area: Rect) {
    use legacy_core::compute_grid_layout;
    let layout = compute_grid_layout(area, &self.grid);
    self.cached_panel_rects.clear();
    for (id, rect) in &layout.full {
        let inner_h = rect.height.saturating_sub(2);
        let inner_w = rect.width.saturating_sub(2);
        if inner_h > 0 && inner_w > 0 {
            if let Some(term) = self.terminal_by_id_mut(*id) {
                term.resize(inner_h, inner_w);
            }
        }
        let is_focused = self.focused_terminal == Some(*id);
        if let Some(term) = self.terminal_by_id(*id) {
            crate::components::embedded_terminal::render_terminal(frame, *rect, term, is_focused);
        }
        self.cached_panel_rects
            .push((legacy_core::PanelLeaf::Terminal(*id), *rect));
    }
    // Task 4 will render layout.minimized here.
}
```

(Note: the two-step `_mut` resize then `&` render avoids a double mutable borrow.)

- [ ] **Step 2: Call it from `render`**

In `render/mod.rs`, replace these lines:
```rust
let panel_rects = self.layout.compute_rects(main_chunks[0]);
self.cached_panel_rects = panel_rects.clone();
...
self.render_panel_leaves(frame, &panel_rects);
```
with:
```rust
self.render_agent_grid(frame, main_chunks[0]);
```
(The grid renderer now owns `cached_panel_rects`. Leave `render_panel_leaves` defined for now — it is unused in the main path but referenced by nothing else; if clippy flags it dead, add `#[allow(dead_code)]` with a `// removed in Plan 4` note, or delete it and its file if nothing else uses it.)

- [ ] **Step 3: Create the verification harness**

Create `/tmp/legacy_grid_verify.py` — a PTY driver (120×40) that: sets a temp `$HOME`, launches `target/debug/legacy`, spawns N shells via `/shell⏎`, waits, captures the raw frame, strips ANSI, and reports the **distinct top-border `y` rows** and **distinct left-border `x` columns** of the agent tiles (so the grid shape is observable). Model it on the smoothness harness pattern: `pty.openpty`, `TIOCSWINSZ` 40×120, fork/exec with `HOME`/`TERM=xterm-256color`/`SHELL=/bin/bash`, `select`-loop reader, regex `\x1b\[[0-9;?]*[ -/]*[@-~]` → strip. Count tile borders by counting occurrences of the box-drawing top-left corner the renderer uses (the `Block` border draws `┌`/`╭`). Print, for N in {1,2,3,4}, the number of distinct tile origins detected.

- [ ] **Step 4: Build + verify the grid shape**

Run: `cargo build` then `python3 /tmp/legacy_grid_verify.py`.
Expected: 1 agent → 1 tile filling the area; 2 → two tiles side by side (2 distinct x origins, 1 y); 3 → 2 origins on row 1 + 1 on row 2; 4 → 2×2 (2 x-origins, 2 y-origins). Capture the harness output as evidence. Also confirm: with **zero** agents the canvas is empty (no panic, just the status/command/fn bars).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(ui): render spawned agents as a grid canvas"
```

---

### Task 4: Minimized thumbnail strip

Render agents 7+ as compact thumbnails in the reserved bottom strip; focusing one promotes it.

**Files:**
- Create: `crates/legacy-ui/src/components/embedded_terminal/thumbnail.rs` (compact thumbnail renderer)
- Modify: `crates/legacy-ui/src/components/embedded_terminal/mod.rs` (export `render_thumbnail`)
- Modify: `crates/legacy-ui/src/app/render/panels.rs` (render `layout.minimized`, record rects)

**Interfaces:**
- Produces: `pub fn render_thumbnail(frame: &mut Frame, area: Rect, session: &TerminalSession)` — a 1-line-bordered box showing the title + a status glyph (`●` alive / `⚠` has_attention / `✗` exited), truncated to width. No PTY contents.

- [ ] **Step 1: Implement `render_thumbnail`**

```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::session::TerminalSession;

/// Render a minimized agent as a compact titled box (no PTY contents).
pub fn render_thumbnail(frame: &mut Frame, area: Rect, session: &TerminalSession) {
    let (glyph, color) = if !session.alive {
        ("✗", Color::Red)
    } else if session.has_attention {
        ("⚠", Color::Yellow)
    } else {
        ("●", Color::Indexed(240))
    };
    let label = format!(" {} {} ", glyph, session.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color).bg(Color::Black))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width > 0 && inner.height > 0 {
        frame.render_widget(Paragraph::new(label), inner);
    }
}
```

Export it from `mod.rs`: `pub use thumbnail::render_thumbnail;` and `mod thumbnail;`.

- [ ] **Step 2: Render the strip in `render_agent_grid`**

Replace the `// Task 4 will render layout.minimized here.` comment with:
```rust
for (id, rect) in &layout.minimized {
    if let Some(term) = self.terminal_by_id(*id) {
        crate::components::embedded_terminal::render_thumbnail(frame, *rect, term);
    }
    self.cached_panel_rects
        .push((legacy_core::PanelLeaf::Terminal(*id), *rect));
}
```

- [ ] **Step 3: Build + verify the strip**

Extend `/tmp/legacy_grid_verify.py` to spawn **7** shells and confirm: 6 full tiles in the grid region + a bottom strip (`MINIMIZED_STRIP_HEIGHT = 3` rows) containing 1 thumbnail; the least-recently-focused agent is the minimized one. Run `cargo build && python3 /tmp/legacy_grid_verify.py`; capture output.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(ui): minimized agent thumbnail strip"
```

---

### Task 5: Focus cycling + mouse over the grid model

Make `Tab`/`F4` cycle through agent tiles, clicking a tile focus it (promoting a minimized one), and the status bar reflect the grid.

**Files:**
- Modify: `crates/legacy-ui/src/app/terminals.rs` (`cycle_focus` walks `self.grid` order, not `LayoutNode::leaves`)
- Modify: `crates/legacy-ui/src/app/mouse/hit_test.rs` (click on a `Terminal(id)` rect → focus + `grid.touch(id)`)
- Modify: `crates/legacy-ui/src/app/chrome.rs` or wherever `render_status_bar` lives (show focused agent / count) — locate with `grep -rn "fn render_status_bar" crates/legacy-ui/src`

**Interfaces:**
- `cycle_focus` cycles over `self.grid.full()` then `self.grid.minimized()` (full order then minimized order), wrapping; focusing a minimized id calls `self.grid.touch(id)` so it promotes into the full set on the next frame.

- [ ] **Step 1: Rewrite `cycle_focus`**

Replace the `LayoutNode::leaves`-based body with iteration over the grid order:
```rust
pub(super) fn cycle_focus(&mut self) {
    let order: Vec<usize> = self
        .grid
        .full()
        .iter()
        .chain(self.grid.minimized().iter())
        .copied()
        .collect();
    if order.is_empty() {
        self.focused_terminal = None;
        return;
    }
    let next = match self.focused_terminal {
        Some(cur) => {
            let i = order.iter().position(|x| *x == cur).unwrap_or(0);
            order[(i + 1) % order.len()]
        }
        None => order[0],
    };
    self.focused_terminal = Some(next);
    self.grid.touch(next);
    if let Some(t) = self.terminal_by_id_mut(next) {
        t.has_attention = false;
    }
}
```

- [ ] **Step 2: Update mouse hit-testing**

In `mouse/hit_test.rs`, where a click resolves to a `PanelLeaf::Terminal(id)` rect: set `self.focused_terminal = Some(id); self.grid.touch(id);` (drop any `active_panel`/file-panel handling on the main surface — there are no file panels in the main path now; leave file-panel hit-testing code in place but it will not match since `cached_panel_rects` only holds terminals).

- [ ] **Step 3: Status bar**

Update `render_status_bar` to show e.g. `"<focused title> · N agents"` (or `"no agents"` when empty), reading `self.grid.len()` and the focused terminal's title. Keep it within the existing single status line.

- [ ] **Step 4: Build + verify**

Run `cargo build && cargo clippy -p legacy-ui -- -W clippy::all`. Then drive the harness: spawn 3 agents, send `Tab` twice, confirm the focused tile (cyan border) advances; spawn a 7th, `Tab` to the minimized one, confirm it promotes into the grid next frame. Capture frames.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(ui): grid-aware focus cycling and mouse selection"
```

---

## Self-Review

- **Spec coverage:** grid canvas (T3), 6-cap minimize strip (T4), empty canvas (T3 step 4), focus/mouse (T5), stable ids so close doesn't corrupt the grid (T1), LRU maintained (T2). ✅
- **Altitude:** integration tasks verified by driving the real app (PTY harness), not unit tests — `App` has no test constructor. Each task leaves the app compiling + runnable.
- **Out of scope (later plans):** hideable explorer / file sidebar return (Plan 3); deleting file-manager subsystems + `LayoutNode` (Plan 4); prefix-key navigation (Plan 3 — `Tab`/`Ctrl-W` remain the interim controls).
- **200-line watch:** `panels.rs` grows (new `render_agent_grid`); if it nears 200, move `render_agent_grid` into a new `render/agent_grid.rs`. `thumbnail.rs` is small and standalone.

## Open risk for the executor
`render_panel_leaves` and the `LayoutNode` field become unused in the main path after T3. Do not delete them here (Plan 4 owns that demolition) unless clippy's dead-code lint blocks the build — in which case `#[allow(dead_code)]` them with a `// Plan 4: remove` marker rather than deleting, to keep this plan's diff focused.
