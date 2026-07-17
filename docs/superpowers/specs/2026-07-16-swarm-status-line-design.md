# Claude-style swarm status line + width-matched progress bar

**Date:** 2026-07-16
**Pane:** the crew / `/smith` chat pane, live swarm run.

## Problem

During a swarm run the pane shows two bottom rows above the composer:

- A status line (`chatswarmview`): spinner, task title, elapsed, `done/total`,
  with elapsed + counter right-aligned to the pane edge.
- A progress bar (`chatprog`): a full-width row of `█`/`░` blocks.

The full-width bar reads as heavy — it spans the whole pane regardless of how
little text sits above it. The status line also lacks the input/output token
signal Claude Code shows, and its elapsed is only ever `Ns` (no `Xm Ys`).

## Target

One text row plus a bar sized to that row's words:

```
✳ Building the widget… (12s · 2/5 · +1)        ↑1.2k ↓3.4k
██████████░░░░░░░░░░░░░
```

- **Word** = the real task title (crew's meaningful signal), Claude-style
  `(elapsed · detail)` parenthetical for the meta.
- **Tokens** = split input/output with arrows, right-aligned.
- **Bar** = only as wide as the left words, not the whole pane.

## Changes

### 1. State — `chatswarm.rs`

- Split `SwarmTask.tokens: u64` into `tokens_in: u64` + `tokens_out: u64`, fed
  from the existing `TokenDelta { input, output }` event — the data already
  arrives split; the current code merges it on receipt.
- Add `SwarmTask::tokens(&self) -> u64` (= in + out) so the folded record
  (`chatswarmrec`, the only other reader) needs no logic change.
- Add `SwarmStatus::token_totals(&self) -> (u64, u64)` summing in/out across
  tasks.

### 2. Words line — `chatswarmview.rs`

- **Left text** (accent spinner, muted rest): `{spinner} {title}… ({paren})`,
  where `paren` = `{elapsed} · {done}/{total}`, plus ` · +N` when `others > 0`.
  Elapsed appears only when a task is actually running (`focus` is `Some` and
  `now_ms != 0`).
- **Elapsed format**: `4m 12s` when `secs >= 60`, else `12s`.
- **Tokens**: right-aligned `↑{in} ↓{out}` using the existing
  `chathdr::fmt_tokens` (`1.2k`); shown only when `token_totals` > (0, 0).
- **Degradation** on narrow panes, in priority order: tokens drop first → then
  the parenthetical → then the title truncates. The title always keeps at
  least a stub so the line never fully vanishes above the minimum.
- **Single source of truth**: an internal `layout(pane, cols, now_ms)` computes
  the clamped left string, its column width, and the token string. `block_cells`
  draws from it; a new `pub(crate) words_width(pane, cols, now_ms) -> u16`
  returns the left block's span for the bar. `line_fits` / `swarm_rows` derive
  from the same layout so the row budget and the draw never disagree.

### 3. Bar — `chatprog.rs`

- `bar_cells` gains a `now_ms` param (already available at the call site,
  `chatview.rs`, which passes `now_ms` to `block_cells` one line earlier) and
  sizes the bar to `chatswarmview::words_width(...)` instead of `cols - INSET`.
  Fill math (`done * bar_w / total`) is unchanged.
- `progress_rows` stays `now_ms`-free: it is an existence check (row is 1 or 0
  regardless of bar width), so the layout budget in `chatplace` is unaffected.
  The title is always present, so `words_width` is always ≥ `MIN_BAR` whenever a
  run exists and the pane is wide enough — the budget-vs-draw invariant holds.

## Testing (test-first)

- `chatswarm_tests`: `TokenDelta` updates `tokens_in`/`tokens_out` separately;
  `token_totals` sums; `tokens()` returns the combined value. Existing fixtures
  that set `a.tokens = N` move to `tokens_in`/`tokens_out`.
- `chatswarmview_tests`: parenthetical assembly (`(12s · 2/5 · +1)`), `4m 12s`
  formatting, right-aligned `↑ ↓` tokens, degradation order, and that
  `words_width` is narrower than a wide pane.
- `chatprog_tests`: the two tests asserting full-inset width
  (`the_bar_no_longer_draws_a_count_label`, `stays_on_its_row_and_inside_the_pane`)
  move to the words-matched width; add `bar width == words_width`.

## Out of scope (YAGNI)

Whimsical gerund words, per-task token breakdown in the live line, cost/micros
in the live line.
