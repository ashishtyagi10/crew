# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders as tiles (no overlays). Panes auto-tile into a
near-square grid, drawn cell-by-cell on the GPU with `winit` + `wgpu` +
`glyphon`. See [docs/CREW.md](docs/CREW.md) for the full guide.

It also ships a built-in **swarm orchestration engine** (`crew-hive`): give it a
goal and it decomposes the work into a task graph and runs a pool of agents
toward it — single-goal decomposition or parallel-job batches, bring-your-own-LLM
per agent, with a live task-list view that folds into a per-task record — token
totals, dollar costs (per task + run total), durations, and a Gantt-style run
timeline. See
[Swarm orchestration](#swarm-orchestration-crew-hive) and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

It reads the terminal it is: the whole underline family, dim and conceal, OSC 8
hyperlinks, DECSCUSR cursor shapes, program-requested notifications and progress
(OSC 9 / 9;4), and OSC 133 semantic prompt marks where a shell offers them —
and it derives **command blocks without shell integration**, so `/out` can open
the last command's output on its own, `/blocks` can list what you ran and how
long each took, and every card can mark where its commands began, where its
errors are, and which command you are currently scrolled back into.

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

### Quick install (Windows) — no administrator rights

```powershell
irm https://raw.githubusercontent.com/ashishtyagi10/crew/main/install.ps1 | iex
```

Installs `crew.exe` to `%LOCALAPPDATA%\Programs\crew` and adds it to your
**user** `PATH`. Nothing is written outside your profile — no Program Files,
no `HKLM`, no MSI, no UAC prompt — so it works on a locked-down or managed
machine, and `/update` stays admin-free too. Open a new terminal afterwards so
the `PATH` change reaches it.

Options: `-InstallDir <path>` (or `$env:CREW_INSTALL_DIR`) to install
elsewhere, `-Version v0.17.8` to pin a release, `-NoPath` to leave `PATH`
alone. To pass them, download the script first:

```powershell
irm https://raw.githubusercontent.com/ashishtyagi10/crew/main/install.ps1 -OutFile install.ps1
.\install.ps1 -InstallDir D:\tools\crew
```

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
| Windows (ARM64) | `crew-v*-aarch64-pc-windows-msvc.zip` |

The Windows archives hold a single `crew.exe` with no installer and no
registry footprint — unzip it anywhere you can write and run it.

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
  On Windows the equivalent is re-running `install.ps1`; it moves a running
  `crew.exe` aside rather than failing on the lock.
- **cargo:** `cargo install --git https://github.com/ashishtyagi10/crew crew-app --force`
- **Source checkout:** `git pull && cargo build --release -p crew-app`.
- **In-app:** the **`/update`** command downloads the latest release binary for
  your platform over the running one and **restarts Crew into it**. Progress
  streams into a dedicated **UPDATE card in the left nav** (checking →
  downloading → installed → restarting) — no separate shell or checkout. The
  old `/restart` command was merged into `/update`; an update the background
  check installed quietly is applied by the next `/update` too. A standalone
  `crew --self-update` CLI path remains as a headless fallback.

The prebuilt path only sees a version once its release assets are published.

## App menu / Spotlight

Crew registers itself in your OS app menu on first GUI launch (Spotlight and
Launchpad on macOS, the Start menu on Windows, the applications menu on
Linux). The entry launches the installed binary (`~/.local/bin/crew`) when
present, so `/update` keeps it current.

- `crew install-app` — create or refresh the entry explicitly
- `crew install-app --remove` — remove it
- `CREW_NO_APP_INSTALL=1` — disable automatic registration

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

While a pane is scrolled back, its right border is a live scroll gutter — press
or drag it to move through the buffer. **Cmd+wheel** resizes the font, and a
wheel over the sidebar's LOG scrolls back through its buffered lines.

The pointer changes shape to say what it can do: an I-beam over text, a hand
over a button or a nav row, an open hand over a card's legend row, a resize
arrow on the sidebar's edge — **drag that edge** to widen or narrow the nav.

Inside a pane, **double-click selects a word** and **triple-click the line** —
the gesture every terminal has, and each selection copies. On a card's top
border the mouse does structural things instead: **double-click** zooms, and
**dragging** the card onto another swaps the two. A scrolled-back pane shows a
proportional thumb down its right border beside the `⇡N` count.

**Cmd+Arrow** moves focus the way the eye does: the card to the left, the one
below, the one across. Pane cycling (Cmd+[ / Cmd+]) walks the panes in index
order, which in a tiled grid is not the order they appear in — in a 2×2 grid,
pane 2 sits *below* pane 1, not beside it. Hold **Shift** and the focused pane
comes with you, swapping places with whichever card is that way. Neither wraps
at the edge of the grid.

The chrome answers the pointer: the `[-]` / `[x]` button under the cursor
lights up (`[x]` in the bell colour — it ends a running program), and the
sidebar PANES row under it brightens, because the whole row is a click target.

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

Press **`/keys`** in the input bar for the full list in-app — it scrolls with
the arrow and page keys, and any other key closes it.

| Action | Keys |
|--------|------|
| Next / previous pane | **Ctrl+Tab** / **Ctrl+Shift+Tab** (also Cmd+] / Cmd+[) |
| Jump to pane N | **Cmd+1 … 9** |
| Jump to next active pane | **Cmd+A** |
| Move pane left / right | **Cmd+{** / **Cmd+}** |
| Focus the pane that way on the grid | **Cmd+←↑→↓** |
| Swap the focused pane with that neighbour | **Cmd+Shift+←↑→↓** |
| Focus the input bar | **Cmd+I** |
| Recall a line you typed before | **↑** / **↓** — filtered by what is already in the bar; the top border says `hist 2/5 · git` |
| New shell pane | **Cmd+T** |
| Reopen last session's panes (shells, Far, /crew) | `/restore` |
| Settings / chat pane | **Cmd+,** / **Cmd+J** |
| Toggle sidebar | **Cmd+G** |
| These keys, on screen | **Cmd+/** (or `/keys`) — type to filter, ↑↓ to scroll, Esc to close |
| Zoom focused pane | **Cmd+Z** (or double-click its top border) |
| Broadcast input to all panes | **Cmd+S** |
| Font bigger / smaller / reset | **Cmd+=** / **Cmd+-** / **Cmd+0** |
| Copy visible screen / paste | **Cmd+C** / **Cmd+V** (Cmd+V pastes a clipboard image as a temp PNG path) |
| Open URL / file / dir under cursor | **Cmd+Click** |
| Cycle the theme rotations (dark → light → crt → auto) | **Ctrl+Shift+L** |
| Toggle chat markdown preview ↔ raw source | **Ctrl+Shift+M** |
| Reverse-search the chat composer's send history | **Ctrl+R** |
| Find: the chat transcript, or `/find` in the bar | **Cmd+F** (or **Ctrl+F**) |
| Insert a newline in a terminal | **Shift+Enter** (sends a line feed, not submit) |
| Close pane / maximize window | **Cmd+W** / **Cmd+M** |
| Reopen the pane you just closed | **Cmd+Shift+T** (or `/reopen`) |
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
by the same provider stack as `/crew` (DashScope / OpenRouter / Anthropic, or
a direct OpenAI / Gemini / DeepSeek key — see
[docs/CREW.md](docs/CREW.md#models--rate-limits)).
**`??<question>`** goes the other way: the AI reads the focused terminal's
recent output and opens its explanation in the zoomed file viewer, rendered
as markdown — `??why did this fail` after a broken build gets you a
formatted post-mortem.

Slash commands complete the bar (type `/` for a fuzzy palette): `/crew`
(`/smith`), `/goal <text>`, `/batch <file>`, `/view <file>`, `/md <file>`,
`/diff`, `/settings`, `/find <text>`, `/findall <text>`, `/errors`,
`/errorsall`, `/out`, `/blocks`, `/marks`, `/name <text>`, `/pin`, `/clear`,
`/clearall`, `/clearlog`, `/only`, `/copy`, `/dump`, `/closeall`, `/reopen`,
`/restore`, `/blame`, `/leading`, `/invisibles`, `/pwd`, `/about`, `/log`,
`/model`, `/update`, `/broadcast`, `/zoom`, `/sidebar`, `/keys`, `/far`,
`/todo`, `/dash`, `/usage`, `/disk`, and the look: `/theme`, `/gradient`,
`/font`, `/weight`, `/smooth`, `/gamma`, `/grain`, `/leading`, `/density`,
`/opacity`, `/crt`, `/motion`, `/contrast`, `/shapes`, `/focus`, `/notify` — then
`/exit`. Commands with a fixed value set (like `/theme`) expand into an
arrow-selectable **value picker**. Fish-style autosuggest from history, `cd`
completion with `$VAR` expansion, and `Up`/`Down` history recall persisted to
`$XDG_CONFIG/crew/history` round it out. `/diff` opens the working tree's
colored git diff (status, stat, full diff) in its own pane — Codex-style
change review beside your shells; `/view <file>` opens a zoomed **file
viewer** pane — code, markdown (rendered), CSV, diffs and more, one pane, read
only (`/md` is kept as an alias); `/todo` opens a **todo list** pane — type
`pay rent tomorrow 5pm @home` and the due date and `@project` tag are
recognised as you type (tinted live, stripped from the title on Enter),
overdue items surface to the top, and a toast fires when an item comes due.

## Sidebar

A docked left panel (toggle with **Cmd+G**) with a live clock, CPU/MEM/DISK
**ring gauges**, a moving **CPU curve** under them, load average, host info,
network rates over a **twin chart** that draws the two directions apart, a git
section for the working directory, a **LOG** tail, and a list of open panes
(click a row to focus it) headed by the **crew mix** — one chip per pane on a
row per state, working / waiting / idle, with the crew total on the section
rule. The charts scroll on the
sidebar's once-a-second refresh, so they animate at no extra redraw cost.

Every part of it answers to the width you give it (drag the inner edge):

- **A chart with a moving ceiling writes the ceiling down.** The CPU curve is
  scaled to its own rolling minute and the network twin to the louder
  direction, so a machine idling under 10% still draws a shape — and each
  section's rule carries the scale it is drawn against (`─ SYSTEM peak 55% ──`,
  `─ NET peak 64 KB/s ──`).
- **Prose ellipsizes; a row of values drops whole values.** A narrow nav shows
  two load averages, or the busier network direction, whole — never half a
  number. The LOAD rule's key names exactly the averages that survived.
- **The LOG grows into whatever the column has spare**, up to twenty lines, and
  its rule says how much of the buffer is showing (`─ LOG 8/64 ──`). A wheel
  over it scrolls back.
- **The rings spread and then centre** rather than staying pinned left, and the
  two network rates go to opposite ends of the row once there is room.

## What a pane tells you

Crew's cards carry what it knows about a pane on their **borders**, never in
the program's own columns — a terminal's grid belongs to whatever is running in
it.

- **Git state** of the directory the pane is in: `main ●3 ↑2 ↓1`, shed detail
  first as the card narrows, queried off-thread so a slow repo never stalls the
  UI.
- **How long** the foreground command has been running (`9s`, `2m14`, `1h05`),
  past five seconds.
- **What arrived while you were away**: a count on the border, in the sidebar
  and on the minimized thumbnail. It clears when you type into the pane or
  scroll back to the live bottom.
- **Where each command began** (a tick) and **where the errors are** (a red
  bar) — block structure and failure marks without any shell integration,
  since crew already watches each pane's foreground process. `/marks off` for a
  plain frame.
- **Which command you are reading**, once you scroll back into it: `╶ cargo
  build` beside the `⇡N`, since the prompt that started the output is off the
  top of the window by then. Other terminals pin a sticky prompt line inside
  the viewport; that costs a row of the program's grid.
- **Which block went wrong**, when the shell says so. Crew reads **OSC 133**
  where it is offered — the one thing polling cannot see is how a command
  *ended* — and marks that block's first row in the alarm colour, names the
  status beside the command (`╶ cargo build ✗101`), and raises a `failed` card
  rather than a quiet `done` when it happens. A shell that says nothing keeps
  exactly the blocks it had; "no answer" is never drawn as success.
- **Progress a program reports** (OSC 9;4), filling the bottom border — or
  sweeping when it is working without a number.
- **Where you are** in a scrollback, a document or a transcript: a proportional
  thumb, with landmark ticks beside it (a diff's hunks, a document's headings,
  a conversation's turns) and search hits in the search's own colour.

## Reading what happened

- **`/out [n]`** opens a command's output on its own in the file viewer — the
  last one, or the one *n* commands back. **`/copy out`** puts the same slice on
  the clipboard.
- **`/errors`** walks back to the most recent failure in a pane; **`/errorsall`**
  says which panes have failures and how many, then lands on the first.
- **`/diff`** reviews the working tree in the viewer, pairing each removed line
  with the added line that replaced it and drawing only the run that actually
  differs at full strength — word edges respected, trailing whitespace on added
  lines marked. `]` and `[` walk files and hunks, **`v` lays it out side by
  side** (each half carrying its own file's line numbers), and the same
  treatment applies to a fenced diff in an agent's reply.
- **`/blocks`** lists what you ran in the pane, newest first: how long each
  took, which of them failed, and the number `/out` counts back with — so
  `/blocks` says what you ran and `/out 2` opens the output of the third one
  back.
- **`/blame`** answers who last touched each line of the file in the viewer,
  collapsed to the boundaries where one commit's work ends and the next begins.
- **`/reopen`** (**Cmd+Shift+T**) undoes the last pane close: a shell in the
  directory that one was standing in, the viewer back on its file. The last
  eight, so a `/only` walks back a pane at a time.
- **`/pin`** keeps a pane on the grid when the LRU would demote it.

## The cursor

The cursor takes the shape the program asks for — block, bar or underline
(DECSCUSR), so an editor's insert mode reads as one. Unfocused panes draw an
outline instead, so exactly one cursor on the canvas is filled in.

## Text decorations

The full underline family — single, double, curly (the spell-check squiggle),
dotted, dashed — plus strikethrough and SGR 58's separate underline colour,
drawn as GPU rules that stay continuous across cells. **Dim** (SGR 2) and
**conceal** (SGR 8) are honoured too: half of what an agent CLI prints is dim,
and a password prompt that hides its field means it.

Links are marked as links. URLs are tinted and underlined; **OSC 8 hyperlinks**
are drawn as links whatever their text says, and Cmd+click opens the program's
target (only `http`/`https`/`mailto`/`file`, named on the status line). **File
references** — `src/main.rs:42`, `./deploy.sh`, `Cargo.toml` — wear a dotted
rule instead, because they open here rather than in a browser, and a clicked
`path:line` lands on that line.

What a program paints with **coloured spaces** — a TUI's status line, a
progress bar, `fzf`'s selected row — renders as it should, and so does a
selection dragged across empty space.

## Markdown

Crew renders markdown natively (a `pulldown-cmark`-based engine drawn straight
to GPU cells — headings, lists, tables aligned by display width, fenced code
cards, links):

- **Chat panes** render agent replies as formatted markdown by default;
  **Ctrl+Shift+M** flips the focused chat pane to the raw source and back.
  **Cmd+Click** opens a rendered link. Task lists (`- [ ]` / `- [x]`) render
  as **checklists** — ☐ open, green ✓ with dimmed text when done — and
  ```` ```diff ````/```` ```patch ```` fences colour added/removed/hunk lines
  (an untagged fence that reads as a diff is auto-detected and coloured the
  same).
- **The heading you are underneath stays on the top row** while you scroll a
  document, with the ladder above it (`crew › Themes › CRT`).
- **A picture a document names is drawn** — `![alt](src)` in markdown renders
  as the picture, resolved against the document and read off the frame thread.
- **`/doc <file>` opens a document in a window of its own** — no nav, no input
  bar, no tiles: one file, framed, filling its own window, sized to a reading
  measure. `w` inside the viewer pops the document you are already reading out
  of the grid and into one. The grid goes on being a grid.
- **`Cmd+N` opens another crew window** — a whole second canvas: its own grid
  of panes, its own focus and zoom, its own input bar. Closing a window closes
  that window; the last one quits. A session remembers which window each pane
  was in.
- **A markdown document in a window is an editor.** It opens rendered with a
  cursor in it; the arrows move through the *render*, typing goes in where you
  are looking, Enter continues the block you are in, and **Cmd+S** writes the
  file **with only your edit in it** — no re-wrapped paragraphs, no rewritten
  bullets. Click to place the cursor, **Cmd+Z** to take a word back. No `#` or
  `**` ever appears on screen.
- **`Cmd+E` labels everything on the pane worth reaching** — every URL, file
  reference and hash wears a letter; pressing it copies, pressing its capital
  opens (a URL in the browser, a file in the viewer). Labels come off the home
  row and go to the newest output first, so the thing a program just printed is
  the cheapest key. Esc — or any letter that starts no label — ends the mode.
- **A program can show you a picture.** Crew speaks the terminal graphics
  protocol (kitty's `APC G` form — `kitten icat`, `timg`, matplotlib's kitty
  backend), so a plot or a screenshot arrives inside the output: anchored to
  the line it came in on, scrolling with it, clipped to the pane, decoded off
  the frame thread and drawn on the sub-cell paint layer.
- **`/view <file>`** (alias `/md`) opens a zoomed **file viewer** pane — a
  single, read-only pane over the file, rendered by format: markdown
  (headings/lists/links/code fences), a numbered-gutter code view, aligned
  CSV columns, colored diffs, or a metadata card for anything else. Arrows/
  PageUp/PageDown/Home/End scroll, **r** reloads from disk, **s** toggles raw
  text (markdown and CSV files), **/** searches with `n`/`N` for next/previous
  hit, **e** opens `$EDITOR`, **o** hands the file to the OS default app,
  **Cmd+Click** opens a rendered markdown link, **Esc** closes. **A picture
  opens as the picture** — PNG, JPEG, GIF, BMP and WebP are drawn on the
  sub-cell paint layer, fitted to the pane and centred, decoded off the winit
  thread; transparent pixels let the page through.

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

The pane speaks a tiny **construct language** (seven infrastructure commands),
and plain language does the rest: "have every agent take a crack at this"
sends one task to every agent **in parallel** (replies stream back
fastest-first), "keep refining it" iterates on the crew's own answer, "keep
working until …" loops until a judge agent rules the goal met — the broker's
intent router picks the execution shape (`CREW_INTENT=0` turns it off).
`@a+b <task>` fans out to a subset, bare `/model` shows a **grouped provider
picker** — your subscriptions (a signed-in `claude` or `codex` seat serves
smith work with **no API key**; signed-out ones show the exact sign-in
command, and a device-flow provider like Qwen **signs in right in the pane**:
pick its number, approve the code card in your browser, done — tokens live
in the OS keychain and refresh themselves), your keys, and installed CLIs,
with `/model <n>` switching provider
persistently — `/model <agent> <model>` pins agents to **different models
side by side**, and
the footer reports live totals — with Tab completion for `@agent` names and
slash constructs in the composer, one-letter aliases (`/m` → `/model`), and
did-you-mean on typos. Long constructs run as **concurrent background tasks**
(default cap 4): each reply is tagged with a dim `#N` task chip, the footer
lists what's running, and `/stop [#n]` cancels one task or all of them.
`@file` mentions in the composer fuzzy-complete against the project tree and
splice the file's contents into the outgoing message — a path with spaces
rides a quoted `@"my notes.md"` mention, and a file **dragged onto the pane**
becomes one too (terminals get the shell-quoted path instead). **Ctrl+R**
reverse-searches what you've sent, shell-style; **Cmd+F** searches the
transcript and jumps to each match.

Agents can also touch the workspace through built-in **sys tools** — bounded
`sys:run` (non-interactive shell, 30s/64KB caps), `sys:read_file` (chunked
64KB reads), `sys:write_file`, and `sys:list_dir` — callable mid-relay the
same way as MCP tools. `CREW_SYS_MODE=readonly` blocks the mutating ones,
`CREW_SYS_TOOLS=0` turns the surface off, and `/doctor` shows the working
directory and sandbox mode. An optional token budget
(`CREW_BROKER_TOKEN_BUDGET`) hard-stops a runaway thread.

It also borrows the flagship moves of the big coding agents: **plan mode**
(ask for "a plan for …" and nothing runs until you approve it — enter or
"approve" runs it, esc or "reject" discards — à la Claude Code),
**workspace checkpoints**
(a checkpoint is taken automatically before every task that can change files,
as a hidden commit under
`refs/crew/` without touching HEAD or your index, bare `/restore` lists,
`/restore <n>` brings a snapshot's files back — à la Cline),
**transcript export** (`/export` writes the conversation to
`crew-transcript-<stamp>.md` — à la OpenCode), **AI commit messages**
(say "commit this" and the crew drafts a Conventional Commits message for
your diff; nothing is committed until you say "apply" — à la Aider),
**AI code review** (ask "look over my changes" for findings on the working
diff worst-first — à la Codex), **session resume** (the conversation
auto-saves to `./.crew/`, and asking to "pick up where we left off" in a
fresh pane folds the last session into the next task — à la Claude
Code's `--continue`), **`/doctor`** (a ✓/✗ health check of the whole AI
stack: provider key, agent CLIs, MCP servers with their tools, skills,
memory — each failure names its fix), **AI standups** (ask "what did I ship
this week?" for an update from recent commits: done by theme, in progress,
risks). The old slash forms of all of these — fan, loop, goal, plan,
commit, review, standup, resume, skill, memory, mcp — are retired; plain
language replaced them. The transcript folds itself, which
folds older messages away when a long session gets heavy; `/diff` (in the
pane or the input bar) completes the loop with Codex-style change review.

The pane is extensible the way other coding tools are — three drop-in
surfaces, no rebuild, edits picked up live (`/reload` forces it; no restart
needed) — see [docs/CREW.md](docs/CREW.md#multi-agent-relay-crew):

- **Memory** — Claude Code-style `#` shortcut: `#always use pnpm` in the pane
  appends to `./.crew/memory.md`, and every task from then on carries the
  merged memory (user + project files, 2 KB cap) as a standing block the
  agents follow; ask "what do you remember?" to see it.
- **Skills** — markdown prompt playbooks in `~/.config/crew/skills/` or
  `./.crew/skills/` (optional `name:`/`description:` frontmatter; project
  overrides user). A skill can also be a **directory with a `SKILL.md`** plus
  supporting files, and oversized playbooks disclose **progressively**: past
  8 KB the relay gets the description + heading outline + path, and agents
  read sections on demand with chunked `sys:read_file` calls. There is no
  command: a task that names a skill picks its playbook up by itself, and
  when skills are loaded but unmatched the crew sees a one-line roster of
  them.
- **Plugin agents** — a JSON manifest in `~/.config/crew/agents/` or
  `./.crew/agents/` (`{"name", "command", "args": […, "{}"], "role"}`) turns
  any headless CLI into a roster agent; installed manifests appear in
  the roster and make `/crew` usable with **no API key at all**.
- **MCP** — servers declared in `~/.config/crew/mcp.json` or `./.crew/mcp.json`
  (the standard `mcpServers` schema) connect lazily over stdio; `/doctor`
  lists each server with its tools, relay prompts advertise them, and agents
  call one by ending a
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
2 exchange(s) · ~950 tok (approx)`, and each settled reply that reported real
usage closes with its own muted trailer — `900 in / 50 out · $0.012`. Long
system/telemetry cards (turn summaries, `/doctor` output) **auto-fold** to a
header + first line + ` … +N` — click to expand, click the header to fold
back. New cards fade in from the page colour;
fenced ```code``` in replies renders as a
solid tinted field — one rectangle, the language on its top row, the same width
on every line — whose colour is walked per theme so it reads on paper and on a
phosphor tube alike; the transcript sits against the composer rather than
floating at the top of the pane; a composer with
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
property: font family/size, font smoothing (the `/smooth` ladder), text gamma
(the `/gamma` ladder), line spacing
(`/leading`), density, nav width + visibility, theme, accent, paper texture +
grain, card border marks, revealed invisibles, launch-maximized, and the whole
notification block (master + per-event toggles, min-secs threshold, watched
output patterns as a one-per-line text area). Every key in the config file has
a field here, and a test that parses `config.rs` says so. **Cmd+S / Alt+S**
saves, and **`/keys`** lists how to reach and change each field without the
mouse — as it now does for every pane kind that answers to keys of its own. Settings persist to
`$XDG_CONFIG/crew/config.toml` and apply live on Save. The config file also
accepts `accent = "#rrggbb"` to override Crew's accent; omit it (or give an
invalid value) to use the active theme's default accent. It applies at launch —
quit and reopen Crew to pick up edits made outside the `/settings` pane.

**Themes.** Crew ships **twelve palettes** in four rotations. `dark` rotates
`paper-dark` (a high-contrast newspaper look), `sepia-dark` (warm cream ink on
dark sepia), `nebula` (an orchid→rose gradient dusk) and `harbor` (a blue-slate
page under an azure light); `light` rotates `paper-light`, `sepia-light`,
`blossom` and `fern` (a faint mint page under a green-teal light); `crt`
rotates the four phosphor tubes — `crt-green`, `crt-amber`, `crt-blue` and
`crt-violet`, each one hue at six brightnesses on a near-black tube; and `auto`
follows the OS appearance, serving the dark pool in dark mode and the light one
in light mode (re-wire the pairing with `theme_dark` / `theme_light`). A
rotation changes palette every 10 minutes.

Almost every colour in a palette is **derived, not chosen**: the ramp produces
the whole text ladder and all sixteen ANSI slots from the page and the ink, a
wash produces the search highlight, and an alarm derivation produces the bell —
each with a contract test that fails when a shipped value drifts from what the
system would produce. What a palette actually picks is its page, its ink, its
accent and its gradient poles.

`/theme` offers the four rotations and then every palette by name, and shows
you the colours rather than the names: a rotation draws one chip per palette it
would serve, and a named palette draws its whole hand — the ink it writes in,
its accent, and the four ANSI slots every program in a pane paints with, each
over that palette's own page. Better still, **arrowing onto a palette puts it
on**, and arrowing off (or dismissing the picker) puts back the one you had —
because a swatch tells you what a palette *is* and only wearing it tells you
what the screen you are looking at will *look like*. `/gradient` previews its
named pairs the same way. `Ctrl+Shift+L` cycles the rotations; the palettes retired
in an earlier roster cut still parse, so an old config keeps working. `/crt
on|off|auto` overrides the tube post-process independently of the theme.

Light themes render ink at Medium weight over 1.2× "newsprint" grain so they
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
| `crew-theme` | Theme presets + palette contracts (13 themes, rotation modes, contrast thresholds) |
| `crew-hive` | Swarm orchestration engine (planner, scheduler, agents, blackboard, telemetry) |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full diagram (app +
engine internals).

Hard rules: every `.rs` file stays ≤200 lines; `cargo clippy --workspace
--all-targets` is warning-free.

## License

MIT or Apache-2.0, at your option.

crew embeds **[Lilex](https://github.com/mishamyrt/Lilex)** 2.700 (SIL Open
Font License 1.1 — see `assets/fonts/OFL.txt`) as its built-in typeface, so the
grid never depends on what happens to be installed on the machine. Any
installed coding face you prefer still wins: pick one with `/font`, or let the
theme choose (`crew --list-fonts` shows what it can see).
