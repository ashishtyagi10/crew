# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders as tiles (no overlays). Panes auto-tile into a
near-square grid, drawn cell-by-cell on the GPU with `winit` + `wgpu` +
`glyphon`. See [docs/CREW.md](docs/CREW.md) for the full guide.

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

## Run

```sh
cargo run --release -p crew-app
```

## Panes

Panes auto-tile into a near-square grid. Each pane has a title bar showing its
index, the program-set title (often the cwd), and right-aligned status glyphs
(`⇡N` scrollback, `●` new output, `!` bell, `»` broadcast input). The focused
pane has a near-white border and a bright block cursor.

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
`/shell`, `/run <cmd>`, `/edit <file>`, `/settings`, `/find <text>`, `/name <text>`, `/clear`, `/clearall`, `/only`, `/closeall`, `/copy`, `/dump`, `/open`,
`/pwd`, `/font`, `/reload`, `/update`, `/broadcast`, `/zoom`, `/sidebar`, `/keys`, `/about`, `/far`, `/crew`, `/exit`), fish-style autosuggest from history, `cd`
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

Each agent gets a clean message and can **hand off** to a peer by beginning its
reply with `TO <agent>: …`, or **end the thread** with `DONE` (optionally
`DONE: <answer>`). The broker logs every hop as `from → to` with the reply, so
the whole conversation is visible in the pane. A hop counter caps each thread
(default 6) so a relay can never loop forever, and every agent call has a
timeout — a hung agent is killed and logged, never blocking the UI.

Agents run headlessly off the render thread (in a broker subprocess), so the
window stays responsive. **Adding a fourth agent takes one adapter**: add a
constructor in `crates/crew-plugin/src/broker/agents.rs` and register it in
`known_adapters` — the routing engine is untouched. See
[docs/CREW.md](docs/CREW.md) for the protocol and architecture.

## Settings

`/settings` opens a form for font family, font size, and the sidebar. Settings
persist to `$XDG_CONFIG/crew/config.toml` and apply live on Save.

## Architecture

Crew is a Cargo workspace with four crates:

| Crate | Purpose |
|-------|---------|
| `crew-app` | Window, panes, input, in-pane UI |
| `crew-render` | GPU rendering (`wgpu` + `glyphon`) |
| `crew-term` | PTY + terminal grid (`alacritty_terminal` + `portable-pty`) |
| `crew-plugin` | Chat / agent plugins |

Hard rules: every `.rs` file stays ≤200 lines; `cargo clippy --workspace
--all-targets` is warning-free.

## License

MIT or Apache-2.0, at your option.
