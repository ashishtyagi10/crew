# Crew

A from-scratch, native **GPU terminal** written in Rust — an AI-oriented terminal
where everything renders in the terminal as tiles (no overlays). Crew is the
successor to this repo's original terminal file-manager project; the crates under
`crates/smith-*` are the product.

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
- **Terminal model** — `alacritty_terminal` + `portable-pty` (`crates/smith-term`).
- **In-pane UI** — `ratatui` widgets are laid out into a `Buffer` and converted to
  GPU cells (the settings form, command palette, and help overlay use this).
- **Crates** — `crew-app` (window, panes, input), `crew-render` (GPU), `crew-term`
  (PTY + grid), `crew-plugin` (chat/agent plugins + the `/smith` relay broker),
  `crew-theme` (the palette presets + their contracts — see
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
- `crew panes` — list the running instance's addressable panes (for inter-pane ask)
- `crew ask <id|label> "<question>"` — ask the agent in another pane (see below)
- `crew ask --all|--any "<question>"` — broadcast to every pane (see below)
- `crew ask <pane>@<instance> "<question>"` — ask a pane in another crew instance
- `crew instances` — list crew instances running on this host (federation)
- `crew federate` — show federation status (on/off, bind/port) and how to enable it

## Inter-pane ask

An agent working in one pane can query an agent in another pane of the same
running Crew — visibly, in-session, with a wait governed by whether the target
is actually generating a reply rather than a fixed timeout.

- **Discover**: `crew panes` prints a roster — each pane's stable `p<N>` id, its
  `/name` label, kind (terminal / swarm), the foreground agent (e.g. `claude`),
  its directory, and whether it's busy. Address a pane by id (`p2`) or label.
- **Ask**: `crew ask schema "which API version does the client target?"`. Crew
  injects the question into the `schema` pane's live session (you see it land),
  the agent there answers on a line beginning with `CREW-ANS-<id>:`, and the
  answer prints back to `crew ask`'s stdout:

  ```
  ANSWERED: v2 — see api/v2/client.rs
  ```

- **Smart wait**: Crew keeps waiting while the target genuinely produces output,
  and returns `NO_ANSWER <reason>` (idle / stalled / unreachable) once it stops —
  so the asking agent never hangs and can fall back to another approach. Exit
  code: 0 answered, 2 no-answer, 3 unreachable/no crew running.

- **Broadcast** (v2): ask *every* pane at once instead of naming one.
  - `crew ask --all "what's blocking you?"` — fan the question into every other
    terminal pane, wait for them all, and print one aggregate (each pane's answer,
    or why it stayed silent).
  - `crew ask --any "who has the staging DB URL?"` — same fan-out, but the first
    real answer wins and the rest are dropped. Use it as a query-by-need: don't
    know who knows, ask the room.

  ```
  [schema] ANSWERED: v2 — see api/v2/client.rs
  [tests]  no answer (idle)
  ```

  Exit code: 0 if anyone answered, 2 if panes were reached but none answered,
  3 if no pane was eligible. Broadcast reuses the same visible-injection and
  smart-wait engine per pane — it only widens *who* is asked.

- **Federation** (v3): reach an agent in *another* crew instance — same host or
  across the network — reusing the identical engine end to end.
  - **Same host:** run a named instance `CREW_INSTANCE=alpha crew` (each gets its
    own socket; unnamed is the default; `crew instances` lists them), then
    `crew ask schema@alpha "which API version?"`. The `@alpha` picks that
    instance's socket; the `schema` pane is resolved by *that* crew exactly like
    a local ask.
  - **Across hosts (opt-in):** the operator of the host you want to reach turns
    federation ON by starting crew with a shared token —
    `CREW_FEDERATE_TOKEN=<secret> crew` — which binds a relay
    (`CREW_FEDERATE_BIND`/`CREW_FEDERATE_PORT`, default `0.0.0.0:7733`). Nothing
    binds without a token, so a host is never reachable it didn't choose to be.
    Then, with the same token in your environment,
    `crew ask schema@crew://their-host/alpha "…"` dials the relay, which — after
    checking the token — bridges your request to that host's `alpha` instance and
    relays the reply back. Same visible injection, smart wait, and verdicts;
    only *who* you can reach widened.
  - **Security:** the relay carries the shared token and JSON envelope in the
    clear — run it over a trusted network or an SSH/WireGuard tunnel, or behind a
    TLS-terminating proxy. It is opt-in and token-gated by design; it never
    discovers or reaches a host that hasn't turned it on. (Per-host invites,
    token rotation, and built-in TLS are roadmap.)

The engine (envelope, sentinel protocol, verdict, liveness) is identical across
local, broadcast, and federated asks — see `docs/vision/sentinel-network.md`.

## Panes

Panes auto-tile into a near-square grid. Each pane has a **title bar** (top row)
showing its index, the program-set title (often the cwd), and right-aligned
status glyphs:

| Glyph | Meaning |
|-------|---------|
| `⇡N`  | viewing scrollback, N lines back from the live bottom |
| `╶ name` | which command's output the top of the window is inside (while scrolled back) |
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
along the bottom of the content area, each showing the pane's **number** and
title, an activity marker on the left, and — on the right — **how many lines
arrived while it was down there**, ordered least-recently-active first. The
strip is where a pane goes when you have not touched it for a while, which is
exactly where "what did I miss?" is loudest; the marker alone could only ever
answer "something". A thumbnail with no room for both keeps the marker, which
is the one that says the pane is alive. The focused pane is protected
from demotion. To restore a minimized pane to the full grid, click its thumbnail,
click its entry in the sidebar's PANES list, or use **Cmd+1 … 9** to jump to it.

**Colours that read on the page they land on.** Six roles were constants
chosen by eye on a dark theme and never measured against a light one: the
terminal cursor, a URL in terminal output, the selection wash, the two
load-average warning colours and the network sparkline. On the light themes
they read at 1.4–2.4 contrast — the *focused* cursor was four times fainter
than the unfocused ones, so the pane you were typing in had the faintest cursor
on the canvas, and the warning amber was effectively invisible.

Each keeps its **hue** — a link is blue, a warning amber, an alarm red, and
those meanings are not the palette's to take — and gives up its **lightness**,
walking away from the page until it clears WCAG's floor (4.5 for anything read,
3.0 for a mark only seen). A colour that already clears comes back untouched,
so the dark themes are pixel-identical. A contract test measures all nine
palettes at once.

**Spatial navigation.** **Cmd+←↑→↓** focuses the card that lies that way on
the grid, chosen from the same rects the mouse hit-tests against — so keyboard
focus and the pointer can never disagree about where a tile is. Pane cycling
(**Cmd+[** / **Cmd+]**) steps in *index* order, which a tiled grid does not
follow: with four panes, pane 2 sits below pane 1, not beside it. Add
**Shift** and the focused pane travels with the gesture, swapping with the
neighbour in that direction. Neither wraps at the grid's edge — a wrap in a
spatial gesture reads as the whole canvas jumping. Zoomed, where there is no
geometry to navigate, Cmd+Arrow falls back to stepping through the panes.

**Pointer feedback.** The `[-]` and `[x]` on a card's border light under the
cursor — `[-]` in the accent, `[x]` in the bell colour, so the control that
ends a running program says so before it is clicked. A hovered sidebar PANES
row lifts its ink to full contrast rather than washing a background behind it:
the page's contrast budget is spent on the theme's own wash, so hover buys its
emphasis with ink. Both repaint only when the target changes, so sweeping the
pointer across the canvas costs one frame per thing it crosses.

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

**A command that failed says so.** When the shell reports an exit status (see
OSC 133 above), a command that finished non-zero raises a `failed` card in the
bell colour naming the status — `✗ cargo test (2m14) — exit 101 failed in
crew` — where a successful one raises a quiet `done`. It is the same event and
the same switch (**Settings → NOTIFICATIONS → Agent done**): a failure *is* a
command finishing, and splitting the preference in two would ask you to say
twice that you want to hear about commands finishing. What differs is how
loudly it is drawn, because "it is done" and "it went wrong" are not the same
news and only one of them is worth getting up for. A pane you were not looking
at raises `✗` as its attention marker rather than `✓`.

**Toast cards.** The same events also step onto the canvas as cards at the
top-right of the content area, each holding one line (`done`, `failed`, `bell`,
`waiting`, `exited`, `match`, `due`, `error` — the last two and `failed` /
`waiting` / `exited` border in the bell colour). They are not decoration you have to catch
in time:

- **Rest the pointer on the stack and it holds** — every card in it, until the
  pointer leaves. Auto-hiding content that cannot be paused is unreadable to
  anyone who reads slowly or looks up a moment late (WCAG *Pause, Stop,
  Hide*). The whole stack holds rather than just the card under the cursor:
  expiring its neighbours would slide the stack up and move the card out from
  under the pointer.
- **Click a card to go where it points.** A toast raised by a pane focuses
  that pane — restoring it out of the nav if it was minimized. A card that
  names no pane is simply dismissed. Either way the card leaves.
- **The same thing said twice is one card that says `×2`.** A repeat counts up
  on the card's legend and restarts its life *where it is* — promoting it to
  the bottom of the stack would slide every other card, and the pointer may be
  resting on one. It matters most for the case it was written for: a watched
  pattern that matches every line of output used to push a card per match and,
  at four cards, evict every other notification crew had to do it. The count
  survives the hover rewrite (`waiting ×4 → open`), since the reason you are
  hovering may well be that it happened four times.

Hovering says so: the stroke lights in the accent and the legend reads
`waiting → open`, or `note ✕` for a card with nowhere to go. The pane is
remembered by name and resolved when clicked, so a card outliving its pane
says so rather than opening whatever now sits at that index; an `exited` card
deliberately offers nothing, since that pane is already gone.

## Keyboard shortcuts

**Hold a modifier to see what it does.** Rest on `Cmd` (`Ctrl` off macOS) for
450ms and a row of chips appears above the input bar naming what that modifier
reaches from where you are — pane jumps and focus arrows always, plus `↵ send`
on the input bar or `K clear · Z zoom · W close` in a pane. `Ctrl` (`Alt` off
macOS) answers with the walks that live there: next pane, theme, gradient,
palette. Press anything or let go and it disappears.

It opens only on a modifier held **alone** — a chord in progress belongs to
someone who already knows the binding — and only after the dwell, so an
ordinary `Cmd+C` never flashes a panel on its way past. `Shift` never opens
it: it reaches nothing by itself and is held through every capital you type.
The full table is still `/keys`, which the hints are deliberately much shorter
than.


Press **`/keys`** in the input bar for this list in-app: the bindings above,
then a section for **every pane kind that answers to keys of its own** — an
agent pane, the file viewer, a `/far` file panel, the `/todo` list and
`/settings`. Most of those keys used to be written down here in the manual and
nowhere a user could find them without reading the manual: the viewer's whole
set, the panel's function-key row, and six of the todo list's eight actions.
A test reads each pane's own key map and holds the overlay to it, the same way
the overlay and this page are held to each other — so a new pane kind is one
row in a table rather than a rediscovery two releases later. It scrolls — arrows
and page keys walk it, Home/End jump its ends — so the list is never cut off by
the window it is drawn in, and **typing filters it**: forty-odd bindings is a
document, and the fastest way through a document is to say what you are looking
for. Both halves of a row are searched (the chord as well as the words), a
section heading survives only while something under it does, and a search that
matches nothing says so rather than emptying the panel. What you typed is shown
where the version normally sits. **Esc** closes it (so does any key that is not
a letter, a space or Backspace), and the filter is forgotten on the way out.

| Action | Keys |
|--------|------|
| Next / previous pane | **Ctrl+Tab** / **Ctrl+Shift+Tab** (also Cmd+] / Cmd+[) |
| Jump to pane N | **Cmd+1 … 9** |
| Jump to next active pane | **Cmd+A** |
| Jump to next pane waiting on you | **Cmd+.** |
| Move pane left / right | **Cmd+{** / **Cmd+}** |
| Focus the pane that way on the grid | **Cmd+←↑→↓** |
| Swap the focused pane with that neighbour | **Cmd+Shift+←↑→↓** |
| Focus the input bar | **Cmd+I** |
| Find: in a chat transcript, or `/find` in the bar | **Cmd+F** |
| New shell pane | **Cmd+T** |
| Settings / chat pane | **Cmd+,** / **Cmd+J** |
| Toggle sidebar | **Cmd+G** |
| Zoom focused pane | **Cmd+Z** (or double-click its top border) |
| Broadcast input to all panes | **Cmd+S** |
| Font bigger / smaller / reset | **Cmd+=** / **Cmd+-** / **Cmd+0** |
| Copy visible screen / paste | **Cmd+C** / **Cmd+V** |
| Open URL / file / dir under cursor | **Cmd+Click** |
| Cycle themes (dark → light → crt → auto) | **Ctrl+Shift+L** |
| Toggle chat markdown preview ↔ raw source | **Ctrl+Shift+M** |
| Toggle the chat's compact transcript view | **Ctrl+O** (falls through to the terminal when the focused pane isn't a chat) |
| Reverse-search the chat composer's send history | **Ctrl+R** (again steps to the next older match) |
| Find in the chat transcript | **Cmd+F** (or **Ctrl+F**; Enter/Ctrl+F step older, Up newer) |
| Complete the leading `@agent` name or slash construct | **Tab** |
| Insert a newline in a terminal | **Shift+Enter** (line feed, not submit) |
| Close pane / maximize window | **Cmd+W** / **Cmd+M** |
| Reopen the pane you just closed | **Cmd+Shift+T** (or `/reopen`) |
| Clear focused pane scrollback | **Cmd+K** (or `/clear`) |
| Scroll any pane | **Shift+PageUp** / **Shift+PageDown**, or mouse wheel |
| Scroll to top / bottom | **Shift+Home** / **Shift+End** |
| Quit | **Cmd+Q** — twice to confirm when panes are open, same as the window close button |

Click a pane to focus it (click the input bar to focus that).

**The mouse on a card.** Inside a pane, the click run is the one every terminal
has: once arms a drag-selection, **double-click selects the word** under the
cursor, **triple-click the whole line** — and each selection copies, the same
rule releasing a drag follows. A path stays one word, and a soft-wrapped
command comes back whole. A fourth click starts the run over rather than
latching the widest gesture. It works on any pane kind: the transcript in an
agent pane selects by word exactly as a shell does.

On a card's **top border** — the legend row, which holds nothing to select —
the mouse does structural things instead. **Double-click** toggles zoom (where
the convention puts it: a window's title bar, not its contents). **Drag** picks
the card up: the card under the pointer lights in the accent, and releasing
swaps the two, the mouse equivalent of the **Cmd+Shift+←↑→↓** chord above.

**The scroll gutter.** While a pane is scrolled back, its right border is a
live gutter: press it to jump to that point in the buffer, drag it to cross the
whole scrollback in one gesture. At the live bottom there is no gutter — there
is nothing behind it to reach.

**Cmd+wheel** (Ctrl+wheel off macOS) resizes the font, the same step
**Cmd+=** / **Cmd+-** takes.

**The pointer's shape.** The cursor says what the thing under it does before
anything is clicked: an I-beam over text a click would select, a hand over the
border buttons, the nav rows and the `+N` tile, an open hand over a card's
legend row (the handle it is carried by) that closes while one is in hand, and
a column-resize arrow on the sidebar's edge.

**Resizing the sidebar.** Drag its inner edge. The width was a figure in the
Settings form and nowhere else; it is now also a handle, clamped to the same
160–320 px the form clamps to. The nav is chrome, not a pane, so the grid never
changes shape — it is handed a narrower content rect exactly as it is when the
window is resized. The width persists when you let go.

**The `+N` tile** now **names the panes behind it** — numbered the way `Cmd+N`
numbers them — instead of only counting them. Which panes are hidden is the one
thing you would look at that tile to find out.

**The `+N` tile.** When the minimized strip cannot show every thumbnail at a
readable width, the last slot becomes a `+N` tile standing for the rest.
Clicking it reveals the first pane behind it (focus is always a restore path);
clicking again walks to the next.

**Scrollback position.** A scrolled-back pane shows `⇡N` on its top border —
how many lines up you are — and a proportional thumb down its right border:
where in the buffer that is, and how much of the buffer there is. Both clear
when you return to the live edge. The thumb rides the border rather than a
content column, so a program's column count never changes because someone
scrolled.

**Which command you are reading.** Scroll far enough back and the prompt that
started the output on screen is off the top of the window. Beside the `⇡N`,
the border names it: `╶ cargo build`, in the same colour and with the same
tick the left border marks a command's first row with. Other terminals answer
this by pinning a sticky prompt line to the top of the viewport — a row of the
program's grid, which is not crew's to spend — and by asking the shell to
emit OSC 133. Crew already knows where every command's output begins and ends
(see `/out`), so it is a lookup, not a feature the shell has to opt into.
Long names lose their tail, never their head (`cargo test --workspa…`), and
the badge never reaches into the pane's legend: at a width where it would, it
simply is not drawn. It clears at the live bottom, where the prompt is on
screen saying this itself, and it follows `/marks` like the other border
markings.

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
  provider stack `/smith`'s inbuilt agents use (DashScope → OpenRouter →
  Anthropic, mock under `CREW_BROKER_MOCK_REPLY`) on a worker thread, and the
  suggested command lands **back in the input bar** — ready to edit or Enter —
  with a status flash. If you've typed something new meanwhile it never
  clobbers you (the suggestion flashes on the status line instead). Fenced or
  backticked replies are distilled to the bare command; no provider key ⇒ a
  status hint, never a hang (30s deadline).
- **`??<question>`** — ask the AI **about the focused pane**: the newest ~120
  lines (8 KB cap) of the focused terminal's scrollback go to the provider
  with your question (bare `??` asks it to explain what happened, focusing on
  errors), and the markdown answer opens in the **zoomed `/view` file
  viewer** — headings, code fences and all. Warp's "ask AI about this error",
  as a two-keystroke prefix. Non-terminal focus or an empty pane gets a
  status hint; the same one-ask-at-a-time and worker-thread rules as `?` apply.
- **Coloured input** — the bar paints what you type, so it says what it makes
  of it before you press Enter: a leading slash command reads **accent** when it
  resolves, muted while it is still being typed, and in the **alarm colour when
  nothing begins with it** (half of a real command is muted on its
  way there; a word no command starts with is marked as one that will not run). Flags recede, and a quoted run is marked from its opening
  quote to its closing one — an unterminated quote marks to the end of the
  line, which is how you see that it is unterminated. Three marks and no more:
  the bar is one row, and a syntax highlighter's worth of colour on twelve
  characters is decoration rather than information.
- **Slash commands** — type `/` for a command palette (↑/↓ to pick, Tab/→ to
  fill, Enter to run): `/smith`, `/goal <text>`, `/batch <file>`, `/view <file>`,
  `/md <file>`, `/diff`, `/settings`, `/find <text>`, `/findall <text>`, `/name <text>`, `/clear`, `/clearall`,
  `/clearlog`, `/only`, `/closeall`, `/pwd`, `/about`, `/log`, `/copy`, `/dump`,
  `/font`, `/theme`, `/notify`, `/update`, `/broadcast`, `/zoom`,
  `/sidebar`, `/keys`, `/far`, `/todo`, `/exit`. The palette is **fuzzy** — prefix
  matches rank first,
  then subsequence matches (typing `dmp` after the slash finds `/dump`) — and **scrolls** to the
  selection when the match list is long. The **characters that matched are
  marked** in each row, so a fuzzy hit explains itself rather than looking like
  a mis-match; the **descriptions line up in a column** so the list reads down
  instead of being scanned; and a command that also has a **chord** shows it
  right-aligned (`/clear … Cmd+K`) — the way to stop needing the palette for it.
  As the card narrows the chord goes first, then the description truncates,
  then the label: the description is what a row is for. When several commands share a prefix,
  the **shortest** is ghosted as the autosuggestion (e.g. `/clear` ghosts before
  `/clearlog`, which is one keystroke further). Commands with a **fixed set of
  values** (like `/theme`) expand into a **value picker**: select the command
  (or type its trailing space) and the palette lists the choices to arrow through
  and `Enter` — no need to remember or type the exact value. Values that *are*
  colours show them: **`/gradient` draws each named pair as a four-cell ramp**
  between its poles, and **the colour pickers put their choice on while you look at it** — arrow onto a
  named palette and the whole window wears it, arrow off and the one you had
  comes straight back, including when you dismiss the picker without choosing.
  A strip of swatches tells you what a palette *is*; only wearing it tells you
  what the screen you are looking at will *look like*, which is the question
  you are actually asking. A rotation mode names a *pool*, not a palette, so
  those rows leave the theme alone: previewing "one of these four" by picking
  one would show something the choice does not promise. The preview is only a
  preview — no config write, no accent re-resolution, no scheme push to the
  programs in the panes; those are what *choosing* does. **`/gradient` rides
  the same rule**: arrowing onto a named pair puts its poles on the canvas,
  because a four-cell ramp beside a name is not the light that pair casts under
  everything you are looking at. Its *level* rows (`off`, `subtle`, `lively`)
  are not previewed — they say how far the poles breathe, not which colours
  they are. Walking from a palette row into a gradient row and dismissing the
  picker restores the pair you had, not whichever one you looked at last.

  Beside the name, **`/theme` also draws the colours themselves**: a rotation
  mode shows one chip per palette in its pool, and a **named palette shows its
  whole hand** — the ink it writes in, its accent, and the four ANSI slots
  every program in a pane is about to paint with (red and green carry meaning;
  yellow and blue are where two palettes with the same page most visibly
  disagree). Every chip is that palette's page with one of its colours across
  the top half, since a dark pool's pages are all nearly black and a column of
  them would be one smudge. One chip is enough to pick a *pool* out of four
  and far too little to pick a *palette* out of twelve — and no two of the
  twelve draw the same strip, which a test holds them to. Reading a colour's name and pressing Enter to find out what it looks
  like is the one thing a picker exists to prevent. Pickers also **mark the
  value you are already on** (· current), and `/gradient` groups its named
  pairs under a heading of their own so the levels and the colours do not
  read as one list. (`/shell` and
  `/run <cmd>` still dispatch if typed, but bare text and `!` replaced their
  palette rows.)
- **`/broadcast`, `/zoom`, `/sidebar`** — palette-discoverable toggles that mirror
  the `Cmd+S` / `Cmd+Z` / `Cmd+G` chords, for when the chord slips your mind.
- **`/font <n>`** — sets the font size to an exact value (clamped 12–32), unlike
  the `Cmd+=`/`Cmd+-` chords that step by one; no argument reports the current size
  (and rotation state, if on). **`/font random`** toggles a 10-minute rotation
  through the installed monospace families (same clock as the theme rotation) —
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
- **`/update`** — downloads the latest release binary over the running one and
  **restarts Crew into it** (a fresh detached process; the old process exits
  after a brief "restarting…" beat). Progress streams into the left-nav UPDATE
  card. The quiet background check installs updates without restarting — a
  blinking nav legend (`crew vA → vB · /update`) says one is waiting, and the
  next `/update` applies it instantly. The old `/restart` command merged into `/update`.
  The fresh process re-reads `config.toml`, so external config edits ride
  along too.
- **`/theme [name]`** — switches the theme live and persists it. There are
  four themes — **`dark`**, **`light`**, **`crt`**, and **`auto`** — and each
  one *rotates* through a pool of palettes every 10 minutes (dark paper
  palettes, light paper palettes, CRT phosphor palettes; `auto` borrows the
  dark or light paper pool to **follow the OS appearance**, flipping live when
  the system switches modes). A fresh install with no saved theme follows the
  OS out of the box; picking `dark`, `light`, or `crt` opts out of following,
  and picking `auto` opts back in. (If the platform never reports an
  appearance, `auto` assumes dark.) Two `config.toml` keys re-pair what `auto`
  serves per appearance: `theme_dark` and `theme_light` each name a pool
  (`dark` | `light` | `crt`) or a pinned palette — `theme_dark = "crt"` makes
  night phosphor while day stays light paper. No argument reports the current
  selection.
  Selecting `/theme` in the palette opens an arrow-selectable **picker** of
  the four themes, so you don't have to type the name. `Ctrl+Shift+L` cycles
  `dark → light → crt → auto`. The old names — the individual palettes
  (`paper-dark`, `crt-green`, …) and the pre-consolidation rotation modes
  (`random-dark`/`random`, `random-light`) — still resolve for back-compat
  but aren't listed. See [Themes](#themes).
- **`/crt [on|off|auto]`** — the CRT tube post-process (screen curvature,
  scanlines, phosphor glow, a slight flicker while a pane is busy), independent
  of which theme is active. `auto` follows the theme — on for `crt`, off
  otherwise — and is the default; `on`/`off` pin it either way, so you can run a
  paper theme through the tube or a CRT palette flat. No argument reports the
  current setting.
- **`/weight [medium|semibold|bold|…]`** — the weight the text is rendered at,
  live and persisted. Useful when a font renders thin at your size or on a
  low-contrast display. No argument reports the current weight; selecting
  `/weight` in the palette opens a value picker.
- **`/smooth [off|light|medium|heavy|<0-255>]`** — CoreText-style font
  smoothing: a fractional stem darkening applied to every glyph, emulating what
  macOS terminals get natively so the same font reads as full here as in
  Terminal.app or Ghostty (crew also renders unhinted, like CoreText, at every
  strength — including `off`). Live and persisted; no argument reports the
  current strength; selecting `/smooth` in the palette opens a value picker.
  The named ladder is also the **Smoothing** field in `/settings` — both write
  the same `font_smooth` key. The default came down from 100 to 70 in 0.19.28:
  the darkening had been calibrated by eye while the encoded blend was still
  eating a quarter of the glyph's light, so it was making up part of that
  deficit as well as doing its own job. `/gamma` corrects the blend honestly
  now, and the two at their old values delivered more light than the outline
  asks for. A config still carrying the old default is moved once on upgrade; a
  strength you chose is left alone. The darkening *accumulates* coverage rather than
  taking the brighter of a pixel and its neighbour's spill, so the letters
  built from curves take the same widening as the ones built from stems — a
  saturating dilation cannot darken a pixel whose own coverage already beats
  what its neighbour lends it, which is every pixel on the flank of an `o`, an
  `s` or a `/`, and those letters used to read a shade lighter than an `l` in
  the same word. A strength still means what it always meant: the spill is
  calibrated so the ink a given number lays down is unchanged, and only where
  it lands moved.
- **`/gamma [off|light|medium|full|<0-255>]`** — the other half of the CoreText
  look: the coverage curve. Crew picks a non-sRGB surface on purpose, so text
  blends on gamma-encoded values — the web and CoreText look — and that costs a
  half-covered edge pixel most of the light it should emit. Measured over the
  embedded font at body size, white text on a dark page delivers about **60% of
  its correct linear luminance**, and reads thin for it; dark text on a bright
  page has the same error with the sign flipped and reads blotted. `/gamma`
  bends the mask back, by polarity: up on a dark page, down on a bright one.
  `full` is the whole sRGB correction — the coverage a glyph asks for is
  exactly the light it gets. `medium` (the default) is about half of it, which
  puts the midtone at Apple's historical text gamma. Both curves fix 0 and 1,
  so a glyph's empty pixels and its solid interior never move and only the
  antialiased rim — most of a small glyph — is touched. Live and persisted; no
  argument reports the current amount. The named ladder is also the **Text
  gamma** field in `/settings`, beside Smoothing, and both write `font_gamma`.
  The polarity is read **per run, from each run's own two colours** — not from
  the theme. Crew draws dark text on bright badges inside dark themes, bright
  text on dark chips inside light ones, and the cursor inverts both at once; a
  theme-wide answer gets every one of those backwards, which is worse than not
  correcting at all, because the curve then doubles the error it was there to
  cancel. Runs split on polarity, so a character that appears in both is two
  atlas entries rather than one bitmap bent whichever way it happened to be
  shaped first.
The palette **remembers what you run.** Among commands that match what you have
typed equally well, the ones you actually use come first; the rest fall back to
the order they are declared in, which means something to whoever last edited
the table and nothing to the person typing. Type `/` on an empty bar and your
ten most recent commands lead the list. It is persisted, since a shortcut that
resets every launch is not one.

Recency reorders **within** a match-quality band and never across one: a prefix
match still beats a fuzzy match, always: typing **de** after the slash can
never float a command that does not begin with those letters above one that
does. A learned list that can reorder the *kind* of match is a list you can no
longer aim at.

- **`/shapes [auto|off|on]`** — say it with a shape as well as a colour. WCAG
  1.4.1 is one line long and easy to fail without noticing: colour must never
  be the **only** thing carrying a piece of information. About one man in
  twelve cannot separate red from green, every colour cue vanishes on a
  monochrome CRT theme, and none of them survive a screenshot pasted into a
  ticket in greyscale.

  Crew mostly passed already, by accident of taste rather than by rule —
  attention markers are distinct glyphs (`!`, `⚑`, `✓`, `⊗`, `?`) that happen
  to share the bell colour, broadcast is `»` and not just magenta, every toast
  names itself in its legend, and a busy sidebar row spins. Two places did
  not, and this turns them on: the **load gauges** mark their band (`!` past
  70%, `‼` past 90%, riding the label's trailing space so no column is spent),
  and a **working pane in the minimized strip** draws a half-filled `◐` rather
  than the same solid `●` as a pane that merely spoke recently, which was told
  apart by a brightness pulse — colour and motion, the two channels this is
  about.

  `auto` follows macOS's *Accessibility → Display → Differentiate without
  color*. It is off unless asked, because a glyph in every gauge row is noise
  for a reader who can see the colour: the rule is *never colour alone* for
  anyone who needs it, not *always both* for everyone. Live and persisted (the
  `shape_cues` key; **Settings → APPEARANCE → Shape cues**).

- **`/contrast [auto|normal|high]`** — the WCAG floor every derived colour is
  held to. Crew derives its readable roles rather than hard-coding them: the
  terminal cursor, links, selection, the warning amber, the sparkline are each
  walked in Oklch until they clear a measured contrast floor against the page
  they land on. This says which floor.

  `auto` is the default and **follows the OS**: macOS's *Settings →
  Accessibility → Display → Increase contrast* is where a user has already
  said this once. `normal` is the WCAG **AA** band (4.5 for text and
  text-like marks, 3.0 for marks you only have to see); `high` is **AAA**
  (7.0 and 4.5) — the standard's own next step, which is what "increase
  contrast" means in the only vocabulary that has one.

  High contrast also quiets the two effects that *spend* contrast: the
  spotlight over unfocused panes, and the page's gradient wash, which lifts
  the background the ink sits on and has only 4–16% headroom over it. Both
  drop to a third of their strength rather than to zero — the spotlight is the
  cue that says which pane has focus, and losing that is itself an
  accessibility loss. Live and persisted (the same `contrast` key as
  **Settings → APPEARANCE → Contrast**); no argument reports the setting and
  the band it resolved to.

- **`/focus`** (`Ctrl+Shift+F`) — **focus mode**: crew stops interrupting.
  While it is on, notifications are **held rather than dropped** (they still
  write the LOG, still flash the input bar, still raise the pane's own
  attention marker — they just do not step onto the canvas as cards); a pane
  blocked on a human is still badged but **never pulls focus**, since being
  yanked into another pane mid-sentence is the most expensive interruption
  crew can produce; and the spotlight over unfocused panes deepens from 15% to
  42%, so the pane you chose is unmistakable while its neighbours stay
  readable. The input bar's legend reads `◉ focus` for as long as it lasts.

  Leaving reports what happened in one card — `3 notifications held while
  focused` — so the mode costs you awareness only until you come out of it.
  That is the difference from `/notify off`, which drops the events: afterwards
  you cannot tell what you missed.

- **`/density [compact|cozy|roomy]`** — how tightly crew packs the canvas: the
  gutter between pane cards and the blank rows between chat cards, moved
  together. In a cell grid those are the two spaces that are genuinely empty —
  the line height *is* the cell, and shrinking it is the font size, which
  `/font` and `Cmd+±` already own. `cozy` is exactly the layout crew has always
  drawn, so the setting changes nothing until you turn it; `compact` halves the
  gutter and drops the chat spacer (each card still opens with its sender's
  coloured gutter glyph, so the boundary is drawn in ink rather than in space);
  `roomy` opens both up, which is the one to reach for on a large display.
  Live and persisted — the same `density` key as **Settings → APPEARANCE →
  Density**.

- **`/leading [tight|normal|relaxed|loose]`** — how much air sits between rows
  of text. This is the knob `/density` deliberately does *not* have: density
  moves the spaces that are genuinely empty (gutters, blank rows), on the
  grounds that in a cell grid the line height *is* the cell. That reasoning
  holds for gutters and not for the reader who wants the same glyphs with more
  room between them, and whose only lever until now was the font size — which
  fixes the tracking by making everything bigger. Density is how much crew
  fits on the canvas; leading is how the text reads.

  Only the cell's **height** takes the ratio. Widening it would space the
  letters of every word apart, which is a different typographic decision
  wearing the same name, and would break the monospace contract every program
  in a pane draws against. `normal` is `1.25 × font_size`, exactly what crew
  has always drawn, so the setting changes nothing until you turn it; `tight`
  is `1.10` (stopping short of solid, where a monospace face's descenders meet
  the ascenders below); `relaxed` `1.45`; `loose` `1.65` (stopping there
  because the cell is also the cursor and the selection band, and past it a
  highlighted row reads as a stripe with the text loose inside it). The cell
  changes, so the grid is remeasured and every pane's program is told its new
  size. Live and persisted — the same `leading` key as **Settings →
  APPEARANCE → Line spacing**.

- **`/motion [auto|off|subtle|full]`** — how much crew moves. **`auto` is the
  default and follows the operating system**: macOS's *Settings → Accessibility
  → Display → Reduce motion* is where a user has almost certainly already said
  they want less of this, and an app that ignores that switch makes them hunt
  for a private setting they set once. With the switch off, `auto` is full
  motion; with it on, crew's `off` is a genuine off — every animation window
  collapses to zero, the final state draws once, and nothing reschedules a
  frame. An explicit level overrules the OS in **both** directions: `/motion
  full` keeps crew moving under Reduce Motion, and `/motion off` stays off
  without it. Live and persisted (the same `motion` key as **Settings →
  APPEARANCE → Motion**, whose picker shows `auto (off)` / `auto (full)` so
  the deferral still tells you what it decided). No argument reports the
  preference, what it resolved to, and whether the OS is asking.

- **`/gradient [off|subtle|lively|<name>|<#a> <#b>|reset]`** — the canvas's
  colour. With a level it sets how far the gradient breathes (the same
  `gradient` key as **Settings → APPEARANCE → Gradient colour**). With a
  **name** or two hex colours it replaces the theme's poles with a pair of
  your own — the wash, the dot lattice, every card's stroke and the footer
  meters all run between them.

  Eight named pairs come with crew: **`aurora`** (teal→violet), **`tide`**
  (cyan→blue), **`orchid`** (violet→rose), **`moss`** (green→teal),
  **`ember`** (amber→red), **`sand`** (sand→clay), **`dusk`**
  (indigo→magenta) and **`mono`** (no colour at all). Selecting `/gradient`
  in the palette lists them with the ladder, so picking one is arrowing
  through a list rather than inventing a hex code — and anything off the
  shelf still works: `/gradient #7aa2f7 #bb9af7`. `reset` gives the theme its
  own gradient back; no argument reports what is in force.

  A pair chosen by name is stored **by name**, so a preset re-tuned in a
  later release reaches everyone who picked it. **`Ctrl+Shift+G`** steps
  through the shelf without opening anything — the colour's answer to
  `Ctrl+Shift+L`, and the walk passes back through the theme's own gradient
  once a lap, so the key that got you somewhere can get you home.

  **Only the hue is yours.** A custom pair is re-lit to the active theme's
  own pole lightness, at draw time, so it keeps tracking the ten-minute
  palette rotation and so `#ffffff` cannot bleach the page. The wash lies
  under your text with 4-16% contrast headroom over the page it lifts — that
  is not headroom a colour picker gets to spend. You choose the colour; crew
  chooses how bright it is.
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
  `!opencode` (distinct from `/smith`, which opens the multi-agent broker relay
  pane). Run panes execute under **bash job control** (`set -m`, then `exec`
  back into your shell), so Crew can tell "a command is running" from "a
  prompt is waiting" — that signal is what makes bare input divert away from
  a busy pane instead of typing into a running program.
- **`/view <file>`** (alias `/md`) — opens a zoomed **file viewer** pane: a
  single, read-only pane over the file, rendered by format (markdown,
  numbered-gutter code, aligned CSV, colored diffs, or a metadata card for
  anything else). ↑/↓ and PageUp/PageDown/Home/End scroll, `r` reloads from
  disk — a **wrapped row says it is one**, with a `↪` in the gutter where its
  line number would be, since a blank gutter beside a wrapped line and a blank
  gutter beside an empty line are the same blank — `s` toggles raw text
  (markdown and CSV), `/` searches (`n`/`N` for
  next/previous hit — the needle is drawn on the pane's **last row** as you
  type it, with a caret while typing and the tally once confirmed, or "no
  matches" in the alarm colour; the hits are marked down the card's **gutter**
  in the search's own colour, over the landmark ticks and under the thumb),
  **`]`/`[` step the document's structure** — in a diff,
  file to file and hunk to hunk; in markdown, heading to heading — **`v`
  lays a diff out side by side** (see `/diff` below), `e`
  opens `$EDITOR`, `o` hands the file to the OS default app, **Cmd+click**
  opens a rendered markdown link, `Esc` closes. At either end `]`/`[` do
  nothing rather than wrapping: a document has an end, and jumping back to the
  top from it is how you lose your place. The card's **right border carries the
  position**: a proportional thumb (drawn from the top of the file, not only
  once you have scrolled — a document's gutter answers *where am I*, where a
  shell's only says how much is behind you) with the landmarks marked as dim
  ticks beside it, so a long file shows its shape before you move.
  Chat panes render markdown too — see [Markdown](#markdown).
- **The pointer knows a link when it is over one.** URLs and file references
  are drawn as links — tinted, and ruled underneath, so they read as clickable
  without depending on hue — but the pointer over one wore the same I-beam as
  the prose beside it, on text that is one modifier away from opening a browser
  or a viewer. Over a marked run the pointer is now a **hand** and the run goes
  **bold**: bold rather than a colour change, because the run is already
  carrying the link colour to say *what* it is, and a hover changing that hue
  would be saying the second thing in the same channel as the first. It is the
  same hand a border button gets — both are "this does something when you press
  it", and a third shape for the distinction is one nobody has learned.

  What counts as a link is exactly what is *drawn* as one, which is
  deliberately less than Cmd+click will open: answering "any token that names a
  file on disk" means a filesystem check, and this runs on every pointer move.
  The hover promises what the drawing promised; the click is free to find more.
  Row text is reconstructed for the one row under the pointer, at pointer-move
  time — never pre-scanned during layout.

- **Tabs are expanded, always.** A tab has zero display width, and the guard
  every cell surface in crew places glyphs through skips zero-width characters
  — so a tab-indented file opened in the viewer drew with its indentation not
  merely misaligned but *missing*, on every line of every Go file, Makefile and
  kernel-style C source. Tabs now advance to the next multiple of **8**
  columns: the terminal's own tab stop, so `cat file` in a pane and `/view
  file` beside it agree about how far in a line starts. The expansion happens
  to the text before any rung sees it, so the syntax colouring, the wrapping,
  the search and the diff pairing all agree about which column a character is
  in.
- **`/invisibles [on|off]`** reveals the characters that say something without
  printing anything: a tab wears an arrow in its first column, trailing spaces
  show as middle dots, and the carriage return a CRLF file leaves at the end of
  every line shows its own mark — the three that cause real trouble (a tab
  where spaces were meant, whitespace nobody can see, and a line ending that
  makes a shell script fail with a message about a command that does not
  exist). They are drawn in the muted ink, and they are *marked* rather than
  merely substituted, so a `·` that is genuinely in the file is not dimmed
  along with them. Off by default — this is a diagnostic view, and its marks
  are noise in a file that has nothing wrong with it. Also a checkbox in
  `/settings` (**Reveal invisibles**). Note that the expansion above is not
  part of this switch: that is the difference between drawing a file's
  indentation and dropping it.

- **Commands that take a path list one.** Type `/view `, `/md `, `/dump ` or
  `/batch ` and the bar opens a **file picker** over the directory the partial
  names — folders first, then files by name — filtered as you type and
  case-insensitively. Picking a folder fills `<cmd> dir/` and leaves the bar
  open, so the next listing is what is inside it: the same key walks into a
  tree and picks out of it. Hidden entries appear only once the partial says
  so (a leading `.`), the rule every shell's completion follows. Tab still
  ghosts the first match inline.

  This is one `read_dir` of one directory per keystroke, never a walk: it runs
  on the thread every pane is drawn from, and a stall there freezes the whole
  grid. `/view`, `/md` and `/batch` had shipped as commands you type a path
  into with **no completion at all** — the ghost knew only about `/dump` — so
  the palette's descriptions and the completion list are now held against each
  other by a test.

- **`/blame`** — asks who last touched each line of the file the viewer has
  open, and draws it in a column beside the line numbers. Runs are collapsed:
  a line is labelled only when its commit differs from the line above, so the
  column reads as *boundaries* — this block arrived here, that block arrived
  there — rather than as the same sha repeated forty times. `git blame` walks
  a file's whole history, so the read runs on a worker thread (a blocked winit
  thread freezes every pane in the grid, agents included); a failure says why
  on the status line, because a gutter that never appears looks exactly like
  one still loading.

  It is a toggle, not a mode — `/blame` again puts the column away. The column
  is `sha author` where the pane can afford it, the sha alone where it cannot,
  and nothing at all below that; it is never more than a third of the pane,
  since a blame column that crowds out the code it annotates has answered the
  wrong question. Reloading the file (`r`, or the `$EDITOR` handoff) drops the
  blame: it is a per-line answer about text that just changed.
- **`/notify [on|off|add <text>|clear]`** — drive the notification block from
  the bar: toggle the master switch, add a watched output pattern, or clear
  the patterns (the full set of knobs lives in `/settings`).
- **`/diff`** — reviews the working tree's git changes **in the file viewer**:
  a `git status --short` summary, the `diff --stat`, then the full unified
  diff, rendered by crew's own diff rung rather than dumped as `git`'s colours
  into a scrollback. That means each removed line is **paired with the added
  line that replaced it** and only the run that actually differs is drawn at
  full strength — the text the two share recedes toward the page — so you read
  *what* changed instead of hunting for it inside two lines of near-identical
  code. Word edges are respected (`foo_bar` → `foo_baz` marks the whole
  identifier, not the letter); **trailing whitespace on an added line** is
  shown as middle dots in the alarm colour — the review nit every diff tool
  marks, because it is invisible by construction and nobody meant to add it;
  runs that do not correspond line-for-line are
  left unmarked, because a guess drawn as a mark is a lie about what changed;
  and a pair that differs almost everywhere is left plain, since marking all
  of both is not a mark. Hunk headings are set apart from the function
  context after them.

  The repo reviewed is the **focused pane's** directory, so `/diff` in a pane
  working in another checkout reviews that checkout. The three `git` reads run
  **off the winit thread** (a big repo takes seconds, and anything blocking
  that thread freezes every pane, agents included); the pane opens the moment
  they land, and the viewer's scrolling, search and `r`-reload all apply. A
  clean tree says so instead of opening an empty pane. Pairs with the crew
  pane's automatic checkpoints (`/restore` lists them) for reviewing what
  agents changed.

  **`v` splits the review into two columns**: what was there on the left, what
  is there now on the right, on the same row. A unified diff is a compression —
  the two versions of a file interleaved into one column, which is what makes
  it fit in an email and what makes it hard to read — and the one thing crew's
  pairing cannot recover from it is *position*: a removed line and its
  replacement occupy the same place in the file, and stacking them says they
  happen one after another. Everything the unified rung knows comes with the
  split, because both read the same paint: the pairing, the word-level
  refinement, the hunk headings. Each side also carries **its own file's line
  numbers**, tracked from the hunk header — a unified gutter can only count
  rows of the diff, and the number you quote to someone is the one in the file.

  A pair wraps *both* sides at the half width and pads the shorter to the
  taller, so the two versions never slide out of step exactly where the lines
  are long enough to need the help; a side with no line is blank rather than a
  copy of its partner. Below about 61 columns there is no honest split and the
  unified rung takes it back — the toggle is a request, not a promise the width
  can always keep. It is per pane rather than a setting: it is a way of reading
  *this* review at *this* width.
- Panes crew opens on generated files — `/out`, `/diff`, `/about` — are
  **named after what they are** (`out · cargo build`, `diff · crew`, `what's
  new · 0.18.75`) rather than after the temp file the text happens to live in.
- **`/marks [on|off]`** — the things a pane's card draws on its border about
  the pane's own output: the ticks where each command began, the name of the
  command you are scrolled back into, the bars beside error lines, and — for a
  shell that says so — the block that **exited non-zero**. On by default — a grid of panes saying where the failures
  are without being read is most of their value — but they are crew drawing on
  its own chrome about someone else's output, and a plain frame is a reasonable
  thing to want. `/errors` and `/out` still work with the marks off; they read
  the same thing, they just do not draw it. Also a checkbox in `/settings`
  (**Card border marks**).
- **Exit status, when the shell says so (OSC 133).** Crew works out where each
  command's output begins and ends by watching the pane's foreground process —
  no shell configuration, no integration to install — but that is the one thing
  polling cannot see all of: a process crew never saw start tells it nothing
  about how the command *ended*, and no amount of watching recovers an exit
  code. A shell with an OSC 133 integration reports it directly (`ESC ] 133 ; D
  ; 1 ST`), and when it does, crew marks that block's first row on the left
  border in the alarm colour and names the status beside the command while you
  are scrolled inside it (`╶ cargo build ✗101`). A prompt mark (`A`) also
  closes the block exactly, a full poll before the process watch would notice.

  It is an upgrade, not a requirement: a shell that says nothing keeps exactly
  the blocks it had before, and "said nothing" is deliberately not drawn as
  success — it is not the same claim.

- **`/pin`** — keeps the focused pane **on the grid**: pinned panes are exempt
  from the LRU demotion that sends the least-recently-active pane to the
  minimized strip. The LRU is right about which pane you have not touched and
  wrong about whether that matters — the pane you are least likely to touch is
  often the agent you most want to keep watching. A pinned card marks itself on
  its top border; running `/pin` again gives the pane back to the LRU. More
  pins than tiles is not an error and cannot make room that does not exist: the
  oldest pins keep their tiles and the rest demote like anything else.
- **`/blocks`** — what you ran in this pane, newest first: how long each took,
  which of them failed, and the number that reaches its output. A pane's
  scrollback is one long column in which everything that ever ran is mixed
  together, and the question people actually ask of it — *what did I run in
  here, and which of them went wrong* — has to be answered by reading it. Crew
  already knows, so this is a listing rather than a search.

  Each row is numbered the way `/out`'s argument is, which is the point of
  pairing them: `/blocks` says what you ran and `/out 2` opens the output of
  the third one back. A block still running says so and counts up; one whose
  shell reported no exit status shows `·` rather than a tick, because crew only
  knows how a command ended when the shell says so and drawing "no answer" as
  success would be inventing one. It opens in the file viewer like `/out` and
  `/diff` — writing a summary of a pane's history *into* that history is how a
  listing becomes one more thing to scroll past.

- **`/out [n]`** — opens the focused pane's **last command's output on its own**,
  in the file viewer. A long build's output is buried the moment the prompt
  comes back: mixed in with what you ran before it and whatever the shell
  printed after. Crew knows where that output started and ended without any
  shell integration — it already watches each pane's foreground process, and
  the two transitions it sees (idle to running, running back to idle) are the
  two edges of the output — so `/out` slices exactly those lines into a pane
  you can scroll, search, and walk with `]`/`[` while the terminal carries on
  underneath. A command still running reports what it has printed so far.
  An argument counts back — `/out 1` is the run before the last one, `/out 3`
  the one before the three you have tried since — through the few dozen each
  pane remembers; a command that printed nothing is skipped rather than
  counted, so the numbers mean what they look like. Asked for one that is not
  there, `/out` lists what is (`0:cargo · 1:ls · 2:git`).

  The granularity is one second, and honestly so: a command that starts and
  finishes between two polls leaves no span, and one still flushing output when
  the prompt returns can carry a line or two of the next thing.

  The same spans **tick the card's left border** where each command began, so a
  screenful of scrollback shows where one thing you ran ends and the next
  starts — the block structure other terminals need shell integration for,
  drawn as chrome rather than in the program's own columns. An error bar on the
  same row wins it: "this failed" outranks "this began".
- **`/closeall`** and **`/only`** **ask once**: the first run says what it
  would close and the same command again does it. A closed pane takes its
  scrollback, its running command and its agent with it, and both commands sit
  one fuzzy keystroke from `/clear` in the palette. A different command in
  between replaces the question rather than answering it, and a question older
  than ten seconds is asked again.
- **`/errorsall`** — the fleet-wide version: counts the errors in **every**
  terminal pane's scrollback (bounded and paged, like `/findall`), reports
  which panes have them and how many — `4 errors in 2 panes: →#3 (3) #5 (1)` —
  and lands on the first, walked to its most recent one. With six agents
  running, "which of these went wrong" is the question you have before you go
  looking in any of them.
- **`/errors`** — scrolls the focused terminal back to the most recent line
  that reads as an **error**, and reports how many are in view. A long build
  scrolls its own failure off the screen, and finding it again otherwise means
  remembering a word from it or paging up through the noise. Repeating `/errors`
  steps to the one before, the way a repeated `/find` does — a failing build has
  more than one, and the one you want is rarely the last.

  What counts as an error is deliberately narrow: it has to *announce itself* at
  the start of a line (after any indent, quote bar or box edge a TUI drew) or
  right after a `file:line:col` prefix — `error[E0433]`, `error:`, `fatal:`,
  `panicked at`, `npm ERR!`, `Traceback (most recent call last):`, `FAILED`,
  `not ok`, `✗`, and `: error TS…`. Prose that merely mentions errors is not
  one: a jump that lands on "errors are handled below" teaches you not to trust
  the jump.

  The same reading marks the card: every visible line that is an error puts a
  **red bar on the pane's left border**, at its row. The border rather than the
  content — a terminal's columns belong to the program running in it, and a
  marker in column zero would overwrite the first character of the message it
  is pointing at — so a failing build shows *where* its failures are from
  across the grid, with nothing typed.
- **Cmd+C** copies **what the focused pane shows**, whatever kind it is — a
  terminal screen, a rendered diff, a transcript, a todo list. It used to
  answer only in terminal panes and do nothing at all, silently, everywhere
  else. A mouse selection still wins over the whole screen, and a pane with
  nothing on it says so.
- **`/copy out`** — copies just the **last command's output**, the same slice
  `/out` opens. What you actually want when you are about to paste a failure
  into an issue: the run that failed, without the four before it.
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
- **`/usage`** — opens the **usage** pane: what crew has spent over the last
  seven days, drawn rather than totalled. A **heatmap** of tokens by hour (a
  row per day, a column per hour, shaded against the week's own peak, so a
  quiet week reads as clearly as a busy one), a **donut** splitting the tokens
  sent from the tokens received, and an **area chart** of what each day cost —
  with the week's spend, token split and peak day named above them. The pane
  follows the ledger while it is open, so a request landing now moves the
  charts. Everything it shows comes from `usage.jsonl` beside your config,
  which crew was already keeping for the chat footer's rolling 5h/7d windows.
- **`/todo`** — opens a **todo list** pane over one global list (stored in
  `$XDG_CONFIG/crew/todos.toml`, shared by every window and pane). Type into
  its composer and press `Enter` to add an item; a **natural-language due
  date** anywhere in the text (`tomorrow`, `fri 5pm`, `aug 15`, `in 2 weeks`,
  `17:30`) is tinted live as you type — the composer's legend previews the
  parse (`due fri 17:00`) — and is stripped from the title on save, and an
  **`@project`** token becomes a free-form tag (a popup completes tags
  already in use; a new word after `@` creates one). The list sorts overdue
  (bell-coloured) → upcoming by due → undated, with done items sunk and
  dimmed. The composer has a real cursor: `←`/`→` move by char,
  `Alt+Left` / `Alt+Right` hop words, `Ctrl+A` / `Ctrl+E` (or bare
  `Home`/`End`) jump to the draft's ends, typing/paste insert at the
  cursor and forward-Delete deletes at it; on a wrapped multi-row draft
  `↑`/`↓` travel the lines first and only the edges hand off to the
  list. `↑`/`↓`/`Tab` move between composer and rows —
  `PageUp`/`PageDown` hop a whole visible page of items and `Home`/`End`
  jump to the first/last (all filter-aware); on a row
  `Space`/`Enter` toggle done, `d`/`Backspace` delete, `e` re-opens the item
  in the composer for editing, `+`/`-` postpone/advance its due a calendar
  day (`+` on an undated item starts it at tomorrow); the mouse works too — click the `[ ]`
  checkbox to toggle, the `✗` at the row's end to delete. Done items
  auto-hide; `h` on the list shows them again — sunk, dimmed, `[x]`,
  newest completion first — so `Space` can un-do one (`h` again hides). A lone `@tag` +
  `Enter` filters the list to that project (`@` alone clears the filter),
  and `]`/`[` on the list cycle the filter through the known tags — no
  typing, "no filter" is one stop on the ring.
  When an item's due time passes while crew runs, a one-time **`due` toast**
  fires (persisted, so restarts don't re-toast). `Esc` walks back one layer
  at a time — popup, draft, then the pane. Restored by `/restore`.
- **`/smith`** — opens **agent smith**, a **multi-agent pane** where the
  installed CLI coding agents (claude, codex, opencode) message each other to
  work a task. See [Multi-agent relay](#multi-agent-relay-smith-alias-crew) below.
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
- **`/reopen`** (**Cmd+Shift+T**) undoes the last close. `Cmd+W` and the `[x]`
  button take a pane in one keystroke and never ask first — that is the point of
  them — so the pane they take is written down instead. A reopened pane is a
  *new* pane in the same place: a shell in the directory that one was standing
  in (its live `cd`-tracked directory, not the one it was spawned in), the
  viewer back on its file, `/smith` or the todo list back on the grid. What
  cannot come back is what died with the PTY — the scrollback, the environment,
  whatever was running. Crew remembers the last 8 closes, so undoing a `/only`
  or a `/closeall` walks the whole grid back one pane at a time; the status line
  says how many are left. Pane kinds with no honest restore (settings, a swarm
  view) are skipped rather than half-reopened.
- **`/restore`** reopens the last session's panes: every quit path (Cmd+Q,
  window close, `/exit`) snapshots each restorable pane to `session.toml`
  beside the config — terminal panes save the shell's **live** working
  directory (asked from the OS, so it follows your `cd`s; hidden panes
  included), Far panes their active panel's directory, and the `/smith` chat
  pane its presence — up to 6, deduped, stale paths and unknown kinds
  skipped on load (older `dirs`-only files from v0.5.73–74 still restore).
  Panes minimized into the left nav restore minimized. Restore is deliberately pull-based: launching keeps
  the welcome screen, and the shells come back only when you ask — and asking
  consumes the snapshot (the next quit re-saves from the live panes). Closing
  every restorable pane and quitting clears it; a run that never opened one
  (welcome-screen quit) leaves the saved session untouched. When a
  snapshot exists, the welcome screen says so — `3 panes from last session ·
  /restore` under the keyboard hint — so the feature introduces itself.
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
- **Drag & drop**: a file dragged from the OS file manager lands in the
  **focused** pane — a chat composer gets an `@path ` mention (quoted as
  `@"path" ` when the path has spaces, cwd-relative when it can be), a
  terminal gets the shell-quoted absolute path. Multiple files land
  space-separated in arrival order.

**Pasting something that would run.** A terminal sends a paste as if you had
typed it, newlines included, so a multi-line block runs line by line — the
oldest footgun there is. Crew asks first, but only when the answer matters: a
program that enabled **bracketed paste** (every modern shell, editor and agent
CLI) receives the block wrapped and decides for itself, so nothing runs and
nothing is asked. Without it, a multi-line paste is held with a count —
`12 lines would run here — ⌘V again to paste` — and the second **Cmd+V** sends
it. One trailing newline is not multi-line: copying a line out of a file takes
its terminator with it, and holding that would train you to confirm
everything. A hold older than fifteen seconds is dropped rather than sent,
because a confirmation you have forgotten giving is not a confirmation.

## When a program says something itself

Two escapes let a program speak for itself instead of being guessed at, and
crew honours both.

**A notification it asks for.** `ESC ] 9 ; text ST` (the iTerm2/ConEmu
spelling) and `ESC ] 777 ; notify ; title ; body ST` (the one most Linux
tooling emits) raise a real crew notification — toast, activity log, pane
marker — carrying the program's own words with the pane's name beside them.
Every other notification crew raises is inferred from behaviour; this one was
requested, so it rides the master `notify` switch alone rather than a
per-kind toggle.

**Progress it reports.** `ESC ] 9 ; 4 ; state ; percent ST` draws a bar along
the card's **bottom border** — the border rather than a content row for the
same reason the scroll thumb rides one: a terminal's columns belong to the
program running in it, and a bar that stole a row would resize the grid under
it. State `1` fills proportionally, `2` and `4` (error and warning) fill in the
alarm colour, `3` (working, with no number) sweeps a short block back and forth
rather than parking at an arbitrary percentage, and `0` clears it.

## What arrived while you were away

A grid means most panes are producing output while you read one of the others,
and coming back to one always asks the same question: *where does the new part
start?* Each terminal pane remembers how many lines it held when you last read
it and draws the boundary as a **rule under the last line you had seen** — not
a banner row, which would cover a line of output; the thing being marked is the
gap *between* two lines, and an underline on the row above is exactly that.

The card's border carries the **count** beside its activity dot: the dot has
always said "something happened here", and the number is the difference between
glancing over and going back. It caps at `99+`.

The count appears in all three places a pane is listed: its own card, its
**minimized thumbnail**, and the sidebar's **PANES** row — the one view that
lists panes you cannot see — and never on the pane you are looking at.

The mark follows the tail while you are watching a pane and nothing new is
below what you have seen — so looking away now marks everything up to here as
read. It resets when you **type into** the pane, because answering is reading,
and when you **scroll back to the live bottom**, because arriving there means
you have been past everything above it.

## Scrollback

Scrolling **scales with the speed of the gesture**: ticks that keep arriving
build a multiplier (capped at six, so a flick crosses a long log without
crossing the whole scrollback), and a pause puts it back to one line a tick —
which is what makes the same wheel usable for reading and for travelling. Mouse
wheel or **Shift+PageUp/PageDown** scroll a pane's history (Shift+Home/End
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
**`/findall <text>`** searches **every terminal pane's full scrollback**
(hidden panes included, bounded per pane), focuses the first matching pane
(restoring it if minimized), scrolls it to the most recent match, and reports
the fleet-wide tally — `12 matches for 'error' in 3 panes (#2 #4 #7)` — using
the same pane numbers as Cmd+1..9 and the tile badges (the landed pane is
arrow-marked: `(#2 →#4 #7)`). **Repeating `/findall`** with the same term
**cycles to the next matching pane**, wrapping — n/N stepping at fleet
granularity — while a follow-up `/find <text>` steps upward through the
focused pane's matches as usual.

## What programs paint

A TUI draws with **coloured spaces**: a status line, a progress bar, a selected
row in `fzf`, a diff block, the panel behind a menu. Crew keeps those now. The
flat-canvas rule it has always had is unchanged — the near-grey an agent CLI
paints behind the line you just sent is still flattened to the page, along with
any low-saturation or bright "highlight" background on a dark theme — but a
background that carries meaning survives, and a blank cell that carries one is
drawn.

Selecting empty space is visible for the same reason.

Plain blank cells are still dropped before any colour work happens: a terminal
is mostly empty, and every kept blank is a cell shaped and a quad drawn.

## How long it has been going

A pane's legend names the foreground command; the border now says **how long it
has been running** — `9s`, `2m14`, `1h05` — once it has been going more than
five seconds. Every command is briefly a running command, and a clock that
appears on every `ls` is chrome; a build at nine seconds and a build at nine
minutes look identical without one, and the second is news.

It sits before the git badge, being the more perishable of the two: a branch
does not change while you look away. The minutes and hours forms are
zero-padded so the number does not jitter in width as it counts.

## Git on the card

Every pane's top border carries the git state of **the directory that pane is
in**: `main ●3 ↑2 ↓1` — branch, changed files, commits ahead and behind the
upstream. The sidebar has always shown this for crew's own working directory;
the badge shows it per pane, which is what you want when one pane runs an agent
in one worktree and another runs tests in a second. A clean repo is just its
branch — no tick, no zeroes.

The border is a scarce row, so the badge sheds detail in a fixed order as the
card narrows: behind, then ahead, then the dirty count, then the branch itself
truncates with an ellipsis, and below four columns it draws nothing rather than
a letter and a dot. It never reaches the legend.

Queries run **off the winit thread**, one `git status` at a time across the
whole fleet, and no directory is asked more than once every three seconds —
`git status` takes seconds on a large or network-mounted repo, and running one
inline freezes every pane.

## The cursor

Crew draws the cursor **shape a program asks for** (DECSCUSR): a filled block
(`ESC[2 q`), a **bar** (`ESC[6 q`) and an **underline** (`ESC[4 q`). Editors use
the bar to mean insert mode and the block to mean normal mode, so a terminal
that draws every cursor as a block loses the mode indicator entirely.

A pane that is **not focused** draws an **outline** instead, whatever shape it
is otherwise in — with a canvas full of panes the useful question is which one
takes the keys, and a shape answers that at a glance where a dimmer version of
the same block did not. The outline's colour is floored against the page, since
it is a fraction of the ink a filled block is.

Only the block repaints the cell it lands on (inverting it, so the glyph stays
readable); the bar and the underline are rules drawn beside the glyph, which
keeps its own colours.

## Loud and quiet

**SGR 2 (dim)** is honoured. Agent CLIs put half of what they print in it —
the reasoning, the file lists, the "thinking" line — to say *this is context,
not the answer*; crew rendered every one of those at full strength, so
everything a CLI said was equally loud. A dim cell is now mixed toward the
page in linear light (so its hue survives — a dim red is still red) and then
**floored at a lower contrast than body text**: quieter, because the program
asked for quieter, but never below readable, because half a session's output
arrives that way and a program that guessed the page colour wrong would
otherwise land it on top of the background.

**SGR 8 (conceal)** is honoured too: a program hiding what you type — a
password prompt — has it hidden. The cell keeps its background, so a concealed
field still occupies the space it claimed, and the characters are still in the
grid for a selection to copy.

## File references

Agents cite files constantly — `src/main.rs:42`, `./deploy.sh`, `Cargo.toml` —
in terminal output and in chat replies alike, and Cmd+click has always resolved
them in both. Nothing said so: the reference and the
prose around it were the same ink. Those references are now marked in the link
colour with a **dotted** rule, where a URL wears a solid one: same colour,
different rule, because a URL leaves for the browser and a path opens here.

The matcher is deliberately narrow, since a mark on ordinary prose teaches
people to ignore the marks. Two shapes qualify — something with a directory
separator, or a bare filename with a real extension — so `and/or`, `TCP/IP`,
`e.g.`, `10:30`, `v1.0` and `Fig.2` are left alone.

**A clicked `path:line` now opens at that line.** It never did: the position
was part of the token, so the file was looked up under a name it does not have
and the click quietly did nothing. The line is landed at the top of the
window, since the lines after the one you were sent to are the ones you came
to read.

## Text decorations

Crew draws the whole underline family, not just the one: **SGR 4** (single),
**4:2** (double), **4:3** (the spell-check squiggle), **4:4** (dotted), **4:5**
(dashed) and **SGR 9** (strikethrough), each in the cell's own colour or in the
separate colour **SGR 58** sets. Language servers, `git diff --word-diff`,
`rustc`'s inline diagnostics and every TUI that marks a misspelling reach for
these; a terminal that drops them shows a diagnostic as plain text.

The rules are drawn as GPU quads rather than glyphs, phased on the pane's own
pixel grid — so a squiggle running across six columns is **one continuous
wave**, not six restarts, and the underlined space between two words carries
the rule instead of breaking it. Rule thickness follows the cell height, so
they stay visible at large font sizes and never smear at small ones.

URLs in terminal panes are **underlined as well as tinted**, so a link is
legible without depending on hue (Cmd+click still opens it).

**OSC 8 hyperlinks** work too — the escape that lets a program attach a target
to arbitrary text (`ls --hyperlink`, `gh`, `cargo`, test runners linking a
failing file). Those cells are tinted and ruled like a URL even when the words
are prose, and **Cmd+click opens the program's target, not the text**. Because
that target is chosen by whatever is writing to the pane rather than by the
person clicking, crew opens only `http://`, `https://`, `mailto:` and `file://`
links (case-insensitively) and refuses the rest by name on the status line —
which always shows the URL actually opening, since link text can say one thing
and point at another.

## Markdown

Crew renders markdown natively: a `pulldown-cmark`-based engine (`md/`) folds
the event stream into styled blocks and lays them out straight onto GPU cells —
headings, lists, block quotes, tables (columns aligned by display width, so
CJK/emoji don't skew them), fenced code as bordered cards, and links. Task
lists render as **checklists**: `- [ ]` draws a ☐, `- [x]` a green ✓ with the
item text dimmed — done reads as done. ```` ```diff ````/```` ```patch ````
fences colour added/removed/hunk lines like the viewer's diff view — **and
carry its word-level marks**: each removed line is paired with the added line
that replaced it and only the run that differs is drawn at full strength, so a
diff an agent pastes into chat reads the same way `/diff` does. The pairing is
read back off the ink the renderer already gave each line, which is why one
refinement serves both surfaces. Unequal runs are not paired and a wholesale
rewrite is left plain, exactly as in the viewer. An
**untagged fence that reads as a diff** (a `diff --git` opener, or a `@@` hunk
header alongside real `+`/`-` change lines) is auto-detected and coloured the
same — agents rarely tag their patches. Nesting
depth is capped so pathological input can't blow the stack, and HTML blocks
render verbatim instead of disappearing.

- **Chat panes** (the `/smith` pane, Cmd+J chat) render message bodies as
  formatted markdown by default; single line breaks are preserved, since
  agent replies rely on them. **`Ctrl+Shift+M`** flips the focused chat pane
  to the raw source and back. **Cmd/Ctrl+click** on a rendered link opens it
  (hit-testing maps display columns through character widths, so links after
  emoji still click correctly).
- **`/view <file>`** (alias `/md`) opens a zoomed **file viewer** pane over
  one file — a single, read-only pane rendered by format: markdown, aligned
  CSV columns (both rendered by default — **s** toggles raw source on
  either), a numbered-gutter code view, colored diffs, or a metadata card
  for anything else (binary, unsupported extension, missing extractor tool —
  **o** hands it to the OS default app). Wrapped/rendered lines are
  precomputed once per width, so scrolling is free. **↑/↓** scroll by line and
  **PageUp/PageDown** by ten, **Home/End** jump to the top/bottom, **r**
  reloads the file from disk, **e** opens `$EDITOR`, **/** searches with
  **n**/**N** for next/previous hit, **Cmd+click** opens a rendered markdown
  link, the mouse wheel scrolls the pane, and **Esc** closes it. Relative
  paths resolve against the input bar's working directory.

## Multi-agent relay (`/smith`, alias `/crew`)

`/smith` opens **agent smith** — a pane that lets independent headless CLI
coding agents talk to each other to work a task you give them. The pane's own
voice (status notes, swarm summaries) speaks as `agent smith`; its routing
channel stays `crew` for session restore. Any registered agent can be sender or
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

**Swarm runs stream inline, then get out of the way.** When a plain `/smith`
message runs as a broker-side swarm, the pane opens a **live task-list block**
above the composer while the run is in flight — one row per planned task with a
state glyph (a spinner while it runs), its title, live elapsed seconds, and a
right-aligned token count. The agents' own replies stream into the transcript
as they arrive. When every task reaches a terminal state the block simply
**disappears** — no folded summary record, no `Σ` token/cost/time line, no
per-task accounting is left behind (the design deliberately favours a clean,
Opencode-style stream over a progress-and-billing readout). A clean run also
adds no "swarm done" or "task started/done" status lines; only the exceptional
endings stay visible — a task **error**, a user **stop**, or a run that
**finished with failed tasks**. Telemetry still rides a bounded broadcast bus
(4096-event ring) that drives the live block; if a burst ever outruns the
drain, the skipped count is reported in the transcript — `telemetry gap: N
events dropped (bus overflow)`.

Message bodies are newline-aware, and fenced ```code``` blocks render as
bordered cards — a muted `╭─ lang` header, verbatim hard-wrapped lines on a
dimmed background, `╰─` footer; ```` ```diff ````/```` ```patch ```` fences
(and untagged fences that read as a diff) colour added/removed/hunk lines in
place (see [Markdown](#markdown)). A just-landed card **fades in** from the page
colour over ~400ms (the fade drives redraws without reading as "busy"). A
settled reply whose `stats` event reported real usage closes with a muted
**usage trailer** — `900 in / 50 out · $0.012` — so each reply carries its own
bill, formatted exactly like the summary footer's totals. **Long
system/telemetry cards auto-fold**: a system-voice card past three body lines
(turn summaries, `/doctor` output, roster dumps) renders as its header + first
line + ` … +N`; a plain click expands it, a click on its header folds it back
(a drag-selection never toggles, and Ctrl+O's compact view wins outright). The composer on the bottom rows shows an
affordance bar (`@agent` chips in roster colours, `Enter send · Esc close`
hints) above a `❯` prompt that highlights a valid leading `@mention` in that
agent's colour. **Ctrl+R** opens a shell-style **reverse search** over what
you've sent this session — typed text filters (substring first, then
subsequence), Ctrl+R steps older, Enter recalls the match into the composer
without sending, Esc restores what you had typed. **Cmd+F** (or Ctrl+F) opens
**find-in-transcript**: the query filters matching messages as you type,
Enter/Ctrl+F steps to the next older match and Up to the next newer, each
jump scrolling the match into view under a highlight wash. While the transcript overflows, the last column shows a
proportional scrollbar, and messages arriving out of view raise a `↓ N new`
pill that clears at the live bottom. A fresh pane greets with the detected
crew (names, roles) and an example `@agent` prompt.

**Constructs.** Inside the pane, lines starting with `/` drive the broker
itself (Tab completes both `@agents` and slash constructs; one-letter **aliases**
`/h /d /m /r` expand to help/diff/model/reload,
and a typo gets a **did-you-mean** suggestion):

- **`/help`** — list the constructs; bare **`/model`** — the whole model
  story: a **grouped provider picker** — "your subscriptions" (signed-in
  CLI-delegated providers like a Claude Pro/Max seat via the `claude` CLI or
  a ChatGPT seat via `codex`), "your keys", and "installed CLIs", each entry
  numbered — followed by the roster with each agent's role and model (also
  in the pane footer, along with the live task count and session turn/token
  totals, the model pins, the sys-tool sandbox mode, and the token budget).
  Signed-out delegated providers list grayed (`○`) with the **exact sign-in
  command** (`claude auth login`, `codex login`); a signed-out **device-flow
  provider** (Qwen/DashScope) lists *numbered* instead — picking its number
  **runs the sign-in right in the pane**: a code card streams into the chat
  (user code + verification URL), crew polls while you approve in the
  browser (`/stop` cancels), and on success the provider is selected and the
  grant stored — no key ever pasted. **`/model <n>`** otherwise switches
  to entry *n*, storing the provider pin so it survives restarts.
- **`/model <agent> <model|default>`** — pin an agent to a model for the
  session. Pins apply per agent, so **planner, coder, and reviewer can run
  three different models side by side**; every change re-emits the roster so
  the pane's model badges update live.
- **fan-out and loops, in plain language** — the former fan and loop commands
  are retired: the intent router classifies each plain message and picks its
  execution shape. "Have every agent take a crack at this" sends the same task
  to every agent **in parallel** (one thread per call; replies stream back
  fastest-first with per-agent latency, and the turn closes with combined
  stats); "keep refining it over a few rounds" runs relay rounds, each handed
  the previous round's answer to improve on. `CREW_INTENT=0` restores
  plain-swarm routing. **`@a+b <task>`** still fans out to just that subset.
- **"keep working until …"** — relay rounds until a judge agent (elected by
  the model) rules `MET:`/`NOT MET:` on the goal; NOT-MET reasons feed
  the next round. Caps at 5 rounds (a backstop — the model's own `@done` or
  the judge's MET ends a healthy run first; the `/goal` slash form is
  retired). The **command bar's** `/goal` is a different engine: there the
  goal is planned into a task graph and run as a
  [swarm](#swarm-orchestration-crew-hive) under a cost ceiling.
- **"draft a plan for …"** — plan mode (à la Claude Code; `/plan` retired):
  an agent (prefix `@agent` to pick who) drafts a numbered
  plan and **nothing executes** until you approve. Enter — or saying
  "approve" / "run it" — hands the plan to the relay; esc — or "reject" /
  "drop it" — discards it. The verdict words are matched exactly, before
  any model call, so a misrouted message can never run or drop a plan; any
  other message leaves the draft pending on the session.
- **automatic checkpoints** — Cline-style workspace snapshot before every task
  that can change files: the working
  tree (tracked + untracked, `.gitignore` respected) is committed through a
  temporary index and pinned under `refs/crew/` — HEAD, your index, and
  branches are never touched, and snapshots survive broker restarts.
  bare **`/restore`** lists them oldest-first; **`/restore <n>`** puts that
  snapshot's files back and removes the files that appeared after it, naming
  each one it deleted. Ignored files (build output, secrets) are never
  candidates, and neither is anything that predates the snapshot.
- **skills, no command** — a task that names a loaded playbook picks it up
  by itself, and when skills are loaded but unmatched the relay carries a
  one-line roster of them; asking "what skills are loaded?" lists them (the
  `/skill` slash form is retired — see *Extending* below).
- **`#<note>`** — standing **project memory** (à la Claude
  Code's `#` shortcut): `#always run tests with --workspace` appends the note
  to `./.crew/memory.md`, and from then on **every task** carries the merged
  memory (user `~/.config/crew/memory.md` first, project second, 2 KB cap)
  as a STANDING MEMORY block the agents are told to follow. Ask "what do you
  remember?" to see it (the `/memory` slash form is retired). Unlike skills,
  memory is always on; edit or delete the file to forget.
- **`/reload`** — pick up extension edits without a restart: re-reads skills
  and plugin manifests, forces MCP to re-read `mcp.json` and reconnect on
  next use, and re-emits the roster so the pane's badges update.
- **`/diff`** — the working tree's `git diff --stat` inline in the
  transcript; **`/doctor`** — the broker's working directory and sys-tool
  sandbox mode.
- **"commit this"** — an **AI-written commit message** (à la Aider; the
  `/commit` slash form is retired, plain language replaced it): an agent
  reads the diff (staged wins; otherwise everything the tree changed,
  12 KB cap) and drafts a Conventional Commits message — subject ≤72 chars,
  body only when the change warrants it. **Nothing is committed until you
  say "apply"** — the confirm is matched exactly, never by the model, so a
  misrouted message can draft but can never commit. Asking again re-drafts.
  A clean tree, a missing repo, or an empty draft each get a status line
  instead.
- **"look over my changes"** — an **AI code review** of the same diff the
  commit draft sees (à la Codex; the `/review` slash form is retired): the
  reviewer reports findings worst-first — `blocker — file:line — what and
  why`, then `warn`, then `nit` — closing with a one-line verdict (or "no
  findings" for a clean diff). Read-only: nothing to apply, pairs naturally
  with "commit this" before you ship.
- **"what did I ship this week?"** — an **AI standup update** from the
  repo's recent commits (the `/standup` slash form is retired): an agent
  groups what shipped by theme, infers what's still in progress, and calls
  out risks — first person, paste-ready for the morning thread. History
  summarization — the complement of the code review (the diff you haven't
  committed) and the commit draft (the message for it). An empty window or
  a fresh repo reports "nothing to report" instead of erroring.
- **`/doctor`** — a **health check for the AI stack** (à la Claude Code's
  `/doctor`): one ✓/✗/– checklist covering the provider that will answer
  (and which key it found — a serving subscription reads e.g. "claude-code
  subscription (via the claude CLI — swarm planning degrades to the
  relay)"), a **per-provider auth line** (signed in / signed out with the
  sign-in command / key present / no key / not installed — states only,
  never a key value), a **token store** line saying where OAuth grants live
  (the macOS keychain via `security`, or a 0600 file where no keychain
  exists), the claude/codex/opencode CLIs on `$PATH`,
  `/bin/bash` (run panes' job control), git, and how many skills, plugin
  agents, and MCP servers loaded — each MCP server listed with its tools or
  its failure (the retired `/mcp` listing folded in) — plus standing memory,
  a resumable session, and the sys-tool mode — each ✗ line names its fix.
- **"pick up where we left off"** — **continue the previous session** (à la
  Claude Code's `--continue`; the `/resume` slash form is retired): the
  broker auto-saves the conversation — your tasks and every agent reply —
  to `./.crew/session-live.md` as it streams (32 KB cap; past it the oldest
  half is folded into a `[compacted earlier session: …]` summary header by
  one bounded model call, or dropped when keyless — never an error; the
  `crew` system voice is skipped), and on the next broker
  start it rotates to `./.crew/last-session.md`. Asking to resume in a
  fresh pane folds that file's tail (2 KB) into your **next task** as a
  PREVIOUS SESSION context block — consumed once — so the crew picks up
  where the last pane left off, even after a crash.
- **`/export`** — write the pane's transcript to
  `crew-transcript-<stamp>.md` in the working directory (à la OpenCode),
  one `## sender · time · latency` section per message. The transcript folds
  older messages away when a long session gets heavy. Both — like `/theme`
  and `/exit` — are answered by the pane itself, so they work even while the
  broker is busy.
- **the footer** / **`/stop [#n]`** — long constructs run as **concurrent
  background tasks** (default cap 4, `CREW_MAX_TASKS`): submitting a second
  task doesn't wait for the first, every streamed reply is tagged with a dim
  `#N` chip naming its task, the pane footer lists what's running (`#id ·
  age`), and `/stop #n` cancels one task — bare `/stop` cancels them all —
  at its next checkpoint (between hops/rounds). Quick constructs and
  `/doctor` answer immediately while tasks are in flight.

**Built-in sys tools.** Agents can touch the workspace without any MCP server:
four bounded tools ride the same `@tool` surface — **`sys:run`** (one
non-interactive shell command via `/bin/sh -c`, 120s deadline —
`CREW_SYS_TIMEOUT_MS` overrides, and the timeout message says so — 64 KB per pipe,
its whole process group reaped on timeout so backgrounded children can't
linger), **`sys:read_file`** (UTF-8, 64 KB per call; a truncation note carries
the byte `offset` to continue with, so agents read big files in chunks),
**`sys:write_file`** (create/overwrite), and **`sys:list_dir`** (≤500 entries,
sizes shown). `CREW_SYS_MODE=readonly` blocks the mutating pair (`run`,
`write_file`), `CREW_SYS_TOOLS=0` turns the surface off entirely, and `/doctor`
shows the active mode. An approximate per-thread **token budget**
(`CREW_BROKER_TOKEN_BUDGET`, default unlimited) terminates a thread that blows
past it.

**`@file` mentions.** In the composer, a trailing `@<query>` pops a fuzzy file
picker over the project tree (filename-prefix first, then path matches; ↑/↓
navigate, Tab/Enter accept, Esc closes just the popup). On send, each
mentioned file's contents are spliced into the outgoing message as a
`--- file: … ---` block (64 KB cap; binary or missing files are skipped), so
you can hand agents exact context without pasting. The leading `@agent`
selector is left alone, and typed mentions render as tinted chips. A path
with spaces rides a **quoted mention** — `@"docs/my notes.md"` — one chip
through its closing quote; the picker mints the quoted form itself when it
accepts such a path, and so does a file **dragged onto the pane** (see
[Clipboard](#clipboard)).

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
  There is no command: a relay or swarm task that **names a skill** gets its
  playbook woven in automatically (at most two per task), and when skills
  are loaded but unmatched the prompt carries a one-line roster (origin,
  directory marker, and `N KB → outline` for the framed ones), so every
  agent in the thread knows what it could ask for.
- **Plugin agents** join the roster from JSON manifests in
  `~/.config/crew/agents/*.json` or `./.crew/agents/*.json`:
  `{"name": "aider", "command": "aider", "args": ["--message", "{}"],
  "role": "repo-wide edits"}`. `{}` is the message placeholder (appended when
  missing); manifests whose command isn't on `$PATH` are skipped, and a
  manifest can't shadow an inbuilt agent. With manifests present, `/smith`
  works even with **no API key at all**.
- **MCP servers** are declared in `~/.config/crew/mcp.json` or
  `./.crew/mcp.json` with the familiar schema —
  `{"mcpServers": {"fs": {"command": "mcp-server-fs", "args": ["--root", "."],
  "env": {}}}}` — and connect lazily over stdio (JSON-RPC 2.0, hard
  per-request deadlines, killed with the pane). `/doctor` lists each server's
  tools. When servers are configured, every relay prompt advertises the tools
  and an agent calls one by ending its reply with
  `` `@tool <server>:<tool> {"arg": …}` `` — the broker runs the tool, logs
  the call and result as visible hops, feeds the result back to the same
  agent (up to 4 tool rounds per hop), then normal `@next`/`@done` routing
  resumes.

**Models & rate-limits.** When no agent CLIs are installed, `/smith` runs its
inbuilt API agents — **planner** (capable tier), **coder**, and **reviewer**
(standard tier) — over an LLM. **Subscriptions come first**: before any key,
discovery asks the CLI-delegated providers for their own signed-in state
(`claude auth status`, `codex login status` — consent-based, never another
app's token store), and a live login routes plain smith tasks through that
CLI via the existing relay, exactly as `@claude <task>` would
(`CREW_SUBSCRIPTIONS=0` disables the rung; `CREW_PROVIDER=claude-code|codex`
pins it explicitly). Where a provider openly permits third-party OAuth, crew
runs the **device-code flow itself** (today: Qwen/DashScope via `/model`'s
in-pane sign-in): the granted tokens live in the **OS keychain** (macOS
`security`; a 0600 file elsewhere), access tokens **refresh automatically**
before a model call, and only a hard refresh failure surfaces — as exactly
one "sign-in expired — open /model" line, never a nag loop. A stored grant
then serves *every* model call (classify, planner, workers, judges) exactly
as a key would. With no subscription, key discovery is unchanged and
prefers `DASHSCOPE_API_KEY`
(Alibaba Cloud Model Studio — Qwen commercial models, `qwen-max` →
`qwen-plus` → `qwen-turbo`, override with `CREW_DASHSCOPE_MODEL=a,b,…`; the
endpoint defaults to the international region, point `CREW_DASHSCOPE_BASE_URL`
at the China host if your key lives there), then `OPENROUTER_API_KEY` (free
models by default), then `ANTHROPIC_API_KEY`, and last a **direct vendor key**
(see below); set `CREW_PROVIDER=dashscope|openrouter|anthropic|openai|gemini|deepseek`
to pin one explicitly — a pin works even when that vendor's key is the only one
you hold, and even when several are set. Keys
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

**Direct vendor keys.** If you hold a key from the vendor itself, Crew talks to
that vendor directly rather than routing you through OpenRouter. Each of these
speaks the OpenAI chat-completions wire, so they share one client — only the
endpoint, the key and the default model chain differ:

| `CREW_PROVIDER` | Key | Default chain | Override the chain | Override the endpoint |
|---|---|---|---|---|
| `openai` | `OPENAI_API_KEY` | `gpt-5` → `gpt-4.1` | `CREW_OPENAI_MODEL` | `CREW_OPENAI_BASE_URL` |
| `gemini` | `GEMINI_API_KEY` | `gemini-2.5-pro` → `gemini-2.5-flash` | `CREW_GEMINI_MODEL` | `CREW_GEMINI_BASE_URL` |
| `deepseek` | `DEEPSEEK_API_KEY` | `deepseek-chat` → `deepseek-reasoner` | `CREW_DEEPSEEK_MODEL` | `CREW_DEEPSEEK_BASE_URL` |

Chain overrides are comma-separated and tried in order, exactly like
`CREW_OPENROUTER_MODEL`: `export CREW_GEMINI_MODEL="gemini-2.5-flash,gemini-2.5-pro"`.
Gemini is reached over Google's own OpenAI-compatibility endpoint, so no Google
SDK is involved.

These probe **last** in auto-discovery, deliberately: adding a vendor can never
change which provider an existing install already resolves to. So if you also
have `DASHSCOPE_API_KEY` or `OPENROUTER_API_KEY` set, a direct key is only
picked up when you name it — `CREW_PROVIDER=gemini`. The model picker routes a
row to its native vendor when you hold that vendor's key, and to OpenRouter
otherwise. Default model ids are never invented: every slug above is a real
`crew_hive::catalog` row for that vendor, which is also why xAI, Mistral and
Groq are absent despite speaking the same wire.

**Isolation & threading.** Agents run in a broker **subprocess** (the
`crew-broker-plugin` binary) over Crew's JSON-line plugin protocol, so all the
slow agent calls happen off the render thread and the window stays responsive.
An adapter normalizes each agent's stdout before it is ever shown or relayed
(claude `-p --output-format text` and `codex exec` print the reply on stdout;
opencode's `--format json` event stream is parsed for the assistant text).

**Architecture.** The reusable broker lives in `crates/smith-plugin/src/broker/`:
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
sys tools; `CREW_SYS_TIMEOUT_MS` (default 120000) bounds each `sys:run`;
`CREW_HTTP_TIMEOUT_MS` (default 120000) bounds each HTTP attempt to a provider,
deliberately under `CREW_BROKER_TIMEOUT_MS` so a stalled endpoint names the
transport and still leaves the model fallback chain a turn;
`CREW_STREAM_TEXT=0` stops streamed text being forwarded at all, restoring the
pre-streaming behaviour for a regressed run or a deterministic test;
`CREW_INTENT=0` disables the intent router — every plain message then runs as
a swarm instead of the model first choosing its execution shape (a direct
reply, an all-agents fan-out, refinement rounds, a plan awaiting approval, or
the swarm); `CREW_SUBSCRIPTIONS=0` disables the signed-in-subscription rung —
crew then never runs `claude auth status` / `codex login status` and plain
tasks fall back to key discovery and the keyless relay exactly as before. The pane also prints a per-turn timeline + cost summary (`turn done
— planner 4.2s → … · N exchange(s) · ~X tok (approx)`) at the end of every
task, and accumulates the spend into the header's `~N tok` meter.

**Reaching crew from a phone (Telegram).** `CREW_TELEGRAM_TOKEN` is a bot token
from `@BotFather` — with none set the channel is registered but inert and never
opens a socket. `CREW_TELEGRAM_CHATS` is a comma-separated allowlist of chat ids
crew will listen to and reply to; it is empty by default, and empty means
**nobody**, not everybody — an assistant with a public address is an assistant
anyone can drive. Crew prints the id of any chat it turns away, so the first
rejected message tells you what to put there. `crew daemon channels` shows every
way in and whether it is usable.

**Pointing a pane at a different binary.** Each plugin-backed pane runs a child
process, and each resolves its command the same way: an environment override
first, then a sibling of the running executable. `CREW_BROKER_PLUGIN` replaces
the `/smith` broker — which by default is **this** binary re-invoked with
`--broker-plugin`, so `/smith` works wherever Crew is installed with no second
binary to ship. `CREW_CHAT_PLUGIN` and `CREW_ORCHESTRATOR_PLUGIN` do the same
for the echo and orchestrator plugins. Point one at a debug build to run a
pane against uncommitted work while the rest of the app stays on the installed
release. `CREW_PANE` names the sending pane in an inter-pane `crew ask`
message (default `an agent`); Crew sets it for panes it spawns.

## Swarm orchestration (`crew-hive`)

The `/smith` relay is a few CLI agents talking turn-by-turn. **`crew-hive`** is the
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
  records results, and emits state transitions. With `with_replan`, the first
  failed task triggers ONE **mid-run re-plan**: the planner gets the goal,
  the completed outputs (budget-clipped) and the failure, and its replacement
  sub-graph supersedes the not-yet-run remainder — completed work never
  re-runs, replacement tasks are forced to `Api`/`Standard` (the `parse_plan`
  invariant, re-applied), and a planner error (or a keyless/mock run, which
  gets no replanner) keeps plain **cascade-cancel** of the dependents. A
  panicking agent becomes a failed task (the run survives); `with_cancel`
  gives cooperative, graceful shutdown (stop new dispatch, cancel unstarted,
  drain in-flight).
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

A running swarm pane draws a **timeline** down its right third: one bar per
task on a shared axis, coloured as its state glyph is, with running tasks
reaching the "now" rule and growing while you watch. It answers the question
the task list cannot — whether the scheduler really ran things at once, since
six tasks run one after another and six run together produce identical lists.
The bars give way to the task names on a pane too narrow for both. Timings are
observed by the pane, not reported by the engine (which has no clock), so a
task that starts and finishes between two frames reads as instantaneous.

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
activity history persists instead of vanishing after a few seconds. Those five
rows are a window onto a hundred buffered entries: **scroll the LOG with the
wheel** to reach the older ones, and its rule shows `⇡N` while you are back
from the live tail. A line arriving mid-scroll steps the window with the buffer
rather than sliding out from under you; scroll back down to follow again.
Below it, a
**PANES** list of the open panes (index, name, a `▸` focus marker, and an
activity dot) fills the remaining height. Click a PANES row to focus that pane; the row
under the pointer brightens to say so. The panel's **card legend shows the running version**
(`crew vX.Y.Z`), so the build is always visible at a glance.

## Settings

`/settings` opens a **two-column bento form** covering **every configurable
property** — an APPEARANCE card in the left column, WINDOW and NOTIFICATIONS
stacked on the right (collapsing to one column on a narrow pane); Tab/wheel
move focus, Enter commits a field, **Cmd+S / Alt+S** saves and closes.

The **accent** field also says how its colour *reads*: `4.6:1` beside the
swatch, drawn in the alarm colour when it falls below `3.0` — the same floor
the palette suite holds every shipped `accent_default` to. Crew derives every
other colour against a measured floor; the one colour a person picks by hand
was the only one nobody was measuring, and an accent that cannot be read is the
mistake this field makes easy.

Fields whose value names a colour **draw it** inside the right end of their
box — the theme pickers show a rotation's pool as one chip per palette and a
named palette as its whole hand (ink, accent, and four ANSI slots, each over
that palette's own page), and the accent field shows its own hex. The same chips the command palette's pickers use, for the same
reason: reading a name and pressing Save to find out what it looks like is the
form failing at its job. The chips answer to the value, so a field showing
something that names no palette draws nothing rather than the last thing that
did, and a box too narrow to hold both the value and the chips keeps the
value.


- **APPEARANCE** — **Font family** (type-to-search over installed monospace
  families), **Font size**, **Paper grain** (0–2 amplitude), **Smoothing**
  (←/→/Space cycle `off · light · medium · heavy` — the same ladder and
  `font_smooth` key as `/smooth`; a custom numeric strength shows as its
  number), **Theme**
  (←/→/Space cycle through the four rotations and every palette), **Accent
  (#hex)** (override the
  theme accent; clear to use the default), **Glass** (←/→/Space cycle
  `off · low · medium · high`), **Motion** (`off · subtle · full`),
  **Gradient colour** (`off · subtle · lively`), **Paper texture** (on/off),
  **Drifting background** (on/off).
- **WINDOW** — **Nav width**, **Opacity %**, **Show nav**, **Launch maximized**.
- **NOTIFICATIONS** — the master switch plus per-event toggles (**cmd done**,
  **bell**, **pane exit**), the **min secs** threshold, and the watched
  output **patterns** as a one-per-line text area.

Settings persist to `$XDG_CONFIG/crew/config.toml` and apply live on Save.

### Motion

Crew animates like an instrument panel: elements assemble, marks travel, and
nothing teleports. **Motion** (APPEARANCE) sets how much of that you get.

- **`full`** — the default; the full choreography at its nominal timing.
- **`subtle`** — the same movements at 60% duration, quick enough to read as
  responsiveness rather than as an effect.
- **`off`** — genuinely off, not fast. Every animation's duration collapses to
  zero, so each one draws its final state once and schedules no further frames;
  an idle crew at `off` repaints exactly as rarely as one with no animation at
  all. This is the reduce-motion setting.

Motion never costs an idle crew anything. Every animation is a *bounded*
timeline, and one predicate — `wants_animation_frame` — decides whether the app
schedules another frame at all. An animation has to appear there to be drawn a
second time, and it has to end or the app would never sleep again; both are
asserted by tests rather than merely intended.

Motion never introduces a colour. Everything moving is drawn in the palette the
active theme already declares — the border, the ink, the accent — so a theme
change carries the motion with it, and a new theme cannot ship without it.

**Working panes sweep.** While a pane is actually busy, a soft band of light
travels down its glass sheet and back. It is gated on busy and nothing else: a
working pane already repaints, so the sweep costs no extra frames, and an idle
crew never draws one — which is how a surface that moves stays compatible with
never repainting. The band rides the sheet's own fill, so it introduces no
colour of its own.

**A live caret.** A card that is still streaming ends in a block caret that
pulses between the muted and accent colours. It pulses rather than blinking:
a caret that vanishes half the time reads, at a glance, like the text stopped.
At `Motion = off` the caret stays — it carries information — but holds still.

**Readouts count.** The summary footer's cost and token totals sweep to their
new values instead of snapping, and the 5h-budget and context meters fill rather
than jumping a cell at a time. Each pane animates its own numbers. A value seen
for the first time is simply shown — only a value that *changes* sweeps.

**Cards assemble.** A new pane doesn't appear — its frame draws itself outward
from the four corners, so the card is *built* in front of you. Only the frame
stroke animates: the legend is there from the first frame, because a pane you
cannot yet name is worse than one that simply appeared.

**Cards collapse.** A closed pane leaves its frame behind for a moment and it
retracts back into its corners — the reverse of the way it was drawn. Minimizing
does the same but travels left toward the nav, which is where the pane actually
went, so the two dismissals never read as the same gesture. The pane itself is
gone immediately either way; only the frame lingers.

**Zoom travels.** Cmd+Z expands the pane out of its own tile rather than cutting
to full size, and collapses back into it.

**Focus brackets.** The focused pane carries HUD corner marks: short accent runs
down the card's edges from each of the four corners, which grow out as focus
arrives and travel with it when you move between panes. They stay clear of the
top border, where the legend and the `[-][x]` buttons live.

### Glass

Two of those fields shape the frosted look, and they are separate knobs:

**Glass** (APPEARANCE) sets how frosted the **cards** are. Every pane, panel and
the input bar sits on a translucent sheet: a tinted fill that fades from the top
down, a bright specular hairline along the upper edge, a soft drop shadow, and a
whisper of frost grain. The *look* is derived from whichever theme is active
rather than configured per palette, so **every theme** — light, dark and CRT —
gets its own treatment automatically: dark themes lift a lighter sheet off the
page, light themes lean on a whiter sheet plus a real shadow (a light page can't
get lighter), and CRT runs the most *luminous* sheet of the family — a
translucent phosphor-tinted panel with an inner edge-glow bleeding in from the
frame, so each pane body reads as lit by its own border (the old "CRT stays
faintest" restraint is gone). `medium` is the default; `off` restores flat
cards and costs nothing to draw. Overlay popups (the command menu, the attach
picker, the key prompt) stay opaque by design.

**Opacity %** (WINDOW) makes the **window itself** translucent, so your desktop
shows through the page. Text, pane fills and selections stay solid — only the
bare page goes sheer. `100` is opaque; the value floors at **35%**, because a
window dialled any sheerer is one you can't find again. Works with the CRT
post-process too — the tube shapes light, not transparency.

## Themes

Crew offers **four themes** — **`dark`**, **`light`**, **`crt`**, and
**`auto`** — and each one is a *rotation*: it cycles through a pool of
hand-tuned palettes every 10 minutes. `dark` rotates the dark paper/ink looks,
`light` rotates the light ones, `crt` rotates the old-school phosphor tubes,
and `auto` follows the **OS appearance** — the dark pool while the
system is in dark mode, the light one in light mode, flipping live (through
the develop-fade) the moment the system switches. With no theme saved at all,
crew defaults to `auto`, so a fresh install matches the system from the first
frame. `auto`'s pairing is yours to re-wire: `theme_dark` / `theme_light` in
`config.toml` swap in a different pool (`crt` at night is the classic) or pin
a single palette per appearance. The twenty-six palettes
below are those pool members (eleven paper/ink looks designed to read like a
page rather than a screen, eight "modern glow" looks in the Gemini/Codex
idiom, five CRT tubes, and the two — `harbor` and `fern` — drawn after the
cut; `crt-violet` was retired in that cut and has since come back); they're no longer selected on their own, but each
name still resolves if you type it. A palette's own appearance decides its
pool — the modern glow palettes are dark and light *pages* like any other, so
they rotate inside `dark` and `light` rather than standing apart as themes of
their own. The picker offers the four rotations first and then **every
palette by name**, under a heading — they have always parsed, and not offering
them meant you had to know the name of the one you wanted, which is the
opposite of what a picker is for.

- **`paper-dark`** (default dark-pool member) — a high-contrast "newspaper" look: a near-black
  page (`#0c0805`) with near-white ink (`#ececec`) and grey rules. Terminal
  output keeps muted-but-readable ANSI colours so error/diff cues survive.
- **`paper-light`** — a warm off-white page (`#f4f1ea`) with soft dark ink and
  ink-toned ANSI colours (sage, brick, faded indigo). No pure black or white
  anywhere; every surface reads as the same sheet of paper.
- **`harbor`** — a deep blue-slate page under an azure light, its gradient
  running azure into teal. The cool end of the dark pool, where `paper-dark` is
  neutral and `sepia-dark` is warm.
- **`fern`** — a faint mint page under a deep green-teal light; the only light
  palette whose accent is green, so it cannot be mistaken for the two warm ones
  at a glance.
- **`crt-violet`** — the fourth phosphor: a violet tube, the glow of a vector
  display rather than a terminal. Its ladder is one hue at six brightnesses,
  like the other tubes, and it is the only one of the four whose phosphor
  leaves room for a warm pink alarm — the other three cannot tell their bell
  apart from their status by hue at all.
- **`sepia-dark`** — dark sepia paper with warm cream ink.
- **`sepia-light`** — an aged-newsprint cream page with dark sepia ink.
- **`midnight-ink`** — a warm slate-charcoal page with cool off-white ink.
- **`graphite`** — a soft charcoal page; the gentlest of the darks.
- **`moss-blotter`** — a deep moss-green desk blotter with warm paper-white
  ink and botanical accents (dark).
- **`coldpress-gray`** — a cool pale-gray page with light graphite ink.
- **`salmon-broadsheet`** — an FT-style salmon-pink broadsheet page (light).
- **`ivory-ledger`** — an ivory page with ledger-green ink (light).
- **`glacier-bond`** — a cold blue-gray bond page — overcast north light —
  with crisp near-black ink and slate-blue accents (light).
- **`aurora`** — blue→violet gradient glass on near-black (modern, dark).
- **`nebula`** — an orchid→rose gradient dusk (modern, dark).
- **`graphene`** — neutral near-black with a mint accent (modern, dark).
- **`cobalt`** — an electric blue→cyan current (modern, dark).
- **`daybreak`** — blue→violet on a cool white page (modern, light).
- **`blossom`** — violet→rose on a warm white page (modern, light).
- **`meadow`** — emerald→teal on a neutral white page (modern, light).
- **`cirrus`** — blue→cyan on the coolest white page (modern, light).
- **`crt-green`** — the classic green-phosphor terminal: neon green on a
  near-black tube, with a monochrome-green ANSI palette (brightness tiers) for
  that single-gun look.
- **`crt-amber`** — the warm amber variation of the green tube.
- **`crt-blue`** — a cool blue phosphor variation (Tron).
- **`crt-violet`** — a neon violet phosphor variation.
- **`crt-paperwhite`** — the P4 white tube (early Macintosh/VT420):
  near-white ink with a faint blue-gray cast on a true black tube.

**The CRT tubes are holographic.** Each phosphor carries its own tube tuning
(scanline weight, bloom strength and radius, streaming-flicker character), so
green runs a hot driven-hard raster while blue runs a cold TRON edge. Hot
pixels feed a real half-res gaussian bloom — a focused border *radiates*
tens of pixels instead of stopping at the stroke — and pane glass becomes a
luminous translucent sheet in the phosphor's hue. The chrome is drawn in
light, TRON/JARVIS-style: a focused frame's four corners run white-hot so
the bloom turns them into glowing nodes, gaining focus fires a ~600ms
ignition sweep (the whole frame ignites at the node colour and decays to
rest), and a streaming pane's frame breathes on a slow ~2.4s cycle. All of
it is focus-led — unfocused panes stay a thin quiet trace — and all of it is
bounded: an idle tube renders a byte-identical frame every time. Paper
themes are untouched by any of this.

**The modern glow palettes are pages that carry light.** Each one owns two
saturated poles that drive all three of its signatures: a gradient light-ring
around the focused frame, a slow wash of pole light under the page, and a fine
dot lattice woven over it on the text cell's pitch. They ride the bloom chain for their halo but never the tube —
curvature, scanlines and the bezel vignette are all zero — so they sit in the
`dark` and `light` rotations, not in `crt`.

**The page drifts.** The wash is two broad pools of pole light on an elliptical
orbit under the page, and they turn: one revolution every six seconds while a
pane is working, and — with **Settings → APPEARANCE → Drifting background** on,
the default — one every ninety seconds when nothing is happening at all. Idle
motion is a texture, not a signal, so it is fifteen times slower than the busy
kind and drawn at about six frames a second.

This is the only animation in crew that repaints a window nothing else needed
repainted, so it is fenced on four things, any one of which stops it: the
setting, **Motion** not being `off`, a theme that has a wash at all, and crew
holding the OS focus — a window you are not looking at repaints for nobody.
Turn the setting off and an idle crew goes back to drawing exactly nothing, its
last frame held wherever the pools had reached. The phase is accumulated from
frame deltas rather than read off the clock, so the motion is continuous across
every pause instead of teleporting after a quiet minute.

**Where you are in a buffer is a colour too.** Scroll a pane back and the
`⇡N` on its top border and the thumb down its right border both take the
theme gradient, sampled at your position: deep in the history they wear one
pole, at the live edge the other, and dragging the gutter walks them between.
It is the same gradient the card's own stroke runs, so the thumb reads as part
of the frame it rides rather than as a widget parked on it.

**The light gathers where you are working.** The wash's orbit is not centred
on the page any more — it slides toward the focused card, so the page is
brightest under the pane you are typing into and falls away from the ones you
are not. On a four-pane grid the focused frame is one stroke among four; the
wash under it is half the window, which is why this reads from the corner of
the eye when a border colour does not. Focus the input bar and the light comes
down to meet it.

It travels rather than cutting — the same exponential smoothing the panes use
to glide to their tiles, a little slower, so a card arrives and the light
fills in behind it. Bounded like everything else: it settles, and an idle crew
still repaints nothing. At **Motion = off** it snaps. With nothing focused the
gather fades out where it stands instead of dragging a bright field back
across the page.

**And the gradient's colour breathes.** The two poles every gradient surface
is drawn between — the wash, the dot lattice, every card's stroke, the footer
meters — lean around the hue wheel over time, so the canvas warms and cools
instead of holding one fixed pair of swatches. **Settings → APPEARANCE →
Gradient colour** sets how far: `subtle` (the default) leans ±16°, one
colour's neighbourhood, so a violet theme visits indigo and magenta and is
never anything else; `lively` leans ±38°, far enough that the two ends read as
different lights on the same room; `off` pins the poles to the theme's own
colours forever.

It is a *breath*, not a rotation — the offset is a sine, so the colour leans
one way, comes back through the palette's exact colour, and leans the other. A
monotonic turn would eventually walk every theme through every hue and stop
being that theme.

The rotation happens in OKLCH and moves **hue only**: lightness and chroma
come back out and go straight back in, and out-of-gamut hues lose chroma
rather than clipping a channel. That is the safety argument, and it is
measured — across all eight palettes a pole's contrast against its own page
moves by under 8% at the widest rung, and no offset a hand-edited config can
reach takes one below the WCAG 3.0 non-text floor. The breath also rides the
frames the wash was already drawing (four times slower than the pools orbit),
so it costs nothing extra, holds when the wash holds, and stops dead at
**Motion = off**.

**Light themes read like print.** The six light *paper* themes (`paper-light`,
`sepia-light`, `coldpress-gray`, `salmon-broadsheet`, `ivory-ledger`,
`glacier-bond`) render
base text at **Medium (500) weight** — dark themes use Normal (400) — and
carry a **1.2× "newsprint" grain** multiplier, so the page reads as paper
instead of a washed-out screen.

A faint procedural **grain** + edge vignette is drawn behind everything (GPU) —
it reads as paper texture on the paper themes and as a subtle **tube glow** on
the CRT ones. Every palette's colours are picked for measured WCAG contrast.

**Switching:** `/theme dark` | `/theme light` | `/theme crt` — selecting
`/theme` in the palette opens an arrow-selectable picker — or cycle all four
live with **`Ctrl+Shift+L`** (`dark → light → crt → auto`). The choice persists
to `config.toml`.

**Each theme rotates** to a different palette from its pool every **10 minutes**:

- **`/theme dark`** — rotates every dark page: the paper looks (`paper-dark`,
  `sepia-dark`, `midnight-ink`, `graphite`, `moss-blotter`) and the modern
  glow ones (`aurora`, `nebula`, `graphene`, `cobalt`).
- **`/theme light`** — rotates every light page: the paper looks
  (`paper-light`, `sepia-light`, `coldpress-gray`, `salmon-broadsheet`,
  `ivory-ledger`, `glacier-bond`) and the modern glow ones (`daybreak`,
  `blossom`, `meadow`, `cirrus`).
- **`/theme crt`** — rotates the CRT phosphor palettes (`crt-green`,
  `crt-amber`, `crt-blue`, `crt-violet`, `crt-paperwhite`).

Selecting a theme switches immediately to a pick from its pool, so the effect
is visible right away.

**Back-compat.** The old names still resolve when typed or loaded from an
existing config: any individual palette name (`/theme crt-green`) pins that one
palette (no rotation), the pre-consolidation modes `random-dark`/`random`
and `random-light` still work, and `modern` / `modern-light` — briefly themes
of their own — now resolve to the pool that swallowed them (`dark` and `light`
respectively), so a saved config keeps opening on the appearance it asked for.
None of these appear in the picker.

**Programs keep reading after a switch.** Terminal panes answer color queries
(OSC 10/11) and set `$COLORFGBG` from the active theme, so CLIs that probe the
background pick the right palette at launch. Scheme-aware TUIs can do better:
crew supports **DECSET 2031** (the contour convention) — a program that
enables it gets a `CSI ? 997 ; Ps n` report the moment crew's light/dark
scheme changes (OS flip under `auto`, `/theme`, the hotkey cycle), and
`CSI ? 996 n` answers the current scheme on demand, so neovim-class programs
re-query OSC 10/11 and repaint mid-session. Everything else samples **once at
startup** — after a live theme switch it keeps painting colors tuned to the
old background. Crew therefore enforces a **minimum-contrast floor** on
program-painted text (à la iTerm2's Minimum Contrast): any foreground within a
3.0 WCAG ratio of its background is darkened (light page) or lightened (dark
page) in linear light — hue preserved — just enough to read. White-on-white
after switching a running claude/codex pane to `paper-light` stays legible.

**Config keys** (`$XDG_CONFIG/crew/config.toml`, applied on launch — quit and reopen to pick up external edits):

| Key | Default | Meaning |
|-----|---------|---------|
| `theme` | unset = `auto` | a theme (`dark`, `light`, `crt`, `auto`), one of the palette names (pins it), or a legacy `random-*` / `modern*` mode; unset follows the OS appearance |
| `theme_dark` | unset | while `theme = "auto"`: what dark mode serves — a pool (`dark`\|`light`\|`crt`) or a palette name; unset = the dark pool |
| `theme_light` | unset | while `theme = "auto"`: same for light mode; unset = the light pool |
| `accent` | theme default | `"#rrggbb"` override for the accent (chrome only); omit to use the theme's accent |
| `paper_texture` | `true` | turn the paper grain + vignette pass on/off |
| `paper_grain` | `1.3` | grain strength (`0.0`–`2.0`; `0` = no grain) |
