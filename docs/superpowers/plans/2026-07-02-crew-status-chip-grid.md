# Crew status chip grid Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the crew pane's per-agent pulse lanes (with sparkline + share bar) and the separate roster/activity rows with a single dense, wrapping KPI chip grid; keep the session header line and the turn waterfall.

**Architecture:** A new pure `chatchips.rs` builds one compact colored cluster per agent (`▸planner qwen-max ⠙3.2s 4.1k 38% 42%`), packs clusters left-to-right at equal width (wrapping to fill the pane), and reports how many rows that takes. `chatview.rs` composes three zones — `chathdr::header_cells` (session line, unchanged), the chip grid, then `chatpulse::waterfall_cells`. `ChatPane`'s `top_rows`/`pulse_lanes` are replaced by a chip-grid row count.

**Tech Stack:** Rust, `crew_render::CellView`, existing `crate::chatwidth::{place_row, str_w}` for width-aware placement, `crate::chatroster::agent_color`, `crate::chathdr::fmt_tokens`.

**Spec:** docs/superpowers/specs/2026-07-02-crew-status-chip-grid-design.md

## Global Constraints
- crew-app files stay focused; unit tests in a `#[cfg(test)] mod tests` in-file (match neighbors like `chatpulse.rs`).
- Do NOT delete `spark.rs` or `spark::line_cells` — they're used by `net.rs` and `statspane.rs`'s `cpu_row`. Only remove `chatpulse`'s use of them.
- `header_cells` (the session line) and `waterfall_cells` (the turn timeline) are kept as-is.
- Per-agent chip field order: `marker+name · model · state · tok · ctx% · share%`. Width-drop priority (richest→sparsest): drop `share%`, then `ctx%`, then `tok`, then `model`; the minimum is `marker+name state`. One drop level is chosen for the whole grid so clusters stay uniform.
- Colors from the theme: agent name in `agent_color(name)` (bold while active); values in `theme().text_muted`.
- Commit messages end with: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`. Run `cargo fmt` before each commit (pre-commit runs fmt + check).

---

### Task 1: `chatchips.rs` — cluster text, width-drop, pack math

**Files:**
- Create: `crates/crew-app/src/chatchips.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod chatchips;` beside `mod chatpulse;`)

**Interfaces produced (used by Tasks 2–3):**
- `pub(crate) struct AgentView { pub name: String, pub model: String, pub state: String, pub tok: u64, pub ctx_pct: Option<u8>, pub share_pct: Option<u8>, pub active: bool }`
- `pub(crate) fn cluster_runs(v: &AgentView, level: u8) -> Vec<(String, (u8,u8,u8), bool)>` — (text, color, bold) runs for one cluster at a drop level.
- `pub(crate) fn cluster_width(v: &AgentView, level: u8) -> usize`
- `pub(crate) fn choose_level(views: &[AgentView], cols: u16) -> Option<u8>` — richest level whose widest cluster fits `cols`; `None` if even level 4 doesn't fit.
- `pub(crate) fn per_row(cluster_w: usize, cols: u16) -> usize` — clusters per row (≥1) given a 2-space gutter.
- `pub(crate) fn grid_rows(views: &[AgentView], cols: u16) -> u16` — rows the grid needs (0 when nothing fits).

- [ ] **Step 1: Write the failing tests**

In `crates/crew-app/src/chatchips.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn v(name: &str, active: bool) -> AgentView {
        AgentView {
            name: name.into(),
            model: "qwen-max".into(),
            state: if active { "\u{2819}3.2s".into() } else { "\u{00b7}2\u{00d7}".into() },
            tok: 4_100,
            ctx_pct: Some(38),
            share_pct: Some(42),
            active,
        }
    }

    #[test]
    fn cluster_runs_full_level_has_all_fields_in_order() {
        let runs = cluster_runs(&v("planner", true), 0);
        let joined: String = runs.iter().map(|(s, _, _)| s.as_str()).collect();
        assert!(joined.starts_with("\u{25b8}planner"), "marker+name: {joined}");
        assert!(joined.contains("qwen-max"), "model: {joined}");
        assert!(joined.contains("\u{2819}3.2s"), "state: {joined}");
        assert!(joined.contains("4.1k"), "tok: {joined}");
        assert!(joined.contains("38%"), "ctx: {joined}");
        assert!(joined.contains("42%"), "share: {joined}");
        // Name run is the agent colour and bold while active.
        assert!(runs.iter().any(|(s, _, bold)| s.contains("planner") && *bold));
    }

    #[test]
    fn drop_levels_shed_fields_from_the_right() {
        let a = v("planner", false);
        let full = cluster_runs(&a, 0).iter().map(|(s, _, _)| s.clone()).collect::<String>();
        assert!(full.contains("42%") && full.contains("38%") && full.contains("4.1k"));
        let l1: String = cluster_runs(&a, 1).iter().map(|(s, _, _)| s.clone()).collect();
        assert!(!l1.contains("42%") && l1.contains("38%"), "L1 drops share: {l1}");
        let l2: String = cluster_runs(&a, 2).iter().map(|(s, _, _)| s.clone()).collect();
        assert!(!l2.contains("38%") && l2.contains("4.1k"), "L2 drops ctx: {l2}");
        let l3: String = cluster_runs(&a, 3).iter().map(|(s, _, _)| s.clone()).collect();
        assert!(!l3.contains("4.1k") && l3.contains("qwen-max"), "L3 drops tok: {l3}");
        let l4: String = cluster_runs(&a, 4).iter().map(|(s, _, _)| s.clone()).collect();
        assert!(!l4.contains("qwen-max") && l4.contains("planner"), "L4 drops model: {l4}");
        assert!(l4.contains("2\u{00d7}"), "L4 keeps state: {l4}");
    }

    #[test]
    fn choose_level_prefers_richer_when_wide_and_degrades_when_narrow() {
        let views = vec![v("planner", false), v("coder", false)];
        assert_eq!(choose_level(&views, 200), Some(0), "wide → full");
        let l = choose_level(&views, 22).expect("still fits a minimal cluster");
        assert!(l >= 3, "narrow → a sparse level, got {l}");
        assert_eq!(choose_level(&views, 3), None, "too narrow for any cluster");
    }

    #[test]
    fn grid_rows_packs_multiple_per_row_and_wraps() {
        let views = vec![v("a", false), v("b", false), v("c", false)];
        // Very wide: all three on one row.
        assert_eq!(grid_rows(&views, 240), 1);
        // Enough for one cluster per row: three rows.
        let w = cluster_width(&views[0], choose_level(&views, 26).unwrap()) as u16;
        assert_eq!(grid_rows(&views, w + 2), 3);
        // Nothing fits.
        assert_eq!(grid_rows(&views, 3), 0);
    }

    #[test]
    fn per_row_is_at_least_one_and_accounts_for_the_gutter() {
        assert_eq!(per_row(10, 10), 1);
        assert_eq!(per_row(10, 21), 1); // 10 + 2 gutter + 10 = 22 > 21
        assert_eq!(per_row(10, 22), 2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatchips 2>&1 | tail -5`
Expected: compile error — module/types missing.

- [ ] **Step 3: Implement `chatchips.rs`**

```rust
//! The crew pane's dense agent chip grid: one compact colored cluster per
//! agent (`▸planner qwen-max ⠙3.2s 4.1k 38% 42%`), packed equal-width and
//! wrapped to fill the pane. Replaces the per-agent pulse lanes. Pure text +
//! geometry; `chatview` turns the chosen layout into cells.
use crew_render::CellView;

use crate::chathdr::fmt_tokens;
use crate::chatroster::agent_color;

/// A 2-space gutter between clusters on a row.
const GUTTER: usize = 2;
/// Sparsest drop level (marker+name+state only).
const MAX_LEVEL: u8 = 4;

/// One agent's snapshot for the grid.
pub(crate) struct AgentView {
    pub name: String,
    pub model: String,
    /// Already-formatted state token (e.g. "⠙3.2s", "·2×", "idle").
    pub state: String,
    pub tok: u64,
    pub ctx_pct: Option<u8>,
    pub share_pct: Option<u8>,
    pub active: bool,
}

/// (text, colour, bold) runs for one cluster at `level` (0 = richest).
/// Fields shed from the right as `level` rises: share%, ctx%, tok, model.
pub(crate) fn cluster_runs(v: &AgentView, level: u8) -> Vec<(String, (u8, u8, u8), bool)> {
    let t = crew_theme::theme();
    let color = agent_color(&v.name);
    let mut runs: Vec<(String, (u8, u8, u8), bool)> = Vec::new();
    let marker = if v.active { "\u{25b8}" } else { "\u{25aa}" }; // ▸ / ▪
    runs.push((format!("{marker}{}", v.name), color, v.active));
    if level < 4 && !v.model.is_empty() {
        runs.push((format!(" {}", v.model), t.text_muted, false));
    }
    runs.push((format!(" {}", v.state), t.text_muted, false));
    if level < 3 && v.tok > 0 {
        runs.push((format!(" {}", fmt_tokens(v.tok)), t.text_muted, false));
    }
    if level < 2 {
        if let Some(p) = v.ctx_pct {
            runs.push((format!(" {p}%"), t.text_muted, false));
        }
    }
    if level < 1 {
        if let Some(p) = v.share_pct {
            runs.push((format!(" {p}%"), t.text_muted, false));
        }
    }
    runs
}

/// Rendered width (display columns) of a cluster at `level`.
pub(crate) fn cluster_width(v: &AgentView, level: u8) -> usize {
    cluster_runs(v, level)
        .iter()
        .map(|(s, _, _)| crate::chatwidth::str_w(s))
        .sum()
}

/// The widest cluster across `views` at `level`.
fn max_width(views: &[AgentView], level: u8) -> usize {
    views.iter().map(|v| cluster_width(v, level)).max().unwrap_or(0)
}

/// The richest level (0 best) whose widest cluster fits `cols`; `None` if even
/// the sparsest cluster overflows.
pub(crate) fn choose_level(views: &[AgentView], cols: u16) -> Option<u8> {
    if views.is_empty() {
        return None;
    }
    for level in 0..=MAX_LEVEL {
        if max_width(views, level) <= cols as usize {
            return Some(level);
        }
    }
    None
}

/// Clusters that fit on one row of `cols`, given equal `cluster_w` + gutter.
pub(crate) fn per_row(cluster_w: usize, cols: u16) -> usize {
    let cols = cols as usize;
    if cluster_w == 0 || cluster_w > cols {
        return 1;
    }
    // n clusters need n*w + (n-1)*gutter columns.
    ((cols + GUTTER) / (cluster_w + GUTTER)).max(1)
}

/// Rows the grid needs for `views` at `cols` (0 when nothing fits).
pub(crate) fn grid_rows(views: &[AgentView], cols: u16) -> u16 {
    let Some(level) = choose_level(views, cols) else {
        return 0;
    };
    let w = max_width(views, level);
    let cols_per = per_row(w, cols);
    views.len().div_ceil(cols_per) as u16
}

/// Place the chip grid starting at `start_row`, filling `cols`. Clusters are
/// padded to the chosen level's max width and separated by the gutter, wrapping
/// to new rows. Returns the cells (empty when nothing fits).
pub(crate) fn grid_cells(views: &[AgentView], cols: u16, start_row: u16) -> Vec<CellView> {
    let Some(level) = choose_level(views, cols) else {
        return Vec::new();
    };
    let w = max_width(views, level);
    let cols_per = per_row(w, cols);
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    for (i, v) in views.iter().enumerate() {
        let row = start_row + (i / cols_per) as u16;
        let col0 = (i % cols_per) * (w + GUTTER);
        let mut x = col0 as u16;
        let max_col = (col0 + w) as u16;
        for (s, color, bold) in cluster_runs(v, level) {
            x = crate::chatwidth::place_row(
                x,
                max_col,
                s.chars().map(|c| (c, color)),
                |px, c, fg| {
                    cells.push(CellView {
                        col: px,
                        row,
                        c,
                        fg,
                        bg: t.page_bg,
                        bold,
                        italic: false,
                    });
                },
            );
        }
    }
    cells
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app chatchips 2>&1 | tail -5`
Expected: all Task-1 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatchips.rs crates/crew-app/src/main.rs
git commit -m "feat(crew-app): agent chip grid formatter + pack math

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: ChatPane status-row accounting

**Files:**
- Modify: `crates/crew-app/src/chatview.rs` (the `impl ChatPane` block with `top_rows`/`pulse_lanes`, ~lines 10–32)

**Interfaces:**
- Consumes: Task 1 `chatchips::{AgentView, grid_rows}`.
- Produces: `ChatPane::agent_views(&self) -> Vec<chatchips::AgentView>` and `ChatPane::status_rows(&self, cols: u16, rows: u16) -> u16` (total top rows: 1 session line + grid rows + waterfall row when a turn ran). `top_rows` is redefined in terms of it; `pulse_lanes` is removed.

- [ ] **Step 1: Write the failing test**

Add to `crates/crew-app/src/chat_tests.rs` (reuses the `pane()` helper):

```rust
#[test]
fn status_rows_counts_session_grid_and_waterfall() {
    let mut p = pane();
    p.agents = vec![
        crew_plugin::AgentInfo { name: "planner".into(), role: String::new(), model: "m".into() },
        crew_plugin::AgentInfo { name: "coder".into(), role: String::new(), model: "m".into() },
    ];
    // Idle, wide pane: session line + one grid row (both agents fit), no
    // waterfall yet (no turn has run).
    assert_eq!(p.status_rows(200, 20), 2);
    // A turn ran → the waterfall row is added.
    p.absorb_stats(950, String::new(), 0, 0);
    p.pulse.record_hop("planner", 1200);
    p.pulse.end_turn();
    assert_eq!(p.status_rows(200, 20), 3);
    // Too narrow for any cluster → just the session line.
    assert_eq!(p.status_rows(3, 20), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app status_rows_counts 2>&1 | tail -5`
Expected: FAIL — `status_rows` not defined.

- [ ] **Step 3: Implement**

In `crates/crew-app/src/chatview.rs`, replace the `top_rows` + `pulse_lanes` methods with the following. `tok` is the agent's live context size in tokens (the only real per-agent token number the pane tracks); `ctx_pct` is its fill against the model window (via the existing `crate::ctxlimit::context_limit`); `share_pct` is the agent's share of total reply time across the session (from `agent_stats`).

```rust
    /// One `AgentView` per roster agent, snapshotting live state for the grid.
    pub(crate) fn agent_views(&self) -> Vec<crate::chatchips::AgentView> {
        let names = self.active_names();
        let sum_ms: u64 = self.agent_stats.values().map(|(_, ms)| *ms).sum();
        self.agents
            .iter()
            .map(|a| {
                let active = names.contains(&a.name.as_str());
                let ctx = self.ctx.get(&a.name).copied().unwrap_or(0);
                let ctx_pct = crate::ctxlimit::context_limit(&a.model)
                    .filter(|&l| l > 0)
                    .map(|l| ((ctx * 100) / l).min(100) as u8);
                let agent_ms = self.agent_stats.get(&a.name).map(|(_, ms)| *ms).unwrap_or(0);
                let share_pct = (sum_ms > 0).then(|| ((agent_ms * 100) / sum_ms).min(100) as u8);
                crate::chatchips::AgentView {
                    name: a.name.clone(),
                    model: a.model.clone(),
                    state: self.agent_state_str(&a.name, active),
                    tok: ctx,
                    ctx_pct,
                    share_pct,
                    active,
                }
            })
            .collect()
    }

    /// The state token for an agent chip: live spinner + elapsed while active,
    /// else `·n×` with the reply count, or `idle`.
    fn agent_state_str(&self, name: &str, active: bool) -> String {
        if active {
            if let Some(a) = self.active_agents().iter().find(|a| a.name == name) {
                let f = (a.since.elapsed().as_millis() / 120) as usize % crate::update::SPINNER.len();
                return format!("{}{}s", crate::update::SPINNER[f], a.since.elapsed().as_secs());
            }
        }
        match self.agent_stats.get(name) {
            Some((n, _)) if *n > 0 => format!("\u{00b7}{n}\u{00d7}"),
            _ => "idle".into(),
        }
    }

    /// Total rows consumed above the message body: session line + chip grid +
    /// the turn waterfall (only once a turn has run AND the pane is wide enough
    /// for `waterfall_cells` to draw — it needs `cols >= 30`, so the count must
    /// match or the body sizing drifts). Replaces the old header+lanes /
    /// header+roster+activity accounting.
    pub(crate) fn status_rows(&self, cols: u16, rows: u16) -> u16 {
        if rows < 3 {
            return 0; // too short — plain message fallback
        }
        let grid = crate::chatchips::grid_rows(&self.agent_views(), cols);
        let waterfall = u16::from(!self.pulse.hops().is_empty() && cols >= 30);
        (1 + grid + waterfall).min(rows.saturating_sub(2))
    }

    /// Back-compat name used by callers that only have `rows`; estimates at a
    /// wide pane. `cells` recomputes with real `cols`.
    pub(crate) fn top_rows(&self, rows: u16) -> u16 {
        self.status_rows(u16::MAX, rows)
    }
```

No extra free helpers are needed — `crate::ctxlimit::context_limit(model) -> Option<u64>` and `self.active_agents() -> &[ActiveAgent]` already exist.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-app status_rows_counts 2>&1 | tail -5`
Expected: PASS. Also `cargo test -p crew-app chatview 2>&1 | tail -3` stays green.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatview.rs crates/crew-app/src/chat_tests.rs
git commit -m "feat(crew-app): chip-grid row accounting on ChatPane

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Compose the three zones in `chatview::cells`

**Files:**
- Modify: `crates/crew-app/src/chatview.rs` (the `cells` fn, ~lines 84–140)

**Interfaces:**
- Consumes: Task 1 `chatchips::grid_cells`, Task 2 `status_rows`/`agent_views`; existing `chathdr::header_cells`, `chatpulse::waterfall_cells`.

- [ ] **Step 1: Write the failing test**

Add to `crates/crew-app/src/chat_tests.rs`:

```rust
#[test]
fn cells_render_session_line_agent_chips_and_waterfall() {
    let mut p = pane();
    p.agents = vec![
        crew_plugin::AgentInfo { name: "planner".into(), role: String::new(), model: "qwen".into() },
        crew_plugin::AgentInfo { name: "coder".into(), role: String::new(), model: "qwen".into() },
    ];
    p.absorb_stats(950, String::new(), 0, 0);
    p.pulse.record_hop("planner", 1200);
    p.pulse.end_turn();
    let cells = p.cells(120, 20);
    let text: String = {
        let mut rows: std::collections::BTreeMap<u16, Vec<(u16, char)>> = Default::default();
        for c in &cells { rows.entry(c.row).or_default().push((c.col, c.c)); }
        rows.into_values().map(|mut r| { r.sort(); r.into_iter().map(|(_, c)| c).collect::<String>() }).collect::<Vec<_>>().join("\n")
    };
    assert!(text.contains("crew"), "session line present:\n{text}");
    assert!(text.contains("\u{25b8}planner") || text.contains("\u{25aa}planner"), "planner chip:\n{text}");
    assert!(text.contains("\u{25aa}coder") || text.contains("\u{25b8}coder"), "coder chip:\n{text}");
    assert!(text.contains("turn"), "waterfall row:\n{text}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app cells_render_session 2>&1 | tail -8`
Expected: FAIL — the current `cells` still draws pulse lanes / roster, and the assertions on chip markers/`turn` may not line up (or the roster path draws differently). Confirm it fails before implementing.

- [ ] **Step 3: Implement**

Make exactly two changes to the existing `cells` fn, leaving its message-body tail (from `let bottom = crate::chatinput::composer_rows(rows);` onward — `empty_cells` / `message_cells` + `scrollbar_cells` + `new_pill_cells` + `composer_cells`) UNCHANGED:

1. Change the first line from `let top = pane.top_rows(rows);` to:

```rust
    let top = pane.status_rows(cols, rows);
```

2. Replace the middle block — everything from `let lanes = pane.pulse_lanes(rows);` through the `if lanes > 0 { … pulse_block … } else { … roster_cells … activity_cells … }` (the block that currently ends just before `let bottom = crate::chatinput::composer_rows(rows);`) — with the chip grid + waterfall. The `header_cells(...)` call just above it stays as-is. New middle:

```rust
    // Zone 2: the agent chip grid (rows 1..1+grid).
    let views = pane.agent_views();
    let grid = crate::chatchips::grid_rows(&views, cols);
    cells.extend(crate::chatchips::grid_cells(&views, cols, 1));
    // Zone 3: the turn waterfall below the grid, once a turn ran (and only when
    // wide enough to draw — matches status_rows' cols>=30 gate). `live` is the
    // newest thinking agent's still-growing segment (same expression the old
    // pulse_block used).
    if !pane.pulse.hops().is_empty() && cols >= 30 {
        let live = pane
            .active_agents()
            .last()
            .map(|a| (a.name.as_str(), a.since.elapsed().as_millis() as u64));
        cells.extend(crate::chatpulse::waterfall_cells(cols, 1 + grid, pane.pulse.hops(), live));
    }
```

The `mut cells` binding is the `header_cells(...)` result already present above the old block — keep it; `.extend` onto it. Do not add a `layout_cells` early return here (the existing `cells` fn already handles the `top == 0` fallback at its top via the `top_rows`/`status_rows` value — verify the existing top-of-fn `if top == 0 { … }` guard is intact and now reads `status_rows`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-app cells_render_session 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatview.rs crates/crew-app/src/chat_tests.rs
git commit -m "feat(crew-app): compose crew pane top as session + chip grid + waterfall

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: Remove the dead pulse lanes / roster / activity path; verify

**Files:**
- Modify: `crates/crew-app/src/chatpulse.rs` (remove `lane_cells` and the `use crate::spark::{line_cells, History}` → keep only `History`; keep `waterfall_cells`, `Pulse`, `SPARK_W` only if still referenced — remove `SPARK_W`/`BAR_W`/`CTX_W` if now unused)
- Modify: `crates/crew-app/src/chatview.rs` (remove the now-unused `pulse_block` helper)
- Modify: `crates/crew-app/src/chatroster.rs` / `chatflow.rs` (remove `roster_cells` / `activity_cells` ONLY if no other caller remains — grep first)

**Interfaces:** none produced; this is cleanup + verification.

- [ ] **Step 1: Find dead code**

Run:
```bash
grep -rn "pulse_block\|lane_cells\|roster_cells\|activity_cells" crates/crew-app/src --include=*.rs | grep -v "fn lane_cells\|fn pulse_block\|fn roster_cells\|fn activity_cells"
```
Expected: after Task 3, no non-definition callers of `pulse_block`/`lane_cells`. If `roster_cells`/`activity_cells` still have callers elsewhere (e.g. a tiny-pane fallback), LEAVE them; otherwise remove them and their tests.

- [ ] **Step 2: Delete the dead functions**

Remove `lane_cells` (and its now-unused `line_cells` import, `SPARK_W`/`BAR_W`/`CTX_W` consts if unreferenced) from `chatpulse.rs`; remove `pulse_block` from `chatview.rs`; remove `roster_cells`/`activity_cells` if Step 1 showed no callers. Keep `spark.rs` untouched (used by `net.rs`, `statspane.rs`). Keep `chatpulse::Pulse`, `record_hop`, `end_turn`, `hops`, `hist`, `waterfall_cells`.

- [ ] **Step 3: Verify the whole crate + workspace**

Run each; all must be clean:
```bash
cargo test -p crew-app 2>&1 | tail -3          # all pass
cargo clippy -p crew-app -- -D warnings 2>&1 | grep -E "^warning|^error" || echo CLEAN
cargo test --workspace 2>&1 | grep -E "test result: FAILED" || echo WORKSPACE-GREEN
cargo fmt
```
Expected: crew-app tests pass, clippy CLEAN (no dead-code warnings), workspace green. If clippy flags a leftover unused import/const, remove it.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor(crew-app): drop pulse lanes + legacy roster/activity rows

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
