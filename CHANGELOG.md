# Changelog

Notable changes per release, newest first. Versions are the workspace version
in `Cargo.toml`; every tag builds a release the app picks up through
`/update`.

The top entry must always name the current version — `changelog_covers_the_
current_version` in `crew-app` asserts it, so a release cannot ship without a
line saying what it was.

## 0.13.6

Pixel-snap pane rects. The scene runs in physical pixels end-to-end, but the
grid math handed panes fractional origins — so every glyph could land in any
of four subpixel bins, quadrupling atlas entries and grow churn, and border
hairlines straddled the pixel grid. Pane rects (grid tiles, the zoom tile,
minimized strip thumbnails, zoom-transition frames) now snap edges to whole
device pixels at the single rect source: one atlas bin per glyph, crisper
stems, sharper 1px strokes. Neighbors share the same snapped boundary, so no
seam can open; the glide animation stays fractional in flight and lands on
the snapped target when it settles.

## 0.13.5

CRT goes flat. The holographic glass sheet under every tube pane — a ramped
phosphor fill with a specular hairline and inner edge-glow — read as a drop
shadow that set the panes adrift, floating farther apart than the same grid
on paper-dark. It is retired: CRT panes now sit flush on the page exactly
like the dark themes, and what says "tube" is what should have all along —
a heavier 3.5px frame (the thickest in the app), a stronger bloom on all
four phosphors, and the CRT typeface. Same grid, same gaps, more glow.

## 0.13.4

Focus spotlight. The focused pane holds full ink while every other pane's
content leans 15% toward the page — enough that the eye lands on the active
surface instantly in a full grid, mild enough that unfocused terminals stay
completely readable. The wash rides the same 260ms travel as the focus
brackets: the old pane dims exactly as the new one brightens, one motion.
Frames keep their own focus colors (this is about the ink, not the box),
backgrounds and selection bands keep their shape, and the spotlight follows
the selected pane even while the input bar owns the keyboard.

## 0.13.3

Theme switches are cinematic now. `/theme`, the cycle hotkey, auto-rotation
and an OS appearance flip used to hard-cut every pixel at once; now the new
theme's page washes the window and fades away over ~450ms — the page
*develops* its new look. One full-window quad drawn over everything (under
the CRT tube's curvature when one is active, so the dip stays inside the
glass); detection is a per-frame theme-id diff, so every switch path gets the
fade without knowing about it. Motion off keeps the instant cut, and a grace
frame past the fade guarantees the veil clears exactly to zero.

## 0.13.2

The sidebar's PANES section is alive now. A **crew pulse** line chart sits
under the header — one sample per second of how many panes are doing
background work, auto-scaled, so a working swarm reads as a mountain range
and an idle crew as a flat baseline. Busy panes' rows carry a live spinner
in the accent color where the activity dot sits (attention markers still
win the slot). Costs nothing extra: the chart rides the sidebar's existing
1 Hz refresh and the spinner rides the repaints a busy pane already makes.

## 0.13.1

Grid reflow glides. Opening, closing, minimizing or restoring a pane used to
teleport every surviving pane to its new tile in one frame; now each card
glides there (exponential smoothing toward the placed rect, ~250ms, exact
snap on arrival). Restoring a minimized pane glides it out of wherever it
last sat; leaving zoom glides the pane back into its tile. Clicks during a
glide land on the target tiles — where the panes are heading. Motion off
keeps the old instant snap, and settled panes schedule no frames.

## 0.13.0

Toast notifications. Pane events — agent finished, bell, watched-pattern
match, pane exited, waiting-for-you — and error statuses now step onto the
canvas as toast cards docked at the top-right: a small fieldset card that
slides in, rests for ~5s, then dissolves and slides back out. Waiting, exited
and error toasts border in the bell color; up to four stack, oldest dropped
first. The input-bar flash and LOG entry still happen — the toast is the loud
surface, the bar the quiet one. Motion-gated: at Motion off cards appear and
vanish with no travel, and a resting toast never repaints an idle crew (frames
run only during the slide-in and exit windows).

## 0.12.8

Paper themes are flat again; glass is now the CRT family's look alone. Since
0.12.6 reset stale glass pins, light and dark themes were drawing their
derived frosted sheet — and its drop shadow, painted on the raw pane rect
while the fieldset frame is cell-quantized, ringed every card (and most
visibly the input bar) with a phantom offset box that read as "weird shadow /
misaligned input".

- **Flat dark & light.** The derived glass style for paper themes is now
  fully transparent — no sheet, no drop shadow, no grain; the renderer builds
  no cards at all. CRT themes keep the full holographic sheet and edge-glow.
  Verified on real pixels: paper deltas vs glass-off are 0.0 even at High.
- **The sheet hugs the drawn frame.** Glass cards now span the cell-quantized
  frame (`floor(px/cell)` per axis) instead of the raw rect, so CRT's glow
  can never overhang the border by a stray sub-cell strip either.
- `/glass` strength now only has a visible effect on CRT themes.

## 0.12.7

Auto-focus of blocked panes works on real agents now. Claude Code and Codex
approval prompts were never detected (v0.11.2's heuristic was calibrated
against idealized screens); tested against a live `claude` 2.1.222 run and
Codex's own TUI snapshots, three separate bugs fell out:

- **Waiting is now rendered-tail stability, not PTY-byte silence.** Claude
  Code repaints a blinking `⏺` every ~600 ms while its approval dialog just
  sits there, so the old "quiet for 3 s" byte gate never opened. A pane now
  counts as quiescent when the *text* of its bottom rows stops changing for
  3 s — blinks and OSC title churn don't move that text, while a thinking
  spinner's ticking `(12s · esc to interrupt)` line does, so
  waiting-vs-thinking still holds.

- **The prompt matcher reaches wrapped dialogs.** In a narrow pane, Claude
  Code's wrapped option lines + hint push "Do you want to proceed?" up to 8
  non-empty rows above the bottom — past the old 5-row window. The window is
  now 12 rows, and a `❯`/`›` option selector under a question row matches at
  any depth (Codex uses `›`).

- **Auto-focus no longer forfeits when you were typing.** Focus fired only
  on the exact tick a pane *became* blocked; if that edge landed within 5 s
  of your last keystroke — almost always, in a live session — the move was
  silently skipped forever. Focus is now level-triggered: the pane is
  surfaced as soon as you go hands-off, still once per episode.

Verified end-to-end by a new live-PTY test: a real shell replays the captured
Claude Code dialog with the blink running and must be auto-focused, only
after the stability window, while bytes are still flowing.

## 0.12.6

The CRT theme actually looks like a CRT again. Three fixes to one complaint
("it's just a dark theme with some colors"):

- **Theme switches clear stale look-killing pins.** A bare `/crt` toggle
  persisted `crt = false`, Settings could persist `glass = "off"`, and
  neither was ever cleared — so a months-old pin silently gutted the whole
  post-process (scanlines, bloom, flicker) and the glass sheet on every
  later theme switch. `/theme <x>` (and a composer theme switch) now resets
  the `/crt` pin to auto and glass `off` back to `medium`; a deliberate
  `low`/`high` glass strength survives. A one-shot migration heals existing
  configs on the first launch of 0.12.6.

- **Font smoothing no longer silently reverts.** The v0.12.5 vendored
  glyphon patch covered the render path but not the atlas `grow()` path,
  which re-rasterized every cached glyph unsmoothed — 2px smaller than its
  packed rect and 1px misplaced. The 256² atlas overflows on the first
  Retina frame, so smoothing died seconds into every session. Both
  materialization sites now read the seeded stem-darkened bitmaps.

- **The "updated to crew X" note works for the first time.** `clamped()`
  dropped `last_seen_version` on every config load since the feature
  shipped, so every launch read as a first run — no update note, and no
  version-gated migration could ever fire. The field now survives loading.

## 0.12.5

Font rendering catches up with the native terminals. The same font used to
read thin and rough next to ghostty or Warp because they rasterize through
CoreText — which never hints and dilates stems ("font smoothing") — while
crew's swash path hinted by default and drew exact outlines. Glyphs now
rasterize unhinted at every DPI, and a CoreText-style stem darkening
(fractional dilation, full strength horizontally, half vertically) is
applied to every glyph mask before it reaches the atlas. `/smooth
[off|light|medium|heavy|<0-255>]` tunes the strength live (default 100,
persisted as `font_smooth`); `/smooth off` keeps the unhinted outlines but
drops the darkening. Ships a vendored one-line glyphon patch so seeded
glyph bitmaps actually reach the GPU atlas.

## 0.12.4

Light-trace frames — the holographic overhaul's finale (part 3). On the CRT
themes a focused frame is now drawn in light, not RGB steps: its four
corner cells run white-hot so the bloom turns them into glowing nodes,
gaining focus fires a ~600ms ignition sweep (the whole frame ignites at the
node colour and decays to rest), and a streaming pane's frame breathes on a
slow ~2.4s cycle that rides the redraws it already schedules. Hierarchy
lives in the glow — a grayscale render through the real bloom + composite
chain separates focused from unfocused by halo alone, pixel-asserted — and
idle still converges: sweeps settle, breathing stops at exactly
`border_focused`, and paper themes' frame pixels are untouched.

## 0.12.3

CRT glass turns luminous (holographic overhaul, part 2). The "faintest of
the family" doctrine is repealed and the sheet becomes a translucent
phosphor panel — more opaque than paper-dark's glass, tinted in the tube's
own hue, with an inner edge-glow that makes each pane body read as lit by
its own frame. Paper light/dark pixels are untouched (proven
byte-identical), and the translucent-window path still shows the desktop
through the sheet.

## 0.12.2

The CRT holographic overhaul begins (goal 2026-08-04). `Theme.crt` grows
from a bool into a per-theme `CrtStyle` — the four phosphors finally get
personalities instead of sharing four compile-time constants — and the old
two-ring neighbour sample is replaced by a real half-res gaussian bloom
chain, so a focused border radiates tens of pixels instead of dying at ~8.

## 0.12.1

`/login` and `/logout` — OAuth becomes reachable. The 0.12.0 sign-in
existed but was invisible in practice: the pane's `/model 2` opened the
catalog popup (Enter then accepted a filtered model row, or its API-key
paste card) instead of submitting the broker's numbered provider pick, and
a key already sitting in a shell rc re-labeled the provider "key present",
hiding the sign-in row entirely — so the only path anyone ever saw was
"paste a key". Now: a purely numeric `/model <n>` goes straight to the
broker; `/login` lists every provider that offers a sign-in (device-flow
rows numbered and offered even with a key present, vendor CLIs with their
exact command) and `/login <name|n>` runs the flow in-pane; a stored grant
now OUTRANKS a key — an explicit sign-in is the strongest signal a user can
send, and the grant's endpoint travels with it — with `/logout [provider]`
removing the grant so the key serves again. Proven end-to-end: a broker
with a decoy `DASHSCOPE_API_KEY` still lists the numbered sign-in, completes
the stubbed device flow, and answers chat through the grant's endpoint.

## 0.12.0

Sign in, don't paste keys — the OAuth goal's final slice. A signed-out
device-flow provider (Qwen/DashScope) is now a numbered row in `/model`'s
picker, and picking it runs the RFC 8628 sign-in right in the pane: a code
card streams into the chat, crew polls while you approve in the browser
(`/stop` cancels), and the grant is stored and selected — a clean machine
with zero API keys reaches a working smith pane that answers, fans out, and
drafts plans, proven end-to-end against a stubbed OAuth + chat server.
Grants live in the macOS keychain (`security`, probed never assumed; a 0600
file where no keychain exists — `/doctor`'s new "token store" line says
which). Access tokens refresh transparently before model calls; a hard
refresh failure discards the dead grant and prints exactly one "sign-in
expired — open /model" line. Nothing token-shaped can reach any log sink: a
sweep test greps the session logs, broker stdout/stderr, doctor output, and
the whole redirected HOME after a full stubbed round-trip and asserts zero
hits.

## 0.11.9

*Sign in, don't paste keys: subscriptions become first-class providers.*

- **The provider auth registry** (`broker/auth/`): every provider is a data
  row — name, auth modes (`cli-delegated` / `oauth-device` / `api-key`), key
  variable, and for delegated providers the vendor CLI that owns the
  sign-in. One resolution function orders discovery: explicit pin, then
  **signed-in subscriptions**, then API keys (historic order, unchanged),
  then installed CLIs. Routing and planning code no longer know provider
  names.
- **A signed-in Claude Code or Codex seat now serves smith work with no API
  key.** Crew asks the CLI itself (`claude auth status` / `codex login
  status` — consent-based, never another app's token store; 5s timeout,
  probed once per pane, `CREW_SUBSCRIPTIONS=0` opts out) and routes plain
  tasks through that CLI via the existing relay. The swarm on a
  delegated-only machine degrades to the stub planner exactly as keyless
  always has — `/doctor` says so instead of erroring.
- **`/model` becomes the whole model story**: a grouped picker — "your
  subscriptions", "your keys", "installed CLIs", each entry numbered —
  with signed-out providers grayed showing the exact sign-in command, and
  `/model <n>` switching provider through the stored pin (survives
  restarts). `CREW_PROVIDER=claude-code|codex` pins the delegated rung
  explicitly.
- **`/doctor` gains a per-provider auth line** — signed in / signed out
  (+ sign-in command) / key present / no key / not installed — states
  only; a test greps the full gather→render round-trip with a fake stored
  key to prove no key material can reach the report.

## 0.11.8

*The command diet completes: seven constructs, everything else is conversation.*

(Released as 0.11.8: the autonomy branch merged and bumped past 0.11.7 in one
motion, so that number never shipped an entry of its own.)

- **`/goal`, `/plan`, `/approve`, `/reject`, `/skill`, `/memory` and `/mcp`
  are retired** — constructs shrink from 14 to the infrastructure seven
  (`/help` `/model` `/doctor` `/restore` `/reload` `/diff` `/stop`). "Keep
  working until …" runs the judge loop, "draft a plan for …" enters plan
  mode, and typing an old slash form teaches the phrasing instantly.
- **The plan gate survives as conversation.** With a plan pending, "approve"
  / "run it" executes it and "reject" / "drop it" discards it — matched
  exactly, before any model call, so a misrouted message can never run or
  drop a plan; anything else leaves the draft pending. The pane's enter/esc
  now send those same bare words.
- **Skills apply themselves.** A task that names a loaded playbook gets it
  woven into the relay or swarm prompt automatically (at most two); loaded
  but unmatched skills ride along as a one-line roster. The drop-in
  `.crew/skills` surface is unchanged.
- **Memory answers to plain language** — "what do you remember?" surfaces
  the standing block that already rides every task; `#<note>` still saves
  one. **`/doctor` absorbs the `/mcp` listing**: each server renders with
  its tools (or its failure) under the count line.
- **Transcript history is summarized, never dropped.** When a relay outgrows
  its 8-entry window, the overflow folds into a running `[compacted N
  earlier messages: …]` block by one bounded model call, so an early
  decision stays in every later prompt; the session log folds its oldest
  half the same way. Keyless, mock, or a failed call keep the old clipping —
  degraded context, never an error — and the retained block is byte-capped.
- **The swarm graph unfreezes on failure.** The first failed task pauses
  dispatch and asks the planner — given the goal, the completed outputs
  (budget-clipped) and the error — for a replacement of the not-yet-run
  remainder; completed work is never re-run, and one re-plan per run is the
  hard cap. Keyless/mock runs and planner errors keep today's
  cascade-cancel, and re-planned tasks pass the same Api/Standard forcing
  as the original plan — a re-plan can never widen what a plan may execute.

## 0.11.6

*Smith trusts the model: say it, don't slash it.*

- **The intent router decides the shape.** A plain message gets one cheap
  classification call and routes itself — single reply, fan-out, refinement
  loop, plan-with-approval, or swarm — falling back to the swarm whenever
  the classifier can't run (no key, mock provider, `CREW_INTENT=0`).
- **`/fan`, `/loop`, `/commit`, `/review`, `/standup` and `/resume` are
  retired.** Every capability lives on behind plain language — "have every
  agent take a crack at …", "keep refining …", "commit this", "look over my
  changes", "what did I ship this week?", "pick up where we left off" — and
  typing the old slash form teaches the phrasing instantly.
- **The commit gate survives retirement.** "commit this" only ever drafts;
  the commit is created when you say "apply", matched exactly and never by
  the model, so a misrouted message can't commit.
- **crew-hive prompts are budget-aware.** Dependency outputs are clipped
  fairly (4k per dep, 12k total, visible markers) instead of concatenated
  unbounded.

## 0.11.5

*The tube gets a typeface from this decade.*

- **Modern monospace only.** Pre-Retina `Monaco` is out of the auto-select
  allowlist and out of the CRT themes' font lists — `Lilex` leads the tube
  now (the contemporary take on that IBM-terminal DNA), backed by JetBrains
  Mono, Google Sans Code, and Cascadia Mono. The allowlist gains today's
  faces — Lilex NF, Berkeley Mono, Commit Mono, Martian Mono, Cascadia Mono,
  GeistMono NF — so the rotation and theme resolution can land on them when
  installed. Menlo survives strictly as the never-fail OS tail of each list,
  and a new test pins that no theme may *lead* with a stock fallback face.

## 0.11.4

*Two commands walk in, one walks out.*

- **`/update` and `/restart` merged.** `/update` now downloads the latest
  release, installs it, and — after a brief "restarting…" beat on the UPDATE
  card — relaunches Crew into the new build by itself. An update the quiet
  background check already installed is applied instantly: `/update` sees the
  parked install and restarts without a network round-trip. The blinking nav
  reminder now reads `· /update`. Typing `/restart` gets a pointer to
  `/update` instead of a fuzzy-match guess at `/restore`. Only a run you
  typed ever restarts the app — the background check still parks quietly.

## 0.11.3

*The footer knows who's on shift.*

- **The footer mode line names the agents working right now** instead of a
  static roster, and the composer border sheds the roster strip — one line
  fewer, more signal.

## 0.11.2

*A pane waiting on you no longer waits in silence.*

- **Blocked panes surface themselves.** When a coding agent stops to ask
  something — a `(y/n)`, a "Do you want…", a permission question — or the
  smith pane has a plan pending, the pane raises a `?` attention badge and
  a "waiting for you" note; if you've been idle a few seconds and aren't
  already looking at a blocked pane, focus jumps there (once per episode,
  never while you're typing). **Cmd+.** cycles through waiting panes.
  The terminal-side detector is deliberately conservative: a foreground
  command, three quiet seconds, an unscrolled view, and question-shaped
  text in the last rows — an idle shell prompt never counts, and a
  thinking agent that keeps painting is left alone.

## 0.11.1

*The rough edges from a night of shipping, sanded.*

- **Fold toggles fire on mouse release, not press** — starting a drag
  selection over a folded card no longer expands it mid-gesture, and a
  click that merely focuses the pane can't toggle a card by accident.
- **A streaming card you clicked open stays open when it settles.**
- **Per-reply usage trailers verify the sender** before attaching, so a
  future interleaved broker message can never wear another reply's cost.
- **A contrast tripwire now guards the diff/checkmark inks** in every
  theme (measured floors: ≥4.5 vs the page, ≥3.2 vs the code card), so a
  future palette can't quietly slide red/green under readability.
- **Docs caught up with the night**: README, docs/CREW.md and the `/keys`
  overlay now cover diff fences, checklists, Ctrl+R, usage trailers,
  folding cards, drag-and-drop, quoted mentions and Cmd+F.

## 0.11.0

*Find anything you ever said — or were told.*

- **Cmd+F searches the conversation.** A find bar over the chat transcript:
  type to match case-insensitively across every message (folded system
  cards included), Enter/Ctrl+F/↓ steps to older matches, ↑ to newer, both
  wrapping, with a `find: query (k/N)` count in the legend. Jumping scrolls
  the transcript to the match — expanding a folded card when the hit lives
  in its hidden tail — and washes the matched text with the same highlight
  the terminal `/find` uses. Esc closes without touching your draft; one
  modal at a time with Ctrl+R and the popups.

## 0.10.9

*A bug-hunt pass over everything the night shipped.*

- **File drops are robust now.** A drop mid-sentence no longer glues onto
  the last word (which silently lost the attachment), paths with spaces
  round-trip as `@"quoted mentions"` end to end, a drop while the Ctrl+R
  popup is open lands in the draft instead of vanishing, and dropped-path
  relativization uses the exact cwd the send path resolves against.
- **Ctrl+R popup hygiene.** Accepting a recalled line disarms any palette
  or mention popup that was open — previously the stale popup resurfaced
  and ate the next Enter, able to run an old palette row against the
  recalled text.
- **Clicks no longer fall through popups** onto the fold toggles of system
  cards hidden beneath them.
- **Oversized mentions are refused before reading.** A dropped multi-GB
  file used to be read whole on the UI thread at send before the 64 KB cap
  was checked; the size gate now runs on metadata first.
- **Compact view stops counting metadata.** The per-reply usage trailer no
  longer produces a misleading ` … +1` on every single-line reply under
  Ctrl+O.
- **Two real test flakes fixed** — the phantom full-suite failure traced to
  torn reads of the process-global theme, and a ~25% `every_animation_
  terminates` flake from the unguarded motion level; both now serialize on
  the shared test guard (verified over eight consecutive clean runs).

## 0.10.8

*Drop a file on crew and it lands where you're working.*

- **Files dragged onto the window drop into the focused pane.** Onto a chat
  pane they become an `@mention` — relative when the file lives under the
  pane's project, absolute otherwise — ready for the mention machinery to
  inline at send. Onto a terminal they type the shell-quoted path, spaces
  and quotes escaped, through the same bracketed-paste path as Cmd+V.
  Multiple files append in order. (macOS never reports a cursor position
  during an OS drag, so the focused pane — not the pane under the mouse —
  is the deliberate target.)

## 0.10.7

*Broker noise folds itself away.*

- **Long system/telemetry cards auto-collapse.** A system-voice card (turn
  summaries, `/doctor` dumps, roster lists) longer than three lines renders
  as its header plus first line with the muted ` … +N` count. Click it to
  expand; click the expanded header to fold it back. Agent replies and your
  own messages never fold, the splash art is exempt, and the pane-global
  Ctrl+O compact view still clamps everything as before.

## 0.10.6

*Every reply owns up to what it cost.*

- **Settled agent replies carry a muted usage trailer** — `900 in / 50 out ·
  $0.012` — from the broker's per-reply stats, formatted like the footer.
  Zero-usage replies (CLI agents, dangling segments) show nothing rather
  than `0 in / 0 out`; unpriced models show tokens without a fabricated
  `$0.000`; streaming cards stay trailer-free until the reply settles.

## 0.10.5

*Find that message again: Ctrl+R searches your chat history.*

- **Ctrl+R in the chat composer opens reverse history search.** Type to
  filter your previously sent messages (substring hits first, fuzzy after),
  newest first; Ctrl+R or ↓ steps older, ↑ newer, Enter puts the match back
  in the composer without sending, Esc restores whatever you had typed.
  Chat-pane only — terminal panes still pass every key through.

## 0.10.4

*Agent plans read like plans: task lists render as real checklists.*

- **`- [ ]` / `- [x]` markdown task lists render as checklists in the chat
  pane.** An unchecked item draws `☐` in place of the bullet; a checked item
  draws a green `✓` with its text dimmed, so a Claude Code-style plan shows
  its progress at a glance. Uppercase `[X]` counts as checked, wrapped items
  keep their hanging indent (the dim survives the wrap), nested lists nest,
  and `- [normal] text` or task syntax inside a code fence stays untouched.

## 0.10.3

*Diffs in the chat transcript colour like diffs.*

- **```diff and ```patch fences colour by line.** Added lines green, removed
  lines red, `@@` hunk headers cyan, file headers (`+++`/`---`/`diff --git`/
  `index`) dimmed — the same colours the file viewer gives an opened `.patch`,
  so chat and viewer agree. An *untagged* fence whose body reads as a diff
  (a `diff --git` opener, or a hunk header alongside real `+`/`-` change
  lines) gets the treatment too, since agents rarely tag a paste.
  (This entry was backfilled — 0.10.3 shipped without one.)

## 0.10.2

*The model segment survives a Dock launch — and names the swarm actually
serving you, live.*

- **The smith pane's roster no longer vanishes when crew is launched from the
  Dock or Finder.** Launched that way the app — and so its broker child — runs
  at `/`. The broker treats its CWD as the project, so it found no
  `.crew/specialists.json`, emitted an *empty* roster, and the footer's new
  model segment had nothing to show (while its session log and `/resume`
  quietly wrote to nowhere). Plugin panes now spawn their broker in the pane's
  tracked directory; session restore steers that to the saved project before
  the spawn, so a restored `/smith` gets its project — roster, log and all.
- **A running swarm stamps the roster with its serving model, from memory.**
  The post-planning roster re-emit used to re-read specialists from disk,
  which came back empty exactly when the project dir wasn't writable. The
  invented cast now goes out directly, each specialist carrying the model slug
  serving the run — so line 1 reads `model · N agents` in real time, whatever
  directory the broker woke up in. Discovery still appends CLI and manifest
  agents the cast doesn't name.

## 0.10.1

- **Footer line 1 leads with the serving model.** `qwen-max · 7 agents`
  instead of a name list: when every API specialist agrees on a model, that
  model is the roster's identity, even with CLI agents (which report none)
  riding along. (This entry was backfilled — 0.10.1 shipped without one.)

## 0.10.0

*Read any file where you already are — and a title bar that stopped being
see-through.*

- **`/view <path>` opens any file in a pane, and `/md` is now an alias for it.**
  One read-only viewer replaces the old source|preview split, over a ladder of
  formats: code with syntax colour, markdown **rendered only** (the source half
  is gone — showing markdown source beside its render was a dev tool wearing a
  reading experience's clothes), json/yaml/toml, csv laid out as a table, and
  diffs with `+`/`−` ink.
- **PDF and Word open as honest text extracts, not fake page renders.** A
  monospace grid has no business impersonating a page, so those rungs say what
  they are in a banner and offer `o` to hand the file to the real application.
  Extractors are *probed, never required* — macOS's own `textutil` covers
  docx/rtf, and a missing `pdftotext` degrades that one rung instead of
  erroring. Binaries get a metadata card naming size, kind and mtime.
- **Nothing about it blocks the grid.** Detection, reading and extraction all
  run on a worker thread, so the pane is on screen before a byte is read: a
  stalled network mount or a 300-page `pdftotext` can no longer freeze every
  other pane, agents included. Both caps — 8 MB of bytes, 50 000 rendered lines
  — announce themselves rather than silently truncating.
- **`/far`'s F3 and F4 stopped lying.** Both used to hand the file to the OS and
  throw you out of crew; F3 now views it in place and F4 opens `$EDITOR` in a
  crew pane. Cmd+click resolves a path anywhere — including one an agent wrote
  in a reply — and `e` inside the viewer hands off to `$EDITOR` and **reloads
  when it exits**, keeping your scroll position. Open viewers survive a restart.
- **Search inside a file** with `/`, `n` and `N`; `s` shows raw source; Esc
  closes and restores the focus and zoom you came from.
- **Fixed: the title bar was see-through.** crew asks to be transparency-capable
  at window creation so Opacity % can take effect without a restart — but on
  macOS that leaves the window non-opaque for good, and the title bar is drawn
  by the OS against whatever is behind it. Panes were solid while the chrome
  showed the desktop. Being transparency-*capable* and being transparent are
  separable, so the runtime flag now follows the setting.

## 0.9.0

*Jarvis motion, iteration 6 of 6 — the goal is complete.*

- **The idle invariant is now a test, not an intention.** The condition
  deciding whether crew schedules another frame is a named predicate,
  `wants_animation_frame`, and four tests hold it to its contract: a settled app
  asks for nothing, `Motion = off` schedules nothing even in the same frame as a
  close and a focus change, full motion *does* animate a fresh event (without
  which the first test would pass on an app that never animated), and everything
  in flight is over within half a minute. A timeline that never settled would
  keep crew awake forever — the one failure mode here that costs battery rather
  than pixels.
- **Every theme still reads correctly.** All five glass renders verified
  through the pixel harness against their sheet-off baselines.

Six iterations in: focus brackets, cards that assemble and collapse, zoom that
travels, readouts that count, a live streaming caret, and a scan that sweeps a
working pane's glass — with no new colour anywhere, and an idle crew that
repaints exactly as rarely as it did before any of it.

## 0.8.5

*Jarvis motion, iteration 5 of 6.*

- **The glass sheet sweeps while a pane works.** A soft band of light travels
  down a busy card and back, drawn in `glass.wgsl` as part of the existing card
  pass — no second draw call, no new colour, and it rides the sheet's own fill
  so every theme carries it.
- **Gated on busy and nothing else.** A working pane already repaints at ~15fps,
  so the sweep costs no extra frames; an idle crew draws no scan at all. That is
  the whole trick to having an ambient layer without breaking "an idle crew
  never repaints".
- **The shader change is pixel-tested.** `glass_scan_headless` renders the same
  flat card with and without a scan and asserts the band brightens where it
  passes and nowhere else. A uniform that is plumbed but never read looks
  exactly like a working feature from the Rust side — which is precisely how
  0.7.0 shipped glass that drew nothing.

## 0.8.4

*Jarvis motion, iteration 4 of 6.*

- **Streaming cards carry a live caret.** A block on the end of the last line,
  pulsing between the muted and accent colours, marking text that is still
  arriving as distinct from a reply that happened to end mid-sentence. It
  pulses rather than blinks — a caret that vanishes half the time reads like
  the text stopped. `Motion = off` keeps it (it carries information) but holds
  it still.
- **Where the stream begins is now part of the view.** `View.streaming_from`
  rides the same value already threaded through every card render path, so the
  scroll clamp, the scrollbar, the link hit-test and the transcript cannot
  disagree about which cards are live.

## 0.8.3

*Jarvis motion, iteration 3 of 6.*

- **The footer counts.** Cost and token totals sweep to their new values rather
  than snapping, and the 5h-budget and context meters fill rather than jumping a
  cell. A number that jumps reads as a repaint; the same number sweeping reads
  as an instrument.
- **Counters live on the pane that draws them.** The first version kept them in
  a process-wide registry keyed by name, which read fine until you noticed two
  chat panes would share one `footer.cost` and overwrite each other — and which
  the footer's own tests caught within minutes by leaking values between cases.
  They are `Cell`-based fields on `ChatPane` now, since the footer renders from
  an immutable pane.
- **First sight is settled.** Sweeping every number up from zero the first time
  it was drawn made the footer's output depend on how many times it had been
  drawn before, which is not a property a footer should have. Only a value that
  changes sweeps.

## 0.8.2

*Jarvis motion, iteration 2b of 6.*

- **Dismissed panes collapse.** A closed card's frame retracts into its corners
  — the assemble run backwards — and a minimized one retracts *while travelling
  toward the nav*, which is where the pane has actually gone. The two gestures
  now look different, because they are different.
- **Ghosts, not lingering panes.** The pane leaves `panes` the instant it is
  dismissed; it has to, since focus clamping, the grid LRU and the nav rows all
  read that vector and would otherwise operate on something the user just
  dismissed. What outlives it is an inert record — a rect, a title, a timestamp
  and a direction. It holds no process and no channel, because a ghost that
  could still *do* something would be a pane that refused to die. Each is
  bounded by its own timeline and pruned every frame.
- **Restoring is an arrival.** A pane coming back out of the nav re-stamps its
  birth clock and assembles exactly as a new one does.
- **Zoom travels out of its tile.** Cmd+Z expands from the rect the pane
  occupied instead of cutting to full size, and collapses back into it.

## 0.8.1

*Jarvis motion, iteration 2 of 6.*

- **Panes assemble instead of appearing.** A new card's frame draws itself
  outward from its four corners over its first moments. Only the frame *stroke*
  animates — an early version clipped the top border's legend too, spelling the
  pane's name out one letter at a time, and a card you cannot identify is worse
  than one that simply appeared. The name is there from the first frame and the
  frame draws itself around it.
- **Every pane carries its own birth stamp.** `Pane.born_ms` rather than a
  timeline held by the app: panes are pushed, closed and reordered from a dozen
  places, and anything keyed on position or count would animate the wrong card
  the first time two of those happened in one frame.
- **Motion is read process-wide.** `motion::level()` mirrors `palette::accent` and
  `crew_theme::theme`, so a new animation respects the setting without any
  plumbing — and `apply_config` publishes it, which means Save, session restore
  and an external config edit all land on the same path.

## 0.8.0

*Jarvis motion, iteration 1 of 6 — see `.superpowers/sdd/2026-07-28-jarvis-motion/`.*

- **Motion is a setting now.** Settings → APPEARANCE → **Motion**:
  `off · subtle · full`. `off` is a real off rather than a fast one — every
  animation's duration collapses to zero, so it draws its final state once and
  schedules no further frames, and an idle crew repaints exactly as rarely as
  it did before any of this existed.
- **The focused pane wears HUD brackets.** Short accent runs down the card's
  edges from each of the four corners, growing out as focus arrives and
  travelling with it as you move between panes. They stay off the top border,
  where the legend and the `[-][x]` buttons live — decoration must not overwrite
  information.
- **A motion vocabulary to build the rest on.** `ease.rs` adds an ease-out curve
  and `Timeline`, a *bounded* window that reports when it still has frames to
  draw. `poll` schedules redraws from exactly that, which is what keeps "an idle
  crew never repaints" true while crew animates. No timers, no threads, no
  sleeps: every animation is a pure function of `anim::now_ms()`, so none of it
  can stall the winit thread.

## 0.7.1

- **Glass actually renders now.** 0.7.0 shipped "every pane sits on glass" and
  drew no sheet at all. The card was emitted only for scenes with
  `PaneScene.bordered`, a flag that went dead when panes started drawing their
  frames as cells — every scene the app builds sets it `false`, so `GlassLayer`
  received zero cards and the only visible effect of the feature was the window
  translucency, which rides a different path. The sheet now has its own `glass`
  flag, set on the scene that spans the whole pane rect, and panes, panels, the
  minimized strip and the input bar all carry it. Overlay popups stay opaque.
- **The pixel test can fail now.** The headless harness hand-built its panes
  with `bordered: true` — an arrangement production never produces — which is
  exactly why a feature that drew nothing kept a passing GPU test. It renders
  scenes from `paneview::build_scenes` instead, and compares each theme against
  the same frame with the sheet off rather than against the gap between panes
  (a pane's own cell backgrounds satisfied the old assertion whether or not any
  glass was drawn).
- **Glass moved into Settings; `/glass` is gone.** Strength lives in APPEARANCE
  as **Glass** (←/→/Space through `off · low · medium · high`) and window
  translucency in WINDOW as **Opacity %**, typed as a percentage and floored at
  35%. Saving pushes both to the renderer, so they apply live.

## 0.7.0

- **Crew never disappears in silence again.** A panic on the winit thread used
  to take the whole app down leaving *nothing* behind: a detached crew runs with
  stderr on `/dev/null`, and a panic exits through the normal path so macOS
  files no crash report — the window simply vanished. There is now a panic hook
  that writes the message, location and backtrace to `crash.log` beside the
  config, the detached child's stderr goes to `stderr.log` instead of being
  thrown away, and the next launch says what happened and where to look.
- **The window close button now confirms too.** Cmd+Q has always asked twice
  when panes are open, so a stray keystroke couldn't kill running shells and
  agents — but the traffic-light button called exit outright, and it is the
  easier of the two to hit by accident (a misplaced keystroke usually just
  lands in a pane; a misplaced click doesn't). Both paths now share one guard,
  and the prompt names how many panes are at stake.
- **Frosted glass on every theme.** Each pane card now sits on a translucent
  sheet — a fill that fades from the top down, a specular hairline along the
  upper edge, a soft drop shadow, and a whisper of frost grain. The look is
  *derived* from the active theme rather than configured per palette, so all
  thirteen get their own treatment and no future theme can ship without one:
  dark themes lift a lighter sheet off the page, light themes lean on a whiter
  sheet plus a real shadow, and CRT stays faint and grain-free because the tube
  already supplies bloom and noise. `/glass [off|low|medium|high]`.
- **`/glass window <pct>`** makes the window itself translucent, so the desktop
  shows through the page while text and pane fills stay solid. Works under the
  CRT post-process too. Opacity floors at 35% — a window any sheerer is one you
  cannot find again.
- Three keyboard shortcuts the app has always answered are now in the `/keys`
  overlay: **Ctrl+Shift+L** (cycle themes) and **Ctrl+Shift+M** (chat markdown
  preview ↔ source), which the manual documented and the overlay did not, and
  **Ctrl+O** (compact transcript view), which was implemented, tested, and
  listed nowhere a user could find it.
- The overlay and the manual are now checked against each other, so a binding
  cannot be added to one and forgotten in the other. The manual said `/keys`
  shows "this list in-app"; that is now true.

## 0.6.99

- `/crt` and `/weight` are in the manual. Both have been working, palette-listed
  commands with no mention in either page — `/crt [on|off|auto]` runs the CRT
  tube post-process independently of the theme, so a paper theme can go through
  the tube or a CRT palette can render flat.
- A link in the manual pointing at `#multi-agent-relay-crew` has gone nowhere
  since the `/smith` rename. Internal doc links are now checked against the
  headings they claim to target.

## 0.6.98

- Six environment knobs the code has always read are now in the manual:
  `CREW_HTTP_TIMEOUT_MS` and `CREW_STREAM_TEXT`, the three plugin-path
  overrides (`CREW_BROKER_PLUGIN`, `CREW_CHAT_PLUGIN`,
  `CREW_ORCHESTRATOR_PLUGIN`) that let one pane run a debug build while the
  rest of the app stays on the installed release, and `CREW_PANE`.
- `CREW_SYS_TIMEOUT_MS` was documented twice with two different defaults. The
  stale copy said 30000; it is 120000.
- Every `CREW_*` name in the shipped source is now either documented or
  declared internal in one list, so a new knob cannot arrive unmentioned.

## 0.6.97

- `/export` on a pane with no messages no longer writes a file. It reported
  success and left a 68-byte "0 message(s)" transcript wherever crew happened
  to be running; 64 of them had accumulated in this repo and 54 were committed.
  An export of nothing now says so and touches nothing.
- Stray `crew-transcript-*.md` files are gitignored, so a real export made
  inside a repo cannot be committed by accident.

## 0.6.96

- The input bar says which pane it is talking to. The focused pane's name now
  rides the bottom border as a right-aligned legend, mirroring the working
  directory on the top one — typing there has always acted on the selected
  pane, and the bar never said which one that was. A transient status message
  borrows the slot while it flashes and gives it back.

## 0.6.95

- The slash palette stopped disagreeing with the commands it labels. `/goal`
  was described as "set the crew's shared goal" — a feature that exists
  nowhere; it runs relay rounds until a judge agent rules the goal met. Every
  hint the pane does not deliberately own is now the broker's own `/help`
  sentence, so the two cannot drift apart again.
- `/goal` means two different things and now says so. In the agent smith
  composer it is the judged relay; in the command bar it plans a task graph and
  runs it as a swarm. Same word, two engines — the docs and the command-bar
  palette both name the split.
- The shell's deadline fits a real build. `sys:run` had 30 seconds, which
  `cargo test` cannot finish inside on any real project, so the shell's most
  valuable use was its broken one. It is 120 seconds now, and the timeout
  message names `CREW_SYS_TIMEOUT_MS` instead of leaving you to guess a knob
  exists.

## 0.6.94

- Three providers that shipped fully wired and entirely unmentioned are
  documented: **OpenAI**, **Gemini** and **DeepSeek** each have a key variable,
  an endpoint and a native model chain, and neither the README nor the manual
  ever said so. `docs/CREW.md` now carries the table.
- They probe **last** in discovery on purpose, so adding one never changes
  which provider an existing install resolves to — which means
  `CREW_PROVIDER=gemini` is the only way to reach one while another key is set.
  That pin worked all along and was documented nowhere, including in the
  function's own comment.

## 0.6.93

- An agent that runs out of tool calls in a turn says so. There is a budget of
  four per turn; spending it just ended the turn, leaving the agent's unrun
  `@tool` line standing as its answer with nothing anywhere explaining why it
  stopped halfway through what it was doing.
- The agent is also told how many it has left, and told plainly when the next
  one is its last. A budget it cannot see is one it plans straight past.

## 0.6.92

- `@src/main.rs:120-180` attaches just those lines. `@file` attached the whole
  file and anything past 64 KB was skipped outright — so the file you most
  want to point an agent at one function of was the file you could not point
  it at at all. `:120` is a single line; the size limit now applies to what is
  attached rather than to the file on disk, and the "too large" note names the
  idiom instead of only saying no.
- A file genuinely named `odd:10` still wins over reading that as a range, and
  a colon anywhere else in a name is left alone.

## 0.6.91

- A tool call reads as what it is. Every call an agent makes is logged in the
  pane so you can see what is happening to your machine, but it was shown in
  wire form — `[tool] sys:write_file {"path": "src/lib.rs", "content": "use
  std::fmt;\n…` — where the one thing worth reading is buried in punctuation
  and, for a write, the line is mostly the first 150 bytes of the file. It now
  reads `sys:write_file  src/lib.rs`. A tool whose arguments say nothing
  identifiable still shows them, so nothing goes missing.

## 0.6.90

- The built-in `sys` tool surface is decided once, when a session's broker is
  built, instead of being re-read from the environment on every hint and every
  tool call. It is meant to be stable for the life of a session, and one of
  the variables it read is set and cleared by the test harness while other
  tests are running — which made two tests fail about one run in six.

## 0.6.89

- A test no longer switches text streaming off for the whole process. Two
  tests set `CREW_STREAM_TEXT` and were serialised against each other but not
  against anything else, so any test building a text streamer at that instant
  saw streaming disabled. The switch is handed in now; nothing mutates the
  environment.
- Typing `/stop` drops the queue too. Esc has since the last release; the
  typed form is the same intention and sends the same string, and it was
  still stopping one run and starting everything behind it. `/stop #2` is
  left alone — calling off one of several parallel tasks says nothing about
  the rest of the session.

## 0.6.88

- Interrupting also drops what you queued behind the run. Esc stopped the
  agent, which made the pane idle, which is exactly what flushes the queue —
  so a stop immediately started every follow-up waiting behind it, each one
  written on the premise that the interrupted work was going fine. It says how
  many it dropped.
- Three tests that assert something is absent from a rendered pane now check
  the pane rendered at all first. None of them were passing vacuously; the
  guard is so that they cannot start to.

## 0.6.87

- A pane opened in a project you have worked in before says so, and names
  `/resume`. The previous conversation has been rotated into a resumable file
  since long before this and announced nowhere, so the only way to find out it
  was there was to guess the construct and try it. It offers rather than
  resumes: folding yesterday's conversation into today's first task spends
  context nobody asked for, and yesterday's task is as often unrelated as not.
- A first run in a fresh project still says nothing.

## 0.6.86

- The agent composer suggests the rest of a prompt you have sent before, dim
  after the caret, taken with Tab or Right. The docked input bar has done this
  since long before; the composer — where the long prompts are actually typed
  — did not. Tab keeps its existing meaning and gains this only where it used
  to do nothing.
- A leading `/` is left to the palette, which is already answering the same
  question as a popup.

## 0.6.85

- `/restore` removes the files the task created, instead of putting the edits
  back and leaving the new ones for you to find and delete by hand. An undo
  that covers half a change is not an undo, and the half it skipped was the
  one most likely to be unwanted — the half-written module a run was abandoned
  over. It names every file it deletes.
- Nothing that predates the snapshot is touched, and ignored files — build
  output, an untracked `.env` — are never candidates.

## 0.6.84

- `/review` reviews the files the agent created, and `/commit` describes them.
  Both read one diff, and it came from `git diff` — which cannot see a path git
  has never been told about. A review of a change whose main file is a new
  module was a review of everything except the code, reported as a clean pass.
- `/commit apply` commits what it described. It ran `git commit -am`, which
  stages tracked modifications and nothing else, so the new file stayed behind
  while the pane reported a successful commit. Staging a subset by hand still
  commits exactly that subset — that is a statement about which change you
  mean, and it is still respected.

## 0.6.83

- `/diff` shows files the agent created. It ran `git diff --stat`, which
  compares the working tree to the index — so a new file was untracked and
  invisible, and anything staged was excluded too. Those are the two states an
  agent most often leaves a repository in, and both printed "working tree
  clean — no changes". It now compares everything that exists against the last
  commit, and works in a repository with no commits at all.
- It excludes `.crew/` for the same reason the end-of-task note does, so the
  note and the command it names cannot disagree.

## 0.6.82

- A finished task says which of your files it changed. A clean run printed its
  reply and nothing else, so an agent that edited four files and answered
  "done" left no record anywhere of which four — you had to know about `/diff`
  and remember to run it. No new bookkeeping: the checkpoint taken before
  every task already pins the tree to compare against.
- It stays quiet when nothing of yours changed, which needed care — the broker
  rewrites `.crew/session-live.md` as every reply streams, so the answer to
  "what changed?" would otherwise have been "the transcript" after every
  question ever asked, in every repo but crew's own.

## 0.6.81

- Up and Down in an agent pane recall what you already sent, filtered by what
  you have typed — the docked input bar's rule, which the composer did not
  share. The arrows were classified for the popups and then dropped when no
  popup was open, so changing one word of a long prompt meant retyping it.
- `/keys` shows each description in full. The overlay was a fixed 58 columns
  wide, which left 30 for a description, and eight rows had outgrown it: `Esc`
  read "Discard a pending plan · inter", losing both the interrupt and the
  close, and `@a+b` lost "in parallel".

## 0.6.80

- The provider-key tests assert the same thing on every machine. They read the
  process environment and asserted only when the key was absent, so on any
  machine that had one they passed without testing anything — and they never
  covered an exported but blank key, which is the case that matters.

## 0.6.79

- The `@` picker says that `@a+b` fans a task out to both agents in parallel,
  and `/keys` lists it. The idiom worked and was documented in exactly one
  place the app never shows.

## 0.6.78

- Cmd+click a code block in an agent pane to copy it. Reading an answer and
  using it are different acts, and the second had no support beyond selecting
  text with the mouse. The fence chrome and language tag do not come with it,
  so there is nothing to strip after pasting.

## 0.6.77

- The first automatic snapshot of a session says so, once, and names
  `/restore`. Checkpoints have been automatic and entirely silent since
  0.6.44 — an undo nobody knows about is an undo nobody uses.

## 0.6.76

- The welcome screen names the agent pane. It said "new shell" and "commands"
  and nothing else, so the reason crew is not just a terminal went unmentioned
  on the one screen a first run is guaranteed to see. It also picks a form
  that fits the window instead of vanishing on a narrow one.
- The installer says that agents work with no API key at all when you are
  already signed in to claude, codex or opencode.

## 0.6.75

- Grouped the entries below by what the work was for. Thirteen consecutive
  per-release sections trace history well and read badly; the run was one body
  of work and now says so.

## 0.6.62 – 0.6.74

The second half of the same run. Mostly the consequences of the first half —
six providers where there had been three, ten fewer constructs — arriving as
messages that named the wrong thing, state that outlived what owned it, and
documents describing commands that no longer existed.

### Failures that now say something

- A provider's API error is a sentence, not a JSON envelope pasted into the
  chat and truncated mid-structure. A rejected key names the fix; it is the
  most common provider error and the only one a user can always fix. (0.6.68)
- A missing key names the variable that is actually missing. It said
  `ANTHROPIC_API_KEY` whatever had failed, and the OpenAI-wire client backs
  six providers through six variables. (0.6.69)
- An unknown `/command` says so and guesses, rather than doing nothing.
  (0.6.53)
- A CLI that is installed but not signed in says so. Empty output from it was
  being treated as a successful empty reply. (0.6.60)

### Things that stopped outliving what owned them

- Closing an agent pane stops the agents it was running. The broker died; the
  `claude` or `codex` it had spawned survived, reparented and still working.
  (0.6.72)
- A lost broker no longer leaves phantom running tasks in the footer, or a
  pending plan that still answers to enter. (0.6.67)
- The transcript fold marker counts everything it ever folded, and folds in
  batches rather than copying the whole transcript on every message past the
  cap. (0.6.66)

### The pane fits

- All three footer lines budget themselves and drop the least important
  segment rather than being clipped from the right. A 40-column pane was
  showing "enter runs it" and cutting "esc discards it" — half an
  instruction. (0.6.57, 0.6.58)
- `/keys` documents the agent pane, and fits a default window; it was 58 rows
  and silently truncated. (0.6.65)

### Knowing what you are running

- `crew --version` prints a version instead of launching the window;
  `crew --help` lists the CLI modes. (0.6.71)
- `/about` opens the changelog that shipped with the binary, and a build that
  is not the one that ran last says so on startup. (0.6.63, 0.6.64)
- This changelog exists, bound by a test to the version it ships with.
  (0.6.62)

### Guards, so this does not happen again

- The docs stop describing constructs that no longer exist, and a test fails
  the build if they ever do. (0.6.73)
- An end-to-end test sends every construct the router advertises to a real
  broker. (0.6.74)
- The test harness clears every provider key rather than the three it was
  written against — with an `OPENAI_API_KEY` exported, nobody could run the
  suite green. (0.6.70)

## 0.6.38 – 0.6.61

One night's work, grouped by what it was for rather than by release. Every
version in the range is a real release with its own tag.

### Fewer commands, because the pane says it instead

Thirty-three constructs became twenty-one. Each deletion moved information
somewhere it could be seen rather than asked for.

- `/agents` — the roster is in the footer, and bare `/model` already printed
  the same report. (0.6.40)
- `/tasks` — running tasks are in the footer, with their ids, because
  `/stop #n` needs one to name. The broker had no way to report a task
  ending until this release: `Tasks::reap` runs lazily on the next command,
  so a task finishing was not an observable moment anywhere. (0.6.41)
- `/status` — its turn count, token total and budget moved to `/doctor`,
  which is already the "state of this stack" report. Nothing was dropped
  without a home. (0.6.42)
- `/cwd` — the working directory leads the footer; the sandbox mode it also
  reported had moved to `/doctor` the release before. (0.6.43)
- `/checkpoint` — checkpoints are automatic now (below). (0.6.44)
- `/approve`, `/reject` — a drafted plan is answered with enter or esc. Both
  still route, for hosts driving the broker without a keyboard. (0.6.46)
- `/checkpoints`, `/skills` — folded into `/restore` and `/skill`, which list
  when given no argument. (0.6.54)
- `/compact` — the transcript folds itself and says so. (0.6.55)

### Things that now happen without being asked

- **A checkpoint before every task that can touch files.** Predicting which
  task is worth protecting is asking the user to be right in advance. Silent
  when the tree has not changed, capped at 25 snapshots, and ordered by a
  sequence in the ref name because git timestamps are second-resolution and
  automatic snapshots land inside one second. (0.6.44)
- **A signed-in CLI is a working crew.** `claude`, `codex` and `opencode`
  join the roster on PATH, with no key at all — the adapters had existed
  since the beginning with nothing calling them. A plain message relays to
  one instead of running the offline stub swarm. (0.6.39)
- **Choosing a model chooses the provider.** Holding two vendors' keys used
  to mean picking a model from the wrong one silently answered from the
  other. (0.6.51)
- **The transcript folds itself**, announcing what it folded, where it used
  to drop messages in silence at 500. (0.6.55)

### Providers

- Providers are a table: an endpoint, a key variable, a model chain and the
  vendor it serves. OpenAI, Gemini and DeepSeek joined DashScope, OpenRouter
  and Anthropic. `OPENROUTER_API_KEY` had been the answer for every vendor
  because OpenRouter was the only multi-vendor route; a row now asks for its
  own vendor's key. (0.6.47, 0.6.48)
- Every provider key is probed from the login shell, not just the original
  three — a key crew never imports is indistinguishable from no key at all.
  (0.6.49)
- One copy of the "no provider" advice, shared by the broker's roster line,
  `/doctor`, the Far pane's ask and the pane's empty state. Four wordings
  existed and two had gone stale. (0.6.50)
- A CLI that is installed but not signed in says so, instead of returning an
  empty reply. Empty stdout was treated as success. (0.6.60)

### Reading code in the pane

- **Syntax highlighting** in fenced blocks — comment, string, keyword — from
  a small hand-rolled lexer. Tokenized before wrapping, so a string crossing
  a wrap boundary keeps its colour. (0.6.45)
- Semantic colours clear a floor against body text, not just against the
  page. On the CRT themes code had measured 1.04:1 against prose, which is
  the same colour. (0.6.38)
- The syntax classes sit on a lightness ladder and keywords are marked by
  weight, so highlighting survives a single-phosphor tube where hue cannot
  vary. (0.6.45)

### The pane

- Grouped command palette while browsing, flat while filtering. (0.6.52)
- The footer fits: segments carry a priority and the least important go
  first, rather than the line being clipped from the right. (0.6.57, 0.6.58)
- An unknown `/command` says so and guesses, instead of doing nothing.
  (0.6.53)
