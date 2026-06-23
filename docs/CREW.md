# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders in the terminal as tiles (no overlays). Crew is the
successor to this repo's original terminal file-manager project; the crates under
`crates/crew-*` are the product.

## Architecture

- **Rendering** — `winit` + `wgpu` + `glyphon`/`cosmic-text`. Every cell is drawn
  on the GPU; panes have SDF rounded borders.
- **Terminal model** — `alacritty_terminal` + `portable-pty` (`crates/crew-term`).
- **In-pane UI** — `ratatui` widgets are laid out into a `Buffer` and converted to
  GPU cells (the settings form, command palette, and help overlay use this).
- **Crates** — `crew-app` (window, panes, input), `crew-render` (GPU), `crew-term`
  (PTY + grid), `crew-plugin` (chat/agent plugins).

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
| Close pane / maximize window | **Cmd+W** / **Cmd+M** |
| Scroll focused pane | **Shift+PageUp** / **Shift+PageDown**, or mouse wheel |
| Quit | **Cmd+Q** (press twice to confirm when panes are open) |

Click a pane to focus it (click the input bar to focus that); double-click a
pane to toggle zoom.

Inside a terminal pane, all other keys (arrows, Home/End, PageUp/Down, Ctrl+C,
Shift+Tab, …) pass through to the program. Shells launch as your `$SHELL` login
shell, so your full config and plugins load.

## The input bar

The docked command bar supports:

- **Slash commands** — type `/` for a command palette (↑/↓ to pick, Tab/→ to
  fill, Enter to run): `/shell`, `/run <cmd>`, `/edit <file>`, `/settings`, `/find <text>`, `/name <text>`, `/clear`, `/clearall`, `/only`, `/closeall`, `/copy`, `/dump`, `/open`, `/pwd`, `/font`, `/reload`, `/update`,
  `/broadcast`, `/zoom`, `/sidebar`, `/keys`, `/about`, `/far`, `/crew`, `/exit`. The palette is **fuzzy** — prefix matches rank first,
  then subsequence matches (e.g. `/dmp` finds `/dump`) — and **scrolls** to the
  selection when the match list is long.
- **`/broadcast`, `/zoom`, `/sidebar`** — palette-discoverable toggles that mirror
  the `Cmd+S` / `Cmd+Z` / `Cmd+G` chords, for when the chord slips your mind.
- **`/font <n>`** — sets the font size to an exact value (clamped 12–32), unlike
  the `Cmd+=`/`Cmd+-` chords that step by one; no argument reports the current size.
- **`/reload`** — re-reads `config.toml` from disk and applies it live (font,
  sidebar width/visibility) without rewriting the file, so edits made outside the
  `/settings` pane take effect without a restart.
- **`/only`** — closes every pane except the focused one (a quick "focus mode");
  a no-op when only one pane is open.
- **`/closeall`** — closes every pane and returns to the welcome screen.
- **`/clearall`** — clears the scrollback of every terminal pane at once (`/clear`
  applied across all panes).
- **`/edit <file>`** — opens the file in your terminal editor (`$VISUAL`, else
  `$EDITOR`, else `vi`) in a new pane. Path arguments to `/edit`, `/open`, and
  `/dump` expand `~` and `$VAR`/`${VAR}` and resolve relative paths against the
  working directory. (`/open` instead hands the path to the OS default app.)
- **`/run <cmd>`** — launches `cmd` in its own tiled pane (labeled by the
  command) that stays open after it finishes, so builds, tests, and long-running
  jobs run alongside your shells instead of blocking one.
- **`/copy`** — copies the focused terminal pane's **full scrollback** to the
  system clipboard (Cmd+C copies only the visible screen); the line count is
  flashed on the input bar.
- **`/open [target]`** — opens a URL or path with the OS default app. With no
  argument it opens the most recent http(s) URL visible in the focused terminal
  (a quick "clickable link" without reaching for the mouse); a relative path is
  resolved against the working directory. http(s) URLs in terminal panes are
  **tinted blue** to show they're clickable; **Cmd+click** resolves the text
  under the cursor — a URL opens in the browser, an existing **file** opens in
  `$EDITOR`, and a **directory** becomes the new working directory.
- **`/dump [file]`** — exports the focused terminal pane's full scrollback to a
  file (handy for archiving a long build log or an AI agent's output); the saved
  path — with the line count and size — is shown on the input bar. With no argument it writes a timestamped
  `crew-dump-YYYYMMDD-HHMMSS.txt` in the working directory; with an argument it
  writes there (a relative path resolves against the working directory).
- **`/far`** — opens a Far Manager-style **dual-pane file manager** as a pane in
  the grid (like `/shell`): two side-by-side directory listings with a Far
  function-key bar. `Tab` switches the active panel, `↑`/`↓`/`PgUp`/`PgDn`/`Home`/
  `End` move the cursor, `Enter` descends into a folder (or `..`) or opens a file
  with the OS default, `Backspace` climbs to the parent, `Esc`/`F10` closes it.
- **`/crew`** — opens a **multi-agent pane** where the installed CLI coding
  agents (claude, codex, opencode) message each other to work a task. See
  [Multi-agent relay](#multi-agent-relay-crew) below.
- **Autosuggest** — fish-style ghost text from history; Tab/→ accepts it.
- **History** — **Up/Down** recall previous lines; type a prefix first and they
  recall only entries **starting with it** (zsh/fish-style prefix search; an empty
  input recalls everything). Persisted to
  `$XDG_CONFIG/crew/history` across sessions.
- **Path completion** — `cd <partial>` ghost-completes the first matching
  subdirectory, while `/edit <partial>` and `/open <partial>` complete **files
  and** directories; Tab/→ accepts it. `$VAR`/`${VAR}` are expanded (e.g. `cd $HOME/src`).
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
  terminals it uses bracketed paste when the program enabled it.
- Programs can copy to the system clipboard via **OSC 52**.

## Scrollback

Mouse wheel or **Shift+PageUp/PageDown** scroll a pane's history; an amber `⇡`
in the title bar marks that you're viewing scrollback. **`/find <text>`** scrolls
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

**Routing protocol.** Each agent is told who it is, who its peers are, and how
to route:

- begin a reply with `TO <agent>: <message>` to **hand off** to a peer;
- reply `DONE` (or `DONE: <answer>`) to **end the thread** with no further
  reply — the explicit no-reply signal;
- anything else is a plain reply that **bounces back** to whoever sent it.

This proves out as `A→B` (claude hands to codex), `B→A` (codex replies back),
and a **3-way relay** (claude → codex → opencode, and the answer relayed back to
claude).

**Loop guard & timeouts.** Every message carries a hop counter; once it passes
the limit (default 6) the broker drops the thread and logs that it stopped, so a
relay can never loop forever. Each agent call has a timeout (default 180s) — a
hung agent is killed and logged, and the broker moves on.

**Observability.** Every hop is logged in the pane as `from → to` with the
reply, so the whole conversation — including `[done]`, `[stopped]`, and
`[error]` outcomes — is visible.

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

## Sidebar

A docked left panel (toggle with **Cmd+G**) with stacked, line-divided sections:
a live **TIME** clock, **SYSTEM** CPU/MEM/DISK gauges, a **LOAD** section
(1/5/15-minute load average, coloured by load-per-core), a **HOST** section
(hostname, OS, uptime), a **NET** section (down/up byte rates), and — when the
working directory is a repository — a **GIT** section showing the current branch
(with `↑`/`↓` commits ahead/behind the upstream) and a clean / `● N changed` marker, and a **PANES** list of the open panes (index, name,
a `▸` focus marker, and an activity dot) filling the remaining height. Click a
PANES row to focus that pane (double-click to zoom it).

## Settings

`/settings` opens a form (two columns on a wide pane, one when narrow):

- **Font family** — type-to-search over installed monospace families.
- **Font size**, **Nav width**, **Show nav**.

Settings persist to `$XDG_CONFIG/crew/config.toml` and apply live on Save.

## Theme

The canvas is pure black; terminal content shows its natural ANSI colors. The
accent green is reserved for chrome (borders, the CREW wordmark, the command
palette). A configurable theme is future work.
