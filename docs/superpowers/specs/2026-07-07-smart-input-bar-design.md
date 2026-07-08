# Smart input bar — design

2026-07-07 · Approach A ("smart router over existing pieces"), chosen after brainstorm.

## Goal

Typing a bare command in the input bar just works: `ls` runs in the focused
idle shell, `claude` opens a new pane running claude, a typo gets a hint
instead of a junk pane. Crew decides the destination; the palette shows the
decision before Enter commits it. `/shell` and `/run` become unnecessary for
day-to-day use (they keep working).

Decisions locked during brainstorm:

- **Focused-shell-first**: an idle focused shell receives the text; anything
  else diverts to a smart spawn.
- **PATH-verified + palette preview**: spawn only what resolves to a real
  executable; show the routing decision live in the palette.
- **Busy terminals divert**: input never types into a running program from
  the bar (click the pane to do that).
- **Broadcast is a prefix, not a mode** (revised in review): `* <text>` sends
  one line to every terminal, explicitly. Bar submits no longer consult the
  Cmd+S mode; Cmd+S keeps its pane-level synchronized typing (keystrokes in a
  focused terminal mirror to all terminals), which the bar never owned.

## Routing table

`submit_input` evaluates in order; first match wins.

| # | Input / state | Action | Change? |
|---|---|---|---|
| 1 | empty | no-op | unchanged |
| 2 | `/cmd` | slash dispatch | unchanged |
| 3 | `!cmd` | force-spawn pane via `run_in_pane` (skips all checks) | unchanged — now documented as the escape hatch |
| 4 | `cd [path]` | `try_change_dir` (moves crew's cwd) | unchanged — stays ahead of broadcast, as in today's code |
| 5 | `* <text>` (or `*text`) | bytes to every terminal pane, one-shot; "no terminals" status hint when none receive | new — replaces the bar's dependence on broadcast mode |
| 6 | focused pane is a terminal AND `foreground_pid().is_none()` (shell at prompt) | bytes into that terminal | narrowed (was: any focused terminal, busy or not) |
| 7 | first word resolves (see Detection) | `run_in_pane(line)` — new persistent pane labeled by the first word, focused | new |
| 8 | first word is a shell builtin (`export`, `source`, `alias`, …) | status hint "shell builtin — run it inside a shell pane" | new |
| 9 | otherwise | status hint `not a command — !… forces a pane` | new (was: bytes to focused terminal / "no shell here" hint) |

Consequences worth naming:

- With **no panes open**, `ls` now spawns a pane and runs it (today: hint).
- A focused **chat/md/settings pane** plus `claude` → new pane (today: the
  text went nowhere).
- Driving a REPL/agent by typing plain text into the bar while it is focused
  **stops working** (rule 6 diverts, rule 9 hints). That interaction moves to
  the pane itself; `* <text>` deliberately reaches every terminal, busy or
  not, when you want the firehose.
- With Cmd+S broadcast mode ON, bar submits now follow the same routing as
  ever — the mode only synchronizes keystrokes typed inside terminal panes.
  (Before: mode ON meant every bar submit hit all terminals.)

## Detection (`cmdcheck` module, new)

`resolve(line) -> Verdict { Executable(name), Builtin(name), No }`, pure and
synchronous:

- Strip leading `VAR=value` tokens (`FOO=1 cargo test` checks `cargo`).
- First whitespace-delimited word, surrounding quotes stripped.
- Contains `/` → expand `~`, stat: executable regular file?
- Else walk the **hydrated PATH** checking for an executable file.
- Builtin list: `export set unset source . alias unalias eval` (cd is rule 5).
- Shell keywords (`for`, `while`, `if`) resolve to `No` → hint; `!` forces.
  Interactive aliases also resolve to `No` — the non-interactive `$SHELL -c`
  wouldn't see them anyway (same limitation `/run` has today).

Per-keystroke cost is a handful of `stat` calls; memoize on the first word so
the palette preview doesn't re-stat while the argument tail is typed.

### PATH hydration

Dock-launched crew inherits launchd's minimal PATH, so `claude` in
`~/.local/bin` would run fine (spawn goes through `$SHELL -c`) yet fail
detection. At startup, capture `$SHELL -lc 'printf %s "$PATH"'` **on a
background thread** (winit main-thread rule) and swap it in when it arrives;
until then fall back to the process PATH. Honors `CREW_SHELL_ENV=0` like the
broker's hydration.

## Idle probe

Already exists: `PtyTerm::foreground_pid()` returns `None` when the shell is
its own foreground process group (prompt idle) — used for pane titles today.
Rule 6 calls it at submit time; the palette preview calls it per redraw, so a
program exiting between preview and Enter re-routes correctly (Enter always
re-evaluates). On non-Unix `foreground_pid()` is always `None`, so every
focused terminal counts as idle — graceful degradation to today's behavior.

## Palette preview

`suggest::menu_items` today returns rows only for `/` input. Add one row for
non-empty, non-slash, non-`!` text, mirroring the routing table:

- rule 5 hit (`* …`) → `↵ broadcast to 3 terminals` (submit row)
- rule 6 hit → `↵ type into pane 3 · zsh` (submit row)
- rule 7 hit → `↵ run claude — new pane` (submit row)
- rule 8/9 → dim non-submit hint row (`not a command — ! forces a pane`), so
  a no-op Enter is never a surprise

The row reuses the existing `MenuItem`/`cmdmenu` card; no new UI surface. The
fish-style history ghost is unaffected.

## Existing commands

- `/run`, `/shell` stay in dispatch (scripts, muscle memory) but leave the
  palette `COMMANDS` list; `/run`'s palette slot is redundant with bare text,
  `/shell`'s with Cmd+T.
- `!cmd` unchanged; it is the documented escape hatch for "spawn it anyway".
- Cmd+S keeps pane-level synchronized typing and the `»` border badges; it no
  longer influences bar submits. `/broadcast` (the toggle's slash form) stays.
- Prefix summary the bar now speaks: `/` app command · `!` force a pane ·
  `*` broadcast · bare text = smart routing.

## Testing

- `cmdcheck` unit tests: env-prefix stripping, `~`/relative paths, quotes,
  builtins, keywords, non-executables, PATH-miss.
- Routing tests: pure decision function exercised for each table row (Far
  pane focused, idle terminal, busy terminal via injected foreground state,
  `*` prefix with and without terminals, broadcast mode ON not affecting bar
  submits, empty grid).
- `menu_items` tests for the three preview row shapes.
- Live verification via `.claude/skills/verify`: idle shell + `ls` lands in
  the shell; focus the md viewer + `top` spawns a pane; typo `caude` hints;
  screenshot the preview row states.

## Out of scope (deliberate, Approach B material)

Scratch-pane reuse for quick commands, focus-existing-instead-of-respawn
(`claude` twice), TUI/long-running classification, alias expansion.
