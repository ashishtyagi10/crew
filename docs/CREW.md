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
  `crew-theme` (the thirteen theme presets + palette contracts — see
  [Themes](#themes)), `crew-hive` (the swarm orchestration engine — see
  [Swarm orchestration](#swarm-orchestration-crew-hive) below).
- **Diagram** — see [ARCHITECTURE.md](ARCHITECTURE.md) for the full app + engine
  diagram.

Hard rules: every `.rs` file stays ≤200 lines; `cargo clippy --workspace
--all-targets` is warning-free.

## Build & run

```sh
cargo run --release -p crew-app
```

## CLI modes

Crew runs as a GUI by default; these command-line modes offer headless operation or setup:

- `crew --list-fonts` — print the list of installed monospace fonts that Crew can use
- `crew --self-update` — fetch and install the latest release binary (headless alternative to `/update`)
- `crew install-app` — create or refresh the OS app menu entry (macOS ~/Applications, Spotlight, Windows Start menu, Linux applications menu)
- `crew install-app --remove` — remove the OS app menu entry

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

**Minimize to nav.** Every full tile carries a **`[-]` button** on its top
border. Clicking it hides the pane into the left nav: the pane keeps running
(its process is untouched) but leaves the grid, focus moves to the nearest
visible pane, and its sidebar PANES row gains a right-aligned **`[+]`**.
Click the row — or focus the pane any other way (Cmd+1 … 9) — and it
restores to the grid; focusing a hidden pane always un-hides it. Hidden panes
are skipped by pane cycling and never receive bare input-bar text.

**Attention markers.** A pane you're not looking at — hidden in the nav,
demoted to the thumbnail strip, or just unfocused — flags for you when it
needs input or finished work: the terminal **bell** (Claude Code rings it on
permission prompts when its bell is enabled) raises `!`, a **watched output
pattern** (`notify_patterns` — add prompts like `"Do you want"` to catch
agents that don't ring) raises `⚑`, and a foreground **command finishing**
after `notify_min_secs` raises `✓`. The marker takes over the row's
activity-dot slot in the bell colour and tints the title, blinks for ~4
seconds — redraws are only driven while it blinks, so an ignored marker costs
nothing — then holds steady until the pane is focused, which clears it (the
same rule the activity dot follows). Thumbnail cards in the minimized strip
show the same marker.

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
| Open URL / file / dir under cursor | **Cmd+Click** |
| Cycle themes (fixed presets, then random-dark/light, then auto) | **Ctrl+Shift+L** |
| Toggle chat markdown preview ↔ raw source | **Ctrl+Shift+M** |
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

- **Smart bare-input routing** — plain text (not a slash command, `cd`, or a
  prefix below) routes by context: if the focused pane is a **visible, idle
  shell** (its prompt is waiting), the text is typed into it — the shell is
  the judge of what it means. Otherwise, if the first word resolves to an
  executable on your **login shell's `$PATH`** (hydrated in the background via
  `$SHELL -lc`, so Dock launches see the same commands your terminal does;
  `CREW_SHELL_ENV=0` skips it), the command **spawns in its own pane**.
  A shell builtin (`export`, `source`, …) or an unresolvable word gets a
  status **hint** instead of a mis-fire. While you type, the palette shows a
  one-row **preview** of exactly where Enter will send the line ("↵ type into
  pane 2 · zsh", "↵ run — new pane", …); it stays silent for `/`-led text
  and `cd`.
- **`!<command>`** — always runs the command in its own new pane, regardless
  of focus (the explicit form of the old `/run`).
- **`*<text>`** — broadcasts one line to **every terminal pane** — a one-shot
  alternative to the persistent Cmd+S broadcast mode.
- **`?<plain english>`** — ask the AI for a command (à la Warp AI / GitHub
  Copilot CLI): `?kill whatever is on port 8080` sends the request to the same
  provider stack `/crew`'s inbuilt agents use (DashScope → OpenRouter →
  Anthropic, mock under `CREW_BROKER_MOCK_REPLY`) on a worker thread, and the
  suggested command lands **back in the input bar** — ready to edit or Enter —
  with a status flash. If you've typed something new meanwhile it never
  clobbers you (the suggestion flashes on the status line instead). Fenced or
  backticked replies are distilled to the bare command; no provider key ⇒ a
  status hint, never a hang (30s deadline).
- **`??<question>`** — ask the AI **about the focused pane**: the newest ~120
  lines (8 KB cap) of the focused terminal's scrollback go to the provider
  with your question (bare `??` asks it to explain what happened, focusing on
  errors), and the markdown answer opens in the **zoomed `/md` viewer** —
  headings, code fences and all. Warp's "ask AI about this error", as a
  two-keystroke prefix. Non-terminal focus or an empty pane gets a status
  hint; the same one-ask-at-a-time and worker-thread rules as `?` apply.
- **Slash commands** — type `/` for a command palette (↑/↓ to pick, Tab/→ to
  fill, Enter to run): `/crew`, `/goal <text>`, `/batch <file>`, `/md <file>`,
  `/diff`, `/settings`, `/find <text>`, `/name <text>`, `/clear`, `/clearall`,
  `/clearlog`, `/only`, `/closeall`, `/pwd`, `/about`, `/copy`, `/dump`,
  `/font`, `/restart`, `/theme`, `/notify`, `/update`, `/broadcast`, `/zoom`,
  `/sidebar`, `/keys`, `/far`, `/exit`. The palette is **fuzzy** — prefix
  matches rank first,
  then subsequence matches (e.g. `/dmp` finds `/dump`) — and **scrolls** to the
  selection when the match list is long. When several commands share a prefix,
  the **shortest** is ghosted as the autosuggestion (e.g. `/clear` ghosts before
  `/clearlog`, which is one keystroke further). Commands with a **fixed set of
  values** (like `/theme`) expand into a **value picker**: select the command
  (or type its trailing space) and the palette lists the choices to arrow through
  and `Enter` — no need to remember or type the exact value. (`/shell` and
  `/run <cmd>` still dispatch if typed, but bare text and `!` replaced their
  palette rows.)
- **`/broadcast`, `/zoom`, `/sidebar`** — palette-discoverable toggles that mirror
  the `Cmd+S` / `Cmd+Z` / `Cmd+G` chords, for when the chord slips your mind.
- **`/font <n>`** — sets the font size to an exact value (clamped 12–32), unlike
  the `Cmd+=`/`Cmd+-` chords that step by one; no argument reports the current size
  (and rotation state, if on). **`/font random`** toggles a 10-minute rotation
  through the installed monospace families (same clock as `/theme random`) —
  run it again to stop and return to the pinned family. Rotation only ever
  touches the live renderer, never the pinned `font_family` in Settings, and
  a manual family pick there also turns rotation back off.
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
- **`/theme [name]`** — switches the theme live and persists it (thirteen
  themes — `paper-dark`, `paper-light`, `sepia-dark`, `sepia-light`,
  `midnight-ink`, `graphite`, `coldpress-gray`, `salmon-broadsheet`,
  `ivory-ledger`, `crt-green`, `crt-amber`, `crt-blue`, `crt-violet` — plus
  the rotation modes `random-dark`/`random` (alias), `random-light`, and
  `auto` (follows the OS appearance)); no argument reports the current
  selection. Selecting `/theme` in the palette opens an arrow-selectable
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
- **Run panes** (`!<cmd>`, bare-text spawns, `/run <cmd>`) — the command runs
  in its own tiled pane (labeled by its first word) that stays open after it
  finishes — the pane drops to a fresh shell prompt — so builds, tests, and
  long-running jobs run alongside your shells instead of blocking one. This is
  also how you open a coding-agent CLI in a pane — `!claude`, `!codex`,
  `!opencode` (distinct from `/crew`, which opens the multi-agent broker relay
  pane). Run panes execute under **bash job control** (`set -m`, then `exec`
  back into your shell), so Crew can tell "a command is running" from "a
  prompt is waiting" — that signal is what makes bare input divert away from
  a busy pane instead of typing into a running program.
- **`/md <file>`** — opens a zoomed **markdown viewer** pane: side-by-side
  `source | preview` halves of the file. `Tab` switches the active half,
  ↑/↓ and PageUp/PageDown scroll it, `r` reloads from disk, **Cmd+click**
  opens a link in the preview, `Esc` closes. Chat panes render markdown too —
  see [Markdown](#markdown).
- **`/notify [on|off|add <text>|clear]`** — drive the notification block from
  the bar: toggle the master switch, add a watched output pattern, or clear
  the patterns (the full set of knobs lives in `/settings`).
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
  panel **only while the command line is empty**; `↑`/`↓`/`PgUp`/`PgDn`/`Home`/`End`
  move the cursor, `Enter` descends into a folder (or `..`) or opens a file with
  the OS default, `Backspace` climbs to the parent, `F5`/`F6` copy/move to the
  other panel, `F7` makes a folder, `F8` trashes, `F10` closes. Type on the
  **command line** and press `Enter` to run a command against the **active
  panel** — `cd <path>` navigates that panel in place, anything else runs in
  its directory on a worker thread (a `⟳` note shows while it runs, the
  listings reload when it finishes, and the result flashes in the status bar
  — no new pane is spawned). While typing: `Tab` completes the caret token
  (command name or path), cycling through candidates on repeat presses;
  `↑`/`↓` recall previous commands instead of moving the cursor; fish-style
  ghost text previews a matching history entry, and `→`/`End` accept it.
  `Esc` cancels an active Tab-cycle first (restoring the pre-cycle text), then
  clears the typed command, then closes the pane. Run commands persist to
  `far-history` (a sibling of the input bar's `history` file) across sessions.
  Prefix the command line with **`!`** and a description (e.g. `! list rust
  files`) to ask AI for the shell command — the bar shows `thinking… Ns`
  while a provider call runs (20s timeout), then the landed suggestion
  replaces the bar, highlighted, with a `Enter run · Esc discard · keep
  typing to edit` hint: `Enter` runs it like any typed command, `Esc`
  restores the original `!` text, and typing further just edits the
  suggestion as plain text.
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
- **`/restore`** reopens the last session's shells: every quit path (Cmd+Q,
  window close, `/exit`) snapshots each open terminal pane's **live** shell
  directory (asked from the OS, so it follows your `cd`s; hidden panes
  included) to `session.toml` beside the config — up to 6, deduped, stale
  paths skipped on load. Restore is deliberately pull-based: launching keeps
  the welcome screen, and the shells come back only when you ask. A pane-less
  quit clears the snapshot so `/restore` never resurrects an ancient layout.
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

## Markdown

Crew renders markdown natively: a `pulldown-cmark`-based engine (`md/`) folds
the event stream into styled blocks and lays them out straight onto GPU cells —
headings, lists, block quotes, tables (columns aligned by display width, so
CJK/emoji don't skew them), fenced code as bordered cards, and links. Nesting
depth is capped so pathological input can't blow the stack, and HTML blocks
render verbatim instead of disappearing.

- **Chat panes** (the `/crew` pane, Cmd+J chat) render message bodies as
  formatted markdown by default; single line breaks are preserved, since
  agent replies rely on them. **`Ctrl+Shift+M`** flips the focused chat pane
  to the raw source and back. **Cmd/Ctrl+click** on a rendered link opens it
  (hit-testing maps display columns through character widths, so links after
  emoji still click correctly).
- **`/md <file>`** opens a zoomed **markdown viewer** pane over one file,
  split into side-by-side `source | preview` halves with independent scroll
  (wrapped lines are precomputed once per width, so scrolling is free).
  **Tab** switches the active half, **↑/↓** scroll by line and
  **PageUp/PageDown** by ten, **r** reloads the file from disk, **Cmd+click**
  opens a link from the preview half, the mouse wheel scrolls whichever half
  the cursor is over, and **Esc** closes the pane. Relative paths resolve
  against the input bar's working directory.

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
spinner naming it with live elapsed seconds); below it the **agent roster**
streamed by the broker as a structured `roster` event renders as
**statusline-style rows** — one per agent (`name │ state │ tok │ ctx │ shr`)
with its model badge, a live spinner or reply count, the running token
total (climbing live mid-reply from rate-limited `stats_tick` estimates
while a provider streams), a **context-window meter** (per-agent prompt
fill as a bar + %, sized to the pinned model's window — fed by real usage
in the broker's `stats` events), and a bar for its share of the turn's wall
time; the row sheds its
rightmost segments as the pane narrows. While agents work, the next row
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

**Swarm runs stream inline.** When a plain `/crew` message runs as a broker-side
swarm, the pane opens a **live task-list block** above the composer — one row
per planned task with a state glyph (a spinner while it runs), its title, live
elapsed seconds, a right-aligned token count, and — when the run bills real
API spend — a per-task **cost column** (`$0.0031`; sub-cent costs keep four
decimals). As the pane narrows the metric columns shed in reverse order of
urgency: cost first, then tokens, then elapsed. When every task reaches a
terminal state the block **folds into the transcript** as the run's durable
record: a task list with per-task token totals, costs, and durations, capped
by a `Σ 13.0k tok · $0.04` run-total line (the only place the whole run's
spend surfaces in chat), plus — for runs
where two or more tasks actually started — a **timeline**: a Gantt-style
fenced block mapping each task's start→end span onto a 20-cell bar
(`timeline · 12.4s` header, `█` active / `·` idle), so the run's concurrency
shape and critical path stay readable long after the run ends. Tasks cancelled
before starting are listed (`⊘`) but omitted from the timeline; a task still
running when a broker error folds the block closes its bar at the fold moment.

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
itself (Tab completes both `@agents` and `/constructs`; one-letter **aliases**
`/h /a /s /t /d /m /r` expand to help/agents/status/tasks/diff/model/reload,
and a typo gets a **did-you-mean** suggestion):

- **`/help`** — list the constructs; **`/agents`** — the roster with each
  agent's role and model; **`/status`** — the live task count, session
  turn/token totals, the model pins, the sys-tool sandbox mode, and the token
  budget.
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
- **`#<note>`** / **`/memory`** — standing **project memory** (à la Claude
  Code's `#` shortcut): `#always run tests with --workspace` appends the note
  to `./.crew/memory.md`, and from then on **every task** — plain sends,
  `/fan`, `/loop`, `/goal`, `/skill`, `/approve` — carries the merged memory
  (user `~/.config/crew/memory.md` first, project second, 2 KB cap) as a
  STANDING MEMORY block the agents are told to follow. `/memory` shows what's
  loaded. Unlike skills, memory is always on; edit or delete the file to
  forget.
- **`/mcp`** — list the configured MCP servers and their tools (see
  *Extending* below).
- **`/reload`** — pick up extension edits without a restart: re-reads skills
  and plugin manifests, forces MCP to re-read `mcp.json` and reconnect on
  next use, and re-emits the roster so the pane's badges update.
- **`/diff`** — the working tree's `git diff --stat` inline in the
  transcript; **`/cwd`** — the broker's working directory and sys-tool
  sandbox mode.
- **`/commit`** — an **AI-written commit message** (à la Aider): the coder
  agent reads the diff (staged wins; otherwise unstaged tracked changes,
  12 KB cap) and drafts a Conventional Commits message — subject ≤72 chars,
  body only when the change warrants it. Nothing is committed until you run
  **`/commit apply`**, which creates the commit (`-m` for a staged proposal,
  `-am` for an unstaged one); re-running `/commit` re-drafts. A clean tree,
  a missing repo, or an empty draft each get a status line instead.
- **`/review`** — an **AI code review** of the same diff `/commit` sees (à la
  Codex's `/review`): the reviewer agent reports findings worst-first —
  `blocker — file:line — what and why`, then `warn`, then `nit` — closing
  with a one-line verdict (or "no findings" for a clean diff). Read-only:
  nothing to apply, pairs naturally with `/commit` before you ship.
- **`/standup [days]`** — an **AI standup update** from the repo's recent
  commits (default: the last day, up to 30): the coder groups what shipped
  by theme, infers what's still in progress, and calls out risks — first
  person, paste-ready for the morning thread. History summarization — the
  complement of `/review` (the diff you haven't committed) and `/commit`
  (the message for it). An empty window or a fresh repo reports "nothing to
  report" instead of erroring.
- **`/doctor`** — a **health check for the AI stack** (à la Claude Code's
  `/doctor`): one ✓/✗/– checklist covering the provider that will answer
  (and which key it found), the claude/codex/opencode CLIs on `$PATH`,
  `/bin/bash` (run panes' job control), git, and how many skills, plugin
  agents, and MCP servers loaded, plus standing memory, a resumable session,
  and the sys-tool mode — each ✗ line names its fix.
- **`/resume`** — **continue the previous session** (à la Claude Code's
  `--continue`): the broker auto-saves the conversation — your tasks and
  every agent reply — to `./.crew/session-live.md` as it streams (32 KB cap,
  oldest half dropped; the `crew` system voice is skipped), and on the next
  broker start it rotates to `./.crew/last-session.md`. `/resume` in a fresh
  pane folds that file's tail (2 KB) into your **next task** as a
  PREVIOUS SESSION context block — consumed once — so the crew picks up
  where the last pane left off, even after a crash.
- **`/export`** — write the pane's transcript to
  `crew-transcript-<stamp>.md` in the working directory (à la OpenCode),
  one `## sender · time · latency` section per message. **`/compact`** folds
  older messages away when a long session gets heavy. Both — like `/theme`
  and `/exit` — are answered by the pane itself, so they work even while the
  broker is busy.
- **`/tasks`** / **`/stop [#n]`** — long constructs run as **concurrent
  background tasks** (default cap 4, `CREW_MAX_TASKS`): submitting a second
  task doesn't wait for the first, every streamed reply is tagged with a dim
  `#N` chip naming its task, `/tasks` lists what's running (`#id · label ·
  age`), and `/stop #n` cancels one task — bare `/stop` cancels them all —
  at its next checkpoint (between hops/rounds). Quick constructs and
  `/status` answer immediately while tasks are in flight.

**Built-in sys tools.** Agents can touch the workspace without any MCP server:
four bounded tools ride the same `@tool` surface — **`sys:run`** (one
non-interactive shell command via `/bin/sh -c`, 30s deadline, 64 KB per pipe,
its whole process group reaped on timeout so backgrounded children can't
linger), **`sys:read_file`** (UTF-8, 64 KB per call; a truncation note carries
the byte `offset` to continue with, so agents read big files in chunks),
**`sys:write_file`** (create/overwrite), and **`sys:list_dir`** (≤500 entries,
sizes shown). `CREW_SYS_MODE=readonly` blocks the mutating pair (`run`,
`write_file`), `CREW_SYS_TOOLS=0` turns the surface off entirely, and `/cwd`
or `/status` show the active mode. An approximate per-thread **token budget**
(`CREW_BROKER_TOKEN_BUDGET`, default unlimited) terminates a thread that blows
past it.

**`@file` mentions.** In the composer, a trailing `@<query>` pops a fuzzy file
picker over the project tree (filename-prefix first, then path matches; ↑/↓
navigate, Tab/Enter accept, Esc closes just the popup). On send, each
mentioned file's contents are spliced into the outgoing message as a
`--- file: … ---` block (64 KB cap; binary or missing files are skipped), so
you can hand agents exact context without pasting. The leading `@agent`
selector is left alone, and typed mentions render as tinted chips.

**Extending (skills · plugin agents · MCP).** Three drop-in surfaces, no
rebuild required — the same trio other coding tools ship. All three
hot-reload: skills and manifests are re-read from disk on every use, and
`mcp.json` edits are picked up on the next tool use (or immediately with
`/reload`) — no restart needed:

- **Skills** are markdown playbooks in `~/.config/crew/skills/` (user) or
  `./.crew/skills/` (project; wins on a name clash) — either flat `.md`
  files or **directories with a `SKILL.md`** plus supporting files. Optional
  `---` frontmatter sets `name:` and `description:`; otherwise the file stem
  and first line are used. Skills disclose **progressively**: bodies up to
  8 KB are inlined whole, while an oversized playbook is framed as its
  description + heading outline + path, and agents pull the sections they
  need with chunked `sys:read_file` calls instead of drowning the prompt.
  `/skills` lists them (origin, directory marker, and `N KB → outline` for
  the framed ones); `/skill <name> <task>` runs the normal relay with the
  playbook prepended, so every agent in the thread follows it.
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
inbuilt API agents — **planner** (capable tier), **coder**, and **reviewer**
(standard tier) — over an LLM. Provider discovery prefers `DASHSCOPE_API_KEY`
(Alibaba Cloud Model Studio — Qwen commercial models, `qwen-max` →
`qwen-plus` → `qwen-turbo`, override with `CREW_DASHSCOPE_MODEL=a,b,…`; the
endpoint defaults to the international region, point `CREW_DASHSCOPE_BASE_URL`
at the China host if your key lives there), then `OPENROUTER_API_KEY` (free
models by default), and falls back to `ANTHROPIC_API_KEY`; set
`CREW_PROVIDER=dashscope|openrouter|anthropic` to pin one explicitly. Keys
don't have to be in Crew's own environment: at startup the broker imports any
**missing** provider keys (and `CREW_*` vars) from your login shell
(`$SHELL -ilc env`, bounded to 3s; `CREW_SHELL_ENV=0` disables), so a
Dock-launched Crew sees the keys your `~/.zshenv` exports. To survive
OpenRouter's free-tier throttling, the provider
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
agent call; `CREW_MCP_TIMEOUT_MS` (default 30000) bounds each MCP request;
`CREW_MAX_TASKS` (default 4) caps concurrent background tasks;
`CREW_SYS_TOOLS=0` / `CREW_SYS_MODE=readonly` disable or sandbox the built-in
sys tools; `CREW_SYS_TIMEOUT_MS` (default 30000) bounds each `sys:run`. The pane also prints a per-turn timeline + cost summary (`turn done
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

`/settings` opens a **two-column bento form** covering **every configurable
property** — an APPEARANCE card in the left column, WINDOW and NOTIFICATIONS
stacked on the right (collapsing to one column on a narrow pane); Tab/wheel
move focus, Enter commits a field, **Cmd+S / Alt+S** saves and closes:

- **APPEARANCE** — **Font family** (type-to-search over installed monospace
  families), **Font size**, **Paper grain** (0–2 amplitude), **Theme**
  (←/→/Space cycle through the nine presets), **Accent (#hex)** (override the
  theme accent; clear to use the default), **Paper texture** (on/off).
- **WINDOW** — **Nav width**, **Show nav**, **Launch maximized**.
- **NOTIFICATIONS** — the master switch plus per-event toggles (**cmd done**,
  **bell**, **pane exit**), the **min secs** threshold, and the watched
  output **patterns** as a one-per-line text area.

Settings persist to `$XDG_CONFIG/crew/config.toml` and apply live on Save.

## Themes

Crew ships **thirteen themes**: nine paper/ink looks designed to read like a
page rather than a screen, and four old-school CRT phosphor tubes.

- **`paper-dark`** (default) — a high-contrast "newspaper" look: a near-black
  page (`#0a0a0a`) with near-white ink (`#ececec`) and grey rules. Terminal
  output keeps muted-but-readable ANSI colours so error/diff cues survive.
- **`paper-light`** — a warm off-white page (`#f4f1ea`) with soft dark ink and
  ink-toned ANSI colours (sage, brick, faded indigo). No pure black or white
  anywhere; every surface reads as the same sheet of paper.
- **`sepia-dark`** — dark sepia paper with warm cream ink.
- **`sepia-light`** — an aged-newsprint cream page with dark sepia ink.
- **`midnight-ink`** — a deep navy page with cool off-white ink.
- **`graphite`** — a soft charcoal page; the gentlest of the darks.
- **`coldpress-gray`** — a cool pale-gray page with light graphite ink.
- **`salmon-broadsheet`** — an FT-style salmon-pink broadsheet page (light).
- **`ivory-ledger`** — an ivory page with ledger-green ink (light).
- **`crt-green`** — the classic green-phosphor terminal: neon green on a
  near-black tube, with a monochrome-green ANSI palette (brightness tiers) for
  that single-gun look.
- **`crt-amber`** — the warm amber variation of the green tube.
- **`crt-blue`** — a cool blue phosphor variation (Tron).
- **`crt-violet`** — a neon violet phosphor variation.

**Light themes read like print.** The five light themes (`paper-light`,
`sepia-light`, `coldpress-gray`, `salmon-broadsheet`, `ivory-ledger`) render
base text at **Medium (500) weight** — dark themes use Normal (400) — and
carry a **1.2× "newsprint" grain** multiplier, so the page reads as paper
instead of a washed-out screen.

A faint procedural **grain** + edge vignette is drawn behind everything (GPU) —
it reads as paper texture on the paper themes and as a subtle **tube glow** on
the CRT ones. Every palette's colours are picked for measured WCAG contrast.

**Switching:** `/theme <name>` (e.g. `/theme crt-green`) — selecting `/theme`
in the palette opens an arrow-selectable picker — or cycle through every
theme live with **`Ctrl+Shift+L`**. The choice persists to `config.toml`.

**Rotation modes:** three modes rotate to a different theme every
**10 minutes**, and are also the last three stops on the `Ctrl+Shift+L`
cycle, in this order:

- **`/theme random-dark`** (alias: `/theme random`) — rotates the dark
  themes only.
- **`/theme random-light`** — rotates the light themes only.
- **`/theme auto`** — follows the OS appearance: the light pool by day, the
  dark pool by night, re-checked on every OS appearance change and rotation
  tick.

Each switches immediately to a pick from its pool, so the effect is visible
right away.

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
| `theme` | `"paper-dark"` | one of the thirteen theme names (see above), or a rotation mode (`random`/`random-dark`, `random-light`, `auto`); unknown ⇒ default |
| `accent` | theme default | `"#rrggbb"` override for the accent (chrome only); omit to use the theme's accent |
| `paper_texture` | `true` | turn the paper grain + vignette pass on/off |
| `paper_grain` | `1.3` | grain strength (`0.0`–`2.0`; `0` = no grain) |
