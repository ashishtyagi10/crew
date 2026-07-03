# Long-running agents: concurrent background tasks

2026-07-03. Goal: let crew run several agent tasks in the background at once,
instead of one-at-a-time.

## Problem

Tasks already run on a background worker thread (the pane input never blocks),
but the broker allows only ONE at a time: a second Send while one runs is
rejected with "busy with '…' — /stop cancels it", governed by a single
`session.busy` label, a single `session.cancel` flag, and a single
`worker: Option<JoinHandle>` in `run_broker_stdio`. Long, lengthy work
therefore monopolises the pane. We want to fire a task and keep firing/chatting
while earlier tasks keep working, each addressable and stoppable on its own.

## Design

### Task registry (broker) — `broker/tasks.rs` (new)
Replace the single worker/busy/cancel with a `Tasks` registry owned by
`run_broker_stdio`:
```
struct Task { id: u64, label: String, cancel: Arc<AtomicBool>,
              handle: JoinHandle<()>, started: Instant }
struct Tasks { next_id: u64, running: Vec<Task> }
```
- `spawn(label, cancel) -> id`: assign a monotonic id, push the Task.
- `reap()`: drop finished tasks (`handle.is_finished()`), called each command.
- `running_ids()/describe()`: for `/tasks`.
- `cancel(id)` / `cancel_all()`: set the per-task cancel flag(s).
- `len()`: live count, for the concurrency cap.

`Session.cancel: Arc<AtomicBool>` becomes per-task: each spawned task gets its
OWN `Arc<AtomicBool>`, threaded into the `Broker` via the existing
`with_cancel_flag`. The `Session`'s single `busy`/`cancel` fields are removed
(the registry replaces them); `/status` reads `tasks.len()`.

### `stdio::send` changes
- `/stop [#N]` handled inline: bare `/stop` → `tasks.cancel_all()` + "stopping
  all N tasks…"; `/stop #N` (or `/stop N`) → cancel that id or "no task #N".
- Quick constructs still answer inline (unchanged).
- A task Send: if `tasks.len() >= max` (`CREW_MAX_TASKS`, default 4) → reject
  with "at capacity (N tasks) — /stop one first"; else spawn a new task with a
  fresh cancel flag, emit `msg("crew", "▸ task #N started · <label>")`, and run
  the worker. The old "busy" rejection is gone.
- On worker completion, the worker emits `✓ task #N done · <E> exchanges · <t>`
  (or `✗ task #N stopped` when cancelled / `✗ task #N: <error>`), then the
  registry reaps it.

### Per-task attribution (protocol, no schema change)
Every task's streamed `Message`/`Stats`/`Activity` events must be attributable
to their task so interleaved streams read cleanly. Carry the id in the existing
`Message.meta` field as `"task:<id>"` (meta is already free-form for the host).
The worker's `emit` closure stamps `meta` with its task id on Message events.
`Activity`/`Stats` gain no schema change; the app correlates them by the
active-agent name as today (good enough for v1 — a task rarely shares an agent
with a concurrent task in practice; note this limitation).

### App side (crew-app) — minimal
- The chat pane prefixes a message whose `meta` carries `task:<id>` with a dim
  `#<id>` chip, so interleaved task output is distinguishable.
- The header/session line shows a dim `N tasks` chip when >1 task runs (reuse
  the existing session-chip mechanism from the status redesign).
- No new pane; a task dashboard is a follow-up.

## Constructs summary
- `/tasks` — list running tasks: `#3 · refactor the module · 2m · 6 hops`.
- `/stop` — cancel all running tasks.
- `/stop #N` — cancel task N.
- `/status` — now reports live task count alongside session totals.

## Concurrency & safety
- Cap: `CREW_MAX_TASKS` (default 4). Over cap → reject the new Send (no queue in
  v1).
- Each task is fully independent: its own cancel flag, its own `Broker` (built
  from the session snapshot as today), its own hop/timeout limits.
- Shared `out` (stdout writer) stays `Arc<Mutex<>>` — interleaved emits are
  line-atomic, so concurrent tasks can't corrupt each other's JSON lines.
- On stdin EOF (pane gone), join ALL running task handles (as the single worker
  is joined today) so output isn't truncated mid-line.

## Out of scope (v1)
- Persistence across broker/pane restart (tasks are in-memory).
- Per-task agent/roster isolation (tasks share the registry).
- A dedicated task-dashboard pane.
- Queueing beyond the cap.

## Testing
- `tasks.rs`: spawn assigns increasing ids; reap drops finished; cancel(id)
  flips only that flag; cancel_all flips all; len/describe correct; cap
  enforcement (a pure `admit(len, max) -> bool`).
- `stdio` (mock provider `CREW_BROKER_MOCK_REPLY`): two concurrent Sends both
  run and both emit start+done; a third over cap is rejected; `/stop #N`
  cancels only N; bare `/stop` cancels all; `/tasks` lists them; Message events
  carry `meta = "task:<id>"`.
- crew-app: a message with `meta="task:3"` renders a `#3` chip; the `N tasks`
  header chip appears when >1 runs.
