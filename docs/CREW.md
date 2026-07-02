# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders in the terminal as tiles (no overlays). Crew is the
successor to this repo's original terminal file-manager project; the crates under
`crates/crew-*` are the product.

## Architecture

- **Rendering** — `winit` + `wgpu` + `glyphon`/`cosmic-text`. Every cell is drawn
  on the GPU; panes have SDF rounded borders. The rendering model has four
  invariants: the **cell box is fixed** at `(0.6, 1.25) × font size`, rounded to
  whole physical pixels and independent of the font family (every glyph advance
  — bold and wide CJK/emoji runs included — snaps to a whole number of cells
  via cosmic-text's `monospace_width`, so switching fonts never moves a pane, a
  border, or a column); **colours convert to linear once** at the GPU boundary
  (`crew_render::color`) because the surface is sRGB; **unchanged panes reuse
  last frame's shaped text** (content signatures in `scenecache`); and all cell
  placement is **display-width aware** (`chatwidth` — emoji/CJK advance two
  columns everywhere).
- **Terminal model** — `alacritty_terminal` + `portable-pty` (`crates/crew-term`).
- **In-pane UI** — `ratatui` widgets are laid out into a `Buffer` and converted to
  GPU cells (the settings form, command palette, and help overlay use this).
- **Crates** — `crew-app` (window, panes, input), `crew-render` (GPU), `crew-term`
  (PTY + grid), `crew-plugin` (chat/agent plugins + the `/crew` relay broker),
  `crew-hive` (the swarm orchestration engine — see
  [Swarm orchestration](#swarm-orchestration-crew-hive) below).
- **Diagram** — see [ARCHITECTURE.md](ARCHITECTURE.md) for the full app + engine
  diagram.

Hard rules: every `.rs` file stays ≤200 lines; `cargo clippy --workspace
--all-targets` is warning-free.

## Build & run

```sh
cargo run --release -p crew-app
```

## Panes

Panes auto-tile into a near-square grid. Each pane has a **title bar** (top row)
showing its index, the program-set title (often the cwd), and right-aligned
status glyphs:

| Glyph | Meaning |
|-------|---------|
| `⇡N`  | viewing scrollback, N lines back from the live bottom |
| `●`   | new output in an unfocused pane |
| `!`   | the program rang the bell |
| `»`   | receiving broadcast (synchronized) input |

The focused pane has a near-white border and a bright block cursor; unfocused
panes are grey with a dim cursor.

**Busy indicator.** While a pane is doing background work — a swarm planning or
running with live tasks, or an agent chat awaiting a reply — an **indeterminate
progress sweep** glides back and forth along its bottom border. It animates only
while the pane is actually busy (idle Crew never repaints), so the motion costs
nothing once the work finishes.

**Capacity & visibility.** Crew displays up to **6 panes as full tiles** in the
auto-tiling grid. Additional panes are demoted to a **minimized thumbnail strip**
along the bottom of the content area, each showing the pane's title and an
activity dot, ordered least-recently-active first. The focused pane is protected
from demotion. To restore a minimized pane to the full grid, click its thumbnail,
click its entry in the sidebar's PANES list, or use **Cmd+1 … 9** to jump to it.

## Keyboard shortcuts

Press **`/keys`** in the input bar for this list in-app.

| Action | Keys |
|--------|------|
| Next / previous pane | **Ctrl+Tab** / **Ctrl+Shift+Tab** (also Cmd+] / Cmd+[) |
| Jump to pane N | **Cmd+1 … 9** |
| Jump to next active pane | **Cmd+A** |
| Move pane left / right | **Cmd+{** / **Cmd+}** |
| Focus the input bar | **Cmd+I** |
| New shell pane | **Cmd+T** |
| Settings / chat pane | **Cmd+,** / **Cmd+J** |
| Toggle sidebar | **Cmd+G** |
| Zoom focused pane | **Cmd+Z** (or double-click) |
| Broadcast input to all panes | **Cmd+S** |
| Font bigger / smaller / reset | **Cmd+=** / **Cmd+-** / **Cmd+0** |
| Copy visible screen / paste | **Cmd+C** / **Cmd+V** |
| Insert a newline in a terminal | **Shift+Enter** (line feed, not submit) |
| Close pane / maximize window | **Cmd+W** / **Cmd+M** |
| Clear focused pane scrollback | **Cmd+K** (or `/clear`) |
| Scroll any pane | **Shift+PageUp** / **Shift+PageDown** (Shift+Home/End jump to top/bottom), or mouse wheel |
| Quit | **Cmd+Q** (press twice to confirm when panes are open) |

Click a pane to focus it (click the input bar to focus that); double-click a
pane to toggle zoom.

Inside a terminal pane, all other keys (arrows, Home/End, PageUp/Down, Ctrl+C,
Shift+Tab, …) pass through to the program. **Shift+Enter** sends a line feed
(0x0a) instead of a carriage-return, so agent CLIs and editors insert a newline
rather than submitting. Shells launch as your `$SHELL` login shell, so your full
config and plugins load.

## The input bar

The docked command bar supports:

- **Slash commands** — type `/` for a command palette (↑/↓ to pick, Tab/→ to
  fill, Enter to run): `/shell`, `/crew`, `/run <cmd>`, `/diff`, `/settings`, `/find <text>`, `/name <text>`, `/clear`, `/only`, `/copy`, `/dump`, `/font`, `/restart`, `/theme`, `/update`,
  `/broadcast`, `/zoom`, `/sidebar`, `/keys`, `/far`, `/exit`. The palette is **fuzzy** — prefix matches rank first,
  then subsequence matches (e.g. `/dmp` finds `/dump`) — and **scrolls** to the
  selection when the match list is long. When several commands share a prefix,
  the **shortest** is ghosted as the autosuggestion (e.g. `/clear` ghosts before
  `/clearlog`, which is one keystroke further). Commands with a **fixed set of
  values** (like `/theme`) expand into a **value picker**: select the command
  (or type its trailing space) and the palette lists the choices to arrow through
  and `Enter` — no need to remember or type the exact value.
- **`/broadcast`, `/zoom`, `/sidebar`** — palette-discoverable toggles that mirror
  the `Cmd+S` / `Cmd+Z` / `Cmd+G` chords, for when the chord slips your mind.
- **`/font <n>`** — sets the font size to an exact value (clamped 12–32), unlike
  the `Cmd+=`/`Cmd+-` chords that step by one; no argument reports the current size.
  The font *family* is picked in `/settings` — a type-to-search dropdown over
  every installed monospace family (the active one carries a `✓`); run
  `crew --list-fonts` in any shell to print the same list and check a newly
  installed font is visible to Crew. Inclusion is verified by measurement, not
  font-table flags: a family is listed when a candidate face (flagged
  monospaced or name-matched, so variable fonts like JetBrains Mono count)
  actually renders `i`, `m` and `0` at one shared advance — which is why
  proportional Unicode fallbacks and icon/symbol fonts that ship mis-flagged
  as monospace (Arial Unicode MS, Symbols Nerd Font Mono) don't appear.
- **`/restart`** — relaunches Crew as a fresh detached process and exits this
  one: the way to apply a binary installed by `/update`, and the fresh process
  re-reads `config.toml`, so edits made outside the `/settings` pane take
  effect too.
- **`/theme [name]`** — switches the theme live and persists it (`paper-dark`,
  `paper-light`, `crt-green`, `crt-amber`, `crt-blue`); no argument reports the
  current theme. Selecting `/theme` in the palette opens an arrow-selectable
  **picker** of the themes, so you don't have to type the name. `Ctrl+Shift+L`
  cycles through all of them. See [Themes](#themes).
- **`/only`** — closes every pane except the focused one (a quick "focus mode");
  a no-op when only one pane is open.
- **File operations live in Far and Cmd+click**, not slash commands: the old
  `/edit` and `/open` were dropped. `/far` browses/views/edits/copies files
  (F3/F4/Enter open the selection); **Cmd+click** on terminal text resolves it —
  a URL opens in the browser, an existing **file** opens in `$EDITOR` in a new
  pane, a **directory** becomes the working directory. http(s) URLs are
  **tinted blue** to show they're clickable. Path arguments to `/dump` expand
  `~` and `$VAR`/`${VAR}` and resolve relative paths against the working
  directory.
- **`/run <cmd>`** — launches `cmd` in its own tiled pane (labeled by the
  command) that stays open after it finishes, so builds, tests, and long-running
  jobs run alongside your shells instead of blocking one. This is also how you
  open a coding-agent CLI in a pane — `/run claude`, `/run codex`, `/run opencode`.
  (Distinct from `/crew`, which opens the multi-agent broker relay pane.)
- **`/diff`** — reviews the working tree's git changes in a new pane (à la
  Codex's `/diff`): a `git status --short` summary, the `diff --stat`, then
  the full colored diff, dropping to a fresh prompt afterwards. Pairs with the
  crew pane's `/checkpoint`/`/restore` for reviewing what agents changed.
- **`/copy`** — copies the focused terminal pane's **full scrollback** to the
  system clipboard (Cmd+C copies only the visible screen); the line count is
  flashed on the input bar.
- **`/dump [file]`** — exports the focused terminal pane's full scrollback to a
  file (handy for archiving a long build log or an AI agent's output); the saved
  path — with the line count and size — is shown on the input bar. With no argument it writes a timestamped
  `crew-dump-YYYYMMDD-HHMMSS.txt` in the working directory; with an argument it
  writes there (a relative path resolves against the working directory).
- **`/far`** — opens a Far Manager-style **dual-pane file manager** as a pane in
  the grid (like `/shell`): two side-by-side directory listings with a Far
  function-key bar and a **command line** at the bottom. `Tab` switches the active
  panel, `↑`/`↓`/`PgUp`/`PgDn`/`Home`/`End` move the cursor, `Enter` descends into
  a folder (or `..`) or opens a file with the OS default, `Backspace` climbs to
  the parent, `F5`/`F6` copy/move to the other panel, `F7` makes a folder, `F8`
  trashes, `F10` closes. Type on the **command line** and press `Enter` to run a
  command against the **active panel** — `cd <path>` navigates that panel in
  place, anything else runs in its directory on a worker thread (a `⟳` note
  shows while it runs, the listings reload when it finishes, and the result
  flashes in the status bar — no new pane is spawned); `Esc` clears a typed
  command (and closes the pane when it's empty).
- **`/crew`** — opens a **multi-agent pane** where the installed CLI coding
  agents (claude, codex, opencode) message each other to work a task. See
  [Multi-agent relay](#multi-agent-relay-crew) below.
- **Autosuggest** — fish-style ghost text from history; Tab/→ accepts it.
- **History** — **Up/Down** recall previous lines; type a prefix first and they
  recall only entries **starting with it** (zsh/fish-style prefix search; an empty
  input recalls everything). Persisted to
  `$XDG_CONFIG/crew/history` across sessions.
- **Path completion** — `cd <partial>` ghost-completes the first matching
  subdirectory, while `/dump <partial>` completes **files and** directories;
  Tab/→ accepts it. `$VAR`/`${VAR}` are expanded (e.g. `cd $HOME/src`).
  `cd -` toggles back to the previous directory;
  the working directory is restored on the next launch.
- **Editing** — **Ctrl+W** delete the last word, **Ctrl+U** clear the line.
- **Working directory** — the bar's legend shows Crew's current directory
  (`~`-abbreviated). Type **`cd <path>`** (or bare `cd` for home) to move it; new
  shells (**Cmd+T** / `/shell`) open in that directory.
- **`/name <text>`** titles the focused pane (shown in its title bar); bare
  `/name` clears it back to the program title.
- **Status flashes** — transient messages (e.g. "copied 12 lines", "cd: no such
  directory") appear briefly on the input card's bottom border.
- Anything that isn't a slash command or `cd` is sent to the focused terminal.

## Clipboard

- **Cmd+C** copies the focused terminal's visible screen to the system clipboard.
- **Cmd+V** pastes into the focused surface (terminal, input bar, or chat). For
  terminals it uses bracketed paste when the program enabled it. When the
  clipboard holds an **image** (and no text), it's written to a temp PNG and the
  file path is pasted instead — so agent CLIs can read the image by path.
- Programs can copy to the system clipboard via **OSC 52**.

## Scrollback

Mouse wheel or **Shift+PageUp/PageDown** scroll a pane's history (Shift+Home/End
jump to top/bottom); an amber `⇡` in the title bar marks that you're viewing
scrollback. Scrolling works in **every** pane — terminals and chat scroll their
history, the Far file browser moves its cursor, and the settings form moves
between fields. In a **full-screen program** (the alternate screen — vim, less,
an agent TUI like `claude`) there's no terminal scrollback to move, so the wheel
is **forwarded to the program** instead: as mouse-wheel events when it enabled
mouse reporting, or arrow keys under xterm "alternate scroll" — so scrolling its
own view just works. Typing into a pane clears any leftover mouse-selection
highlight, so a stale selection never lingers over fresh output. **`/find <text>`** scrolls
back to the most recent line containing the text (smart case: case-insensitive
unless the term has an uppercase letter), **highlights every match** in the
viewport with an amber wash, and reports the in-view match count on the status
line (a miss reports too). Returning to the live bottom clears the highlight.

## Multi-agent relay (`/crew`)

`/crew` opens a pane that lets independent headless CLI coding agents talk to
each other to work a task you give them. Any registered agent can be sender or
recipient — claude ↔ codex ↔ opencode.

**Discovery.** On open, the broker probes each known agent (claude, codex,
opencode) to see whether its CLI is installed, and registers only the ones it
finds; the pane lists them (and notes when none are present). Adding a fourth
agent is one adapter (see *Architecture* below) — discovery and routing don't
change.

**Sending a task.** Type a task and press Enter. By default the first detected
agent starts; prefix `@<agent>` (e.g. `@codex refactor this`) to choose who
starts. The agent receives a clean, normalized message — never another agent's
raw CLI output.

**Routing protocol.** Each agent is told who it is, what its peers are good at
(a capability hint per agent), and the task + a transcript of the conversation
so far. It answers, then ends its reply with a final control line:

- `@next <agent>` to **hand off** to a peer (only from the listed peers);
- `@done` (optionally `@done: <answer>`) to **end the thread** — the explicit
  no-reply signal.

Parsing is tolerant of markdown/punctuation wrappers (`**@next codex**`,
`` `@done` ``). If an agent forgets the line, the broker re-asks it once to add
one; a still-missing directive ends the thread rather than mis-routing. This
proves out as `A→B` (claude hands to codex), `B→A` (codex relays back), and a
**3-way relay** (claude → codex → opencode, answer relayed back to claude).

**Loop guard & timeouts.** Every message carries a hop counter; once it passes
the limit (default 6) the broker drops the thread and logs that it stopped, so a
relay can never loop forever. Each agent call has a timeout (default 180s) — a
hung agent is killed and logged, and the broker moves on.

**Observability.** Every hop is logged in the pane as `from → to` with the
reply, so the whole conversation — including `[done]`, `[stopped]`, and
`[error]` outcomes — is visible. The pane renders this as a multi-agent
console: row 0 is a status header (connection dot, message count, a completed
**turns counter**, a running `~N tok` meter, and — while an agent works — a
spinner naming it with live elapsed seconds); row 1 is the **agent roster**
streamed by the broker as a structured `roster` event — a colored chip per
agent with role + model badge, the currently-thinking agent highlighted
(`▸` + bold), and once an agent has replied a dimmed **stat suffix**
(`·3× 4.2s` — replies and average latency, fed by reply-level `stats`
events the broker emits per timed segment). While agents work, row 2
becomes a **live activity row**: one animated chip per working agent —
`⠹ user ⇢ planner 4s` — naming who handed it the task (the user, a relaying
peer, or the goal judge) with a spinner and elapsed seconds, so parallel fans
and hand-offs are visible as they happen. Messages render as
**cards**: a `▍sender` header in the sender's stable colour (hand-off senders
like `planner → coder` colour each name), a muted `· 2m ago · 4.2s` tail
(epoch-ms `ts` + per-reply latency `meta` stamped by the broker), and the
wrapped body beneath. Live agent state flows as structured `activity` events
(`thinking` per dial — carrying who dialed as `from` — and `idle` at turn end)
instead of transcript spam, and each
turn ends with a `stats` event plus a timeline summary: `turn done — planner
4.2s → coder 8.1s · 2 exchange(s) · ~950 tok (approx)`.

Message bodies are newline-aware, and fenced ```code``` blocks render as
bordered cards — a muted `╭─ lang` header, verbatim hard-wrapped lines on a
dimmed background, `╰─` footer. A just-landed card **fades in** from the page
colour over ~400ms (the fade drives redraws without reading as "busy"). The composer on the bottom rows shows an
affordance bar (`@agent` chips in roster colours, `Enter send · Esc close`
hints) above a `❯` prompt that highlights a valid leading `@mention` in that
agent's colour. While the transcript overflows, the last column shows a
proportional scrollbar, and messages arriving out of view raise a `↓ N new`
pill that clears at the live bottom. A fresh pane greets with the detected
crew (names, roles) and an example `@agent` prompt.

**Constructs.** Inside the pane, lines starting with `/` drive the broker
itself (Tab completes both `@agents` and `/constructs`):

- **`/help`** — list the constructs; **`/agents`** — the roster with each
  agent's role and model; **`/status`** — what's running, session turn/token
  totals, and the model pins.
- **`/model <agent> <model|default>`** — pin an agent to a model for the
  session. Pins apply per agent, so **planner, coder, and reviewer can run
  three different models side by side**; every change re-emits the roster so
  the pane's model badges update live.
- **`/fan <task>`** — every agent answers the same task **in parallel** (one
  thread per call); replies stream back fastest-first with per-agent latency,
  and the turn closes with combined stats. **`@a+b <task>`** fans out to just
  that subset.
- **`/loop <n> <task>`** — n relay rounds (≤10), each round handed the
  previous round's answer to improve on.
- **`/goal <text>`** — relay rounds until a judge agent (the reviewer when it
  isn't the worker) rules `MET:`/`NOT MET:` on the goal; NOT-MET reasons feed
  the next round. Caps at 5 rounds.
- **`/plan <task>`** — plan mode (à la Claude Code): an agent (prefix
  `@agent` to pick who) drafts a numbered plan and **nothing executes** until
  **`/approve`** hands the approved plan to the relay; **`/reject`** discards
  it. The draft survives on the session until one or the other.
- **`/checkpoint [label]`** — Cline-style workspace snapshot: the working
  tree (tracked + untracked, `.gitignore` respected) is committed through a
  temporary index and pinned under `refs/crew/` — HEAD, your index, and
  branches are never touched, and snapshots survive broker restarts.
  **`/checkpoints`** lists them oldest-first; **`/restore <n>`** puts that
  snapshot's files back (files created after the snapshot are left in place).
- **`/skills`** — list the loaded prompt playbooks; **`/skill <name> <task>`**
  — run the relay with that playbook prepended to the task (see *Extending*
  below).
- **`/mcp`** — list the configured MCP servers and their tools (see
  *Extending* below).
- **`/export`** — write the pane's transcript to
  `crew-transcript-<stamp>.md` in the working directory (à la OpenCode),
  one `## sender · time · latency` section per message. Answered by the pane
  itself, so it works even while the broker is busy.
- **`/stop`** — cancel the running construct at its next checkpoint (between
  hops/rounds). Long work runs on a worker thread, so `/stop`, quick
  constructs, and `/status` answer immediately while a task is in flight; a
  second task is politely rejected until the first ends.

**Extending (skills · plugin agents · MCP).** Three drop-in surfaces, no
rebuild required — the same trio other coding tools ship:

- **Skills** are markdown playbooks in `~/.config/crew/skills/*.md` (user) or
  `./.crew/skills/*.md` (project; wins on a name clash). Optional `---`
  frontmatter sets `name:` and `description:`; otherwise the file stem and
  first line are used. `/skills` lists them; `/skill <name> <task>` runs the
  normal relay with the playbook prepended, so every agent in the thread
  follows it.
- **Plugin agents** join the roster from JSON manifests in
  `~/.config/crew/agents/*.json` or `./.crew/agents/*.json`:
  `{"name": "aider", "command": "aider", "args": ["--message", "{}"],
  "role": "repo-wide edits"}`. `{}` is the message placeholder (appended when
  missing); manifests whose command isn't on `$PATH` are skipped, and a
  manifest can't shadow an inbuilt agent. With manifests present, `/crew`
  works even with **no API key at all**.
- **MCP servers** are declared in `~/.config/crew/mcp.json` or
  `./.crew/mcp.json` with the familiar schema —
  `{"mcpServers": {"fs": {"command": "mcp-server-fs", "args": ["--root", "."],
  "env": {}}}}` — and connect lazily over stdio (JSON-RPC 2.0, hard
  per-request deadlines, killed with the pane). `/mcp` lists each server's
  tools. When servers are configured, every relay prompt advertises the tools
  and an agent calls one by ending its reply with
  `` `@tool <server>:<tool> {"arg": …}` `` — the broker runs the tool, logs
  the call and result as visible hops, feeds the result back to the same
  agent (up to 4 tool rounds per hop), then normal `@next`/`@done` routing
  resumes.

**Models & rate-limits.** When no agent CLIs are installed, `/crew` runs its
inbuilt API agents (planner/coder/reviewer) over an LLM: it prefers
`OPENROUTER_API_KEY` (free models by default), then `DASHSCOPE_API_KEY`
(Alibaba Cloud Model Studio — Qwen commercial models, `qwen-max` →
`qwen-plus` → `qwen-turbo`, override with `CREW_DASHSCOPE_MODEL=a,b,…`; the
endpoint defaults to the international region, point `CREW_DASHSCOPE_BASE_URL`
at the China host if your key lives there), and falls back to
`ANTHROPIC_API_KEY`. To survive OpenRouter's free-tier throttling, the provider
retries transient rate-limits (honoring `Retry-After`) and then rolls through a
**fallback chain** of free models on *different* upstream providers — so one
provider's limit doesn't stall the relay. Override the whole chain with a
comma-separated list, tried in order:

```sh
export CREW_OPENROUTER_MODEL="deepseek/deepseek-chat-v3.1:free,qwen/qwen3-235b-a22b:free"
```

Free models still share a hard account-wide daily cap; for sustained heavy use,
put a cheap **paid** slug (no daily cap) in the chain, or buy OpenRouter credits.

**Isolation & threading.** Agents run in a broker **subprocess** (the
`crew-broker-plugin` binary) over Crew's JSON-line plugin protocol, so all the
slow agent calls happen off the render thread and the window stays responsive.
An adapter normalizes each agent's stdout before it is ever shown or relayed
(claude `-p --output-format text` and `codex exec` print the reply on stdout;
opencode's `--format json` event stream is parsed for the assistant text).

**Architecture.** The reusable broker lives in `crates/crew-plugin/src/broker/`:
`Envelope { from, to, thread_id, hop, body }` is the message shape, an `Adapter`
turns a body into a clean reply, the `Registry` maps name → adapter (populated by
`discover()`), and the engine drives the relay with the loop guard. **To add an
agent:** write one constructor in `agents.rs` and push it into `known_adapters` —
nothing in the engine changes.

**Tuning (environment).** Keep cost and reliability in check without rebuilding:
`CREW_CLAUDE_MODEL` / `CREW_CODEX_MODEL` / `CREW_OPENCODE_MODEL` point an agent at
a specific (e.g. cheaper) model; `CREW_BROKER_MAX_HOPS` (default 6) caps relay
depth; `CREW_BROKER_TOKEN_BUDGET` (default 0 = unlimited) caps a thread's
approximate token spend; `CREW_BROKER_TIMEOUT_MS` (default 180000) bounds each
agent call; `CREW_MCP_TIMEOUT_MS` (default 30000) bounds each MCP request. The pane also prints a per-turn timeline + cost summary (`turn done
— planner 4.2s → … · N exchange(s) · ~X tok (approx)`) at the end of every
task, and accumulates the spend into the header's `~N tok` meter.

## Swarm orchestration (`crew-hive`)

The `/crew` relay is a few CLI agents talking turn-by-turn. **`crew-hive`** is the
next tier: a headless orchestration **engine** for running *many* agents toward a
single goal — the substrate behind Crew's "command a fleet of agents" direction.
It is a standalone workspace crate (no GPU, no terminal), driven by `crew-app`.

**The loop.** A goal is decomposed into a task-graph, executed over a bounded
pool of agents, and the results merge upward while live telemetry streams out for
the swarm view:

```
goal ─► Planner ─► TaskGraph (DAG) ─► Scheduler ─► Agent pool ─► Blackboard
                                          │             │            │
                                          └── EventBus ◄┴────────────┘
                                                  └─► Fleet telemetry ─► swarm view
```

**Components** (one module each):

- **Planner** (`planner`) — turns a goal into a dependency DAG. `StubPlanner`
  is deterministic (a fan-out + merge, for tests); `LlmPlanner` asks an LLM to
  return the graph as JSON and parses it.
- **Task graph** (`graph`) — `TaskGraph`/`TaskSpec` with validation (no cycles,
  deps exist) and `ready()` readiness; each task carries an `AgentKind` and a
  `ModelTier`.
- **Scheduler** (`sched`) — a `tokio` DAG executor: spawns ready tasks onto a
  `JoinSet` gated by a `Semaphore` (the concurrency cap), waits for fan-in,
  records results, and emits state transitions. A failed task **cascade-cancels**
  its dependents; a panicking agent becomes a failed task (the run survives);
  `with_cancel` gives cooperative, graceful shutdown (stop new dispatch, cancel
  unstarted, drain in-flight).
- **Agents** (`agent`, `apiagent`, `remoteagent`) — a uniform `Agent` trait
  (object-safe, no `async-trait`). `StubAgent` for tests; **`ApiAgent`** is a
  *native* LLM agent — just a future calling a provider, no PTY/subprocess, so a
  fleet scales to thousands; **`RemoteAgent`** dispatches a task over a
  `Transport` to an out-of-process worker.
- **Blackboard** (`board`) — a concurrent `Arc<RwLock>` store: agents `gather`
  their dependencies' `TaskResult`s and write their own, plus free-form
  artifacts. A serializable snapshot crosses the remote boundary.
- **Providers** (`provider`) — bring-your-own-LLM. A `Provider` trait with a
  `MockProvider` (tests) and an `AnthropicProvider` (HTTP `POST /v1/messages` via
  `reqwest`). `ModelTier` maps cost tiers to models —
  Cheap→`claude-haiku-4-5`, Standard→`claude-sonnet-4-6`, Capable→`claude-opus-4-8`.

**Two modes, one engine.** Single-goal decomposition (the planner builds a DAG)
*and* embarrassingly-parallel batches — `batch_graph(jobs)` builds a flat
dependency-free graph the same scheduler runs.

**Cost governance** (`govern`). `budget_governor` watches the event bus,
accumulates cost via a `Fleet`, and trips the scheduler's cancel flag once a
`Budget`'s micro-USD ceiling is crossed — a hard spend cap across the run.

**Swarm view** (`telemetry` + crew-app's `swarm/view`). The `EventBus` (`bus`) is
a non-blocking broadcast of `HiveEvent`s (state, tokens, cost, output); a `Fleet`
aggregates them per-agent. The pane renders the fleet as a **task list** — one row
per task with a state glyph (○ pending · ● running · ✓ done · ✗ failed), its
title, and the agent's last output line while it runs or after it fails — under a
`live / done / failed / cost` HUD row.

**Remote spill & sidecar bridge** (`wire`, `worker`, `remoteagent`). A
newline-delimited JSON protocol (`RemoteTask`/`RemoteReply`) over a `Transport`
trait lets the scheduler dispatch tasks out-of-process. `LoopbackTransport` runs a
handler in-process (and powers the tests); `serve_stdio` is the worker side — the
exact line an external engine (e.g. LangGraph) implements to act as a sidecar.

**Status.** The engine is wired into the app through two commands, each opening
a live swarm pane (task list + a `live / done / failed / cost` HUD, redrawn
every frame on a worker-thread event bridge):

- **`/goal <text>`** — plans the goal into a task-graph off the UI thread, then
  runs it. With `ANTHROPIC_API_KEY` it uses the real `LlmPlanner` + `ApiAgent`
  workers (each task billed at its per-task `ModelTier`); without a key it falls
  back to the deterministic stub backend, so the whole flow works offline.
- **`/batch <file>`** — a file of jobs (one per line) as a flat all-parallel swarm.

Real-LLM `/goal`/`/batch` runs are capped by the `budget_governor` (default
$1.00), and the pane surfaces a cancellation notice when the cap trips. The agent
factory family is complete — `StubFactory`, `ApiFactory`, and `RemoteFactory`
(over a `Transport`) — so the scheduler can run stub, native-API, or remote
graphs through one interface. Design rationale and roadmap:
[`docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md`](superpowers/specs/2026-06-27-crew-agent-swarm-design.md).

## Sidebar

A docked left panel (toggle with **Cmd+G**) with stacked, line-divided sections:
a live **TIME** clock, **SYSTEM** CPU/MEM/DISK gauges followed by a moving
**CPU sparkline**, a **LOAD** section (1/5/15-minute load average, coloured by
load-per-core), a **HOST** section (hostname, OS, uptime), a **NET** section
(down/up byte rates plus an auto-scaled throughput sparkline), and — when the
working directory is a repository — a **GIT** section showing the current branch
(with `↑`/`↓` commits ahead/behind the upstream) and a clean / `● N changed` marker. Below those, a **LOG** section keeps a live tail of
recent status messages (the same lines flashed on the input bar, newest last) so
activity history persists instead of vanishing after a few seconds, and a
**PANES** list of the open panes (index, name, a `▸` focus marker, and an
activity dot) fills the remaining height. Click a PANES row to focus that pane
(double-click to zoom it). The panel's **card legend shows the running version**
(`crew vX.Y.Z`), so the build is always visible at a glance.

## Settings

`/settings` opens a scrollable form covering **every configurable property**
(one row per field; Tab/wheel move focus, the list scrolls to keep it visible):

- **Font family** — type-to-search over installed monospace families.
- **Font size**, **Nav width**, **Show nav**.
- **Theme** — ←/→/Space cycle through the five presets.
- **Accent (#hex)** — override the theme accent; clear to use the default.
- **Paper texture** (on/off) and **Paper grain** (0–2 amplitude).
- **Launch maximized**.
- **Notifications** — the master switch plus per-event toggles (**cmd done**,
  **bell**, **pane exit**), the **min secs** threshold, and the comma-separated
  **patterns** watched in pane output.

Settings persist to `$XDG_CONFIG/crew/config.toml` and apply live on Save.

## Themes

Crew ships five themes: two e-ink-reader looks designed to read like paper
rather than a screen, and three old-school CRT phosphor tubes.

- **`paper-dark`** (default) — a high-contrast "newspaper" look: a near-black
  page (`#0a0a0a`) with near-white ink (`#ececec`) and grey rules. Terminal
  output keeps muted-but-readable ANSI colours so error/diff cues survive.
- **`paper-light`** — a warm off-white page (`#f4f1ea`) with soft dark ink and
  ink-toned ANSI colours (sage, brick, faded indigo). No pure black or white
  anywhere; every surface reads as the same sheet of paper.
- **`crt-green`** — the classic P1 green-phosphor terminal: bright green on a
  near-black tube, with a monochrome-green ANSI palette (brightness tiers) for
  that single-gun look.
- **`crt-amber`** — the warm P3 amber variation of the green tube.
- **`crt-blue`** — a cool cyan-blue phosphor variation.

A faint procedural **grain** + edge vignette is drawn behind everything (GPU) —
it reads as paper texture on the paper themes and as a subtle **tube glow** on
the CRT ones. Every palette's colours are picked for measured WCAG contrast.

**Switching:** `/theme <name>` (e.g. `/theme crt-green`), or cycle through every
theme live with **`Ctrl+Shift+L`**. The choice persists to `config.toml`.

**Programs keep reading after a switch.** Terminal panes answer color queries
(OSC 10/11) and set `$COLORFGBG` from the active theme, so CLIs that probe the
background pick the right palette at launch. But agent CLIs sample **once at
startup** — after a live theme switch they keep painting colors tuned to the
old background. Crew therefore enforces a **minimum-contrast floor** on
program-painted text (à la iTerm2's Minimum Contrast): any foreground within a
3.0 WCAG ratio of its background is darkened (light page) or lightened (dark
page) in linear light — hue preserved — just enough to read. White-on-white
after switching a running claude/codex pane to `paper-light` stays legible.

**Config keys** (`$XDG_CONFIG/crew/config.toml`, applied on launch — `/restart` picks up external edits):

| Key | Default | Meaning |
|-----|---------|---------|
| `theme` | `"paper-dark"` | `paper-dark`, `paper-light`, `crt-green`, `crt-amber`, or `crt-blue`; unknown ⇒ default |
| `accent` | theme default | `"#rrggbb"` override for the accent (chrome only); omit to use the theme's accent |
| `paper_texture` | `true` | turn the paper grain + vignette pass on/off |
| `paper_grain` | `1.3` | grain strength (`0.0`–`2.0`; `0` = no grain) |
