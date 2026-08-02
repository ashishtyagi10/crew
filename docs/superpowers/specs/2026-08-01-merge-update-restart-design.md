# Merge `/update` and `/restart` — design

**Date:** 2026-08-01
**Goal:** One command. `/update` installs the latest release **and restarts Crew
into it**; `/restart` disappears as a separate command. There is no point in a
two-step flow when the second step is always the same.

## Today

- `/update` — background worker installs the new binary over the running one,
  then *parks*: the nav legend blinks `crew vA → vB · /restart` until the user
  types `/restart`. Crew never restarts itself.
- `/restart` — relaunch as a fresh detached process (applies a parked install
  and external config edits) and exit this one.
- Silent auto-update (30 s after launch, then 6-hourly) installs and parks the
  same way.

## After

### `/update` (the one command)

1. **Parked install waiting** (from a silent auto-update or an earlier run):
   restart immediately to apply it — no network round-trip needed.
2. **Run already in flight:** loud → "update already in progress"; silent →
   upgraded to loud (existing takeover), and it now restarts when the install
   lands, because the user asked for an update.
3. **Fresh run:** worker checks GitHub, downloads, installs. On `Installed`
   the UPDATE card shows `✓ updated vX / restarting…` for a short beat
   (`RESTART_DELAY`, 2 s), then Crew relaunches detached and this process
   exits. The normal quit path runs (`exiting()` snapshots the session for
   `/restore`).
4. **Up to date / failed:** note card as today; no restart.

### Silent auto-update — unchanged in spirit

A background check must never interrupt a live session. It still installs and
parks; only the reminder text changes: `crew vA → vB · /update`.

### `/restart` — removed

- Palette row and dispatch arm deleted. `restart_crew()` stays as internal
  machinery (used by the merged flow).
- Typing `/restart` gets a migration hint status:
  `/restart merged into /update — it now restarts after installing`
  (without the stub, the fuzzy matcher would suggest `/restore`, which is
  destructive-adjacent and wrong).

### Failure path

If the detached spawn fails after an install, the card clears, the status
shows `restart failed: …`, and the parked reminder stays — the app keeps
running on the old binary and `/update` can retry the restart.

## Mechanics

- `UpdateTick` regains a `restart: bool`. In `poll_update`, a **loud** run
  whose `Done` deadline elapses sets it (a silent run just clears, as today);
  `poll_panes` responds with `restart_crew()` → `event_loop.exit()`.
- `Done` still parks in both modes: if the restart fails, the reminder legend
  is the persistent fallback.
- `start_update` returns `bool` (exit) so the parked-install shortcut can
  reuse the dispatch `return` path.
- Text: `restartnote::legend` says `/update`; `updatecard` `Done` detail says
  `restarting…`; `cmddefs` `/update` desc covers both halves; `/restart` row
  removed; docs (README, CREW.md, docs.html, index.html) updated.

## Out of scope

- A pure "relaunch to pick up external config edits" command. Quit + reopen
  covers it; most config applies live.
- `crew --self-update` CLI path (already restart-free by nature).
