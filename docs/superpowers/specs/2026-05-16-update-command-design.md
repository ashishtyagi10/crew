# `/update` slash command — Design

> *Historical record: this plan predates the Crew pivot and targets editor crates that have since been removed.*

**Date:** 2026-05-16
**Status:** Approved
**Author:** Ashish Tyagi (with Claude)

## Goal

Add a first-class `/update` slash command (Copilot-style) that lets the user
check for and install a newer release from inside the running TUI. The
existing `--update` and `--check-update` CLI flags continue to work; this
spec is only about the in-TUI command.

## Non-goals

- Auto-update on startup. The install script's old tagline "the app auto-updates
  on startup" is misleading and is dropped.
- Auto-restart after install. The user is told to restart manually so any
  in-flight TUI state is preserved.
- A polished in-TUI progress bar. The blocking `perform_update()` is reused
  as-is; its stdout progress is shown by briefly leaving the alternate screen.

## Architecture

The current `crates/legacy-app/src/update.rs` cannot be called from
`crates/legacy-ui` because `legacy-app` depends on `legacy-ui`, not the reverse.

**Move `update.rs` into `legacy-core`.** Both crates already consume `legacy-core`,
so this is the minimum change that lets the TUI trigger the update flow. The
public API is unchanged: `UpdateStatus`, `check_and_auto_update_async`,
`perform_update`, `print_version`.

## State machine

A new `UpdateState` on `App`:

```
Checking { rx: mpsc::Receiver<UpdateStatus> }
Confirm  { version: String }
Installing
Done     { version: String }
Failed   { message: String }
```

`UpToDate` is not a state — when the background check returns
`UpdateStatus::UpToDate`, we just emit a `feedback.info(...)` and clear state.

## Flow

1. **User types `/update`** → `handle_slash_command` calls
   `check_and_auto_update_async()`, stores the receiver in
   `update_state = Checking { rx }`, and writes
   `feedback.info("Checking for updates…")`.

2. **`App::tick()`** calls `try_recv` on the channel each tick (every
   `tick_rate_ms`). On result:
   - `UpToDate` → clear state, `feedback.info("Already on latest (vX.Y.Z)")`.
   - `Available(v)` → `update_state = Confirm { version: v }`.
   - `Updated(v)` → `update_state = Done { version: v }` (defensive — current
     code path doesn't auto-update, but the variant exists).
   - `Failed(e)` → `update_state = Failed { message: e }`.

3. **Confirm modal** renders centered:
   ```
   ┌─ Update available ────────────────┐
   │ A new version is available:       │
   │   current: v0.3.2                 │
   │   latest:  v0.3.3                 │
   │                                   │
   │ [Y] Install    [N] Cancel         │
   └───────────────────────────────────┘
   ```

4. **Y key** while in `Confirm`:
   - `update_state = Installing`
   - `app.pending_install = true`
   - **Main loop** notices `pending_install` after `dispatch`, clears it, then:
     - `disable_raw_mode()` + `LeaveAlternateScreen` + `DisableMouseCapture`
     - `update::perform_update()` runs synchronously, printing to stdout
     - `enable_raw_mode()` + `EnterAlternateScreen` + `EnableMouseCapture`
     - `app.complete_install(result)` → sets state to `Done { v }` or
       `Failed { msg }`
     - Force redraw on next iteration

5. **N key** while in `Confirm`: clear state.

6. **Done modal**: `"Updated to vX.Y.Z — restart the app to use the new version.  [Enter] dismiss"`. Enter clears state. App keeps running.

7. **Failed modal**: shows the error message and `[Enter] dismiss`.

## Files changed

| Path | Change |
|------|--------|
| `crates/legacy-core/Cargo.toml` | Add `self_update`, `semver`, `tempfile`; reuse workspace `reqwest`, `tar`, `flate2`, `zip`, `dirs` |
| `crates/legacy-core/src/update.rs` | New — moved from `legacy-app/src/update.rs` |
| `crates/legacy-core/src/lib.rs` | `pub mod update;` |
| `crates/legacy-app/src/update.rs` | Deleted |
| `crates/legacy-app/src/main.rs` | `use legacy_core::update;`, post-dispatch install hook |
| `crates/legacy-app/Cargo.toml` | Drop deps that moved |
| `crates/legacy-ui/src/components/update_modal.rs` | New — renders modal for each state |
| `crates/legacy-ui/src/components/mod.rs` | Declare module |
| `crates/legacy-ui/src/components/slash_suggestions.rs` | Register `/update` |
| `crates/legacy-ui/src/app.rs` | `UpdateState`, `update_state` field, `pending_install` flag, `/update` handler, tick poll, Y/N/Enter intercept, render call |
| `install.sh` | Drop "the app auto-updates on startup"; replace with "Run `/update` inside the app to install a new release" |

## Error handling

- Channel `try_recv` is non-blocking, so a stuck check never freezes the UI.
- `check_latest_version` errors → `Failed` modal with the message.
- `perform_update` errors (no matching asset, archive extraction failure,
  filesystem error) → `Failed` modal. The user remains in the old binary;
  no partial-install state is possible because writes go to `~/.local/bin/legacy`
  only after the download fully succeeds.
- If user re-runs `/update` while another check is in flight, the second
  invocation is a no-op (logged as `feedback.info("Update check already
  running…")`).

## Testing

- **Unit:** semver compare in `check_latest_version` (already covered by
  `self_update`'s tests; no new tests needed).
- **Manual:**
  1. `cargo run` — type `/update`, see "Checking for updates…", then
     "Already on latest" (since local version equals the released one).
  2. Temporarily set `package.version = "0.0.1"` and re-run; confirm the
     confirm modal appears, Y triggers install, alt screen is cleanly
     restored, Done modal appears.
  3. Disable network and run `/update`; confirm `Failed` modal renders.

## Open questions

None — design approved.
