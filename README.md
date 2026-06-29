# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders as tiles (no overlays). Panes auto-tile into a
near-square grid, drawn cell-by-cell on the GPU with `winit` + `wgpu` +
`glyphon`. See [docs/CREW.md](docs/CREW.md) for the full guide.

It also ships a built-in **swarm orchestration engine** (`crew-hive`): give it a
goal and it decomposes the work into a task graph and runs a pool of agents
toward it — single-goal decomposition or parallel-job batches, bring-your-own-LLM
per agent, with a live constellation/heatmap view. See
[Swarm orchestration](#swarm-orchestration-crew-hive) and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

Built on **macOS**, **Linux**, and **Windows**.

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
- **Source checkout:** `git pull && cargo build --release -p crew-app`. The in-app
  **`/update`** command runs the `git pull` for a source checkout; rebuild and
  restart afterward.

Restart Crew after updating. The prebuilt path only sees a version once its
release assets are published.

## Run

```sh
cargo run --release -p crew-app
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
| Copy visible screen / paste | **Cmd+C** / **Cmd+V** |
| Close pane / maximize window | **Cmd+W** / **Cmd+M** |
| Scroll focused pane | **Shift+PageUp** / **Shift+PageDown**, or mouse wheel |
| Quit | **Cmd+Q** (press twice to confirm when panes are open) |

## Input bar

The docked command bar supports slash commands (type `/` for a palette:
`/shell`, `/crew`, `/claude`, `/codex`, `/opencode`, `/run <cmd>`, `/edit <file>`, `/settings`, `/find <text>`, `/name <text>`, `/clear`, `/only`, `/copy`, `/dump`, `/open`,
`/font`, `/reload`, `/update`, `/broadcast`, `/zoom`, `/sidebar`, `/keys`, `/far`, `/exit`), fish-style autosuggest from history, `cd`
completion with `$VAR` expansion, and `Up`/`Down` history recall persisted to
`$XDG_CONFIG/crew/history`. Anything that isn't a slash command or `cd` is sent
to the focused terminal.

## Sidebar

A docked left panel (toggle with **Cmd+G**) with a live clock, CPU/MEM/DISK
gauges, load average, host info, network rates, a git section for the working
directory, and a list of open panes (click a row to focus it).

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
UI. The pane prints a cost summary (`~N tokens`) when the task ends.

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
- **Swarm view** — a constellation/heatmap layout over live fleet telemetry
  (color = state, mode auto-switches to a heatmap past ~150 agents).

The engine is complete and tested headlessly (decompose → schedule → run a
fan-out of agents → merge). The in-terminal swarm **pane** (a `/swarm <goal>`
command rendering the constellation live, with drill-down) is wired and headless-
tested on the `feat/crew-app-swarm` branch; the on-screen GPU rendering is the
remaining step. The live LLM path needs `ANTHROPIC_API_KEY`. See
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
[docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md](docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md).

## Settings

`/settings` opens a form for font family, font size, and the sidebar. Settings
persist to `$XDG_CONFIG/crew/config.toml` and apply live on Save.

## Architecture

Crew is a Cargo workspace with five crates:

| Crate | Purpose |
|-------|---------|
| `crew-app` | Window, panes, input, in-pane UI |
| `crew-render` | GPU rendering (`wgpu` + `glyphon`) |
| `crew-term` | PTY + terminal grid (`alacritty_terminal` + `portable-pty`) |
| `crew-plugin` | Chat / agent plugins (the `/crew` relay broker) |
| `crew-hive` | Swarm orchestration engine (planner, scheduler, agents, blackboard, telemetry) |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full diagram (app +
engine internals).

Hard rules: every `.rs` file stays ≤200 lines; `cargo clippy --workspace
--all-targets` is warning-free.

## License

MIT or Apache-2.0, at your option.
