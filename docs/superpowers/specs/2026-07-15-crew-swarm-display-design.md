# Swarm display: one live status line, one folded record

## Problem

A five-task swarm run currently claims six rows above the composer — five task
rows plus the progress bar — and folds into a twelve-line transcript record that
says everything twice: a task list with per-task numbers, then a `chattimeline`
Gantt card re-listing the same tasks.

The live block answers "what is the whole plan?" when the question while a run
is in flight is "what is crew doing right now?". The plan is already visible in
the folded record once the run ends; showing it live costs five rows of
transcript to tell the user something they will see again anyway.

## Goals

- The live block shows only the current activity: one row.
- The folded record states each fact once.
- No number disagrees between the live surface and the folded one.

## Non-goals

- The `@agent` chips on the composer's top border stay. They are live roster
  data (`chat.rs:151` is the only writer), not a stale hardcode.
- `chatflow.rs` is untouched. It serves the live roster/activity path and has
  no connection to the swarm record.

## Design

### 1. The live status line (`chatswarmview.rs`)

The block collapses from "up to `MAX_ROWS` task rows plus a `… n more` overflow
row" to exactly one row.

```
  ⠻ Analyze Technical Specifications +2        12s  2/5
  ████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ╭─ @planner @coder @reviewer ────────────────────────╮
```

**What the line names.** A new selection helper partitions tasks so the choice
is testable without rendering:

- If any task is `Running`: the **oldest** by its `started` stamp, plus ` +N`
  when `N` others run alongside it. Oldest rather than newest so the line does
  not flicker between tasks as parallel agents start and stop.
- If none is: `⠻ Working…`, spinner still turning. This covers the gap between
  the plan arriving (all `Pending`) and the first `AgentSpawned`, and the gap
  between one task settling and the next spawning. The line carries the counter
  but **no elapsed** — elapsed derives from a running task's `started` stamp,
  and there is no running task to derive it from:

  ```
    ⠻ Working…                                        2/5
  ```

**Columns.** Right-aligned from the pane edge: the `2/5` counter outermost,
elapsed `12s` inside it. Per-task tokens leave the live line — one row cannot
carry five tasks' worth, and they survive in the folded record. That retires
`TOKENS_MIN_COLS`. `ELAPSED_MIN_COLS = 16` stays, so elapsed drops on a squeezed
pane while the counter always survives.

**The ` +N` suffix is not clamp-able.** It is the only signal that parallel work
exists, so the title's `fit_end` budget shrinks by the suffix width rather than
letting `+2` truncate away.

**Invariant to hold.** `reserve` (which sizes the title budget) and `next_start`
(which right-aligns the columns) are two expressions of the same per-column
`len + 1` charge. The comment at `chatswarmview.rs:113-117` records a fixed bug
where an extra `-1` double-billed the gap and overlapped the title. The column
set changes here — tokens out, counter in — so both expressions move together.

**Row budget.** `swarm_rows` becomes `1` when there are tasks, else `0`. This
makes it trivially agree with `block_cells`, which is why the two are left
independent rather than factored through a shared `geom` the way `chatprog`
does. `chatplace::msg_rows_budget` already subtracts `block + queued + prog`, so
the transcript picks up the freed rows with no change. Both `chatview` branches
keep their `block_max` filters: the composer does not reliably overdraw stray
rows, so the filters still matter at one row.

**Test gate.** `now_ms == 0` remains the test-frame convention — it suppresses
elapsed and pins the spinner to frame 0.

### 2. The progress bar (`chatprog.rs`)

The bar keeps its row and loses its ` {done}/{total}` label, which moves to the
status line. `geom` drops the label from its return; `bar_w` becomes
`cols - INSET`.

The counter must not be derived twice. The `(done, total)` computation —
terminal states (`Done | Failed | Cancelled`) count as done — moves to a shared
helper on `SwarmStatus` that both `chatprog` and `chatswarmview` call, so the
bar's fill and the line's `2/5` cannot drift.

`geom` remains the single source of truth for the bar's row budget and its
draw. `MIN_BAR = 4` still drops the row when there is no room.

### 3. The folded record (`chatswarmrec.rs`)

`chattimeline.rs` and `chattimeline_tests.rs` are deleted. The append at
`chatswarmrec.rs:49-52`, the import at `:10`, the `mod chattimeline;` at
`main.rs:48`, and `spans()` at `:59-74` all go with them — `spans()` has no
other caller, and a wall-clock total needs no per-task offsets.

Both fences of the timeline's code block come from a single `format!`
(`chattimeline.rs:51`), so they delete together. The record becomes pure
markdown — a list plus a paragraph — and the `╭─ code` card disappears from the
render. No dangling fence.

The per-task list is unchanged. The Σ line changes shape:

```
keyed:    Σ 9.9k tok · $0.09 · 64.2s
keyless:  Σ 9.9k tok · 64.2s
```

**The Σ gate becomes `run_ms.is_some() && (tok > 0 || cost > 0)`.** Today it is
`cost > 0`, so a costless run (stub provider, keyless project) gets no Σ at all
and loses its total. A keyless run still reports `TokenDelta`, so gating on
tokens-or-cost restores its Σ — without the `$` segment.

Both halves of the gate are load-bearing:

- **`run_ms.is_some()`** — three tests (`chatswarm_tests.rs:264, 282, 303`)
  call `record_text` directly and `assert_eq!` on its entire output. They pass
  `None` and must not grow a Σ line they never asserted.
- **`tok > 0 || cost > 0`** — `cancelled_before_start_leaves_elapsed_none`
  (`:248`) folds a run cancelled straight out of `Pending`: no tokens, no cost,
  and it `assert_eq!`s the whole record as `"- ⊘ research"`. Without this half,
  the fold path supplies `Some(…)` and emits `Σ 0 tok · 0.0s` — a summary of
  nothing. A run that consumed nothing gets no Σ.

**The total is wall-clock, passed in.** `record_text` takes
`run_ms: Option<u64>`. `fold_swarm` (`chatswarm.rs:159`) supplies
`Some(run_started.elapsed())`; `None` suppresses the elapsed segment.

Reading `run_started.elapsed()` *inside* `record_text` would be untestable:
every `SwarmStatus` literal in `chatswarm_tests.rs` sets
`run_started: Instant::now()`, so elapsed reads ~0 and the total renders
`"0.0s"` in every test. Passing it in lets tests assert a fixed `Some(64_200)`.
This mirrors the live block's `now_ms` convention, with `Option` making the
intent explicit rather than overloading zero.

Wall-clock is a different number from today's `64.2s`, which was max-span-end
(`chattimeline.rs:28`). Wall-clock includes planning time before the first task
spawns — a truer answer to "how long did this take".

`fmt_elapsed` (`chattime.rs:54`) formats it exactly as the timeline did:
`{:.1}s` under 100s, `MmSSs` above.

**Kept in sync.** `fmt_tok`, `fmt_cost` and `glyph` are `pub(crate)` and shared
by the live block and this record, with doc comments insisting the two never
show different numbers for the same run. Both surfaces keep calling them.

## Testing

**`chatswarmview_tests.rs`** — largely rewritten. The selection helper gets
direct cases: oldest wins under parallelism; `+N` counts the rest; the
`Working…` fallback when nothing runs; `+N` survives a narrow-pane title clamp;
the counter survives below `ELAPSED_MIN_COLS`.

**`chatprog_tests.rs`** — loses its label assertions; bar geometry now spans
`cols - INSET`.

**`chatswarm_tests.rs`** —

| test | action |
|---|---|
| `:336` timeline appended for concurrent runs | delete |
| `:363` asserts no `"timeline"` | delete (vacuous) |
| `:379` running task's bar reaches the edge | delete; its back-dated `run_started` is the model for a new total test |
| `:436` per-task cost + run total | update: pass `Some(3_200)`; `Σ 13.0k tok · $0.04` → `Σ 13.0k tok · $0.04 · 3.2s` |
| `:463` costless runs keep the old shape | unchanged — its `done_task` fixture has `tokens: 0`, so the new gate still yields no Σ |
| `:264, :282, :303` | unchanged; pass `None` → no Σ |
| `:99, :248` | unchanged; `contains`-based and zero-consumption respectively |
| *new* `keyless_runs_get_a_sigma_line_without_cost` | a run with tokens but no cost folds to `Σ 12.4k tok · 3.2s` — the requirement `:463` does *not* cover |

**`chattimeline_tests.rs`** — deleted with the module (8 tests).

## Consequences

- Six rows above the composer become two.
- A finished five-task run folds to six lines instead of twelve.
- The timeline is gone. It shipped 2026-07-13
  (`2026-07-13-swarm-timeline-design.md`) and was already conditional on two or
  more tasks having spans. Concurrency overlap is no longer visualised; per-task
  elapsed on each row and the wall-clock total on the Σ line remain.
