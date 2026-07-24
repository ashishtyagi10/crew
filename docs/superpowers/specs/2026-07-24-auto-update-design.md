# Auto-update with manual-restart reminder — design

**Date:** 2026-07-24
**User ask:** Keep `/update`, add auto-update; `/restart` stays the only
restart path (never auto-restart); after any install, an impossible-to-miss
blinking/highlighted reminder to restart; restart must always load the
latest installed version.

## Today

- `/update` (`update.rs` + `updatefetch.rs`): worker thread checks GitHub,
  downloads + installs over `~/.local/bin/crew`, streams stages into the
  left-nav UPDATE card; `Installed` lingers 5 s then the card clears. No
  restart signal exists at all (`install_parks_then_clears_without_restarting`).
- `/restart` (`restart.rs` → `detach::spawn_detached_copy`) re-execs
  `std::env::current_exe()` — the same path self_update atomically
  replaced — so a parked update already applies on restart. Unpinned by
  any test, and `dispatch.rs:23`'s comment wrongly claims Crew
  "auto-restarts into the new build".
- The nav stats card's fieldset legend is `crew v<CARGO_PKG_VERSION>`
  (`navcard.rs:41`). Attention blinking (`attention.rs`) uses the shared
  `anim` clock: `BLINK_MS` 400 half-period, `PULSE_MS` 4000 then steady.

## Decisions

### 1. Auto-update scheduler (new `autoupdate.rs`)

The auto path REUSES the entire `/update` pipeline (`UpdateState`,
`spawn_worker`) — the delta is only when it runs and how quietly:

- First check ~30 s after launch (startup stays snappy), then every 6 h,
  driven from the existing poll tick (worker threads do all network/disk;
  nothing new on the winit thread).
- A `silent` flag on `UpdateState`: a silent run renders NO UPDATE card
  and posts NO status while checking/downloading; `UpToDate`/`Failed`
  clear silently (auto-retry at the next scheduled check). `Installed`
  behaves identically in both modes (→ §3). `/update` keeps today's loud
  behavior, and a manual `/update` while a silent check runs upgrades the
  in-flight state to loud rather than spawning a second worker.

### 2. Restart stays manual

Unchanged invariant, now also documented correctly: fix the stale
`dispatch.rs` comment. No auto-restart code anywhere.

### 3. Parked-update reminder

New persistent field `CrewApp.parked_update: Option<String>` (the new
version), set whenever any update run (silent or loud) reaches
`Installed`, cleared only by process exit (restart clears it naturally).
Rendered as the **nav stats-card legend** (the `crew vX.Y.Z` fieldset
title the user pointed at): while parked, the legend becomes
`crew v<current> → v<new> · /restart`, blinking on the attention clock
(400 ms half-period) for the first 4 s, then settling to a steady
accent-highlighted legend until restart. Redraws are driven only during
the blink window (attention.rs's cost model). The plain version legend
returns after restart by construction.

### 4. Restart loads latest — pinned

Factor `spawn_detached_copy`'s command construction into a pure
`restart_command()` returning the `(exe, args)` it will spawn; unit-test
that `exe == std::env::current_exe()` and that detach flags are stripped
— documenting (comment) that self_update's atomic replace at that path is
what makes `/restart` (and any relaunch) load the newest installed
binary.

## Non-goals

Auto-restart; release channels; downgrade UI; config knobs for the check
interval; changing `/update`'s visible behavior.

## Error handling

Silent check/install failures are invisible (next cycle retries; no
nagging); a manual `/update` still reports failures loudly. If the GitHub
check is rate-limited or offline, nothing surfaces.

## Testing

1. Scheduler: pure timing tests (due-at-launch-delay, 6 h cadence, no
   double-spawn while a run is in flight, manual upgrade-to-loud).
2. Silent mode: no UPDATE card cells while silent-checking; card appears
   for loud runs (existing tests keep passing).
3. Parked reminder: `Installed` (silent AND loud) sets `parked_update`;
   nav legend cells show `→ v<new> · /restart` when parked, blink phase
   driven by the anim clock, steady accent after the pulse window; plain
   legend when not parked.
4. Restart: `restart_command()` exe equals `current_exe()`, detach flags
   stripped (existing `strip_detach_flags` tests remain).
