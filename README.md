# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders as tiles (no overlays). Panes auto-tile into a
near-square grid, drawn cell-by-cell on the GPU with `winit` + `wgpu` +
`glyphon`. See [docs/CREW.md](docs/CREW.md) for the full guide.

It also ships a built-in **swarm orchestration engine** (`crew-hive`): give it a
goal and it decomposes the work into a task graph and runs a pool of agents
toward it — single-goal decomposition or parallel-job batches, bring-your-own-LLM
per agent, with a live task-list view. See
[Swarm orchestration](#swarm-orchestration-crew-hive) and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

Built on **macOS**, **Linux**, and **Windows**.

Rendering is built for legibility: a **whole-pixel cell grid** whose box never
changes with the font you pick (every glyph advance — bold and wide CJK/emoji
included — snaps to whole cells, so panes, borders, and columns never move),
**pixel-exact themes** (colours convert to linear once at the GPU boundary —
the near-black page really is near-black), **width-aware text everywhere**
(emoji/CJK occupy two cells without overlapping), a **verified font picker**
(families are listed by measuring that they render fixed-pitch Latin, so
variable fonts like JetBrains Mono appear and mis-flagged symbol fonts don't —
check with `crew --list-fonts`), and frame-to-frame **shaped-text reuse** so
unchanged panes cost nothing to redraw.

## Install

### Quick install (macOS / Linux)

```sh
curl -sSfL https://raw.githubusercontent.com/ashishtyagi10/crew/main/install.sh | sh
```

Installs the prebuilt `crew` binary to `~/.local/bin`. Set `INSTALL_DIR` to
choose another location.

### With cargo (any platform with Rust)

```sh
cargo install --git https://github.com/ashishtyagi10/crew crew-app
```

### From GitHub Releases (standalone package)

Download the latest archive for your platform from the [Releases page](https://github.com/ashishtyagi10/crew/releases), extract it, and move the `crew` binary to a directory on your `PATH`.

| Platform | Asset |
|----------|-------|
| macOS (Apple Silicon) | `crew-v*-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `crew-v*-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `crew-v*-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `crew-v*-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `crew-v*-x86_64-pc-windows-msvc.zip` |

### Build from source

```sh
git clone https://github.com/ashishtyagi10/crew.git
cd crew
cargo build --release -p crew-app
# Binary is at target/release/crew
```

## Updating

How you update depends on how you installed:

- **Quick install (prebuilt binary):** re-run the install one-liner — it always
  fetches the latest release and overwrites the binary in `~/.local/bin`
  (idempotent, no sudo):
  ```sh
  curl -sSfL https://raw.githubusercontent.com/ashishtyagi10/crew/main/install.sh | sh
  ```
- **cargo:** `cargo install --git https://github.com/ashishtyagi10/crew crew-app --force`
- **Source checkout:** `git pull && cargo build --release -p crew-app`.
- **In-app:** the **`/update`** command downloads the latest release binary for
  your platform over the running one. Progress streams into a dedicated **UPDATE
  card in the left nav** (checking → downloading → installed) — no separate shell
  or checkout — then **`/restart`** relaunches Crew into the new build whenever
  you're ready. A standalone `crew --self-update` CLI path remains as a headless
  fallback.

The prebuilt path only sees a version once its release assets are published.

## Run

```sh
cargo run --release -p crew-app
```

### Detached mode (the default)

`crew` starts **detached** by default: it re-launches itself in a new session
(no controlling terminal) and returns your prompt immediately, so closing the
launching terminal doesn't `SIGHUP` the window. `--detach` / `-d` are still
accepted as no-ops.

To keep crew attached to the terminal instead (e.g. to see logs while
debugging):

```sh
crew --no-detach   # or: crew --foreground
```

## Panes

Panes auto-tile into a near-square grid. Each pane has a title bar showing its
index, the program-set title (often the cwd), and right-aligned status glyphs
(`⇡N` scrollback, `●` new output, `!` bell, `»` broadcast input). The focused
pane has a near-white border and a bright block cursor.

Crew displays up to **6 panes as full tiles**. Additional panes are demoted to a
minimized thumbnail strip along the bottom of the content area, ordered
least-recently-active first. Click a thumbnail, use the sidebar, or press
**Cmd+1 … 9** to focus a pane and restore it to the full grid.

Any full tile can also be **minimized into the left nav**: click the `[-]`
button on its top border and the pane keeps running but leaves the grid; its
sidebar PANES row gains a `[+]` — click the row (or jump to it with
**Cmd+1 … 9**) to restore it. Focusing a hidden pane always restores it.

Background panes can still flag you down: when a pane you're not looking at
rings the **bell** (Claude Code prompting for input), matches a **watched
output pattern**, or finishes a **long command**, its nav row raises an
**attention marker** — `!` / `⚑` / `✓` in the bell colour — that blinks for a
few seconds, then holds steady until you focus the pane. Thumbnails in the
minimized strip carry the same marker, so an agent waiting on you is visible
no matter where its pane went.

## Keyboard shortcuts

Press **`/keys`** in the input bar for the full list in-app.

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
| Copy visible screen / paste | **Cmd+C** / **Cmd+V** (Cmd+V pastes a clipboard image as a temp PNG path) |
| Open URL / file / dir under cursor | **Cmd+Click** |
| Cycle themes (fixed presets, then random) | **Ctrl+Shift+L** |
| Toggle chat markdown preview ↔ raw source | **Ctrl+Shift+M** |
| Insert a newline in a terminal | **Shift+Enter** (sends a line feed, not submit) |
| Close pane / maximize window | **Cmd+W** / **Cmd+M** |
| Clear focused pane scrollback | **Cmd+K** (or `/clear`) |
| Scroll any pane | **Shift+PageUp** / **Shift+PageDown** (Shift+Home/End for top/bottom), or mouse wheel — in a full-screen app (vim/less/agent TUI) the wheel is forwarded to the program |
| Quit | **Cmd+Q** (press twice to confirm when panes are open) |

## Input bar

The docked command bar routes **bare text smartly**: if the focused pane is an
idle shell, what you type is typed into it; otherwise a first word that
resolves on your login shell's `$PATH` spawns the command in its own pane, and
anything else gets a hint instead of a mis-fire. The palette shows a **preview
row** telling you where the line will go before you press Enter. Three
prefixes make the bar explicit: **`!<cmd>`** always runs the command in a new
pane, **`*<text>`** broadcasts one line to every terminal pane, and
**`?<plain english>`** asks the AI for a command (à la Warp AI / Copilot CLI)
— the suggestion lands back in the input bar, ready to edit or Enter, powered
by the same provider stack as `/crew` (DashScope / OpenRouter / Anthropic).
**`??<question>`** goes the other way: the AI reads the focused terminal's
recent output and opens its explanation in the zoomed markdown viewer —
`??why did this fail` after a broken build gets you a formatted post-mortem.

Slash commands complete the bar (type `/` for a fuzzy palette): `/crew`,
`/goal <text>`, `/batch <file>`, `/md <file>`, `/diff`, `/settings`,
`/find <text>`, `/name <text>`, `/clear`, `/clearall`, `/clearlog`, `/only`,
`/copy`, `/dump`, `/closeall`, `/pwd`, `/about`, `/font`, `/theme`, `/notify`,
`/restart`, `/update`, `/broadcast`, `/zoom`, `/sidebar`, `/keys`, `/far`,
`/exit`. Commands with a fixed value set (like `/theme`) expand into an
arrow-selectable **value picker**. Fish-style autosuggest from history, `cd`
completion with `$VAR` expansion, and `Up`/`Down` history recall persisted to
`$XDG_CONFIG/crew/history` round it out. `/diff` opens the working tree's
colored git diff (status, stat, full diff) in its own pane — Codex-style
change review beside your shells; `/md <file>` opens a zoomed **markdown
viewer** with side-by-side source and preview.

## Sidebar

A docked left panel (toggle with **Cmd+G**) with a live clock, CPU/MEM/DISK
gauges, a moving **CPU sparkline** under them, load average, host info, network
rates with a **throughput sparkline**, a git section for the working directory,
and a list of open panes (click a row to focus it). The sparklines scroll on the
sidebar's once-a-second refresh, so the charts animate at no extra redraw cost.

## Markdown

Crew renders markdown natively (a `pulldown-cmark`-based engine drawn straight
to GPU cells — headings, lists, tables aligned by display width, fenced code
cards, links):

- **Chat panes** render agent replies as formatted markdown by default;
  **Ctrl+Shift+M** flips the focused chat pane to the raw source and back.
  **Cmd+Click** opens a rendered link.
- **`/md <file>`** opens a zoomed **markdown viewer** pane showing the file as
  side-by-side `source | preview` halves: **Tab** switches the active half,
  arrows/PageUp/PageDown scroll it, **r** reloads from disk, **Cmd+Click**
  opens links in the preview, **Esc** closes.

## Multi-agent panes (`/crew`)

`/crew` opens a pane that lets independent CLI coding agents — **claude**,
**codex**, and **opencode** — message each other to work a task. On open, the
pane probes which agent CLIs are installed and lists the ones it found (missing
ones are skipped). Type a task and press Enter; prefix `@<agent>` to choose who
starts (otherwise the first detected agent does).

Each agent gets a clean message plus the task and a transcript so far, and ends
its reply with a control line: **`@next <agent>`** to hand off to a peer, or
**`@done`** to end the thread (the parser tolerates markdown wrappers and
re-asks once if the line is missing). The broker logs every hop as `from → to`
with the reply, so the whole conversation is visible in the pane. A hop counter
caps each thread (default 6), an optional token budget caps spend, and every
agent call has a timeout — a hung agent is killed and logged, never blocking the
UI.

The pane speaks a small **construct language**: `/fan <task>` sends one task to
every agent **in parallel** (replies stream back fastest-first), `@a+b <task>`
fans out to a subset, `/loop <n> <task>` iterates on the crew's own answer,
`/goal <text>` keeps working until a judge agent rules the goal met, `/model
<agent> <model>` pins agents to **different models side by side**, and
`/status` reports live totals — with Tab completion for `@agents` and
`/constructs` in the composer, one-letter aliases (`/s` → `/status`), and
did-you-mean on typos. Long constructs run as **concurrent background tasks**
(default cap 4): each reply is tagged with a dim `#N` task chip, `/tasks`
lists what's running, and `/stop [#n]` cancels one task or all of them.
`@file` mentions in the composer fuzzy-complete against the project tree and
splice the file's contents into the outgoing message.

Agents can also touch the workspace through built-in **sys tools** — bounded
`sys:run` (non-interactive shell, 30s/64KB caps), `sys:read_file` (chunked
64KB reads), `sys:write_file`, and `sys:list_dir` — callable mid-relay the
same way as MCP tools. `CREW_SYS_MODE=readonly` blocks the mutating ones,
`CREW_SYS_TOOLS=0` turns the surface off, and `/cwd` shows the working
directory and sandbox mode. An optional token budget
(`CREW_BROKER_TOKEN_BUDGET`) hard-stops a runaway thread.

It also borrows the flagship moves of the big coding agents: **plan mode**
(`/plan <task>` drafts a numbered plan and nothing runs until `/approve`;
`/reject` discards — à la Claude Code), **workspace checkpoints**
(`/checkpoint [label]` snapshots the working tree as a hidden commit under
`refs/crew/` without touching HEAD or your index, `/checkpoints` lists,
`/restore <n>` brings a snapshot's files back — à la Cline),
**transcript export** (`/export` writes the conversation to
`crew-transcript-<stamp>.md` — à la OpenCode), **AI commit messages**
(`/commit` has the coder draft a Conventional Commits message for your diff;
`/commit apply` creates the commit — à la Aider), **AI code review**
(`/review` reports findings on the working diff worst-first — à la Codex),
and **`/compact`**, which
folds older messages away when a long session gets heavy; `/diff` (in the
pane or the input bar) completes the loop with Codex-style change review.

The pane is extensible the way other coding tools are — three drop-in
surfaces, no rebuild, edits picked up live (`/reload` forces it; no restart
needed) — see [docs/CREW.md](docs/CREW.md#multi-agent-relay-crew):

- **Memory** — Claude Code-style `#` shortcut: `#always use pnpm` in the pane
  appends to `./.crew/memory.md`, and every task from then on carries the
  merged memory (user + project files, 2 KB cap) as a standing block the
  agents follow; `/memory` shows what's loaded.
- **Skills** — markdown prompt playbooks in `~/.config/crew/skills/` or
  `./.crew/skills/` (optional `name:`/`description:` frontmatter; project
  overrides user). A skill can also be a **directory with a `SKILL.md`** plus
  supporting files, and oversized playbooks disclose **progressively**: past
  8 KB the relay gets the description + heading outline + path, and agents
  read sections on demand with chunked `sys:read_file` calls. `/skills` lists
  them; `/skill <name> <task>` runs the relay with the playbook prepended so
  the whole crew follows it.
- **Plugin agents** — a JSON manifest in `~/.config/crew/agents/` or
  `./.crew/agents/` (`{"name", "command", "args": […, "{}"], "role"}`) turns
  any headless CLI into a roster agent; installed manifests appear in
  `/agents` and make `/crew` usable with **no API key at all**.
- **MCP** — servers declared in `~/.config/crew/mcp.json` or `./.crew/mcp.json`
  (the standard `mcpServers` schema) connect lazily over stdio; `/mcp` lists
  their tools, relay prompts advertise them, and agents call one by ending a
  reply with `` `@tool server:tool {"arg": …}` `` — the result is fed back
  (bounded rounds, visible in the transcript) before routing resumes.

The pane itself reads like a multi-agent console: a header with a live status
(`| coder · 12s` while an agent thinks, `| 3 working · 8s` during a parallel
fan, a completed-turns counter, a running `~N tok` meter, connection dot),
**statusline-style agent rows** — one per agent with its model badge, reply
count, running token total, and live bars for **context-window fill** (sized
to the pinned model's window) and its share of the turn's time, the active
agent highlighted —
a **live activity row** while agents work (`⠹ user ⇢ planner 4s`, one animated
chip per working agent naming who handed it the task, so parallel fans and
hand-offs are visible as they happen), and **message cards** (`▍sender · 2m ago · 4.2s`)
that colour each agent consistently and show hand-offs as `from → to`. Every
turn ends with a timeline log line: `turn done — planner 4.2s → coder 8.1s ·
2 exchange(s) · ~950 tok (approx)`. New cards fade in from the page colour;
fenced ```code``` in replies renders as a
bordered card with a language tag on a dimmed background; a composer with
`@agent` chips and key hints frames the input (a valid `@mention` lights up in
the agent's colour); a proportional scrollbar plus a `↓ N new` pill keep long
transcripts navigable; and a fresh pane opens with onboarding — the detected
crew, roles, and an example prompt.

Agents run headlessly off the render thread (in a broker subprocess), so the
window stays responsive. **Adding a fourth agent takes one adapter**: add a
constructor in `crates/crew-plugin/src/broker/agents.rs` and register it in
`known_adapters` — the routing engine is untouched. See
[docs/CREW.md](docs/CREW.md) for the protocol and architecture.

## Swarm orchestration (`crew-hive`)

Beyond the `/crew` relay (a few CLI agents talking turn-by-turn), Crew includes a
full orchestration **engine**, the `crew-hive` crate — the substrate for running
*many* agents toward one goal:

- **Planner** — decomposes a goal into a task-graph (a dependency DAG). Ships a
  deterministic `StubPlanner` and an `LlmPlanner` that asks an LLM for the graph.
- **Scheduler** — a `tokio` DAG executor with a bounded worker pool (concurrency
  cap), dependency fan-in/fan-out, failure cascade-cancel, panic-as-failure
  resilience, and cooperative cancellation.
- **Agents** — a uniform `Agent` trait with three workers: `StubAgent` (tests),
  `ApiAgent` (a native LLM call — just a future, no PTY, so thousands can run),
  and `RemoteAgent` (dispatched over a wire to an out-of-process worker or an
  external engine such as LangGraph).
- **Blackboard** — agents read their dependencies' results and write their own,
  merging work upward (replacing fragile file/sentinel passing).
- **Bring-your-own-LLM** — a `Provider` abstraction (mock + an Anthropic client),
  with per-agent `ModelTier` cost tiering (haiku / sonnet / opus).
- **Two modes, one engine** — single-goal decomposition *and* flat parallel-job
  batches (`batch_graph`); a `budget_governor` enforces a hard cost ceiling.
- **Swarm view** — a live task list over fleet telemetry: one row per task with
  a state glyph (○ pending · ● running · ✓ done · ✗ failed), its title, and the
  agent's last output line while it works.

The engine is wired into the app through two commands, each opening a live
**swarm pane** that renders the task list + a fleet HUD (live / done / failed
/ cost) and updates every frame:

- **`/goal <text>`** — plans the goal into a task-graph off the UI thread, then
  runs it. With `ANTHROPIC_API_KEY` set it uses the real `LlmPlanner` + native
  `ApiAgent` workers (each task billed at the planner's per-task `ModelTier`);
  without a key it falls back to the deterministic stub backend so the full
  flow still works offline.
- **`/batch <file>`** — runs a file of jobs (one per line) as a flat, all-parallel
  swarm — the "many parallel jobs" mode.

Real-LLM `/goal` and `/batch` runs are capped by the `budget_governor` (default
$1.00); the pane shows a "budget exceeded — swarm cancelled" notice if the cap
trips. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
[docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md](docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md).

## Settings

`/settings` opens a **two-column bento form** covering every configurable
property: font family/size, nav width + visibility, theme, accent, paper
texture + grain, launch-maximized, and the whole notification block (master +
per-event toggles, min-secs threshold, watched output patterns as a
one-per-line text area). **Cmd+S / Alt+S** saves. Settings persist to
`$XDG_CONFIG/crew/config.toml` and apply live on Save. The config file also
accepts `accent = "#rrggbb"` to override Crew's accent; omit it (or give an
invalid value) to use the active theme's default accent. It applies at launch —
`/restart` picks up edits made outside the `/settings` pane.

**Themes.** Crew ships **nine themes**: five paper/ink looks — `paper-dark`
(default — a high-contrast "newspaper" look), `paper-light` (a warm paper
page), `sepia-dark` (warm cream ink on dark sepia), `midnight-ink` (cool
off-white on deep navy), and `graphite` (a gentle soft-charcoal page) — and
four **CRT phosphor** tubes: `crt-green`, `crt-amber`, `crt-blue`, and
`crt-violet`, each a neon monochrome glow on a near-black tube. A tenth
option, **`/theme random`**, rotates through the dark themes every 10
minutes. Switch with `/theme <name>` (the palette offers an arrow-selectable
picker) or cycle everything live with `Ctrl+Shift+L`; the choice persists.
Light themes render ink at Medium weight over 3× "newsprint" grain so they
read like paper, not a washed-out screen. A subtle GPU grain + vignette sits
behind everything (it reads as a CRT glow on the phosphor themes). Config
keys: `theme = "paper-dark"`, `paper_texture = true` (grain on/off),
`paper_grain = 1.3` (strength `0.0`–`2.0`). See
[docs/CREW.md](docs/CREW.md#themes).

## Architecture

Crew is a Cargo workspace with six crates:

| Crate | Purpose |
|-------|---------|
| `crew-app` | Window, panes, input, in-pane UI |
| `crew-render` | GPU rendering (`wgpu` + `glyphon`) |
| `crew-term` | PTY + terminal grid (`alacritty_terminal` + `portable-pty`) |
| `crew-plugin` | Chat / agent plugins (the `/crew` relay broker) |
| `crew-theme` | Theme presets + palette contracts (9 themes, contrast thresholds) |
| `crew-hive` | Swarm orchestration engine (planner, scheduler, agents, blackboard, telemetry) |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full diagram (app +
engine internals).

Hard rules: every `.rs` file stays ≤200 lines; `cargo clippy --workspace
--all-targets` is warning-free.

## License

MIT or Apache-2.0, at your option.
