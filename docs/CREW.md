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
  fill, Enter to run): `/shell`, `/settings`, `/find <text>`, `/name <text>`, `/clear`, `/pwd`, `/update`,
  `/keys`, `/exit`.
- **Autosuggest** — fish-style ghost text from history; Tab/→ accepts it.
- **History** — **Up/Down** recall previous lines (persisted to
  `$XDG_CONFIG/crew/history` across sessions).
- **`cd` completion** — typing `cd <partial>` ghost-completes the first matching
  subdirectory; Tab/→ accepts it. `$VAR`/`${VAR}` are expanded (e.g. `cd $HOME/src`).
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
unless the term has an uppercase letter); a miss reports on the status line.
Typing returns to the bottom.

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
