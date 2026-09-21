# Goal — a todo tagged `@project` can run itself: crew picks the agent, opens a pane in the project, and the agent does the work

**Status: SHIPPED v0.22.79, chip made live in v0.22.80; v0.22.82 added
the `&` door (a draft ending in `&` runs on add — the user's first live
test expected exactly that, and `r` was invisible). Set 2026-09-20, built
the same day.** Done-means 1–5 and 7 hold in the tree (`todorun.rs`,
`todoruncmd.rs`, `todopane/projectdir.rs`, `todopane/agentpick.rs`,
`todopane/runlive.rs`), with two honest gaps: the tag popup has no
`bind…` row — `r` on an unplaced project fills the bar with the binding
command instead (v0.22.81), which is the one-keystroke ask the doc wanted
by another door — and the row has no `▶` chip BEFORE a run and no
`no directory` column (the status says it on `r`).
The run chip is `▶agent` while the agent runs and `▷agent` after, with
no exit time. v0.22.79's chip panicked the pane on its first draw (a byte
slice through `▶`); v0.22.80 fixed it. Done-means 6 holds as written. Not
GUI-verified: the evidence is the ladder table, the resolver on a temp
tree, the registry round trip and a real-shell test that reads the project
directory back off the pane a run opened.

## The ask

> write a goal to make todo smarter, if we tag a project using @ .. crew
> should find the best agent for the job (be it claude, opencode, smith),
> open a new pane in that directory and implement the solution autonomously.

## What it means

The `/todo` pane knows three things about a task: its words, its date, and
two tags — `@project` (2026-08-12) and `#assignee` (2026-09-11). Every one of
them is for a human to read. Nothing in the pane can DO a task. Meanwhile
the rest of crew can: a bare line runs in a shell (2026-09-19), the smith
pane runs a swarm in a directory, and the broker already drives `claude`,
`codex` and `opencode` as relay agents (`agents.rs`) with their own
sign-ins. The two halves have never met. A user who writes
`fix the flaky motion-level test @crew` has told crew everything an agent
needs — the work, and where it is — and then has to open a shell, `cd`, and
type the sentence again into a coding agent.

So the todo list becomes a place work is **handed off from**, not only
tracked in. For an item whose `@project` names a directory crew can find,
one keystroke runs it: crew chooses the agent, opens a pane in that
directory, gives the agent the item's words as its task, and gets out of
the way. The item carries the run's state back so the list still answers
"where is everything".

Three readings to settle up front, because each is the difference between
a feature and a trap:

- **`@project` becomes a place, not only a label.** Today it is a free-form
  word. It stays free-form — `@personal`, `@dentist` are fine and nothing
  runs for them — but a project that crew can put a directory to is a
  project that can run. The mapping is learned, shown, and editable; it is
  never guessed silently.
- **"Best agent" is a deterministic ladder, not a model call.** Choosing
  which of three CLIs to launch is not a question that needs an LLM, and a
  model call to decide it would cost money and time before the work even
  starts. The ladder is small, its reasons print in the pane's first line,
  and an explicit `#agent` on the item outranks every rung of it.
- **"Autonomously" means crew does not wait for you to type the task. It
  does not mean the agent skips its own permission model.** Crew launches
  each CLI exactly as the user would at a prompt: Claude Code asks before
  it edits or runs, unless the user has set otherwise in Claude's own
  settings; opencode and codex likewise. Crew passes no skip-permissions
  flag. That is the sentinel-loop rule (consent-based only) and the
  reach-the-world non-negotiable (an agent cannot raise a tool's tier)
  applied to a launch. When the agent stops to ask, the pane is a WAITING
  ON YOU card in the nav and the auto-focus rule brings it forward: the
  pane asks, the list does not.

One reading NOT taken: adding an item does not run it. `buy milk @home` is
a todo. A run is a deliberate act on an item — a key on the row, a chip on
the row, or a command — because a run opens a pane, spends a subscription,
and lets an agent loose in a checkout. The ask's "if we tag a project"
is honoured as: tagging is what makes the item RUNNABLE, and the row says
so the moment the tag resolves.

## Done means

1. **A project can resolve to a directory.** `@crew` resolves when, in
   order: the user bound it (`/todo project crew ~/code/crew`, or the
   tag popup's `bind…` row); a pane is open whose directory's git root, or
   the directory itself, has that base name; a directory of that name
   exists under a parent of any pane's directory (`~/code/crew` when some
   pane sits in `~/code/anything`). Resolution is written to
   `projects.toml` beside `todos.toml` the first time it succeeds, so the
   next resolution is a lookup and the file is the one place to correct
   a wrong guess. An unresolved project never runs; the row says
   `@name · no directory` in its dim column, and `r` on it opens the bind
   picker listing the candidates crew considered.
2. **`r` on a runnable row runs it.** Also `Enter` on the row's `▶` chip
   and `/todo run [@project]` (the first open runnable item of that
   project, or the selected row with no argument). The item's title — the
   words with the tags and the date already stripped — is the task, verbatim.
   Nothing is rewritten by crew.
3. **The agent is chosen by a ladder and the choice is said aloud.** Top
   rung first:
   - `#claude`, `#codex`, `#opencode`, `#smith` as the item's assignee
     names the agent. A `#name` that is not one of these four is a person,
     exactly as today, and does not touch the ladder.
   - a marker file at the project root: `CLAUDE.md` or `.claude/` →
     claude; `opencode.json` or `.opencode/` → opencode; `AGENTS.md` with
     neither of those → codex (it is the file codex reads). Two markers
     tie and fall through.
   - the sign-in rows the `/model` picker already shows
     (`logincmd::rows_cached`): a `SignedIn` CLI beats an `Installed` one;
     among equals the serving provider (`/model`'s pin) wins, then the
     order claude, codex, opencode.
   - smith, when no CLI is installed and a provider serves; `/model` when
     nothing does — the run stops with `no agent — /model signs one in`
     in the status and the item is untouched.
   The pane's first line and the activity log both carry the reason:
   `▶ @crew · claude · CLAUDE.md, signed in`. The ladder is one pure
   function over `(assignee, markers, rows, pin)` and its test is a table.
4. **The pane opens in the project's directory.** A CLI agent runs in a
   terminal pane spawned with `spawn_labeled_terminal_in` at the resolved
   directory, under the persistent wrapper so the pane outlives the agent
   and leaves a prompt in that directory. The invocation gives each CLI the
   task as its opening prompt in the way that CLI takes one, verified
   against the installed binary before it ships, as `agents.rs` was.
   The smith rung opens the agent smith pane with the directory as its
   working directory (`spawn_plugin_pane` grows a `cwd`, which the broker
   already honours via `spawn_in`) and sends the task with `send_now`;
   plan-first, if on, gates it like any other message. The pane is
   labelled `@crew · claude` and focused.
5. **The item remembers the run.** `TodoItem` grows a `run` record — agent,
   started-at, pane label — persisted in `todos.toml` and drawn as a chip:
   `▶ claude · running` while the pane's agent is the foreground process,
   `▶ claude · exited 14:02` after. **An agent's exit is not the task's
   completion.** Nothing marks the item done but the user; the chip is
   there so the list can say what happened without the user reading the
   pane. `r` on an item that already ran runs it again and replaces the
   record.
6. **One smith pane, still.** `spawn_crew_pane`'s singleton rule holds. A
   smith run when the pane exists and is idle sends the task there with a
   `cd` to the project first; when it is busy the run does not queue
   silently — the status says `agent smith is busy — run with #claude, or
   wait` and the item is untouched.
7. **The list explains itself.** The composer legend, the row's dim column
   and `/todo`'s usage line all know the new words: `r` on a runnable row,
   `▶` on the row, `/todo run`, `/todo project <name> <dir>`. The key
   legend and `docs/CREW.md`'s `/todo` section say what a run is and what
   it is not (the agent's own permissions, the item not auto-done).

## How it is built

- `todopane::projectdir` (new) — the registry and the inference: load and
  save `projects.toml`, the three-rung resolution over the open panes'
  `Pane::dir` values, and the candidate list the bind picker shows. Pure
  over a slice of directories so its tests run on a temp tree.
- `todopane::agentpick` (new) — the ladder of done-means 3 as one function
  returning `(Agent, reason)`, plus the per-CLI invocation
  (`program, args`) for a task and a directory. No I/O: marker presence
  and sign-in rows are passed in.
- `todopane::runtask` (new) — the act: resolve, pick, spawn, record. Lives
  on `CrewApp`, next to `dispatchtodo.rs`, which gains `/todo run` and
  `/todo project`. The terminal shape reuses `run_parts` and the wrapper
  in `runpane.rs`; the smith shape reuses `chatspawn.rs` and
  `chatsend.rs`.
- `todopane/item.rs` — the `run` field and its toml round trip; a missing
  field reads as no run, so old files load.
- `todopane/listkeys.rs` — `r`; `todopane/rowchips.rs` — the `▶` chip and
  the run chip; `todopane/legend.rs` and `todopane/headrow.rs` — the
  words. Each is near its cap; anything over a screen gets its own file.
- Live state for the chip: whether the pane's agent is still running is
  the same foreground-process check the idle-shell rule uses, read on the
  todo pane's poll, not stored.
- The activity log line, so a run is forensically visible after the fact
  (`activity.log` is per-session and logs every status).

## What it must not break

- `#name` for people. Only the four agent names change meaning, and only
  at run time; `g` still bands under `#claude` like any other assignee.
- The `@project` filter axis, the tag popup's completion, and every
  ordering function taking `Filters`.
- Esc on the todo pane minimizes; a run focuses the new pane and leaves
  the list where it was in the nav.
- The idle-shell rule and the busy-terminal rule: the bar never types
  into a running agent; a run always opens its own pane.
- The line cap. Three new modules, and the four files touched are read
  against the cap before the first edit.

## Not done

- No auto-run on add, and no scheduled runs; the clock's standing intents
  (`daemon/intent.rs`) are the place a "run this at 7am" would live and
  this goal does not touch them.
- No skip-permissions mode. A later opt-in per project, if ever, is a
  separate goal with its own consent wording.
- No result flowing back into the item beyond the run chip: the agent's
  transcript is in the pane, not summarised into the list.
- No model-chosen agent. If the ladder proves wrong in practice, the fix
  is a rung, not a call.
- Not GUI-verified until the loop runs it: the evidence is the ladder
  table, the resolution tests on a temp tree, the toml round trip, the
  row-chip shot tests and one real-shell marker test proving a run lands
  in the right directory.
