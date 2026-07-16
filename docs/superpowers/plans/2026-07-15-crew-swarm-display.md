# Swarm Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse the live swarm block to a single status line, move the progress bar's `2/5` label onto it, and delete the timeline in favour of a wall-clock total on the folded record's Σ line.

**Architecture:** Three independent surfaces in `crates/crew-app/src/`. `chatswarm.rs` owns the state (`SwarmStatus`/`SwarmTask`) and gains one shared accessor. `chatswarmview.rs` draws the live line; `chatprog.rs` draws the bar below it; `chatswarmrec.rs` renders the transcript record on fold. Tasks 1→2 are ordered (2 consumes 1's helper); Task 3 is independent.

**Tech Stack:** Rust, winit/wgpu GUI. Cell-based rendering (`crew_render::CellView` = one glyph at a `(col, row)`). No async. Tests are plain `#[test]` in sibling `*_tests.rs` files wired via `#[path]`.

**Spec:** `docs/superpowers/specs/2026-07-15-crew-swarm-display-design.md`

## Global Constraints

- **Never use `.chars().take(n)` to clamp text to a width.** A CJK/emoji glyph occupies 2 display columns. Use `crate::chatwidth::fit_end(&chars, 0, max)` and `crate::chatwidth::str_w(s)`.
- **Row budget and draw must agree.** If a `*_rows` function claims a row, the matching `*_cells` function must draw it, and vice versa. `chatprog_tests::a_pane_too_narrow_for_a_legible_bar_drops_the_row` asserts this parametrically.
- **`now_ms == 0` means "test frame"** in the live block: suppress elapsed, pin the spinner to frame 0.
- **`Instant` cannot be mocked.** Never assert an exact value derived from `Instant::now()`. Pass durations in, or assert presence only.
- **Run the full crate's tests**, not just the file you touched: `cargo test -p crew-app`.
- **Pre-commit hooks run `cargo fmt --check` and `cargo check`.** Run `cargo fmt` before committing or the commit is rejected.
- **Do not touch `chatflow.rs`.** It serves the live roster path and is unrelated despite the name.
- **Do not touch the `@agent` chips** in `chatinput.rs`. They are live roster data and are staying.

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `chatswarm.rs` | State. Gains `SwarmStatus::settled()`; `fold_swarm` passes the run's wall-clock ms. | 1, 3 |
| `chatprog.rs` | The bar. `geom` drops the label; `bar_w` spans `cols - INSET`. | 1 |
| `chatswarmview.rs` | The live line. Rewritten: `focus()` + one-row `block_cells`. | 2 |
| `chatswarmrec.rs` | The folded record. Loses the timeline append and `spans()`; new Σ gate. | 3 |
| `chattimeline.rs` | **Deleted.** | 3 |
| `chattimeline_tests.rs` | **Deleted** (8 tests). | 3 |
| `main.rs:48` | Drop `mod chattimeline;`. | 3 |

---

### Task 1: Share the `(settled, total)` count and strip the bar's label

The bar currently owns `done/total` and prints it as a right-hand label. The status line needs the same numbers, and the two must never disagree — so the count moves onto `SwarmStatus` and the label leaves the bar.

**Files:**
- Modify: `crates/crew-app/src/chatswarm.rs` (add method to `impl SwarmStatus`, near `finished()`)
- Modify: `crates/crew-app/src/chatprog.rs:25-89`
- Test: `crates/crew-app/src/chatprog_tests.rs:60-76`

**Interfaces:**
- Consumes: nothing.
- Produces: `SwarmStatus::settled(&self) -> (usize, usize)` — `(tasks in a terminal state, total tasks)`. `pub(crate)`. Task 2 calls this.

- [ ] **Step 1: Write the failing test**

Add to `crates/crew-app/src/chatprog_tests.rs`:

```rust
#[test]
fn settled_counts_terminal_states_and_is_shared_with_the_line() {
    let mut p = pane_with_swarm(4);
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(s.settled(), (0, 4), "nothing settled yet");

    settle(&mut p, 0, TaskState::Done);
    settle(&mut p, 1, TaskState::Failed);
    settle(&mut p, 2, TaskState::Cancelled);
    let s = p.swarm.as_ref().unwrap();
    // The bar tracks "still moving", not "succeeded" — all three terminal
    // states count.
    assert_eq!(s.settled(), (3, 4));
}

#[test]
fn the_bar_no_longer_draws_a_count_label() {
    // The `2/5` moved to the status line above; the bar spans the full inset
    // width so the two surfaces don't print the same number twice.
    let p = pane_with_swarm(4);
    let t = text(&bar_cells(&p, COLS, 5));
    assert!(!t.contains('/'), "{t}");
    assert_eq!(t.chars().count(), (COLS - INSET) as usize, "{t}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app --lib chatprog 2>&1 | tail -20`
Expected: FAIL — `no method named 'settled' found for struct 'SwarmStatus'`.

- [ ] **Step 3: Add `settled()` to `SwarmStatus`**

In `crates/crew-app/src/chatswarm.rs`, inside `impl SwarmStatus`, add after `task_mut`:

```rust
    /// `(settled, total)` — tasks that have reached a terminal state, over the
    /// plan's size. Terminal means done, failed or cancelled: this counts "how
    /// much of the plan has stopped moving", not "how much succeeded".
    ///
    /// Shared by the progress bar (`chatprog`) and the live status line
    /// (`chatswarmview`) so the bar's fill and the line's `2/5` can never
    /// disagree about the same run.
    pub(crate) fn settled(&self) -> (usize, usize) {
        let done = self
            .tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.state,
                    TaskState::Done | TaskState::Failed | TaskState::Cancelled
                )
            })
            .count();
        (done, self.tasks.len())
    }
```

- [ ] **Step 4: Rewrite `geom` and `bar_cells` in `chatprog.rs`**

Replace `chatprog.rs:22-46` (the `geom` doc comment and function) with:

```rust
/// The bar's geometry for `cols`, or `None` when there's no live run (or no
/// room). Both [`progress_rows`] and [`bar_cells`] route through this, so the
/// row a pane budgets and the row it draws can never disagree.
fn geom(pane: &ChatPane, cols: u16) -> Option<(usize, usize, u16)> {
    let s = pane.swarm.as_ref()?;
    let (done, total) = s.settled();
    if total == 0 {
        return None;
    }
    let bar_w = cols.saturating_sub(INSET);
    (bar_w >= MIN_BAR).then_some((done, total, bar_w))
}
```

Then in `bar_cells`, change the destructuring at `chatprog.rs:57` and delete the label loop at `chatprog.rs:85-87`:

```rust
    let Some((done, total, bar_w)) = geom(pane, cols) else {
        return Vec::new();
    };
```

and remove:

```rust
    for (i, c) in label.chars().enumerate() {
        push(INSET + bar_w + i as u16, c, theme.text_muted, false);
    }
```

- [ ] **Step 5: Update the doc comment and the count assertions**

In `chatprog.rs`, change the `bar_cells` doc comment at `:54-55` from:

```rust
/// Render the bar at `row`: filled cells in the accent, the remainder muted,
/// with a `done/total` count on the right.
```

to:

```rust
/// Render the bar at `row`: filled cells in the accent, the remainder muted.
/// The `done/total` count lives on the status line above (`chatswarmview`).
```

In `chatprog_tests.rs`, rename `bar_fills_as_tasks_settle_and_counts_them` to `bar_fills_as_tasks_settle` and delete its two label assertions (`:64` and `:69`):

```rust
#[test]
fn bar_fills_as_tasks_settle() {
    let mut p = pane_with_swarm(4);
    assert_eq!(filled(&bar_cells(&p, COLS, 5)), 0, "nothing settled yet");

    settle(&mut p, 0, TaskState::Done);
    let quarter = filled(&bar_cells(&p, COLS, 5));
    assert!(quarter > 0, "one of four settled should light some cells");

    settle(&mut p, 1, TaskState::Done);
    assert!(
        filled(&bar_cells(&p, COLS, 5)) > quarter,
        "the bar must grow as more tasks settle"
    );
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p crew-app --lib chatprog 2>&1 | tail -20`
Expected: PASS, all `chatprog` tests including `a_pane_too_narrow_for_a_legible_bar_drops_the_row`.

- [ ] **Step 7: Run the whole crate and commit**

```bash
cargo fmt
cargo test -p crew-app 2>&1 | tail -5
git add crates/crew-app/src/chatswarm.rs crates/crew-app/src/chatprog.rs crates/crew-app/src/chatprog_tests.rs
git commit -m "refactor(chat): share the settled count, strip the bar's label

The status line is about to print 2/5, and two surfaces deriving the same
number independently is how they drift. Move the count onto SwarmStatus and
give the bar its full inset width back."
```

---

### Task 2: Collapse the live block to one status line

**Files:**
- Modify: `crates/crew-app/src/chatswarmview.rs` (whole file)
- Test: `crates/crew-app/src/chatswarmview_tests.rs`

**Interfaces:**
- Consumes: `SwarmStatus::settled(&self) -> (usize, usize)` from Task 1.
- Produces: `swarm_rows(&ChatPane, u16) -> u16` (now 0 or 1) and `block_cells(&ChatPane, cols, top_row, now_ms) -> Vec<CellView>` (unchanged signatures, one row of output).

**Context you need:**

`SwarmTask` (`chatswarm.rs:15`) has `pub title: String`, `pub state: TaskState`, `pub started: Option<Instant>`. A task in `TaskState::Running` **always** has `started == Some(_)` — both `AgentSpawned` and `TaskStateChanged(Running)` stamp it via `get_or_insert_with` (`chatswarm.rs:69-79`). The sort below still orders unstamped tasks last, defensively.

- [ ] **Step 1: Write the failing tests**

Replace the whole body of `crates/crew-app/src/chatswarmview_tests.rs` below the `pane_with_swarm` helper (i.e. everything from line 30 onward) with:

```rust
fn run(p: &mut ChatPane, id: u64) {
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(id),
        state: TaskState::Running,
    });
}

fn line(p: &ChatPane, cols: u16, now_ms: u64) -> String {
    let cells = block_cells(p, cols, 10, now_ms);
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == 10).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn no_swarm_no_rows() {
    let p = pane();
    assert_eq!(swarm_rows(&p, 40), 0);
    assert!(block_cells(&p, 80, 5, 0).is_empty());
}

#[test]
fn a_live_run_claims_exactly_one_row_whatever_the_plan_size() {
    // The block used to grow a row per task and cap at 8. It now says what
    // crew is doing, which is always one thing.
    assert_eq!(swarm_rows(&pane_with_swarm(1), 40), 1);
    assert_eq!(swarm_rows(&pane_with_swarm(5), 40), 1);
    assert_eq!(swarm_rows(&pane_with_swarm(20), 40), 1);
}

#[test]
fn the_line_names_the_running_task_and_counts_the_plan() {
    let mut p = pane_with_swarm(5);
    run(&mut p, 2);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-2"), "{l}");
    assert!(l.ends_with("0/5"), "{l}");
}

#[test]
fn only_the_running_task_is_named_not_the_whole_plan() {
    let mut p = pane_with_swarm(3);
    run(&mut p, 1);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-1"), "{l}");
    assert!(!l.contains("task-0"), "{l}");
    assert!(!l.contains("task-2"), "{l}");
    // One row only.
    assert!(block_cells(&p, 80, 10, 0).iter().all(|c| c.row == 10));
}

#[test]
fn the_oldest_running_task_wins_and_the_rest_are_counted() {
    // Parallel agents: naming the newest would make the line flicker as
    // tasks start and stop, so the oldest holds it.
    let mut p = pane_with_swarm(4);
    run(&mut p, 0);
    std::thread::sleep(std::time::Duration::from_millis(2));
    run(&mut p, 1);
    std::thread::sleep(std::time::Duration::from_millis(2));
    run(&mut p, 2);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-0"), "oldest should hold the line: {l}");
    assert!(l.contains("+2"), "two others run alongside it: {l}");
}

#[test]
fn a_lone_running_task_gets_no_plus_suffix() {
    let mut p = pane_with_swarm(3);
    run(&mut p, 0);
    let l = line(&p, 80, 0);
    assert!(!l.contains('+'), "{l}");
}

#[test]
fn nothing_running_shows_a_working_line_with_the_counter() {
    // The gap between the plan arriving (all Pending) and the first spawn.
    let p = pane_with_swarm(5);
    let l = line(&p, 80, 0);
    assert!(l.contains("Working"), "{l}");
    assert!(l.ends_with("0/5"), "{l}");
    assert!(!l.contains("task-"), "{l}");
}

#[test]
fn the_working_line_carries_no_elapsed() {
    // Elapsed derives from a running task's `started`; there isn't one.
    let p = pane_with_swarm(2);
    let l = line(&p, 80, 5_000);
    assert!(l.contains("Working"), "{l}");
    assert!(!l.contains('s'), "no elapsed without a running task: {l}");
}

#[test]
fn running_task_with_nonzero_now_shows_elapsed() {
    let mut p = pane_with_swarm(2);
    run(&mut p, 0);
    let l = line(&p, 80, 5_000);
    assert!(l.contains("0s"), "{l}");
}

#[test]
fn running_task_with_zero_now_shows_no_elapsed() {
    // now_ms == 0 is the test frame: deterministic, no elapsed.
    // (Don't assert `!l.contains('s')` here — the title "task-0" has one.)
    let mut p = pane_with_swarm(2);
    run(&mut p, 0);
    let l = line(&p, 80, 0);
    assert!(!l.contains("0s"), "{l}");
}

#[test]
fn the_counter_survives_a_pane_too_narrow_for_elapsed() {
    // Width rule: elapsed drops below ELAPSED_MIN_COLS, the counter never
    // does — it's the whole point of the line.
    let mut p = pane_with_swarm(5);
    run(&mut p, 0);
    let l = line(&p, ELAPSED_MIN_COLS - 1, 5_000);
    assert!(l.contains("0/5"), "{l}");
    assert!(!l.contains("0s"), "elapsed should have dropped: {l}");
}

#[test]
fn the_plus_suffix_survives_a_title_clamp() {
    // +N is the only signal that parallel work exists, so the title yields
    // columns to it rather than the reverse.
    let mut p = pane();
    let tasks = (0..3)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: "a-very-long-task-title-that-will-not-fit".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    p.absorb_hive_plan(tasks);
    run(&mut p, 0);
    run(&mut p, 1);
    let l = line(&p, 30, 0);
    assert!(l.contains("+1"), "{l}");
    assert!(l.contains("0/3"), "{l}");
}

#[test]
fn cells_never_collide_or_leave_the_pane() {
    // `reserve` and `next_start` are two expressions of the same per-column
    // budget; when they disagree the columns overlap the title.
    let mut p = pane_with_swarm(5);
    run(&mut p, 0);
    run(&mut p, 1);
    for cols in 8..=80u16 {
        let cells = block_cells(&p, cols, 10, 5_000);
        let mut seen: Vec<u16> = cells.iter().map(|c| c.col).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "two glyphs share a column at cols={cols}");
        assert!(
            cells.iter().all(|c| c.col < cols),
            "a cell escaped the pane at cols={cols}"
        );
    }
}

#[test]
fn wide_glyph_titles_advance_by_display_width() {
    let mut p = pane();
    let tasks = vec![TaskSpec {
        id: TaskId(0),
        title: "研究研究研究".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    }];
    p.absorb_hive_plan(tasks);
    run(&mut p, 0);
    for cols in 8..=80u16 {
        let cells = block_cells(&p, cols, 10, 5_000);
        let mut seen: Vec<u16> = cells.iter().map(|c| c.col).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "CJK title overlapped at cols={cols}");
        assert!(cells.iter().all(|c| c.col < cols), "escaped at cols={cols}");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app --lib chatswarmview 2>&1 | tail -20`
Expected: FAIL — `a_live_run_claims_exactly_one_row_whatever_the_plan_size` fails with `assertion left == right failed: left: 5, right: 1`, and the `Working`/`+N` tests fail on missing text.

- [ ] **Step 3: Rewrite `chatswarmview.rs`**

Replace the whole file with:

```rust
//! Draws the live swarm-run status line at the bottom of the chat message
//! area: one row saying what crew is doing right now — a spinner, the running
//! task's title, its elapsed time, and how much of the plan has settled. The
//! plan itself is not shown live; it lands in the transcript when the run
//! folds (`chatswarmrec`). State lives in `chatswarm`.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatswarm::{SwarmStatus, SwarmTask};
use crew_hive::TaskState;

/// Shown when the plan has arrived but nothing is running yet — the gap
/// before the first `AgentSpawned`, and the gap between one task settling and
/// the next spawning.
const WORKING: &str = "Working…";
/// Below this width the elapsed column is dropped (the title needs the room).
/// The counter is never dropped: it's the reason the line exists.
const ELAPSED_MIN_COLS: u16 = 16;

/// Rows the live line occupies in the message area (0 = no live run).
pub(crate) fn swarm_rows(pane: &ChatPane, _rows: u16) -> u16 {
    match &pane.swarm {
        Some(s) if !s.tasks.is_empty() => 1,
        _ => 0,
    }
}

/// The task the line names: the oldest `Running` one, plus how many others are
/// running alongside it. `None` when nothing is running.
///
/// Oldest rather than newest so the line stays put under parallelism — naming
/// the most recent spawn would make it flicker between tasks as agents come
/// and go. A `Running` task always has `started` stamped (`chatswarm::apply`),
/// but unstamped ones sort last so a stamped task still wins if that changes.
fn focus(s: &SwarmStatus) -> Option<(&SwarmTask, usize)> {
    let mut running: Vec<&SwarmTask> = s
        .tasks
        .iter()
        .filter(|t| t.state == TaskState::Running)
        .collect();
    running.sort_by_key(|t| (t.started.is_none(), t.started));
    let first = *running.first()?;
    Some((first, running.len() - 1))
}

fn push_str(v: &mut Vec<CellView>, col: &mut u16, row: u16, s: &str, fg: (u8, u8, u8)) {
    for c in s.chars() {
        // Advance by display width (a wide CJK/emoji glyph occupies two
        // cells) so text after a wide glyph doesn't overlap it; zero-width
        // marks are skipped like `chatwidth::place_row` does.
        let w = crate::chatwidth::char_w(c) as u16;
        if w == 0 {
            continue;
        }
        v.push(CellView {
            col: *col,
            row,
            c,
            fg,
            bg: crew_theme::theme().page_bg,
            bold: false,
            italic: false,
        });
        *col += w;
    }
}

/// Render the status line at `top_row`. `now_ms` drives the spinner (0 in
/// tests = first frame, and suppresses elapsed so tests stay deterministic).
pub(crate) fn block_cells(pane: &ChatPane, cols: u16, top_row: u16, now_ms: u64) -> Vec<CellView> {
    let Some(s) = &pane.swarm else {
        return Vec::new();
    };
    if s.tasks.is_empty() {
        return Vec::new();
    }
    let theme = crew_theme::theme();
    let mut v = Vec::new();

    let (done, total) = s.settled();
    let counter = format!("{done}/{total}");
    let focused = focus(s);
    let title_src = match focused {
        Some((t, _)) => t.title.as_str(),
        None => WORKING,
    };
    // ` +N` marks parallel work. It is not clamp-able below — it's the only
    // signal that other tasks are running, so the title yields columns to it.
    let suffix = match focused {
        Some((_, others)) if others > 0 => format!(" +{others}"),
        _ => String::new(),
    };
    // Elapsed derives from `started` at render time — the per-frame redraw
    // while busy animates it for free — and is gated on `now_ms != 0` so tests
    // that don't care (now_ms == 0) stay deterministic.
    let elapsed = focused
        .filter(|_| now_ms != 0)
        .and_then(|(t, _)| t.started)
        .map(|s| format!("{}s", s.elapsed().as_secs()))
        .filter(|_| cols >= ELAPSED_MIN_COLS);

    let f = (now_ms / 120) as usize % crate::update::SPINNER.len();
    let mut col = 1u16;
    push_str(
        &mut v,
        &mut col,
        top_row,
        &crate::update::SPINNER[f].to_string(),
        crate::palette::accent(),
    );
    push_str(&mut v, &mut col, top_row, " ", crate::palette::accent());

    // Reserve room for the right-aligned columns. The counter always shows;
    // elapsed drops below ELAPSED_MIN_COLS. Each column claims exactly
    // `len + 1` — the same budget `next_start` charges below.
    let mut reserve = 1u16 + counter.len() as u16 + 1;
    if let Some(e) = &elapsed {
        reserve += e.len() as u16 + 1;
    }
    // Columns left for the title and its suffix. The suffix is preferred over
    // the title — it's the only signal parallel work exists — but on a pane
    // with room for neither it drops too, rather than overrunning `reserve`
    // and landing on the counter.
    let avail = cols.saturating_sub(col + reserve);
    let suffix_w = crate::chatwidth::str_w(&suffix) as u16;
    let (suffix, suffix_w) = if suffix_w <= avail {
        (suffix, suffix_w)
    } else {
        (String::new(), 0)
    };
    let max_title = (avail - suffix_w) as usize;
    // Display-width-aware clamp: `.chars().take(n)` counts chars, so a
    // CJK/emoji title (2 display columns per glyph) could select twice as many
    // columns as `max_title` allows, colliding with the elapsed column.
    let title_chars: Vec<char> = title_src.chars().collect();
    let title_end = crate::chatwidth::fit_end(&title_chars, 0, max_title);
    let title: String = title_chars[..title_end].iter().collect();
    push_str(&mut v, &mut col, top_row, &title, theme.text_muted);
    push_str(&mut v, &mut col, top_row, &suffix, theme.text_muted);

    // Right-aligned from the pane edge, each exactly `len + 1` inside whatever
    // sits to its right — the same per-column budget `reserve` charged above,
    // so title and columns can never collide (an extra -1 here once
    // double-billed the gap and overlapped the title):
    // title ... elapsed ... counter.
    let next_start = cols.saturating_sub(counter.len() as u16 + 1);
    let mut ccol = next_start;
    push_str(&mut v, &mut ccol, top_row, &counter, theme.text_muted);
    if let Some(e) = &elapsed {
        let mut ecol = next_start.saturating_sub(e.len() as u16 + 1);
        push_str(&mut v, &mut ecol, top_row, e, theme.text_muted);
    }
    v
}

#[cfg(test)]
#[path = "chatswarmview_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app --lib chatswarmview 2>&1 | tail -20`
Expected: PASS, all tests.

- [ ] **Step 5: Verify no dead imports and the transcript budget still holds**

Run: `cargo test -p crew-app 2>&1 | tail -5`
Expected: PASS. `chatview_tests` and `chatplace` tests must stay green — `msg_rows_budget` subtracts `swarm_rows`, so the transcript silently gains 4 rows.

Run: `cargo check -p crew-app 2>&1 | grep -c 'warning: unused'`
Expected: `0`. The old `glyph` import and the `MAX_ROWS`/`TOKENS_MIN_COLS` constants are gone; nothing else should have been orphaned.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatswarmview.rs crates/crew-app/src/chatswarmview_tests.rs
git commit -m "feat(chat): the live swarm block becomes one status line

Showing the whole plan answered a question nobody was asking mid-run: the
plan lands in the transcript on fold anyway. Name the oldest running task,
count the rest with +N, and give the four freed rows back to the transcript."
```

---

### Task 3: Delete the timeline, give the record a wall-clock total

**Files:**
- Delete: `crates/crew-app/src/chattimeline.rs`, `crates/crew-app/src/chattimeline_tests.rs`
- Modify: `crates/crew-app/src/main.rs:48`
- Modify: `crates/crew-app/src/chatswarmrec.rs:1-75`
- Modify: `crates/crew-app/src/chatswarm.rs:159` (`fold_swarm`)
- Test: `crates/crew-app/src/chatswarm_tests.rs`

**Interfaces:**
- Consumes: nothing from Tasks 1–2.
- Produces: `SwarmStatus::record_text(&self, run_ms: Option<u64>) -> String`. `run_ms` is the run's wall-clock duration; `None` suppresses the elapsed segment of the Σ line.

**Why the signature changes:** wall-clock is `run_started.elapsed()`, read at fold. Reading it *inside* `record_text` would be untestable — every `SwarmStatus` literal in `chatswarm_tests.rs` sets `run_started: Instant::now()`, so elapsed reads ~0 and the total renders `"0.0s"` in every test. The caller supplies it instead.

**The Σ gate is `run_ms.is_some() && (tok > 0 || cost > 0)`.** Both halves matter:
- `run_ms.is_some()` — tests at `:264, :282, :303` `assert_eq!` on the entire output and pass `None`; they must not grow a Σ line.
- `tok > 0 || cost > 0` — `cancelled_before_start_leaves_elapsed_none` (`:248`) folds a run that consumed nothing and asserts the record is exactly `"- ⊘ research"`. Without this half it would gain `Σ 0 tok · 0.0s`, a summary of nothing.

- [ ] **Step 1: Write the failing tests**

In `crates/crew-app/src/chatswarm_tests.rs`, delete these three tests entirely (they test the timeline, which is going away):
- `record_appends_a_timeline_block_for_concurrent_runs` (`:335-359`)
- `single_task_runs_get_no_timeline` (`:361-370`)
- `task_still_running_at_error_fold_gets_an_open_ended_span` (`:372-403`)

Also delete the section comment at `:321` (`// --- Run timeline (2026-07-13-swarm-timeline-design.md) ---`), but **keep the `done_task` helper at `:323-333`** — the cost tests use it.

Then add:

```rust
#[test]
fn keyless_runs_get_a_sigma_line_without_cost() {
    // A keyless/stub run reports TokenDelta but never CostDelta. It used to
    // get no Σ at all and so lost its total; it now gets one, minus the $.
    let run_started = Instant::now();
    let mut a = done_task(0, "research", run_started, 3_200);
    a.tokens = 12_400;
    let s = SwarmStatus {
        tasks: vec![a],
        agent_task: Default::default(),
        run_started,
    };
    let text = s.record_text(Some(3_200));
    assert!(text.contains("\u{03a3} 12.4k tok \u{00b7} 3.2s"), "{text}");
    assert!(!text.contains('$'), "{text}");
}

#[test]
fn a_run_that_consumed_nothing_gets_no_sigma_line() {
    // Σ summarises spend. With no tokens and no cost there is nothing to
    // summarise, and "Σ 0 tok" is noise.
    let run_started = Instant::now();
    let s = SwarmStatus {
        tasks: vec![done_task(0, "solo", run_started, 5_000)],
        agent_task: Default::default(),
        run_started,
    };
    let text = s.record_text(Some(5_000));
    assert!(!text.contains('\u{03a3}'), "{text}");
}

#[test]
fn the_record_carries_no_timeline_block() {
    let run_started = Instant::now();
    let mut a = done_task(0, "research", run_started, 3_200);
    a.tokens = 100;
    let mut b = done_task(1, "merge", run_started + Duration::from_millis(3_000), 9_400);
    b.tokens = 100;
    let s = SwarmStatus {
        tasks: vec![a, b],
        agent_task: Default::default(),
        run_started,
    };
    let text = s.record_text(Some(12_400));
    assert!(!text.contains("timeline"), "{text}");
    assert!(!text.contains('`'), "no code fence survives: {text}");
    assert!(!text.contains('\u{2588}'), "{text}");
}
```

Update `record_shows_per_task_cost_and_a_run_total` (`:436`) — change `:450` and `:460`:

```rust
    let text = s.record_text(Some(3_200));
```

```rust
    // The Σ line totals the whole run: 13k tok, $0.0443 → cent-plus → $0.04,
    // and the run's wall-clock duration.
    assert!(
        text.contains("\u{03a3} 13.0k tok \u{00b7} $0.04 \u{00b7} 3.2s"),
        "{text}"
    );
```

Update the three direct callers to pass `None` — `:279`, `:297-300`, `:318`:

```rust
    assert_eq!(s.record_text(None), "- \u{2713} research \u{00b7} 3.2s");
```

```rust
    assert_eq!(
        s.record_text(None),
        "- \u{2713} research \u{2014} 12.4k tok \u{00b7} 0.9s"
    );
```

```rust
    assert_eq!(s.record_text(None), "- \u{2713} research");
```

And `costless_runs_keep_the_old_record_shape` (`:463`) — update its call at `:471` and sharpen the comment, since it now passes for a subtly different reason (`done_task` has `tokens: 0`, so the new gate finds nothing to summarise):

```rust
#[test]
fn costless_runs_keep_the_old_record_shape() {
    // No cost and no tokens: nothing for Σ to say.
    let run_started = Instant::now();
    let s = SwarmStatus {
        tasks: vec![done_task(0, "solo", run_started, 5_000)],
        agent_task: Default::default(),
        run_started,
    };
    let text = s.record_text(None);
    assert!(!text.contains('$'), "{text}");
    assert!(!text.contains('\u{03a3}'), "{text}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app --lib chatswarm 2>&1 | tail -20`
Expected: FAIL — `this method takes 0 arguments but 1 argument was supplied`.

- [ ] **Step 3: Delete the timeline module**

```bash
git rm crates/crew-app/src/chattimeline.rs crates/crew-app/src/chattimeline_tests.rs
```

Then remove `mod chattimeline;` from `crates/crew-app/src/main.rs:48`.

- [ ] **Step 4: Rewrite `record_text` and drop `spans()`**

Replace `crates/crew-app/src/chatswarmrec.rs:1-75` (the module doc, the imports, and the whole `impl SwarmStatus` block) with:

```rust
//! The folded swarm record: when a run ends, `chatswarm` retires the live
//! status line into a transcript message rendered here — a markdown task list
//! plus a Σ line totalling the run's tokens, spend and wall-clock duration.

use crew_hive::TaskState;

use crate::chatswarm::SwarmStatus;
use crate::chattime::fmt_elapsed;

impl SwarmStatus {
    /// The block as a markdown list — the transcript record on fold.
    ///
    /// `run_ms` is the run's wall-clock duration, supplied by the caller
    /// rather than read from `run_started` here: `Instant` can't be mocked, so
    /// reading the clock inside would make every test's Σ line read "0.0s".
    /// `None` omits the duration.
    pub(crate) fn record_text(&self, run_ms: Option<u64>) -> String {
        let mut out = self
            .tasks
            .iter()
            .map(|t| {
                let glyph = glyph(&t.state);
                let mut line = if t.tokens > 0 {
                    format!("- {glyph} {} — {} tok", t.title, fmt_tok(t.tokens))
                } else {
                    format!("- {glyph} {}", t.title)
                };
                if t.cost_micros > 0 {
                    line.push_str(" \u{00b7} ");
                    line.push_str(&fmt_cost(t.cost_micros));
                }
                if let Some(ms) = t.elapsed_ms {
                    line.push_str(" \u{00b7} ");
                    line.push_str(&fmt_elapsed(ms));
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
        // Run totals — the only place the whole run's spend surfaces in chat
        // (the broker's aggregate Stats carries tokens but not cost).
        //
        // Gated on the run having consumed something: a run cancelled before
        // it started would otherwise summarise itself as "Σ 0 tok · 0.0s".
        // Cost is absent on keyless/stub runs, which still report tokens — so
        // those get a Σ line, just without the `$` part.
        let cost: u64 = self.tasks.iter().map(|t| t.cost_micros).sum();
        let tok: u64 = self.tasks.iter().map(|t| t.tokens).sum();
        if let Some(ms) = run_ms.filter(|_| tok > 0 || cost > 0) {
            out.push_str(&format!("\n\n\u{03a3} {} tok", fmt_tok(tok)));
            if cost > 0 {
                out.push_str(&format!(" \u{00b7} {}", fmt_cost(cost)));
            }
            out.push_str(&format!(" \u{00b7} {}", fmt_elapsed(ms)));
        }
        out
    }
}
```

- [ ] **Step 5: Pass the wall-clock duration from `fold_swarm`**

In `crates/crew-app/src/chatswarm.rs`, change `fold_swarm`'s `push_capped` call (`:157-162`) to:

```rust
        let run_ms = s.run_started.elapsed().as_millis() as u64;
        self.push_capped(Message {
            sender: "crew".into(),
            text: s.record_text(Some(run_ms)),
            ts: String::new(),
            meta: String::new(),
        });
```

Also update the `run_started` doc comment at `chatswarm.rs:38` — it names the timeline, which no longer exists:

```rust
    /// When the plan arrived — the zero point for the run's wall-clock
    /// duration on the folded record's Σ line (`chatswarmrec`).
```

And the module doc at `chatswarm.rs:4-5`:

```rust
//! state the block folds into a transcript message — the durable record of
//! the run. Live rendering lives in `chatswarmview`; the folded record (task
//! list + Σ totals) in `chatswarmrec`.
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p crew-app --lib chatswarm 2>&1 | tail -20`
Expected: PASS, all tests.

- [ ] **Step 7: Verify the module is fully unwired**

Run: `cargo check -p crew-app 2>&1 | grep -iE 'chattimeline|unused|dead_code'`
Expected: no output. `spans()`, the `timeline_block` import, and `mod chattimeline;` are all gone together.

Run: `grep -rn chattimeline crates/ docs/superpowers/plans/ 2>/dev/null | grep -v 2026-07-13`
Expected: no output outside the superseded timeline spec.

- [ ] **Step 8: Run the whole crate and commit**

```bash
cargo fmt
cargo test -p crew-app 2>&1 | tail -5
git add -A crates/crew-app/src/
git commit -m "feat(chat): fold the timeline into a wall-clock total

The record said everything twice — a task list with per-task numbers, then a
Gantt card re-listing the same tasks. Keep the list, delete the timeline, and
put the run's wall-clock duration on the Σ line. Σ now also survives keyless
runs, which report tokens but never cost."
```

---

### Task 4: Verify in the live app

The three surfaces are budgeted independently and composed by `chatview::cells`; only a real run proves they don't overlap.

**Files:** none modified.

- [ ] **Step 1: Read the verify skill**

The GUI harness has non-obvious safety rules — the live app runs an isolated HOME and every synthetic keystroke needs a frontmost-PID check first.

Run: read `.claude/skills/verify/SKILL.md` and follow it.

- [ ] **Step 2: Build and launch the dev app**

The live app spawns the **installed** broker at `~/.local/bin/crew`, not the dev build. That binary predates the specialist store, so its roster still shows the inbuilt `@planner @coder @reviewer`. Build and install first or the run won't reflect this branch.

- [ ] **Step 3: Run a multi-task swarm and screenshot mid-run**

Confirm: exactly one status line above the bar; it names a running task; the bar carries no `2/5`; the counter sits at the right edge of the line.

- [ ] **Step 4: Screenshot after the run folds**

Confirm: no `╭─ code` card; the record is a plain list plus a Σ line ending in a duration.

- [ ] **Step 5: Report findings**

Do not commit. Report what the screenshots show, including anything that looks wrong.

---

## Notes for the implementer

**Left alone deliberately:**

- `swarm_rows` and `block_cells` recompute the row count independently rather than routing through a shared `geom` the way `chatprog` does. At a constant 1 they agree trivially; factoring it would be ceremony.
- `chatview`'s `block_max` filters stay. The composer doesn't reliably overdraw stray rows — only the prompt glyph and text touch every column — so a stray block row would survive visibly even at one row.
- `fmt_tok`, `fmt_cost` and `glyph` stay `pub(crate)` in `chatswarmrec.rs`. `glyph` loses its `chatswarmview` caller but `chatswarmrec` still uses it.

**Pre-existing on this branch, not caused by this work:** `cargo check` warns about an unused `title_of` at `crew-plugin/src/broker/swarmmsg.rs:17` and dead `specialists::touch`/`touch_at`. Leave them; they belong to the in-flight specialist work.
