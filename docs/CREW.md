# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders in the terminal as tiles (no overlays). This repo is
pivoting from the original *Farx* file manager to Crew; the crates under
`crates/crew-*` are the new product.

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
| `⇡`   | viewing scrollback (not at the live bottom) |
| `●`   | new output in an unfocused pane |
| `!`   | the program rang the bell |

The focused pane has a near-white border and a bright block cursor; unfocused
panes are grey with a dim cursor.

## Keyboard shortcuts

Press **`/keys`** in the input bar for this list in-app.

| Action | Keys |
|--------|------|
| Next / previous pane | **Ctrl+Tab** / **Ctrl+Shift+Tab** (also Cmd+] / Cmd+[) |
| Jump to pane N | **Cmd+1 … 9** |
| Move pane left / right | **Cmd+{** / **Cmd+}** |
| Focus the input bar | **Cmd+I** |
| New shell pane | **Cmd+T** |
| Settings / chat pane | **Cmd+,** / **Cmd+J** |
| Toggle sidebar | **Cmd+G** |
| Zoom focused pane | **Cmd+Z** |
| Font bigger / smaller / reset | **Cmd+=** / **Cmd+-** / **Cmd+0** |
| Paste | **Cmd+V** |
| Close pane / maximize window | **Cmd+W** / **Cmd+M** |
| Scroll focused pane | **Shift+PageUp** / **Shift+PageDown**, or mouse wheel |
| Quit | **Cmd+Q** |

Inside a terminal pane, all other keys (arrows, Home/End, PageUp/Down, Ctrl+C,
Shift+Tab, …) pass through to the program. Shells launch as your `$SHELL` login
shell, so your full config and plugins load.

## The input bar

The docked command bar supports:

- **Slash commands** — type `/` for a command palette (↑/↓ to pick, Tab/→ to
  fill, Enter to run): `/shell`, `/settings`, `/update`, `/keys`, `/exit`.
- **Autosuggest** — fish-style ghost text from history; Tab/→ accepts it.
- **History** — **Up/Down** recall previous lines (persisted to
  `$XDG_CONFIG/crew/history` across sessions).
- **Editing** — **Ctrl+W** delete the last word, **Ctrl+U** clear the line.
- Anything that isn't a slash command is sent to the focused terminal.

## Clipboard

- **Cmd+V** pastes into the focused surface (terminal, input bar, or chat). For
  terminals it uses bracketed paste when the program enabled it.
- Programs can copy to the system clipboard via **OSC 52**.

## Sidebar

A docked left panel (toggle with **Cmd+G**) with stacked cards: a live **TIME**
clock, **SYSTEM** CPU/MEM/DISK gauges, and a **HOST** card (hostname, OS,
uptime).

## Settings

`/settings` opens a form (two columns on a wide pane, one when narrow):

- **Font family** — type-to-search over installed monospace families.
- **Font size**, **Nav width**, **Show nav**.

Settings persist to `$XDG_CONFIG/crew/config.toml` and apply live on Save.

## Theme

The canvas is pure black; terminal content shows its natural ANSI colors. The
accent green is reserved for chrome (borders, the CREW wordmark, the command
palette). A configurable theme is future work.
