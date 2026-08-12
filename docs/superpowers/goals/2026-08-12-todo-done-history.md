# Goal — todo pane: done history view (`/todo done`)

**Status: SET 2026-08-12** by the user: a special command should display the *done*
tasks, so finished work is auditable at a glance and doesn't get re-done. Not started.

**What exists today** (0.15.0–0.16.11, `crew-app/src/todopane/`): the store keeps done
items forever as history (`todos.toml` never prunes), and `h` in the list toggles
`show_done` — but that only *interleaves* dimmed done rows at the bottom of the active
list. There is no way to look at the done pile on its own, no answer to "when did I
finish this?", and no completion timestamp at all: `TodoItem` has only `created_ms`,
and `sort_key` (`item.rs:68`) fakes "newest completion first" by sorting done items on
`u64::MAX - created_ms`. This goal adds a real done view and the datum it needs.

## Proposed decisions (defaults taken while drafting — veto before work starts)

- **The command is `/todo done`.** `/todo` today takes no arguments (`cmddefs.rs:130`);
  it grows one optional word. `/todo done` opens (or focuses) the todo pane already in
  done view; bare `/todo` behaves exactly as now. No new pane type — done view is a
  mode of the existing pane, like `@project` filtering is.
- **In-pane, `H` (shift) enters the done view; `h` keeps its current meaning.** `h`
  stays the interleave toggle (shipped 0.16.x, muscle memory exists). Esc leaves done
  view back to the active list — matching the pane's capture-first reflexes.
- **Done view is a log, not a list.** Newest completion first, grouped under day
  headers ("today", "yesterday", "aug 10"), each row showing the title, its `@project`
  chip in the project color, and the completion time. The composer collapses to a
  filter hint — typing `@project` filters the log; typing anything else does *not*
  create a todo from inside the history (the one place the capture-first rule bends).
- **`done_ms: Option<u64>` is the new field, serde-defaulted.** Set on tick, cleared
  on un-tick. Old `todos.toml` files load with `None` (0.12.6 `clamped()` lesson:
  the default must live in serde, not in a rebuild that resets it every load) — those
  legacy items group under a single "earlier" header, no fake dates.
- **Space/Enter in the done view un-dones.** The item leaves the log and reappears in
  the active list — that *is* the "reaching back" gesture `sort_key`'s comment already
  anticipates, now from the view built for it.

## What already exists (assembly, not invention)

- **Filtering machinery**: `display_order` (`item.rs:43`) already takes
  `filter` + `show_done`; done view is a third projection over the same `items` —
  a pure `done_order(items, filter)` sibling, testable the same way.
- **Sort truth moves, not multiplies**: `sort_key`'s done branch swaps
  `created_ms` for `done_ms.unwrap_or(created_ms)` — one line, and the `h`
  interleave view gets honest ordering for free.
- **Day-header rendering**: the chat pane's day dividers and the todo pane's own
  due-date formatter (`duedate.rs`) already own "today/yesterday/date" wording —
  reuse the formatter, don't re-derive it.
- **Selection/scroll math is view-agnostic**: `ensure_visible`/`clamp_scroll`
  (`mod.rs`) work over whatever `visible_len()` reports; done view swaps the order
  vector, keeps the plumbing.
- **`/todo` arg plumbing**: other commands (`/far`, `/view`) already parse a word
  after the command; copy that seam in the dispatcher, not a new one.

## The contract (definition of done)

1. **`/todo done` shows the history.** Only done items, newest completion first,
   grouped by completion day, `@project` filter honored (command arg `@tag` accepted
   too: `/todo done @crew`). Empty history says so quietly instead of a blank pane.
2. **Completion time is real and survives.** Ticking sets `done_ms`, un-ticking
   clears it, both persist through `todos.toml` round-trips; pre-existing done items
   load as `None` and group under "earlier". The interleaved `h` view and the done
   view sort by the same key.
3. **The view is navigable with what the pane already taught.** Up/Down/PageUp/
   PageDown/Home/End traverse it (day headers are not selectable), Space/Enter
   un-dones and returns the item to the active list, `d` deletes it from history
   permanently, Esc exits to the active list, `H` enters from it.
4. **Discoverable.** `/todo` suggestion text mentions `done`; the help overlay's todo
   section gains the `H` / Esc pair.

## Stretch (ranked, separate iterations)

1. **Duplicate-work guard at the composer** — while typing a new todo, if a done
   item's title matches (case-insensitive prefix/fuzzy), show a one-line ghost hint
   "✓ done aug 10 — <title>"; never blocks submission. This is the "avoid duplicate
   work" ask made ambient, not just auditable.
2. **`/todo done` count chip** — the nav sidebar row shows "n done today".
3. **Retention** — a `/todo prune 30d`-style command once history grows unwieldy;
   until then, keeping everything *is* the feature.

## Verification

- Unit, RED transcripts before green (standing rule — concrete values, not shapes):
  `done_ms` set/clear/persist round-trip including legacy files without the field;
  `done_order` grouping across today/yesterday/earlier boundaries (feed fixed
  timestamps, no `now()` in assertions); un-done from the view re-entering
  `display_order` at the right rank; filter + arg parsing (`/todo done @x`).
- Mutation spot-check the day-bucketing arithmetic — boundary off-by-ones are the
  classic vacuous-test survivors.
- Live (`.claude/skills/verify`, isolated HOME — same macOS-perms debt as the todo
  goals before it): tick two items, `/todo done`, see them under "today" with times;
  un-done one; relaunch and confirm the other's timestamp survived.
- Idle invariant: a resting done view requests no frames (`wants_animation_frame`
  gains no new term).
