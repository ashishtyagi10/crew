# Active agents on the footer mode line

**Date:** 2026-08-01
**Status:** Approved

## Problem

The crew/smith composer prints the entire agent roster on its top border
(`chips_on_border`, `chatinput.rs`) — all seven `@agent` chips, always,
whether or not anyone is working. The information is static and redundant:
footer line 1 already says `qwen-max · 7 agents`, and typing `@` pops the
mention picker with the full roster. What the user actually wants to know —
*which agents are working right now* — is shown nowhere by name (the pane
header shows only a single name or a `N working` count).

## Decision

Remove the roster strip from the composer border. Show the names of
actively-working agents on footer line 3 (the `▶▶` routing-mode line), only
while they are working. Placement was chosen over the in-pane swarm block
(only visible during swarm runs) and over an active-only border legend (the
user wants the input box out of this business).

## Design

### Composer (`chatinput.rs`)

- `composer_cells` no longer calls `chips_on_border`; the function is
  deleted. The top border is a plain card border.
- The char-count badge keeps its right-aligned spot; its collision guard
  against chips collapses to the left corner (`chips_end` parameter goes
  away or becomes the constant corner offset).
- The `agents` parameter **stays**: it still drives leading-`@mention`
  colouring (`mention_len`) and `relay_target`.
- Module doc comment updated (it currently describes the legend).

### Footer (`chatsummary.rs`)

Line 3 gains an *active agents* portion between the `▶▶` mode segment and
the running/hints portion:

```
▶▶ swarm mode · @analyst · @coder · running #3 · /stop #3 to cancel
```

- **Source:** `pane.active` via `active_names()` — `Activity` events carry
  real agent names for both relay hops and swarm runs (broker `swarm.rs`
  emits per-task `Activity`), so this one field covers everything. No new
  plumbing beyond `FooterCtx` gaining `active: &[&str]` (filled by
  `footer_ctx`).
- **Rendering:** one `Seg` per name (`@name`), each in its
  `chatroster::agent_color`, joined by line 3's existing ` · ` separator.
- **Cap:** past 3 names, collapse to one segment `N agents working` in
  green — the line's "work in flight" colour. Same 3-name rule as
  `roster_seg` and `running_seg`.
- **Idle:** no active agents → line 3 renders exactly as today (hints when
  idle, `plan ready` when a plan is pending).
- **Budget priority:** active-agent segments take priority 2 (line 3's
  ties break toward the right, so the trailing hints and the `/stop` how
  drop before the names). The `plan ready`/`running #n` segments (0) and
  the `▶▶` mode (1) both outlast the names: on a narrow pane the line
  keeps its identity and the work ids, and the names go.

### Relationship to the pane header

The header keeps its one-name-or-count liveness status (`active_status()`);
the footer carries the full named list. No header changes.

## Not doing (YAGNI)

- No per-agent elapsed times on the footer (header already shows the oldest
  elapsed).
- No changes to the swarm block, mention picker, or roster segment.
- No new broker events or protocol changes.

## Testing

- `footer_lines` is pure — unit tests:
  - active names render as `@name` segments in their roster colours;
  - more than 3 active collapses to `N agents working`;
  - empty `active` leaves line 3 byte-identical to today's output;
  - narrow pane drops names before the `▶▶` mode segment;
  - active names coexist with `running #n` and with `plan ready`.
- `chatinput_tests`: composer top border carries no agent chips; badge
  still right-aligned; `@mention` colouring still works.
- Live `/verify` pass: launch isolated app, submit a task, screenshot the
  footer while agents work and after they settle.
