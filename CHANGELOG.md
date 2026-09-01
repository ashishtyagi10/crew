# Changelog

Notable changes per release, newest first. Versions are the workspace version
in `Cargo.toml`; every tag builds a release the app picks up through
`/update`.

The top entry must always name the current version — `changelog_covers_the_
current_version` in `crew-app` asserts it, so a release cannot ship without a
line saying what it was.

## 0.20.8

**The last effect with no rule gets one.**

The colour system's third phase asked for a defensible number per POOL rather
than a per-theme feel, on every effect. Most of it turned out to be done: the
dot lattice and the gradient wash are per-appearance constants, grain is one
value on every theme, every theme drifts at one rate, glow sits in a per-pool
band and every light page glows less than every dark one — all of it asserted.

**Flicker had nothing.** Twelve numbers between 0.010 and 0.08, each picked by
eye. It now has the same structural rule the glow has, which the shipped values
already satisfy: every paper page wobbles less than every tube, because a
brightness wobble is a property of a phosphor and a page that wobbles like a
tube is a page pretending to be one — plus a ceiling, above which a streaming
pane reads as a failing backlight rather than as a working agent.

What is deliberately NOT collapsed is the variation inside a tube's band. A
slow blue phosphor blooms further than a tight amber one; that is the theme
keeping its own character, which the colour goal's own "what this is not"
section protects. Nothing in this release changes what any theme looks like.

## 0.20.7

**The bridge stops being a test fixture.**

`wire/`, `worker/` and `remoteagent/` have been in crew-hive since June, built
so an external engine could run crew's tasks — and `RemoteFactory` was
constructed in exactly three places, all of them tests. Nothing spawned a
sidecar and no setting selected one, because the wire was not worth crossing:
`RemoteTask` carried `{agent, task, prompt, model, deps}` and the reply carried
a string, so an engine behind it could only be a slower `ApiAgent` with none of
its tools.

**The protocol is a conversation now.** A task carries the tools the worker may
use and whatever state it last returned; the worker streams `delta` text,
asks for a tool by name with `call`, gets a `result` back on the same stream,
and finishes with `done`. That turn is the whole point: **a sidecar never holds
a credential** — it names `sys:run` and crew runs it, through the same gate and
into the same ledger as everything else. An engine that could authenticate on
its own would be a second, unaudited way for crew to reach the world.

**`state` is the sidecar's.** crew hands back whatever it last returned,
verbatim, and never looks inside it. Resumability belongs to the engine that
has cycles; a checkpoint crew could read is a checkpoint crew would eventually
be expected to migrate.

**`CREW_SIDECAR` selects one** (`python3 /path/to/crew_sidecar.py`), and it is
opt-in in every direction: unset by default, probed before it is spawned,
falling back to crew's own agents with a line on stderr if it will not start,
and reported by `/doctor` in all three states. crew still plans the graph and
still owns the tools — the sidecar replaces the agents, never the spine. On a
machine with no Python nothing changes.

`examples/sidecar/crew_sidecar.py` is the whole protocol in one readable file,
and the test suite drives a real Python child through it: a delta, a tool call
answered by crew, an answer, and state coming back — skipped, loudly, on a
machine with no Python, which is the machine the feature is designed to leave
alone.

## 0.20.6

**`crew ask` stops being a one-window command.**

The ask socket belonged to the launch canvas, so every pane in every other
window was unaddressable: `crew panes` listed one window and `crew ask` could
only reach it. A window is not a process. The endpoint now belongs to the
process's owner, which routes each request to the canvas that can answer it —
the roster is every window's panes, a targeted ask reaches any of them, and a
broadcast fans across all of them and comes back as one reply.

Addressing keeps the spelling it had: the first window's panes are still `p0`,
`p1`, and a second window's are `w1p0`. A pane addressed by NAME is looked for
in every window, because a name belongs to the pane rather than to the window
it is in — and a pane actually called `w1p0`, or `worker`, or `web`, is not
mistaken for a window prefix.

**And the morning briefing turns out to need no code.** With the clock (0.20.1)
and integrations (0.20.5) in place it is a standing intent like any other:
`crew daemon at "tomorrow 7am brief me: my calendar, the weather, and what
changed in the repo" --every daily`. It fires unbidden, runs as a trigger — so
it reads the world and must ask before doing anything irreversible — and
answers on the channel it was set from. That was the goal's condition for it:
a briefing with a code path of its own would have meant the clock was built
wrong.

## 0.20.5

**Reaching a new API is one file.**

Tools were the last extension surface crew never opened. Plugin agents load
from a JSON manifest, skills from a `.md`, MCP servers from `mcp.json` — but
reaching an HTTP API meant editing `systools.rs` and cutting a release. So the
fortieth integration cost a commit, and that is the thing this closes.

**A manifest in `~/.config/crew/integrations/` (or a project's
`.crew/integrations/`) is a server of tools.** Name, base URL, auth, and a list
of endpoints with their methods, paths, query parameters and JSON bodies;
`{arg}` placeholders are filled from the model's arguments, URL-encoded in a
URL and type-preserved in a body. They land on the same `@tool server:tool`
surface as everything else — the same gate, the same ledger, the same
retrieval, both engines — and because the surface is built fresh per task, a
file dropped in is live on the next one.

Two rules the format enforces rather than suggests:

* **A manifest never holds a secret.** Every `auth` variant names an
  ENVIRONMENT VARIABLE, and there is no field that takes a token. Manifests get
  copied between machines, pasted into issues and committed to `.crew/` in a
  repository; a format with a `"token"` field is a format that leaks one. A
  missing credential is caught before the request is built, and the message
  names the variable rather than arriving as somebody else's 401.
* **A tool is irreversible unless its manifest says otherwise.** `tier` is
  `read`, `reversible` or `irreversible`, and absent — or misspelled — means
  the strictest. An integration nobody has thought carefully about asks before
  it acts, which is the same default an unknown MCP server already gets, and it
  is what a scheduled run at 3am is held to.

`/doctor` lists every integration with its tool count and whether the variable
it names is actually set; `/reload` reports what it found, because dropping a
manifest in and seeing nothing acknowledge it is indistinguishable from one
that failed to parse. `examples/integrations/weather.json` needs no account and
is the one to copy first.

## 0.20.4

**The agent stops being shown every tool it has.**

`SessionTools::hint` pasted every tool on every connected MCP server into the
task body — on every hop, for every agent, in every swarm. That is free at four
tools and a wall at forty: one Google Workspace server is fifty on its own, and
the token bill is the lesser half of the problem, because selection ACCURACY
collapses first. A model shown two hundred similarly-worded one-line
descriptions picks worse than one shown the twenty that could plausibly matter.

The task now decides. Above a budget of 24, tools are scored against the words
of the task — a tool NAMED `calendar` beats one that mentions calendars in
passing, and a tool the task names outright (`@tool gcal:events`) outranks
everything — and the best of them are what the prompt carries. Three rules keep
that honest:

* **Below the budget nothing is filtered.** A crew with ten tools has exactly
  the prompt it had yesterday, byte for byte.
* **crew's own `sys` tools are never dropped**, however the task is worded.
  They are how an agent does anything at all on this machine.
* **Nothing is unreachable.** What was left out is counted in the prompt (`37
  more tool(s) are connected but not listed here`), and a new **`sys:find_tools
  {"q": "…"}`** searches the whole catalog by name and description. A scorer
  alone cannot promise that the tool an agent needs is never the one that got
  dropped; leaving a door can.

The selection is deterministic — same task, same list, so a cached prompt stays
cached — and the prose hint and the native tool schemas select ALIKE, because
the provider decides which of the two paths runs and a tool present in one and
absent from the other is a tool that appears and disappears depending on which
model is serving.

`sys:find_tools` is classified a read, so even a trigger firing at 3am with
nobody to ask may look for a tool. An empty or malformed query says how many
tools are connected rather than matching all of them — a substring test against
`""` is true of every string, and answering a malformed call with the whole
catalog is how a budget gets undone by accident.

## 0.20.3

**The document window's last mile: the URL you can edit, and Tab through a
table.**

Two things the markdown editor's own goal listed as unbuilt, and the assertion
its whole design was chosen for.

**Cmd+K edits a link's URL.** The frame has named the link under the cursor
since v0.19.93, because a render deliberately hides it — that is what rendering
a link *means*. Now that line is the field you type it in: it opens holding the
current URL, Enter writes the new one over the old one's bytes as a single
undoable splice, and Esc leaves the file exactly as it was. With a selection
and no link the same chord makes one, and cancelling THAT takes the scaffold
back out — an editor that leaves `[half a thought]()` in your document because
you changed your mind is worse than one with no shortcut at all. (Two undos,
not one: an insert over a selection is a delete and a splice, and they do not
coalesce. A single undo left the words deleted and the link gone.)

**Tab walks a table's cells** — to the next cell, on to the first cell of the
next row when the row runs out, skipping the `|---|` divider because a divider
is punctuation rather than a cell anybody types in — and still types two spaces
anywhere else. Both work on the source at a byte offset, which is what the
caret already is, so neither needed anything new tracked.

**And the claim the editor is built on is now asserted against a document
nobody wrote for the purpose.** Four tests open `docs/CREW.md` — the repo's own
manual, thousands of lines of tables, fences, nested lists and unicode — edit
it and save: opening and saving it untouched writes it back byte for byte,
typing one word changes exactly those five bytes at exactly that offset,
deleting inside a table leaves every other row alone, and an edit followed by an
undo writes the file it started as. A serializing editor fails the first of
those, which is the reason this one is not a serializing editor.

## 0.20.2

**Set the alarm from your phone.**

The clock shipped last release with one way in: a terminal on the machine.
That is exactly what you do not have when you think of the errand. A message
on a channel now sets one — **`remind me tomorrow 9am to call the bank`** —
along with `watching` to see what is standing and `cancel w1` to call one off,
and `daily` / `weekly` / `every 30m` anywhere in the sentence makes it repeat.
The answer goes back to the address that set it.

The parse is deliberately narrow, because the failure it must not have is
claiming a task. "book me a flight tomorrow" has a time in it and is not an
alarm; handing "remind me…" to a model instead would produce a cheerful
"will do!" and no alarm, which is the worst outcome available here. Only a
message that starts with `remind`, or is `watching`, or is `cancel` with a real
id, is read as a command — a bare `cancel` stays what it has always been, a
refusal — and a conversation already blocked on an approval has nothing it
says read as a command at all.

`help` on a channel lists the three new words, which the test that pins the
help text against the vocabulary now requires.

## 0.20.1

**Crew gets a clock.**

The resident could hold a conversation and could not hold an appointment. It
routes a message from your phone to an agent and the answer back, and nothing
anywhere in it fired on time — the only natural-language time in the whole tree
belonged to the todo pane.

**`crew daemon at "tomorrow 9am brief me on the calendar"`** now reads the time
out of the sentence, using that same grammar, and hands the rest to an agent
when it comes round. `--every daily|weekly|hourly|30m` repeats it, `--to`
says where the answer goes (and may be left off when exactly one channel is
configured with exactly one address), `crew daemon watching` lists what is
standing, and `crew daemon cancel w1` calls one off.

Four decisions are worth naming, because each is a way this could have been
built wrong:

**A missed firing says it was missed.** A laptop shut over 7am delivers the
briefing late with `(this was due 4h ago — crew was not running then)` rather
than pretending it looked on time. A repeat that slept through several
occurrences rolls forward to the next one and says how many it stepped over —
seven alarms in a burst at breakfast is not what "daily" meant.

**A scheduled run has the least authority in crew, not the most.** Every firing
opens a session of its OWN as a `trigger:` requester, never the session a person
is talking to crew in. Sharing that session would hand a schedule the tier a
human conversation earned, which is the one promotion the action gate exists to
prevent; as it stands, an irreversible tool call a firing reaches for is refused
rather than asked about, because there is nobody awake to ask.

**It survives a restart**, because the watchlist is `watchlist.jsonl` beside the
ledger and under the same discipline: append-only, a cancellation is a
tombstone, a firing is a recorded fact, and what is standing is the fold of the
log. A torn last line costs one entry, not the watchlist.

**It fires once.** The firing is recorded before the work is dispatched — the
poll runs four times a second, and a crash between the two costs one run
instead of repeating it forever.

Underneath, six files were split along the boundaries this work exposed: the
daemon's wire face (`answer`) left its state, the clock left the serve loop,
`crew daemon install` left the CLI router, and the ipc card types left the
protocol envelope.

## 0.20.0

**`/tools` becomes readable where it is actually opened.**

Three faults in the listing shipped over the last two releases, all found by
asking what it looks like rather than whether it compiles.

Its rows were **eighty columns wide** — a clock, a padded tier, a padded
outcome, a padded tool and a requester. A viewer opened as one tile of a 2×2
grid is nearer fifty, so every row wrapped and the listing was unreadable in
the place it is most often opened. A row is now one quiet line — how long ago,
a mark, the tool, its tier — with what was *unusual* on a wrapped detail line
beneath it. The common call (a person at this keyboard, running something that
worked) says no more than that: spelling `ran` beside a tick and `pane` on
every row spent the width saying "normal". A long tool name is cut in the
MIDDLE, since both ends are what tell two calls apart.

Its clock was **wrong**. The time was computed as seconds into the epoch day —
which is UTC — under a doc comment claiming local. Every row was off by the
reader's distance from Greenwich. Times are now relative (`30s ago`, `2d ago`)
like every other time crew shows you, which is right everywhere and needs no
timezone database crew does not have.

And relative times introduced a fault of their own: a listing left open freezes
at `30s ago` while the agents keep working. The header says so, and says how
many calls it is showing — a filtered view of three rows out of nine hundred
should not read as the whole history.

## 0.19.99

**One action, one look — and a way to search the ledger.**

`/tools` shipped last release with no way to narrow a thousand rows. **`/tools
<term>`** matches one term against every column a person searches by: the tool,
the tier, the requester, the decision, the outcome and the note. A term that
matches nothing says so in its own words, because "there is no history" and
"your search found none of it" are different answers and only one of them means
you typed it wrong.

**A tool result on the relay looked nothing like the same result in a swarm.**
The relay's result hop carried raw output with no `[tool]` marker, so the app
never classified it as a tool card: the *call* rendered quiet and folded while
its *result* sat beside it as a full, brightly-coloured agent reply. Both
engines now build the same card, from the same function, and the relay gains
the duration the swarm already had. Its result budget went from 400 chars to
4000 to match — 400 was a display clip from before results could fold, and it
cut most real output off mid-word.

**The input bar has tests.** The most-used surface in the app had none of any
kind. Six now shoot it across widths and read the cells back: nothing outside
the card, nothing colliding on the row you type into, the tail-and-ellipsis for
an overlong line, and the tag on the bottom rule winning the cells it rides —
which only reads correctly because later cells paint over earlier ones, an
ordering that was an accident of two call sites' sequence and is now a stated
contract.

## 0.19.98

**What the agents did to your machine, and what they are waiting on.**

Every tool call an agent makes passes one gate and appends one line to
`~/.config/crew/ledger.jsonl` — its tier, who asked, what the gate decided and
how it ended. That has been true since the gate landed, and nothing had ever
read it: `Ledger::read` had no caller outside its own tests. **`/tools`** opens
it, newest first, in the file viewer. A call that never returned shows `·`
rather than a tick, for the same reason `/blocks` does — the gate writes the
decision when it makes it and the outcome when the call ends, so a crash
between the two leaves a real row with nothing after it. The ledger is
append-only and machine-wide, so the view takes the most recent thousand rows
and says how many it left out; unreadable lines are counted, not skipped in
silence.

While a call is in flight the header now names it — `api-consumer ⋯ sys:run` —
instead of counting up with only the agent's name. A tool wait is not thinking,
and `sys:run` alone may sit two minutes on its deadline.

And the `[tool] ` marker is gone from the card. It is machinery — how the
broker tells the app what kind of card this is, carried in the text only
because that is the field that crosses the wire — and the gutter and the muted
ink already say the same thing in a glyph. Stripped in the one function both
the counting and the drawing pass read, so the two cannot disagree about where
the card wraps.

## 0.19.97

**Tool use reads as tool use, not as four more replies.**

A tool call arrives in the transcript under the agent's name — which is right,
you need to know who reached for it — and that meant it rendered as a full
reply: solid gutter, the agent's roster colour, never folded. A task making
four calls produced nine cards that all looked like the agent talking, and the
one card that was the answer had nothing to distinguish it.

Tool cards now take the quiet voice: the dotted gutter and muted ink the
broker's own notes use, with the caller's name kept. The result lands as its
own card whose first line is `sys:run ✓ 1.2s` — outcome, then how long it took,
which is what separates a slow tool from a hung one while you watch — and its
output is folded to that one line until you click it open.

Successful results used to be dropped entirely, on the reasoning that raw
output would bury the agent's answer. That was sound while every tool card
rendered in full and stopped being sound once they fold. Dropping it left the
agent's paraphrase as the only account of what an API returned, which is the
one thing you cannot check an integration against.

Underneath: `folded` and `foldable` were separate predicates — what the frame
draws and what a click may toggle — and a card the first folds and the second
does not is collapsed with no way to open it. They now share one threshold.

## 0.19.96

**The weird line is gone.**

Reported twice, the second time with a picture of it drawn straight through an
agent CLI's statusline. It was crew's **unread divider**: a rule under the last
row you had read, with `1 new` hung off its right end — which is why the tag
appeared to be part of the statusline.

The first report was answered by giving the rule that tag, on the theory that a
bare full-width line reads as damage because nothing says what it is. That was
half the problem. The other half no tag can fix: **counting buffer lines is not
"output arrived" for a program that redraws itself.** An agent CLI repaints its
input box and statusline in place, growing the buffer by a line each time, so
the boundary lands *inside* the live interface rather than between two lines of
scrollback — constantly, and naming a repaint as news.

So the rule is gone and the **count stays**. It was already on the card's
border, in the sidebar and on the minimized thumbnail; a badge saying *something
happened here* is honest at the granularity it is shown at, and a rule claiming
*and it starts on this row* is not, for any pane running something that paints
its own screen.

Reproduced in the shot harness first — a transcript, an input box and a
statusline, with the divider over it — which showed that even in its best case,
landing on a blank row, it reads as a stray line across the pane.

## 0.19.95

**A second window costs a surface, not a second GPU.**

`Cmd+N` shipped last release with an honest gap written into the goal doc:
each canvas built its own wgpu instance, adapter, device and queue — a whole
parallel driver context for the same card. The device is the expensive half
and the one thing that cannot be shared *after* the fact, since resources
belong to the device that made them.

What is genuinely per-window is the **surface**: the swapchain the compositor
hands you, its configuration, its pixel format and its size. Everything
upstream of that is per-process, and is now built once on the first window.

Measured on this machine, two windows open (debug build, RSS):

| | one window | two windows | the second window cost |
|---|---|---|---|
| before | 178 MB | 233 MB | **~55 MB** |
| after | 178 MB | 187 MB | **~9 MB** |

* **The awkward part is the order.** Choosing an adapter wants a surface to be
  compatible with, and a surface wants an instance — so the instance is made
  first and alone, the first window's surface next, and the adapter and device
  from that. Every window after it is measured against the adapter that
  already exists.
* **The format and the alpha mode stay per surface.** Two windows can be on
  displays that offer different ones, and each window's pipelines are built
  for its own.
* The test asks for the shared handles three times and asserts they are the
  same handle — not that a counter says one, which would still pass if every
  ask built a device and threw it away. It fails with *"a window built its own
  device instead of taking the one that exists"* when the memoization is
  removed.

Still per-window and still worth sharing one day: the glyph atlas and the
render pipelines, which are cheap enough now that the second window's whole
cost is nine megabytes.

## 0.19.94

**`Cmd+N`: a window is a whole canvas.**

Crew has been one window holding a grid of panes since it existed. It can now
hold as many as you open: `Cmd+N` gives you another **canvas** — its own grid,
its own focus, its own zoom, its own input bar and nav — in the same process,
sharing one broker, one theme and one config. Panes belong to the window they
were opened in and go on running while you work in the other.

This is Pillar 1 of
`docs/superpowers/goals/2026-08-30-markdown-editor-in-its-own-window.md`, and
the shape of it is the point: **the canvas type is `CrewApp` itself,
unchanged.** Everything per-window was already a field on it and every method
that reads `self.panes` is already a method *of one canvas*, so a second
window is a second `CrewApp` rather than two hundred call sites learning which
window they meant. What moved up into the new owner is only what a second one
must not duplicate.

* **Closing a window closes that window.** The close button used to call
  `event_loop.exit()`; it asks the owner now, and the owner quits only when
  the last canvas goes. Cmd+Q still takes the app — and its confirmation
  counts the panes in **every** window, because a prompt that says "1 pane
  open" while a second window runs three agents is worse than no prompt.
* **A session remembers which window each pane was in**, and `/restore` brings
  the windows back. A file written before there could be a second window says
  nothing about windows and restores into one, exactly as it always did.
* **The config is one thing about you, not about a window.** Change the font
  in one and the other adopts it, rather than going on at the old size and
  then saving the old value over yours.
* **The bug a planted session caught:** two windows each holding a shell in
  the same directory — which is what happens the moment you open a second
  window and go on working in the same project — saved as *one* pane and
  restored as *one* window. The dedupe key had every field of a saved pane
  except the window it was in.
* The launch notes, the crash report and the upgrade migrations belong to the
  launch rather than to a window, so a second canvas does not repeat them.

Verified by running it: two windows on screen from one process, and a planted
two-window session coming back as two.

**Not per-window yet:** the inter-pane `crew ask` socket is served by the
launch window, so `crew ask` addresses its panes.

## 0.19.93

**Cut, paste, and the URL you cannot see.**

* **Cmd+V pastes and Cmd+X cuts**, as markdown. A paste is an insert, so it
  replaces a selection and comes back out in one undo, and pasting a document
  into a document keeps its markers rather than a rendering of them.
* **The frame names the link the cursor is inside.** A link's target is
  invisible in a render — that is what rendering a link *means* — so the one
  place with room to say it does. Nothing new had to be tracked: the cells
  already carry the URL, so a click can recover it without re-parsing.
* **The goal's last condition has a picture now.** The editor — caret,
  selection, link and unsaved mark in one frame — on a light page and through
  a green tube, at a comfortable width and at one narrow enough that the
  document wraps twice.

That closes the part of
`docs/superpowers/goals/2026-08-30-markdown-editor-in-its-own-window.md` this
run set out to build: a markdown file opens in a window of its own, rendered,
with a cursor already in it, and you write in the render. What the goal still
lists as unbuilt is the per-window canvas (Pillar 1, deliberately sidestepped
by a document window that holds no panes), Tab through table cells, and
editing a link's URL in place.

## 0.19.92

**Selection, and the bold that has nowhere else to come from.**

The editor promises that no `**` ever appears on screen — which means there
has to be some other way to put one in the file. **Shift+arrows** select,
**Cmd+B** and **Cmd+I** wrap the selection (or take the markers off again),
Cmd+A selects the document, Cmd+C copies, and typing or Backspace replaces
what is selected. A selection is a pair of **source bytes**, so its wash
follows the text through a re-wrap instead of being a rectangle of screen.

**Two bugs, both found by looking at a picture of it**, both of which put the
markers on screen — the one thing this is all for:

* **A selection that leaves its block is refused.** Emphasis is an *inline*
  thing: a `**` opened in a heading has no partner in the paragraph below it,
  so markdown renders it as two asterisks. The first shot of this feature had
  `The document w**indow` in its title. A selection crossing a blank line, a
  heading, a list, a quote, a table row or a fence is now refused, and says so
  — while a selection across the wrapped lines of ONE paragraph is still
  allowed, because refusing that would make the key useless in every document
  written at 80 columns.
* **The selection is trimmed before it is wrapped.** Markdown will not read
  `**word **` as bold either — a closing delimiter preceded by a space does
  not flank — so the second shot still had asterisks in it. Selecting a word
  and its trailing space now bolds the word, which is what every other editor
  does.

Also: **`/md <file>` opens a document window** rather than a pane. It is the
markdown-shaped door, and the thing behind it is now an editor; `/view` keeps
opening a pane, which is what Cmd+click on a path, `/diff` and `/out` want.

## 0.19.91

**Undo, and a click that puts the cursor where you are looking.**

Typing into a document (0.19.90) is only usable if you can take it back, so:
**Cmd+Z** undoes and **Cmd+Shift+Z** redoes, a *word* at a time rather than a
letter — undoing a sentence one keystroke at a time is the same as having no
undo. A run ends where a person expects it to: at a space, at a newline, when
the cursor is moved by hand, and where typing turns into deleting.

* **An undo restores bytes, not a re-rendering.** A change is a byte range,
  the text that was there and the text that replaced it, so reverting one puts
  the file back exactly — which an editor that re-serializes a tree can only
  approximate. The test runs a long mixed session (typing, Enter, five
  backspaces, a caret move, a forward delete), undoes all of it, and asserts
  the file is the one that was read, byte for byte.
* **A newline being deleted ends the run it is in.** It did not: the break was
  checked against the character coming in rather than the one the run had most
  recently taken, so backspacing over a line ending glued the letters before
  it into the same change — and undoing gave back one newline short. Runs of
  deletion grow from their front, and the rule is now symmetric with typing:
  a run ends when a break character is **consumed**.
* **Click to place the cursor.** Past the end of a line it goes to the end of
  it; on a row of pure furniture (a rule, a code field's border) it finds the
  nearest row with somewhere to stand — a click always means *put it here*,
  and the nearest here is the honest answer.
* **Delete** removes the character at the caret, as Backspace removes the one
  behind it.

## 0.19.90

**You can type into the render.**

0.19.89 put a cursor in a rendered document. This one lets you write with it:
in a `/doc` window, a letter goes in where you are looking, Backspace takes
out the one behind it, Enter ends the block and starts **another of the same
kind**, and **Cmd+S** writes the file.

* **Enter knows what block it is in.** A single newline inside a paragraph is
  a *soft* break — CommonMark joins the two sides with a space — so pressing
  Enter in prose and getting one would look like nothing happened; prose gets
  a blank line. A list gets another item, a numbered list gets the next
  number, a nested item keeps its indent, a quote keeps its bar. That is read
  off the source line the caret is on, which is the one place the markers are
  still visible — and the reason the buffer is the source.
* **What it writes is what it read, with the edit spliced in.** The caret is a
  byte, so an edit is a splice and nothing else *can* move: no re-wrapped
  paragraphs, no `*` bullets rewritten as `-`, no setext heading turned into
  ATX, no odd spacing tidied. The test opens a document written in every style
  a formatter likes to "fix", types one character, saves, and asserts the diff
  is **one line**. A save with nothing typed writes the file unchanged, byte
  for byte.
* **A caret can stand after the last character.** It sits *before* the one it
  is drawn on, so each row ends with a position no character provides — the
  place a line is extended and a document is appended to. Without it there is
  nowhere to stand at the end of a file and nothing can ever be added to one.
* **While a document has a caret, the window is an editor**: `o`, `r`, `w`,
  `s` type themselves instead of running the viewer's commands. A chord crew
  has no answer for is left alone rather than typed.
* The frame's legend carries a `●` while anything is unsaved, and Esc on
  unsaved changes asks once before discarding — and typing again after that
  puts the guard back.

**Not yet:** undo, selection, Cmd+B/I, and clicking to place the caret. The
next slice is the one that makes it comfortable rather than possible.

## 0.19.89

**A cursor in the render — the markdown editor starts here.**

`/doc README.md` opens the document rendered, and now with a **caret already
in it**: the arrow keys, Home and End move a cursor through the *rendered*
document — no `#`, no `**`, no `](` anywhere on screen — and it scrolls to
follow. This is the first slice of
`docs/superpowers/goals/2026-08-30-markdown-editor-in-its-own-window.md`.

The whole thing rests on one field, and this release is mostly that field:

* **Every rendered character knows the byte it came from.** pulldown-cmark
  hands a source range to every event; `md::source` stamps it where the inline
  fold can read it, `MdSpan` carries it, a wrap **splits it by the bytes it
  dropped** (not the characters, or every wrapped line after a non-ASCII
  character points at the wrong byte), and every `CardCell` ends up holding
  the byte its character came from.
* **A run that is not a verbatim copy of its source carries nothing.** An
  entity is one character from five bytes; an escape one from two; the space
  CommonMark puts where a soft break was is a space where the file has a
  newline — and that last one has the *same length*, so the length test alone
  was not enough and the test caught it. Claiming a byte four out is far worse
  than admitting there is none.
* **A cell with no byte is not a place the caret can be.** A list bullet, a
  table's rules, a code field's border: the renderer put them there and the
  file does not contain them, so the caret steps over them. That is not a
  limitation worked around — it falls out of the provenance.
* **The caret IS its byte.** The row and column are only where the current
  width happens to put it, so a resize re-wraps the document, throws the
  position away and finds the byte again — by binary search over rows, because
  a hundred-thousand-row document must not be walked to answer it.
* The invariant is asserted directly: for every rendered character of a
  document containing every shape in the grammar, at three widths, the byte it
  claims holds that character.

**Not yet:** typing. This slice moves a cursor and saves nothing — the next
one splices at the offset, which is where the "untouched bytes never move"
guarantee gets to be structural rather than promised. The goal doc records why
the buffer ended up being the source text rather than the parsed tree.

## 0.19.88

**A picture stopped being drawn over the heading that names where it is.**

The close of a ten-release loop is the frame nobody had taken: every surface
the last nine added, on a light page and through a green tube, at three window
shapes, and scrolled to the awkward places. One thing was wrong, and only in
the state that puts two of the new features on the same row.

* **The clip is a box now, not a size.** A document scrolled so its picture is
  half off the top drew the picture **over the sticky heading band** — paint is
  drawn over a cell's background, and the picture's clip only knew the pane's
  width and height, so it had no way to be told about a row it must not enter.
  It takes a full box, and the viewer keeps pictures out of the first row when
  a heading band is up and the last when a search is open. Both are chrome the
  document scrolls *under*.
* Shots added for the document window on the light theme and the green tube,
  for a picture cut by the top of the pane, and for hint mode on both themes.

**Judged and not changed:** a picture keeps its own colours on a CRT theme.
The tubes look monochrome because their *ansi table* is one phosphor ramp, not
because crew collapses what a program sends — a truecolour run comes out
orange on the green tube today, and has since the tubes shipped. A picture is
the strongest possible case of a program sending exact colours, so it keeps
them, and the tube stays a palette rather than a filter.

## 0.19.87

**The other spelling of "here is a picture".**

0.19.81 taught crew kitty's graphics protocol. Plenty of tools write iTerm2's
instead — `imgcat` and a great deal of scripting — so crew reads that one now
as well: `OSC 1337 ; File=inline=1:<base64>`, landing in the same command, at
the cursor, reserving the same rows, decoded on the same worker.

* `width=`/`height=` are read in the units they were written in: cells bare,
  pixels with `px`, a share of the window with `%` (which is not an answer the
  parser has, so the picture comes as big as it is and is fitted).
* `inline=0` — the protocol's own default — means *download this file*. A
  terminal has no business doing that because a program said so, and crew does
  not.
* **The four OSC sequences crew already listens for are untouched.** The cwd
  report, notifications, progress and the shell marks reach the ANSI parser as
  they always did: the splitter reads only far enough into an OSC to know
  whether it says `1337;File=`, and takes nothing out of the stream until it
  does. Both halves of that are asserted by cutting the stream at **every byte
  offset** — an OSC 7 survives every cut, and so does a picture.

## 0.19.86

**The heading you are underneath, kept where the address is.**

A document is read in sections, and the moment a section is longer than the
window the one thing the window stops telling you is which section you are in.
The card's gutter has always marked *where* the headings are — that answers
"how far", and never "under what". Scroll into the middle of a long spec and
the pane is prose with no address.

The top row now carries the heading above you, as a band in the page's own
hand, **with the ladder it sits under collapsed into it** — `crew › Themes ›
CRT`, not just `CRT`. It costs a row of the document and returns the question
that row was raising.

* Nothing sticks at the top of a file, where the document's own first line is
  the address, and nothing sticks when the heading is already on screen — a
  title repeated one row above itself is noise.
* A landmark with no nesting has no ladder: a diff's hunk is *after* its file
  header, not inside it, so there the band is the one landmark and no more.
* `Mark` learned its `depth` from the heading level the renderer already knew,
  rather than from parsing `#`s back out of a label the renderer had stripped.

## 0.19.85

**A picture a document names is the picture.**

`![alt](chart.png)` in markdown rendered as the words `alt` and nothing else:
the one part of a document that is not words came out as words. Crew draws
real pictures now — the `/view` image rung, a program's own output — so a
picture a document *names* is drawn too, in the viewer and in a document
window.

* **The engine reserves the room; the renderer fills it.** The markdown engine
  has no pixels, no cell size and no worker thread, so an image paragraph
  becomes twelve rows that say *a picture belongs here, this is row 3 of 12,
  and here is its source*. A renderer that can only see the rows currently on
  screen still knows the whole box to fit the picture into.
* **The rows are counted after the mapping, not before.** The engine wraps by
  character and the card layout re-chunks by display column; a row index taken
  before that is one a wide glyph can move.
* **The source resolves against the document**, so a README opened from
  anywhere finds its own images. A remote `https://` source stays alt text — a
  terminal should not make a network fetch because a document said so.
* **Read on a worker, cached per path, process-wide.** The frame never touches
  a file: the first ask starts a read and returns nothing, the next frame has
  the picture, and something that is not a picture fails once instead of
  spawning a worker on every frame forever.
* The height is fixed rather than derived from the file, and the picture is
  letterboxed inside it: a document whose lines moved once the pictures landed
  would reflow under the reader.

`doc_shot_illustrated` is the frame this shipped from — a README with its chart
in it. (The first take of that fixture photographed a code block instead: four
spaces of Rust source indentation inside a continued string literal is four
spaces of markdown, which is an indented code block.)

## 0.19.84

**A document gets a window of its own.**

Crew is one window holding a grid of panes, and that is the right shape for the
work — a shell beside an agent beside a diff. It is the wrong shape for the one
thing you read for twenty minutes at a time. `/doc <file>` (and `w` inside a
viewer pane, which pops the document you are already reading out of the grid)
opens that file in a **window**: no nav, no input bar, no tiles — one file,
framed, filling it, sized to a reading measure rather than a canvas.

* Every viewer key works there: scroll, `/` search, `]`/`[`, `s`, `r`, `e`,
  `o`. Esc or the close button ends it; opening a file already in a window
  raises that window instead of stacking a second copy.
* The frame's legend names the file **and how far through it you are** — a
  window has no card border to draw a scroll thumb on.
* `window_event`'s `WindowId` was discarded for as long as there was only ever
  one window (`_id`, since 0.4). It routes now: an event that belongs to a
  document window never reaches the grid's handler, because a resize sent to
  the wrong window resizes the wrong surface.
* A window can only be created inside a winit callback holding the *active*
  event loop, and a key handler is not one — so the ask is queued and opened on
  the next tick. A path that is not a file says so where it was typed, rather
  than silently doing nothing a tick later.
* The picture has a shot of its own (`doc_shot_window`, three window shapes).
  A surface that exists only on a real display is a surface with no picture of
  itself, so the frame, the legend and the document are rendered off-screen
  through the same path the window draws.

This is the first slice of
`docs/superpowers/goals/2026-08-30-markdown-editor-in-its-own-window.md`, which
is also new in this release: a markdown document in a crew window, rendered on
arrival, **edited in the render** — no `**` on screen, images drawn, and a save
whose diff touches only what was edited. A document window deliberately holds
no panes, so none of that goal's per-window canvas refactor was needed to get
the window itself.

## 0.19.83

**Cmd+E: reach anything on the pane with one letter.**

A terminal's output is full of things you want to *do something with* — the URL
a server printed, the file a compiler named, the hash a commit came back as.
Crew has always known where they are (it draws a rule under every one of them),
and the only ways to act on one were the mouse and the scrollback search: both
ask you to leave the keyboard for something already on the screen.

`Cmd+E` labels them. Every URL, file reference and hash on the focused pane
wears a letter; pressing it **copies**, pressing its **capital opens** — a URL
in the browser, a file in the viewer, a hash to the clipboard, which is what a
commit id was wanted for anyway.

* **Labels go to the newest output first.** They come off the home row and are
  handed out from the bottom of the pane up: the last thing a program printed
  is what you almost always want, and it should be the cheapest key to press.
* **No label is a prefix of another.** Single letters while they last, then
  pairs — never a mix, or the first press would have to guess whether you were
  finished.
* **The pane's own text leans toward the page** while the labels are up, so the
  tags are what the eye lands on. It lasts exactly as long as the mode does.
* **A miss ends the mode**, as does Esc, as does any key that is not a label.
  A mode that sits there swallowing keys is worse than no mode — and a pane
  with nothing to reach says so rather than opening one.
* A number is not a hash: a run of digits is a port, a byte count or a line
  number, and labelling every one of those would bury the two things on screen
  that really are object ids.

Also in this release: a goal doc for the next big piece —
`docs/superpowers/goals/2026-08-30-markdown-editor-in-its-own-window.md`, a
markdown document in a crew window of its own, rendered on arrival, edited in
the render.

## 0.19.82

**Bold text stopped being a different typeface.**

Reported from a live window: the focused pane's legend was in a different font
from the pane's own text — and only while it was focused. The legend was not
the problem. It is the one part of the frame drawn **bold**, and cosmic-text
answers a family+weight query by *distance*: a family with no Bold face loses
to any family that has one, silently, with no error anywhere.

Six of the seventeen coding faces installed on the machine this was found on —
**Cascadia Code, MonoLisa, Geist Mono, Google Sans Code, ComicMono, Operator
Mono** — ship Regular and Medium and no Bold. Under any of them, *every* bold
cell in crew shaped from **Menlo**: the pane legend, an agent's `**emphasis**`,
a code fence's header, a heading in the viewer. With `/font random` on, which
face you got changed every ten minutes.

A weight a family does not have is not a request it can answer, so crew stops
making it. Bold now shapes at the heaviest weight the family really has, and a
single-weight face simply stays its own weight instead of becoming somebody
else's. The base weight is resolved the same way, so `/weight medium` on a
Regular-only family is exact rather than nearly-right.

The regression test shapes bold in **every allowlisted family installed on the
machine** and asserts the glyphs came back from the family that was asked for
— it fails with `bold in Cascadia Code shaped from Menlo` if the clamp is
removed.

## 0.19.81

**A program can put a picture on a pane.**

Crew speaks the terminal graphics protocol — kitty's `APC G` form, which is
what `kitten icat`, `timg`, `chafa` and matplotlib's kitty backend all write —
so a plot, a diagram or a screenshot arrives as part of a program's output
instead of as a path you have to go and open. 0.19.80 taught the viewer to draw
a picture; this is the same layer, reached from the other end.

* **The stream is split, not sniffed.** An image lands where the cursor is, so
  the bytes ahead of the sequence are handed to the parser *first* and the
  placement is recorded between them — not at wherever the read happened to
  end. A sequence cut in half by a buffer boundary is still one picture, and a
  chunked transmission (`m=1`, which is how every real screenshot arrives) is
  joined back into one.
* **The anchor is an absolute buffer line**, so a picture scrolls with the
  text it arrived in — off the top of the pane, and back when you scroll up.
  Paint is free rectangles and nothing else clips it, so it is cut to the pane
  on the way past rather than drawn over the pane above.
* **Nothing decodes on the frame thread.** A PNG arrives inside a `try_read`;
  running a decoder there would freeze every pane in the grid. Each picture
  goes to a worker and lands a frame or two later — and that wait is the only
  reason crew repaints while it waits.
* **The terminal makes room.** How many rows a picture claims is read from its
  own header before it is decoded, and the screen scrolls past it, so the next
  line of output does not print on top of it.
* **Producers ask first, and crew answers.** `a=q` gets `OK` for the formats
  it can draw and `ENOTSUPPORTED` for the rest, so a tool can fall back rather
  than write a picture into a void.
* Not one byte of a sequence ever reaches the screen — a terminal that prints
  the base64 is worse than one that ignores the picture entirely — and an APC
  that is not a graphics command (a keyboard-protocol probe, tmux passthrough)
  is swallowed the same way.

## 0.19.80

**The viewer shows pictures.**

`/view photo.png` used to draw a card *about* the file: "binary file — nothing
to render". Every rung of the viewer's ladder ends in glyphs, because a
terminal's unit is a cell and a cell can only say one character.

But crew has drawn *below* the cell since 0.19.29 — `Paint` is a rectangle in
fractional cell units, the layer every chart is on — so a picture can be laid
down as a grid of small quads at whatever resolution the pane can carry,
independent of the font. PNG, JPEG, GIF, BMP and WebP now open as themselves,
fitted to the pane and centred, with a banner naming the format and the file's
real dimensions.

* **Decoded and downscaled on the viewer's worker thread**, the same place
  every other rung's bytes are read. A forty-megapixel photo run through a
  decoder on the winit thread would freeze every pane in the grid, agents
  included.
* **Runs of one colour become one quad.** A screenshot or a logo is mostly
  flat, so the merge takes a frame from tens of thousands of rectangles to
  hundreds without changing a pixel of the result.
* **Transparent samples are not drawn at all** — a logo with no background
  lands on the page it is being read on, and stays right when the theme
  changes.
* **Cells are twice as tall as they are wide**, so the fit is computed in
  square units and converted back into rows; a square picture comes out square.
* **The format is read from the bytes, not the name**, and before the binary
  sniff that used to refuse it: a JPEG called `notes.md` is shown as a JPEG.
  (The BMP sniff checks the header's reserved bytes, so a sentence beginning
  "BM" is still a sentence.)
* `image`'s `jpeg`, `gif`, `bmp` and `webp` decoders were switched on — the
  crate was already a dependency, for the app icon and the shot harness.

## 0.19.79

**The caret leaves a wake.**

A cursor in a cell grid is a teleporting object: it is in one cell on one frame
and a different cell on the next, and nothing on the page says the two were the
same thing. At a prompt that is invisible — you are looking at the character
you just typed — but the moment it jumps (a `Ctrl+A` to the head of the line, a
TUI moving its selection, a paste landing) the eye has to *find* the caret
again.

The focused pane's caret now drags a short streak behind it: quads in the
cursor's own colour over the ground it just covered, sliced along the travel
with the alpha ramping into the caret so the same rectangle reads as speed
rather than as a selection. It is gone in 130ms — shorter than the gap between
two keystrokes at speed — and nothing is ever drawn *later* than the program
asked for: the cursor is exactly where it belongs on the very first frame, and
the wake is a trace of a move that already happened.

* A jump too long to join with a bar (>24 columns, or across rows) leaves a
  ghost on the cell the caret **left** instead — the same statement in the
  space of one cell, rather than a flash across the whole pane.
* Only the focused pane's caret is followed. An unfocused pane draws the
  hollow cursor, and a background program stepping its own must never be a
  reason for crew to schedule a frame.
* Bounded like every other animation here, so the "an idle crew never repaints"
  invariant holds, and at `/motion off` no wake is drawn at all.
* The terminal shot harness grew two frames for it (`term-wake`,
  `term-wake-jump`) — it lives on the paint layer, which is the one layer a
  cell assertion cannot see.

## 0.19.78

**Three things a terminal has to get right, all found by looking at one.**

The terminal shot harness grew the layers crew draws *over* a pane's own
cells — a live selection, `/find` washes, URL tinting — plus a full-screen TUI
on the alternate screen. Links were already perfect. The rest were not:

* **The selected row of a TUI was the least legible line on the screen.** A
  file picker draws it `black on green`; crew's ansi table LIFTS black so
  `\x1b[30m` reads on the page, which hands a mid-grey to a cell whose
  background is a light green. Measured: the picked row at **2.98:1**, the
  plain rows around it at **9.65**. Text the program painted a background
  behind now has its own, higher floor (4.5) — that pair is not a guess but
  the thing most meant to be read. Ordinary output keeps the lower rescue
  floor, so a program's own quieter colours still survive.
* **`/find` could not find any text containing a full-width character.** The
  grid is column-indexed, so `全角` sits on it as `全 _ 角 _` and a needle
  written `全角` never matched. Matching now runs over the row's characters
  and maps each hit back to the columns it has to wash.
* **A match could be washed invisible.** The highlight replaces the background
  the terminal had already floored the foreground against, and nothing
  re-checked it: a match inside a painted row came out as a solid block with
  the text gone. The one thing you searched for was the one thing you could
  not read.

## 0.19.77

**Everything a full-width character wears now covers both of its columns.**

A wide glyph owns two columns of the grid, and alacritty parks a SPACE in the
second one carrying the character's own colours and flags. Crew passed that
space on as a cell of its own, which meant a two-column character shaped a
two-column glyph and then advanced a **third** column for the blank behind it.
It only survived the "drop empty cells" filter on rows that were underlined,
selected or painted by a TUI — which is to say, on exactly the rows where it
showed. A full-width character is now one cell, and the renderer knows it owns
two columns, so its background, its underline and the block cursor on it all
span both: a selection over Japanese was a row of stripes, an underline broke
under every wide glyph, and a TUI's painted status bar came out perforated.

**The UPDATE card stopped clipping the one thing it exists to say.** A note
was written on a single row of a narrow nav column and cut mid-word, with the
card's second row left blank — and `update failed: …` is a note. Notes now
wrap over every row the card has and ellipsise when there is still more, and a
failure wears the bell colour and a `!` lead instead of the same quiet `·` as
"already up to date".

## 0.19.76

**A line of Japanese was drawn wrong three different ways.**

The one surface never in a frame was the one crew is: a real terminal grid.
Every shot harness here photographs something crew draws itself — the chat
transcript, the left nav, the drawn panes, Far, the menus, the todo list — and
the glass and CRT shots put a *Far* pane in the window because a Far pane is
easy to build. So the ansi palette, bold, the underline family, the block
cursor, a live selection and full-width glyphs on the terminal's own grid had
all been asserted on as cells and never looked at. They are now swept at three
tile sizes and on light and green pages, and the first frame showed CJK text
spaced out like `日 本 語`.

Three separate faults, stacked:

* **The spacer column got a blank.** A full-width character occupies two
  columns and the second carries no cell of its own, so the blank the shaper
  was handed there gave a two-cell character a **three**-cell advance.
* **Full-width glyphs were exempt from the advance correction**, on the belief
  that they "keep their existing two-cell behavior". They do not:
  `monospace_width` snaps to the *nearest* cell multiple, and the CJK face the
  fallback reaches at **weight 500 — the weight every light theme uses** —
  advances under 1.5 cells, so it snapped to **one** and drew a two-column
  character over its neighbour. The two faults cancelled into a row that added
  up while every glyph sat in the wrong place.
* **The first frame after a bad font was banished skipped correction
  entirely** — the two passes were joined by a `||`, so the frame that dropped
  a broken face never went on to measure anything.

Nothing else moved: ASCII, bold and the box-drawing range were already on the
grid and stay there.

## 0.19.75

**A Japanese character in a reply took the whole frame down.**

macOS ships `GB18030 Bitmap`, a bitmap-only face whose em is zero, so every
metric scaled out of it comes back **infinite**. Crew already knew the face
misbehaved — the cell-advance correction has sidestepped non-finite advances
since it was written — but sidestepping is not enough: the infinite advance
flowed into the glyph's own box, into every `x` after it on the row, and, three
layers down in the shaper's subpixel binning, into an `f32 as i32` that
overflowed. One `日` in an agent's reply was enough. Nothing finite can correct
an infinite advance (letter-spacing is *added* to it), so the face is now
dropped from the font database on sight and the line re-shaped through whatever
else covers the script. The synthesised box-glyph path grew a matching floor:
a cell box larger than any cell is declined, not allocated.

Found by giving the markdown engine its first look in a frame. Every other
surface has a shot harness; the half of the markdown grammar an agent actually
replies with — tables, nested and ordered lists, task lists, block quotes,
rules, unbreakable URLs, CJK — had only ever been asserted on as data. It is
now swept at three tile widths and on light and green pages, and the sweep
also found:

* **Table column alignment was parsed away.** The delimiter row is the only
  thing in table syntax that exists purely to say how a column reads, and an
  agent writing `---:` under a column of numbers got them left-aligned like
  prose. `|---:|` now right-aligns, `|:--:|` centres, and the header is placed
  like the column it heads.
* **Every nesting level wore the same bullet.** Two spaces of indent was the
  only thing separating a sub-point from its point; the levels now step
  `•` → `◦` → `▪` and cycle.

## 0.19.74

**The todo pane got shot at five sizes, and three things fell out.**

`/todo` was the one whole pane in the app with no pixel coverage at all —
every other surface has a shot harness, and every one of those sweeps found
something. It is now swept at five tile sizes (a narrow column, a short strip,
quarter, half and the whole window) in four states (the list, the `@project`
popup, the done history and an empty pane) on the dark page, a light page and
a green tube.

* **The title kept one column before the chip beside it** while every other
  pair on the row kept two — so a title that happened to fill its budget read
  straight into its tag: `…and reverts @crew` was one phrase. Two now,
  everywhere.
* **A narrow tile hard-broke titles in half.** The right-hand `@project` and
  due are laid out first and take what they need, so on a 36-cell pane
  `ship the release notes` was left three columns and came out as `shi` /
  `p the release notes`. Rows now **stack** below a floor measured against the
  title they actually carry: the title takes the full width on its own lines
  and its chips — with the `✗` that deletes the item — drop to a row beneath
  it. A short title beside a tag and a due still shares one line, because
  moving it down would buy nothing.
* **A short tile showed three of six items and looked exactly like a list of
  three.** The list now carries a **scroll thumb** down its rightmost column,
  proportional to how much is off-screen and reaching both ends of the track —
  drawn only while the list overflows, the same rule the terminal card's own
  border thumb follows.

## 0.19.73

**The PANES section lost its tally, and the three drawn panes got looked at.**

The working / waiting / idle breakdown under the `PANES` rule is gone. It held
three rows — a chip gutter, three state labels, three counts and a pulse wash
behind them — to say what the pane rows directly below it already say, one
pane at a time and in the place you act on it: the spinner IS working, the
bell IS waiting, and a row with neither is idle. `PANES n` on the rule keeps
the one number a glance wants, and the list starts on the row under it, so a
nav shows three more panes before it runs out of height.

Then a **width and theme sweep for `/usage`, `/dash` and `/disk`** — the three
panes that are pure drawing. Each had one shot: one width, one height, one
theme, which is the size a widget is designed at. Nine now, and they found:

* **Both drawn panes were one layout for every pane.** On a full window they
  finished halfway down and left the rest paper — a week of hours drawn as a
  one-row strip, seven days of cost as a four-row smear — while at a quarter
  tile the cost band asked for five rows, could not have them, and was dropped
  whole with six rows sitting empty above it. Both now divide the rows they
  have, the way the left nav does: every band at its floor, then the slack to
  the histories — the heatmap up to three rows a day, then the cost curve.
* **The `/usage` donut** was painted one column right of where the total in its
  hole was written, so `2.2M` put its first character on the ring; and at
  nearly two rows of radius it was drawn from the same row as the word TOKENS
  and ran its top arc through it. Its two swatch dots were placed outside the
  canvas they were drawn on and had never once appeared — dropped, since `in`
  and `out` are already written in their slice's colour.
* **The `/disk` map's picked tile** wrote the page's own near-white ink on a
  bright pastel fill: the one tile you had selected was the one label you could
  not read. Ink is chosen against the tile now, composited at the alpha it is
  actually drawn with.
* **Its header** was one string clipped at the pane's edge, so a narrow pane
  showed a path cut mid-component and no total at all. The reading is placed
  first now and the path takes what is left, elided from the left so the tail
  survives.
* **Its tiles collided.** Six colours picked by name hash, eight children: in
  this repo's own root `crates` and `.git` came out byte-identical, and so did
  `target` and `docs`. Two touching tiles the same colour read as one region,
  which is the one thing a map of areas is for. The colours are dealt now, so
  no two that touch match.

## 0.19.72

**The hold panel is gone.** Resting a thumb on `Cmd` for 450ms put a card of
shortcut chips on the canvas above the input bar. It was meant to answer
"what does this modifier reach from here" without making anyone open `/keys`
— but a modifier is held for a moment in the middle of half the things you do,
and a panel that appears while you are still deciding is in the way of the
decision. `Cmd` is inert again until it is half of a chord.

Removed rather than defaulted off: the module, both state fields, the dwell
check in the poll tick, the scene push, and the manual's section on it — 437
lines out, 6 in. `/keys` (and `Cmd+/`) is still the answer to what the
bindings are.

## 0.19.71

**Font sweeps for the two surfaces a reader changes the font on.** The card
already had one; the input bar and `/keys` did not, and both are laid out in
COLUMNS — a column is a different number of pixels at every size, so the same
window is a wide panel at 10px and a cramped one at 26. A budget that fits at
13 and overflows at 22 is exactly the class of bug a fixed-size shot cannot
see, and the input bar is the one surface on screen in every session.

Both hold, at 10, 13, 16, 19, 22 and 26px: the bar's cwd stays on the top
rule, its status flash stays on the bottom one, and `/keys` gives its key
column no more than the 45% of the panel it promises and wraps every
description under itself rather than clipping it. That is a null result, and
the sweeps are worth having anyway — the two shots that DID find something in
this loop found it the same way, by being pointed at a size nobody had
rendered.

## 0.19.70

**Sextants.** U+1FB00–1FB3B, the 2×3 half-cell grid — the newest of the cell
graphics families and the reason a modern charting library can plot at six
times a cell's resolution with none of braille's dot gaps. A pane running one
of those was drawing every cell of its plot from a font: sixty characters
whose entire definition is "these sixths of the cell are on".

The encoding has four holes in it. The empty and full patterns are `space` and
`█`, and the two single-column ones are `▌` and `▐`, so those four never got
sextant code points and every code point after each hole shifts down. Getting
that off by one draws the wrong sixths for half the range — and every one of
them still looks like a plausible sextant, which is why the contract walks all
sixty and checks that none of them repeats another, that the first is the
top-left sixth alone, that the last is everything but it, and that `▌`'s own
pattern did not sneak in.

## 0.19.69

**Nobody had ever looked at a crew frame through the tube.** Three of crew's
twelve themes wear the CRT post-process — an off-screen scene target, a
half-res bloom chain, and a composite that adds the halo back over the top —
and every shot crew has ever taken went straight from the cell grid to the
readback, which is the path a NON-CRT theme takes. The chain had a headless
test, but it drives synthetic patterns; the shot suite skipped the chain
outright. So the look a quarter of the palette ships had never been in a
picture, and a card whose rules are now exactly one pixel had never been
through the one pass whose job is to spread light sideways.

The harness runs it now (`draw_crt`), and the answer is good: the rule keeps
its hard core at **1.000** through the tube, the halo lifts the surround about
three levels, and the scanlines take alternate rows down and up on a
three-pixel period. A halo that had eaten the core would have stopped being a
halo and started being a blur — that is what the new contract holds.

## 0.19.68

**The tick and the cross came from two different typefaces.** With the
geometry drawn, the census showed what crew's chrome still borrowed: `✗` and
`⌘` from SF Mono, `❯` from Stelo, `☐` from a Nerd Font, `⚑ ↵ ⇡` from Menlo —
while `✓`, which sits *next to* `✗` in every confirm prompt crew draws, came
from the body face. A tick and a cross side by side are the pair the eye is
most likely to compare, and they were two designers' work at two weights.

`✓ ✔ ✗ ✘ ☐ ☑ ☒ ❯ ❮` are drawn now, all from one primitive — a capped line
segment of the rules' own weight — so they share a colour, a stroke and an
optical size with every other mark and every rule around them. A checked
ballot is its box plus its tick, built rather than borrowed.

`⚑ ↵ ⇡ ⌘` are deliberately left to the font: a flag, a return arrow and the
command loop are drawings, not constructions, and a hand-built one reads worse
than a designed one. Four characters of crew's 74-symbol chrome now come from
somewhere else; it was thirty.

## 0.19.67

**crew's own marks were coming from five different typefaces, and one of them
was Apple Color Emoji.** Shaped through crew's stack on a stock Mac with Lilex
chosen, the geometric characters its chrome is written in resolve like this:

```text
Lilex               ○ ●
SF Mono             ▶ ▸ ▴ ▾ ◂ ◀ ▲ ▼ ■ □
Stelo               ◆ ◇ ▪ ▫
Apple Color Emoji   ⏺
```

Five faces is five ideas of how big a mark is, how heavy it is, and where it
sits above the baseline — in one row of chrome, next to each other. And the
emoji one is worse than inconsistent: a colour glyph carries its own pixels,
so the activity dot was the same red-and-white bitmap on every theme crew has,
ignoring the palette outright.

Every one of those characters is *defined* as a shape rather than drawn as a
letterform, so crew draws them: discs, rings, triangles in four directions and
two sizes, squares filled and hollow, diamonds — one size relationship, one
weight taken from the same derivation the rules use, centred on the same
centre, so a `●` in a list lines up with the `─` above it. The same on any
machine and under any font.

A new contract asserts that **no character in crew's chrome may resolve to a
colour glyph**, and prints the face census when it runs. crew now draws 55 of
the 74; what is left over goes to monochrome outline faces.

## 0.19.66

**One answer to "how thick is a rule", and it tracks the font.** The drawn box
glyphs derived their stroke as `cell_height / 16` TRUNCATING — one pixel from
a 16-pixel cell all the way to a 31-pixel one, so every font size from 13 up
to 25 framed its cards with the identical hairline while the text inside them
nearly doubled. A card at a display size read as a thin wire around big
letters. It is [`deco::thickness`](crates/crew-render/src/deco.rs) now — the
same answer an underline already got, because a `─` and an underlined word in
the same pane are both rules and there is no reading of "crisp" under which
they should be different weights. A new sweep shoots the same card at 10, 13,
16, 19, 22 and 26px and holds the rule hard-edged at every one, never thinner
at a larger size, and thicker by the time the text has doubled.

**Edges are graded in fiftieths, not ninths.** A shape given as an
inside/outside predicate was sampled on a 3×3 grid, which can only ever say
ninths — and a near-horizontal roof graded in ninths terraces visibly now that
a canvas pixel is a screen pixel. The coarse pass is unchanged and still
answers for the pixels wholly inside a shape or wholly outside it, which is
nearly all of them; a pixel whose nine samples DISAGREE is on the edge and is
re-sampled at 7×7. The extra work is bounded by the shape's perimeter, and a
mark thin enough to fall between the coarse samples is exactly as invisible as
it was before — that case wants the distance path, which is what its own
contract says.

Also rejected, with numbers: choosing each glyph's subpixel phase to land its
stems on the pixel grid — autohinting by search, since crew renders unhinted.
Swept over 62 glyphs at two sizes it buys **2.0%** and **1.1%** more ink
concentration, which does not pay for up to three quarters of a pixel of
horizontal jitter between neighbouring letters.

## 0.19.65

**`/grain` — the one look knob you could not type.** The paper grain measures
a standard deviation of about **six levels** on a dark page at its default,
which is a lot of texture next to a terminal that has none. Whether that reads
as *paper* or as *noise* is taste, not a bug — but it was taste you could only
exercise by opening `/settings` and finding a numeric field called
"Grain (0-2)", while `/smooth`, `/gamma`, `/weight`, `/leading`, `/opacity`
and `/density` are all named ladders you can type at the input bar and watch
change under you. `/grain [off|light|medium|heavy|<0-2>]` is the same knob
those all are: one `paper_grain` key, two surfaces, applied live. A custom
`/grain 0.4` reports as its number, not as the nearest name.

And the two pickers that had gone stale: 0.19.62 moved `/smooth`'s default to
`off` and `/gamma`'s to `full`, and both value pickers went on offering the
OLD step as "the default" — the descriptions name constants that live in
another crate, so nothing complained. A parity test holds all three ladders
now: exactly one step may claim the default, and it must be the step whose
value the code actually starts at.

## 0.19.64

**Braille is drawn now too — which is what the monitors in the panes plot
with.** btop, gotop, bandwhich and every other tool of that generation draw
their graphs out of braille, because eight dots per cell is four times the
vertical resolution a block ramp gives. A whole chart made of font glyphs is a
whole chart taking the letterform pipeline: rasterized wherever the typeface
put its dots, dilated sideways, lifted at the rim. Crew draws the 2×4 grid
itself now, on a grid the cell owns, so two adjacent cells' dots sit in the
same columns and a rising line reads as a rising line. Each dot is a square,
not a disc — at the four-pixel sub-cell a terminal actually gives it, a disc
is a square with its corners smudged, and the smudge is what the plotted line
would be made of. crew's own spinner comes along for the ride.

A rectangle's coverage is the overlap of two intervals, and `Canvas::rect`
was **guessing at it** — a predicate sampled on a 3×3 grid, whose samples sit
at a sixth, a half and five sixths of the pixel, so an edge inside the first
sixth read as FULL coverage and one in the last sixth as none. That ninth is a
screen pixel now that the canvas rasterizes at device resolution, so it is
computed exactly: every bar, heat cell, treemap tile and snapped rule gets its
true edge, and a whole-pixel rectangle still picks up no fringe at all.

Also: `the_cache_survives_an_unrelated_state_mutation_at_the_same_width` had
been failing about one run in three. The view cache is keyed on the theme,
the theme is a global, and any test switching one in parallel invalidated the
cache between this test's two renders. It takes the theme guard now.

## 0.19.63

**The drawn widgets are drawn at the screen's resolution now — and the box
glyphs cover what the programs in the panes draw with.**

`Canvas` picked a fixed four pixels per cell width. A cell is about eight
device pixels across at crew's default font and sixteen on a Retina display,
so every dial, gauge, area chart, treemap and meter crew draws was
**rasterized at half the screen's resolution and blown up** — a quarter of it
on Retina. Shot side by side, `/net`'s twin chart goes from a visible 2px
staircase along its curve to a clean one. Going FINER is the same defect the
other way up, and three surfaces had walked into it by hard-coding twelve:
quads rasterize at pixel centres with no multisampling, so a canvas pixel
narrower than a device pixel is kept or dropped, not blended — a third of the
coverage those surfaces computed never reached the screen. The canvas asks the
frame how wide a cell is now, and all four local guesses are gone.

The box glyphs gained the **double** and **dashed** runs. Those are not crew's
own furniture — crew frames with the light set — but they are what lazygit,
ncdu, midnight commander and half the ncurses dialogs ever written frame with,
and a pane running one of those was drawing its whole frame out of the font.
The doubles are the hard half: at a turn the outer stroke of each arm runs
past the far side of the other's band and the inner stroke stops at its near
side, and a T-junction's inner stroke steps aside for the branch rather than
walling it off. `╬` is four corners around a hole, not a thick `┼`.

Two of those junction rules were wrong in ways every geometric assertion in
the suite still passed. What found them was printing the masks out and looking.

## 0.19.62

**Crisp: the frames are drawn now, and the text is the weight its outline
asked for.** Three measurements, each one a thing the eye had been reporting
and nothing had been counting.

A card frame's `─` spanned **four rows of pixels with one row at full ink** —
0.20 / 0.78 / 1.00 / 0.25, averaged along the rule so the paper grain cancels.
Every frame, divider, meter and shade in crew is a box-drawing character, and
every one of them was travelling the letterform pipeline: the font's outline,
then the CoreText-style stem darkening spilling coverage sideways, then the
coverage curve lifting the spill. All three are right for a letter and wrong
for a rectangle. Crew draws them now — U+2500–254B, the rounded corners, and
the whole block/shade range — as pixel rectangles in the cell's own box.
Same frame after: **one row at 1.000, zero fringe pixels**, both axes. A row
of `─` is the identical bitmap in every cell, so a long rule cannot wobble,
and `┼` is cut from the same centred span as `─` and `│`, so junctions meet.

The stem darkening is **off by default**. It was added when it was crew's only
text correction and was quietly covering the encoded blend's deficit as well
as doing its own job; `/gamma` took that over honestly, 0.19.28 rebalanced the
pair, and this release asks the question that rebalance did not. Swept over
eight glyphs at two sizes and both polarities: the curve alone delivers 100%
of the outline's light on a dark page and 100% on a bright one, on **45% fewer
inked pixels**, where the old pair delivered 98% and **145%**. The 145% had
never been looked at — the calibration contract only ever rendered white ink
on a black page, so the bright-page overshoot that made paper themes read
blotted went five releases unmeasured. It asserts both polarities now.

And the curve itself now bends by **each run's own colours**. `a^(1/2.2)` is
exact for white on black and for nothing else; over crew's actual pairs it
overshoots, most at the low end, and that overshoot lands on the outermost
pixel of every stroke — which is where a halo comes from. Crew's body pair
asks for about 84% of the correction, a muted comment for less. It needed
nothing plumbed: the amount is already a byte in every glyph's cache key. Two
things had to follow it — the prewarm, which had been painting its whole
working set in flat white-on-black and would have missed every body glyph by
one byte, and the blank cell, which would have minted a second set of atlas
entries for a character with no ink to correct.

## 0.19.61

**Every name crew cuts now says it was cut.** Two of the last three releases
fixed a surface that shortened a name and drew the result as though it were
the whole thing — the `+N` tile's `8 crew · claude-opus-5 revi`, `/far`'s
header saying `· 3.3`. Grepping for the shape that causes it — a
`chars().take(n)` with nothing appended — found six more, all of them text a
person reads.

`/disk`'s treemap drew `vend` in a narrow tile, which is not a directory
anybody has; `ven…` reads as a name that did not fit, which is the truth.
`/blame`'s author column turned "Ashish Tyagi" into "Ashish T", inventing a
person (the label is still exactly the gutter's width, so the text column
cannot shift). A batch job's title is what the pane lists while the prompt is
kept whole beside it, so a cut title should look cut. And a markdown code
block's language label, a fresh agent pane's one line of guidance, and the
file viewer's banner round it out — the last two being the `/keys` lesson in
miniature: a clipped instruction teaches the half that fits, and nobody can
see that the rest existed.

All six go through `chatwidth::clip_w`, which is width-aware and is what every
honest clip in crew already used.

## 0.19.60

**A sheer window holds every overlay solid.** v0.19.51 decided what a
translucent window keeps opaque — the focused card, the input bar and the nav
— on the rule that "the surface your eye is on never has a wallpaper behind
its text". Overlays were not on the list. So the command palette you are
choosing from, the `/keys` panel you are reading and the toast you were meant
to see all had the desktop coming through them, while the card *behind* them
was held solid. An overlay is by definition the surface on top, and every one
of them is now handed to the solidity pass — collected after the scenes exist,
because an overlay's rect is only known once it is placed. The cap rose from 8
rects to 16 (a full toast stack plus an open palette is nine on a busy frame),
and the focused card goes in first so it is never what the clamp drops.

**The README was missing twelve shipped commands.** `/dash`, `/disk`,
`/usage`, `/log`, `/focus`, `/opacity`, `/density`, `/motion`, `/contrast`,
`/shapes`, `/weight` and `/smith` are all in the palette and all in
docs/CREW.md — and were absent from the page most people actually read.
Shipping a feature into a list nobody updated is how it stays invisible after
it exists. The keybindings have had a parity test against the manual for
releases; the commands never did. Three tests now hold it in both directions:
every command in either palette (the bar's and the agent composer's) appears
in README.md and docs/CREW.md, and neither manual advertises a name no palette
offers.

## 0.19.59

**A highlight survives its own spaces.** `/far` — crew's dual-pane file
manager, five thousand lines across twenty-one modules — had never been
rendered off-screen and looked at. One PNG was enough.

`tui::to_cells`, the bridge from a ratatui buffer to crew's cells, skipped
every space cell, background or no background. So any bar drawn with a `bg`
**shattered around its own spaces** and never reached past its last glyph: the
active panel's header drew as three disconnected green blocks with its ` · `
separators knocked out, and the cursor bar covered the glyphs of the selected
row rather than the row. The function-key bar already knew — its pills are
padded with half-block glyphs instead of spaces, a workaround applied in the
one place somebody had noticed. A blank whose background is not the page's own
is a *highlight*, and it is emitted now, which repairs every bar in every
ratatui-backed surface at once rather than only this pane.

The panel header had a second fault: it was fitted to the panel's **whole**
width, and a ratatui block title owns only what is between the borders. The
last two columns were clipped by the block with nothing to show for it — at a
tile width the header read `· 3.3` and the size lost its unit. It now budgets
three columns off, the two borders plus one of rule, the breath every other
card in crew keeps; and a panel too narrow for the count and size drops them
and keeps the directory, instead of handing the block a title it was certain
to cut.

## 0.19.58

**Browsing history says where you are, and what is filtering it.** Up in the
input bar does not walk back through everything you have typed — it recalls
only lines starting with whatever was in the bar when browsing began, the
zsh/fish rule. Both halves of that were invisible. A recalled line looks
exactly like a line you typed, so nothing said you were in history at all;
nothing said a prefix was filtering the walk, so pressing Up twice and getting
two `git` commands out of a mixed history looked arbitrary; and at the oldest
match Up simply stops doing anything, which is indistinguishable from a key
that has stopped working.

`hist 2/5 · git` now rides the **right end of the top rule** — the one border
slot in crew that was always empty, and the mirror of the focused-pane tag on
the bottom one. It says you are browsing, how far back among the *matches* you
have gone (counted the way Up travels), and what the filter is; at `5/5` the
reason Up does nothing is on screen. A deep working directory gives way to it
rather than being overwritten by it, and still keeps the directory you are
actually in.

## 0.19.57

**The question stands as long as the answer does.** `/closeall` and `/only`
ask before doing something you cannot take back: run the command, read the
question, run it again to mean yes. The window in which that second run counts
is **ten seconds**. The question was on screen for **three**. For the other
seven, nothing anywhere said that pressing Enter on `/closeall` again would
close every pane in the window, and nothing said when the window had shut
either — a trigger you cannot see is worse than a question you cannot see.

The ask is a *state* now, not a flash. While the window stands, the input
bar's bottom rule carries the question in the bell colour, outranking both a
transient status and the standing pane name, and it goes the instant the
command is answered or the window expires: the bar stops saying it in the same
moment the second run stops meaning yes.

And a confirmation you have moved on from is no longer armed. `/closeall`
asks, you go and change the gradient instead, and ten seconds later a second
`/closeall` used to fire on the first press, because nothing in between
disarmed it. Any other slash command now does — `/only` and `/closeall`
themselves still answer their own question.

## 0.19.56

**The shortcuts panel finally has a shortcut.** `Cmd+/` (and `Cmd+?` on the
shifted key) opens the keys overlay. It had spent its entire life reachable
only by typing `/keys` into the input bar — a keyboard-shortcuts panel with no
keyboard shortcut — while `Cmd+/` is the chord every other application on the
machine uses for exactly this. It is listed in the overlay's own table and in
both manuals now, because the docs-parity test would not have it otherwise.

Reaching for it exposed a second thing. Typing while the overlay is open
filters the list, which is a good rule that was being applied to *every* key,
super chords included: `Cmd+T` with the panel up put a "t" in the filter box
and opened no shell, and so did `Cmd+W`, `Cmd+J` and every other command crew
has. A Cmd chord is a command, not a keystroke for a search box. A held super
key now closes the overlay and lets the chord through — except `Cmd+/`, which
*is* this overlay, and so simply puts it away.

**The pane card, drawn all at once.** The frame every pane wears is the
busiest surface in crew — twenty-odd readings share its four borders, each
with its own test, and nothing had ever drawn them together. `cardshot_tests`
now does: the numbered hue legend, `[-][x]`, the activity dot and unread
count, bell, broadcast, pin, git badge, elapsed clock, the OSC 9;4 progress
bar, the scroll thumb and its landmark/search/command/error ticks, the focus
brackets, the command-at-top label — loaded, focused and quiet, at two tile
widths. No defects found: the card is exact under load, and the `[-][x]`
buttons that *look* brighter than the legend on an unfocused card measure the
same colour to the byte. Denser glyphs, not a colour bug.

## 0.19.55

**The `+N` tile answers the whole question.** When the grid runs out of full
tiles a pane is demoted to the minimized strip, and when the strip runs out
too, one `+N` tile stands in for the rest. Its entire job is to say *which*
panes are behind it — and it was answering about two-thirds of that.
`overflow_cells` did `take(rows)` and `take(cols)` and said nothing about
either: under a legend reading `+6` the tile listed four panes, and a long
title read `8 crew · claude-opus-5 revi`, as if the pane were called "revi".
Names now ellipsize like every other clipped string in crew, the last row is
spent on `+3 more` when the list outruns the tile, and the leading pane number
— which is how `Cmd+N` reaches it — wears the accent, the way the welcome
hint's chords and the `/keys` key column now do.

**The toast stack held still.** Four cards, hovered and at rest, on three
pages: no defects — the card is structurally exact and the ordinary card that
*looks* faint beside an alert on a light page measures 5.9:1, which is the
hierarchy working rather than a fault. That is now a contract across all
twelve themes (`every_page_carries_the_toast`): the legend ≥ 4.5, the card's
text ≥ 7.0, an alert's bell stroke ≥ 3.0.

## 0.19.54

**The first screen, shot whole — text not markdown, air, and colour.** The
empty-screen welcome is what anybody sees first, and rendered off-screen at
four window shapes it was doing three things wrong. Its "new in …" line is
lifted from the newest changelog entry's bold headline *verbatim*, so the
app's first frame was showing its own markdown — ``new in 0.19.53 · `/keys`,
shot whole``, backticks and all. There is no markdown renderer on this
surface; a backtick here is a character to look past.

Every centred line was admitted on `w < cols`, which allows a line exactly one
column narrower than the card — so in a tall narrow window the opening hint
sat *touching* both frame strokes, which reads as a rendering fault rather
than a layout. Two columns of air on each side now, for the hint, the
tagline, the headline and the restore offer alike.

And nothing on it was crew's own colour. The one line that tells a new user
what to press — `Cmd+T  shell · Cmd+J  agents · /  commands` — was the same
muted grey as the prose around it. The **chords now wear the accent** and the
words stay muted, so the characters you type are the ones that stand out;
`/restore` on the relaunch offer is a chord by the same rule. Structurally,
`welcometext` takes the words and `welcomeart` the drawing, putting
`welcome.rs` under the 200-line cap (322 → 170).

## 0.19.53

**`/keys`, shot whole — nothing collides, nothing is clipped.** The surface
whose entire job is to teach the app had five bindings whose keys ran straight
into their own descriptions: the key column was the constant 26 and
`{left:<26}` pads to a *minimum*, never to a gap, so every binding wider than
that read `Cmd+wheelFont size + / - / reset` or `Triple-clickSelect the word`.
The column is now measured from the widest key there is — capped at 45% of the
panel, because the description is the half that teaches — and a key that still
overruns takes two spaces of its own.

Worse, at any window narrower than the size the panel asks for, ratatui was
cutting descriptions off in silence: "Find: in a chat transcript, or /find in
the ba". That is the exact lesson this module's own doc comment records from
v0.6.57, and it had only ever been fixed for the preferred width. Descriptions
now **wrap**, indented under themselves — the list scrolls, so extra rows cost
nothing. And a section heading (`in an agent pane`) was one more dim row at
the keys' indent; it now carries a rule to the panel's edge, the way every
card in crew states an edge.

Under it: `helplayout` owns the row model and lays the list out in *display*
lines, so `max_scroll` counts what is on screen rather than what is in the
table; `helpkeys` takes the scroll/filter keys, putting `help.rs` back under
the 200-line cap (284 → 172); `helpshot_tests` shoots the panel at three
widths, three states and two themes.

## 0.19.52

**The bar you type into, shot whole.** The docked input bar is on screen in
every session and was the only surface never rendered off-screen and *looked
at*. Three tells were missing the moment it was. A line longer than the field
scrolls to follow the caret, and did so in silence — a 76-character `rg`
invocation read as if it began at `fn build_frame'`; the prompt's gutter (the
blank column after `>`) now carries a `…` while the head is off screen, and
stays blank when it is not. The focused pane's name rides the bottom border as
a tag, clipped to `cols - 4` — the whole bar — so a pane titled with a command
line turned the border into a second line of prose; a tag now gets a tag's
budget (a third of the bar, capped at 28 columns) while a *status flash* keeps
the generous one, because a sentence the bar says once is not a standing
label. And blurring the bar dimmed its border and its prompt but left the cwd
at full accent, so the brightest mark on the canvas belonged to the surface
you had just left — the legend now recedes to `legend_off` with the rest of
the card.

Under it: `inputlegend` owns both border slots, `inputshot_tests` shoots the
bar in nine states across three themes and two widths, and `shotdraw_tests`
carries the GPU plumbing split out of `shotgpu_tests` so a widget drawing its
OWN card is shot at the full canvas instead of nested inside the harness's.

## 0.19.51

**Transparency you can dial, and it knows what to leave solid.** Crew could
already be made translucent — Settings → WINDOW → Opacity % — but it was a
number in a form, and what it did was uniform: the window's alpha rides the
page colour, so the desktop came through everything at once, the card being
read and the bar being typed into included. `/opacity
[off|subtle|medium|sheer|<35-100>]` now puts the knob on the input bar, live
and persisted, and a new scene pass decides what stays solid: **the focused
card, the input bar and the left nav**. The desktop shows through the canvas
around your panes and through the cards you are *not* reading; the surface
your eye is on never has a wallpaper behind its text, and the solidity follows
focus the way the spotlight and the page's light already do.

The named steps are deliberately shy — 97%, 93%, 88% — because translucency is
a texture, not a window into the wallpaper; any percent down to the 35% floor
still works if you want the aquarium. The pass writes **only the alpha
channel**, so the gradient wash, the dot lattice and the paper grain inside a
solidified card are exactly what they would be in an opaque window — a
page-coloured sheet under the cells would have flattened the backdrop precisely
where the wash gathers its light, on the focused card.

## 0.19.50

**The line across your output was the unread divider, and it could not clear
itself.** Crew rules the row under "the last line you had read" so that coming
back to a pane shows where the new part starts. The mark was supposed to
follow the tail in the pane you are focused on — but the guard asked for
`count(total, read_at) == 0` before advancing, which is only true when the mark
is *already* at the tail. It could never fire once a single line had arrived.
So output landed in the pane you were watching, the rule was stamped under the
last line you had "read", and it sat there in `theme.activity` for the rest of
the session: a full-width hairline in the middle of your own scrollback, with
nothing on screen saying what it was. Only typing into the pane or scrolling
back to the bottom cleared it.

Watching is reading. A focused pane at its live bottom now marks the tail read
every frame — the same rule `scroll` and `termwrite` already state, which is
what that guard was written to say. And wherever the rule does still draw (a
pane you are not in, or one you have scrolled back in) it names itself: `12
new`, right-aligned in the row's trailing blanks, in the rule's own colour,
dropped entirely rather than covering a glyph. An anonymous full-width line
reads as damage; a labelled one reads as a boundary.

## 0.19.49

**The font you pinned stops changing with the theme.** `font_family` in config
was applied once at startup and then overwritten by the first theme tick.
Pinning a face already turned off the `/font random` rotation, so a pin
defeated one override and not the other — and `theme = "light"` is itself a
rotation, re-rolling a palette on every launch and every ten minutes, each one
leading with a different face. The rotation deliberately never writes config,
so config went on naming the family you chose while the screen showed Comic
Mono, then MonoLisa, then whatever came next. There was no way back either: a
pinned face need not appear in any theme's preference list, and resolution only
ever draws from those. A theme states a preference; an explicit pin is an
answer. `/font random` is untouched — that is the user asking for the font to
move, and the theme still wins that tie — and `/font` now names the pin that is
holding, so a theme that has stopped changing the typeface reads differently
from a broken one.

## 0.19.48

**A pane's progress bar disappears when it fills.** A program reports progress
over OSC 9;4 and almost none of them clear it before exiting, so a pane whose
program had reached 100 carried a saturated `activity`-coloured stroke pinned
along its bottom border for the rest of the session — a full-width line with
nothing on screen saying what it was. A bar at 100 is not a reading; it is a
line. Every progress bar outside a terminal vanishes when it fills, and this
one does too. The indeterminate comet is untouched: it moves, so it reads as
activity rather than as chrome.

## 0.19.47

**The PANES donut is one chip per pane.** A ring of one pane is a solid disc:
a large black circle spending three rows and seven columns of a docked nav to
say "1". The donut answered with an ANGLE, and the two things a crew is
actually asked — how many, and doing what — are both counts. Each state now
gets a row of chips, one per pane, with its name and count beside it; the
three rows share a left edge, so the states can be compared along a common
baseline the way a pie never allows. Past six the last chip marks its own
overflow and the number beside it stays exact, an empty state keeps a hollow
chip so its row still has an edge, and the crew total rides the section rule
(`PANES 4`) the way the LOG's depth and the charts' peaks already do.

**Settings and /todo, shot whole.** The two form-shaped panes — a bento of
fieldset cards with boxed inputs, pickers and a pinned Save/Cancel row, and a
list negotiating a checkbox, a title, a project chip and a due label for the
same columns — reflow with width, which is exactly the shape that passes every
unit test and still reads badly on a tile. Neither had ever been looked at.

**A focused control declares itself on every preset.** Focus in the settings
form is drawn by swapping muted ink for accent ink. `palette::accent` is
floored against the PAGE — that says the colour can be READ, not that it can
be told from the one it is standing in for. Measured across the set: sepia-dark
**1.04** and crt-violet **1.06**, accent and muted at the same lightness, so a
focused input's border differed by hue alone and on a single-phosphor tube not
at all. `focus_accent` floors it at 1.6, and at 1.8 on the tubes, where
lightness is the whole of the signal.

**The /todo list has a measure.** A row puts its title on the left and its
`@project` and due label on the right, so on a full-window pane the due date
sat ninety columns from the task it belonged to — the same defect the command
palette's chords had. The cap is applied at every entry rather than once at
the draw, because the scroll math and the click hit-test read the same widths
and a wrapped title's HEIGHT depends on the width it wrapped at.

## 0.19.46

**The file viewer, shot whole.** It is the largest surface in the app — a
source rung, a markdown rung, a CSV table, a unified diff, a side-by-side
review, a blame gutter, an outline — and the only one of that size with no
picture of itself. `view_shot` renders each rung at a full window and a half
tile, on a light page and on a tube.

**A `/theme` no longer leaves an open viewer wearing the old palette.** The
viewer caches its rendering, and those lines carry BAKED colours — `ink`,
`text_muted`, the whole syntax ladder — decided once when the cache was
built. The cache key was width, raw, blame width, invisibles and split; it was
never the theme. So `/theme` (and the auto theme flipping at dusk, and the OS
switching appearance) left every open viewer in the previous palette until
something else happened to resize the pane. Dark to light, that is a file
drawn in **(232, 232, 232) ink on a (26, 22, 20) page**: not dimmer, gone.

**A diff's gutter counts the file, not the patch.** Every other rung numbers
rows by their position in the file, which for a diff meant numbering the
patch — `diff --git` was line 1, the first real code line 6. The
side-by-side rung has always shown the source's own numbers from the `@@`
arithmetic, so the two views of one review disagreed about what line you were
looking at depending on which way you had pressed `v`. `diffnums` gives the
unified rung the same numbers: the new file's for context and additions, the
old file's for a deletion, and none at all for the headers.

**Added and removed are two different colours on a tube.** They were red and
green from the theme's raw slots — hue as the signal, which is the right call
on paper and no call at all on a single-phosphor screen, where they measured
**1.00:1** against each other and a review said what changed only through the
marker glyph in column one. On the four tubes the deletion now takes a rung
below the addition (1.70–1.74:1), with its own lower page floor for the same
reason comments have one: a deletion is what the code no longer says.

## 0.19.45

**The tube got its light trace back.** `pane_card_glowing` chose between the
CRT light trace and the modern gradient ring on `theme.modern.is_some()` —
and since "every theme gets the gradient" (0.19.25's ancestor, 0.18.25) every
preset carries a `ModernStyle`. The tubes had been taking the ring and the
trace had been dead code for nineteen releases: no corner nodes, no ignition
decay, no breathing, on the four themes that exist for exactly that. The
branch is `is_tube()` now — the theme's own predicate — and there is a test on
the frame the app actually builds, not on the function nobody was calling.

**Which pane am I typing into?** The frame is the only thing that answers, and
on **fern** a focused frame read **1.60:1** against an unfocused one — two
cards as good as identical. `focused_stroke` floors it at 2.5 for every
preset, a floor and not a restyle: eleven of the twelve clear it untouched.
Measured across the set: paper-dark 8.33, crt-green 6.28, sepia-dark 5.24,
nebula 3.76, harbor 2.77, blossom 2.59 — and fern, alone, under.

**The pixel harness had been photographing half-built frames.** Its fixture
panes were born at shot time, so `assemble_t` was ~0 and every glass, modern
and CRT PNG in this repo showed a card in its first frames of assembly —
corner brackets, no edges. `crt_shot_grayscale_focus_hierarchy` had been red on
main that whole time, and its four numeric claims were sampling pixel literals
that had drifted six pixels off the strokes they were aimed at: it was
comparing one patch of page background with another (31.4 vs 30.5) and calling
it a hierarchy. It writes its PNG and asserts what a shot can honestly assert;
the hierarchy claim moved to the stroke colours, where it found fern.

**And `modern_shot_every_palette` shot two palettes eight times.** Its loop
listed `Nebula` four times and `Blossom` four times, overwriting the same two
files. Harbor and Fern — half the family, and the light pages the test says it
exists for — had never been in a frame.

## 0.19.44

**The menu card, shot whole — and a guard that had stopped guarding.** One
widget draws the slash palette, the attach popup, the model picker, `/todo`'s
tag menu and every value suggestion. None of its five callers had ever looked
at it rendered. `menu_shot` does, across widths and presets.

**A wide palette is a list, not a band.** The chord right-aligns to the row's
far edge, so on a full-window pane `Cmd+D` sat ninety columns from the `/dash`
it belongs to with nothing in between. The list is laid out at the width its
own rows need (`cmdrow::content_w`) and centred in what is left. **A clipped
description now ellipsizes** — `Write the frame's cells to a fi` read as a
rendering fault where `…to a f…` reads as a narrow card.

**Two colours the palette never got to decide.** Every menu description and
chord was `(120, 130, 140)`, compiled into `cmdmenu`; every directory in the
file manager was `(120, 200, 255)`. Both cleared every contrast floor and were
still wrong: a single-phosphor tube can draw one hue, and neither of those is
it. They come from the theme's own muted and cyan roles now.

**And the guard that should have caught that had stopped running.** The
single-phosphor exclusion was spelled `crt.is_none() || modern.is_some()` —
but every preset now carries a `ModernStyle` as a bloom vehicle, so it excluded
all twelve. `chatink`'s syntax-ladder assertions, the stiffest colour contract
in the app, had not been applied to a single tube for as long as that was true.
It is `is_tube()` now, the theme's own predicate, and both tests count the
tubes they checked and fail at zero.

## 0.19.43

**The crew pane, shot whole.** Every chat test so far asserted on cells, which
say what the layout decided and nothing about what the frame looks like — the
same blind spot that let the left nav ship three widgets that each passed alone
and drew wrong stacked. `chat_shot` renders the WHOLE pane — header, transcript,
composer, summary footer — at a quarter tile, a half and a full window, on a
light page and on a tube. Three things were wrong in the first frame it took.

**The transcript sits against the composer.** A session shorter than its pane
was pinned to the top with eight blank rows under it, so the newest reply and
the box you answer it in were the two things furthest apart on screen. It is
bottom-anchored now, the way a shell's output sits above its prompt. The slack
moving revealed that three separate places re-derived where the first line
lands — the fold hit-test, the find wash and the draw — so `chatplace::top_pad`
is now the one of them anybody reads.

**A fenced code block is a block.** It used to draw `╭─ rust`, then a background
that stopped at the end of each line, then a lone `╰─`: two stub corners and a
ragged right edge. It is one tinted rectangle now — uniform width, the language
on its top row, a blank row closing it, the quote bar of a fence inside a
blockquote left outside it. The field's colour was a fixed 18% mix that measured
1.65:1 against the page on sepia-dark and **1.39:1 on the tubes**, where bloom
and scanlines finish it off; it is walked per preset to a floor now, capped so
it never starts swallowing the code standing on it. Cmd+click still copies the
code and only the code.

## 0.19.42

**The SYSTEM gauges became analog instrument dials.** Rings gave way to a 240°
scale with lit ticks and a tapered hand, drawn from signed distance fields
(`plot/sdf.rs`) rather than a 3×3 inside/outside predicate sampled on a canvas
coarser than the screen — which is why every curve stepped in 2px blocks and
anything thinner than a canvas pixel drew nothing at all. `round_box` gave the
footer meters, the scroll thumb and the progress bar the corners they always
claimed to have. Bounding each shape to its own box took a build from 452µs to
167µs for byte-identical output.

## 0.19.41

**The nav answers to the width you give it, and both accessibility switches
reach it.**

The sections used the left half of a wide nav and nothing else: the rings
pinned at column 3, the network rates one short run with twenty columns after
them, the PANES legend with "working" at column 9 and its count at column 35.
The rings now spread up to a cap and then *centre* — past the cap they stop
reading as one answer in three parts — the two rates go to opposite ends of the
row once there is real room for it, and the legend's counts right-align to the
legend's own edge.

**A chart with a moving ceiling writes the ceiling down.** Scaling the CPU
curve to its own rolling minute and the network twin to the louder direction is
what lets an idle machine draw a shape at all; it also took the units off both.
Each rule carries the scale now — `─ SYSTEM peak 55% ──`, `─ NET peak
64 KB/s ──` — from the same derivation the shape is drawn with. The **LOG**'s
rule says how much of the buffer is showing (`─ LOG 8/64 ──`), because a tail
showing eight of sixty-four looked exactly like a log with eight lines in it.

**Reduce motion** now reaches the nav: the attention marker holds still instead
of blinking (and costs no redraws doing it) and the busy-pane spinner holds one
frame, through one derivation so a new spinner cannot be added that ignores the
setting. **High contrast** found a bug in 0.19.40's accent floor — the memo was
keyed on (accent, page), and the OS switch raises the text floor without
touching either, so the request landed everywhere in the palette except the one
colour the user picked.

And the seam `navlayout` was built for is asserted end to end: the row a pane's
title is actually *drawn* on is fed back through the click hit-test, across
every nav height, log depth, git state and crew size.

## 0.19.40

**The accent could take the whole nav with it.** Every theme's own
`accent_default` clears the contrast floor against its page — that is what
picking it meant. The accent the *user* sets never had to. Crew's own brand
green, `#00ffa0`, and the value anyone carries over from a dark theme, reads
at **1.2 against every light page in the set**: on paper-light the section
legends, the clock, the load, the PANES key and the CPU trace were all a mint
that is not there. `spark`, `warn`, `danger`, `cursor` and `link` each grew a
floor of their own; the accent was the one colour on the canvas that never
did, and the only one the user can change. `palette::accent()` is now floored
against the page it lands on (memoised per accent+page), with `raw_accent()`
kept for round-trips. A saturated yellow cannot reach the floor at *any*
lightness on a cream page, so the new `readable::enforced` gives up chroma
instead of giving up the floor.

Held by a contract that walks the nav's **drawn cells** on all twelve themes —
including with a hostile accent — rather than a list of colours someone
remembered to add.

**And the sections say what they are measuring.** LOAD shipped with a trailing
`1·5·15m` hint behind a width check the docked nav has never passed, so three
bare numbers have never said what they were. The rule is the widest and
emptiest part of a narrow section, so the key goes there
(`─ LOAD 1·5·15m ───`). The three loads are one measurement at three ages, so
they differ by rank rather than hue now: the 1-minute figure keeps the load
colour, the two history figures step back through `readable::secondary`.

**Nothing in the nav is cut in half any more.** Every section drew its own
copy of "write at column 3, `.take(cols - 4)`" — a *character* clip. At the
narrow end of what the resize edge allows, the nav showed `Mac.lan · Darw`,
`↑ 0 B` and a load average of `3.`, and a half-written number is not a smaller
reading, it is a wrong one. One `navtext` now holds both rules: **prose
ellipsizes** (host name, pane title, log line), and **a row of values drops
whole values** — a narrow nav shows two load averages, or the busier network
direction, whole. LOAD's key names exactly the averages that survived.

**The PANES backdrop stopped scribbling.** The pulse chart was drawing its
curve — a faint line straight through "working" and "waiting", and across the
donut. A line crossing a word is a scribble however faint it is. It is fill
only now, starting clear of the ring, which no longer touches the card border.
LOG lines from the same minute print their stamp once, so the stamps read as a
scale instead of a stack of identical `23:12`s.

## 0.19.39

**The left nav, looked at as a column.** Ten releases of drawing built the
sidebar's widgets one at a time, each with its own off-screen shot on its own
wide card — and each passing. Shot as the tall narrow column it actually is,
three of them were drawing wrong. NET's centre line was thinner than a canvas
pixel and had never rendered at all, so what looked like an axis on an idle
link was the two direction curves' full-alpha strokes lying on each other: a
solid saturated band on a machine doing nothing. The CPU chart was pinned to
0–100, the one question the gauge above it already answers, which on a laptop
idling under 10% made it a two-pixel smear. LOG lines cut mid-word. Now: each
direction fades toward silence and the axis is a real hairline; the CPU trace
is scaled to its own rolling minute and stands on a baseline; log lines
ellipsize, with the fixed `HH:MM` stamp dimmed so the message keeps the ink.

**And the third of the nav that was empty.** The column's row offsets were
`+` chains re-derived in four places — the draw, the paint layer, and two hit
paths — agreeing only by hand. They are now one `navlayout::layout`, which is
also where the column's slack gets spent: the LOG grows into whatever the
fixed sections and the pane list leave, up to twenty lines, instead of showing
five onto sixty-four buffered ones under a third of a screen of nothing.

## 0.19.38

**`/dash` — the machine and the week on one screen.** The last of ten drawing
releases, and the one that says what the others were for: nothing in this pane
is new data and nothing in it is a new widget. Three **ring gauges** beside a
four-minute **CPU curve**, the **network** with both directions on one axis, a
**heatmap** of seven days of token use, and an **area chart** of what each day
cost — the sidebar's questions asked at a size worth looking at, out of parts
built to be composed.

Bands draw in priority order, so a short pane keeps the machine and loses the
history rather than disappearing.

The twin network chart also gets a floor under its auto-scale: scaling to the
window's own peak made an idle machine's background chatter fill the chart and
read as a saturated link. Below 64 KB/s it draws small now, because below that
nothing is happening.

## 0.19.37

**The card's own readings are drawn.** The scroll thumb on a pane's right
border and the progress bar on its bottom border were runs of box-drawing
glyphs, so both moved a cell at a time: a thumb had 38 stops in a 200,000-line
scrollback, and a build at 3% drew nothing.

Both are drawn now — the thumb slides to a fraction of a cell, the bar lands
where the number says, and an indeterminate report sweeps as a comet whose
leading edge is brightest, so it says which way it is going. Landmark ticks
and search hits stay glyphs: they mark particular rows, and a row is exactly
one cell.

## 0.19.36

**`/disk` — where the space went.** A new pane draws the current directory as
a **treemap**: one tile per entry, its area its share of the bytes, so the
thing filling your disk is the thing filling the pane. The walk runs off the
UI thread and the map fills in as totals land; symlinks count as themselves
rather than as the tree they point at.

Arrows pick tiles in size order (the next key is the next biggest thing),
`Enter` descends, `Backspace` goes up, `r` rescans, `Esc` closes — and the
mouse works the way a map should: click a tile to pick it, click it again to
go in.

## 0.19.35

**The swarm pane shows *when*.** A task list says what ran and how it ended;
six tasks run one after another and six run at once look identical in it. A
running swarm now draws a **timeline** down its right third — one bar per task
on a shared axis, coloured as its state glyph is, with running tasks reaching
a "now" rule and growing while you watch.

So `/goal` fan-out is visible rather than inferred. The bars give way to the
task names on a pane too narrow for both, and the axis stops growing once
everything has finished.

## 0.19.34

**`/usage` — what crew has spent, drawn.** The ledger behind the footer's 5h/7d
countdowns has always held seven days of detail; nothing showed it. The new
pane draws it three ways: a **heatmap** of tokens by hour (a row per day, a
column per hour, shaded against the week's own peak so a quiet week reads as
clearly as a busy one), a **donut** splitting tokens sent from tokens
received, and an **area chart** of cost per day — under the week's spend,
split and peak day.

Hours count back from now rather than from midnight, so the last cell is the
hour you are in. The pane follows the ledger while it is open.

## 0.19.33

**The footer's meters are drawn.** The crew pane's statusline had two dithered
eight-cell gauges — the 5h budget and the context fill — and eight cells only
have eight stops: 12% and 24% drew the same first cell, and small movements
moved nothing at all.

They are capsules now, with the theme's gradient along the fill, landing where
the number says to a fraction of a cell. A meter barely started still draws a
mark rather than reading as empty.

This is the first drawing on a pane rather than a sidebar panel: panes carry a
paint layer from here on, which is what the next widgets need.

## 0.19.32

**NET says which way the bytes went.** The section traced a single line of
rx + tx summed, so pulling a container image and pushing a backup drew the
same chart. It is a twin chart now: **down** grows up out of a centre line,
**up** grows down from it, both on one shared scale so a trickle of uploads
can never look as tall as a flood of downloads. An idle network draws its
axis rather than a gap.

This retires the last block-glyph chart in crew — every chart in the app is
drawn now, and the `▁`–`█` sparkline ramp is gone.

## 0.19.31

**SYSTEM reads as three rings.** CPU, MEM and DISK were three labelled bars
with a track behind each and a number at the end of the row. They are arc
gauges now — the sweep from twelve o'clock says "a third" or "nearly full"
without measuring, the percentage sits in the hole, and the name sits under
it. Same tier colours (accent → amber past 70% → red past 90%) and the same
shape cues as before.

A nav too narrow for three rings keeps the bars, and both take the same rows,
so dragging the nav changes the shape of the answer and never moves the
sections below it.

## 0.19.30

**The crew is a pie now.** Under the sidebar's PANES header sat a one-row pulse
sparkline: eight height levels saying how many panes were busy, with no way to
say what the rest of them were doing.

In its place is a **donut** — working, waiting on you, idle — with the pane
count in its hole and a legend beside it naming each colour and its count. A
category with no members dims rather than disappearing, so the key never
reorders itself under you, and a sidebar too narrow for the legend keeps the
ring. The pulse history did not go away: it is drawn behind the whole block as
a faint area chart, so the ring says the present and the wash behind it says
the last minute.

`plot::pie` is general — slices from twelve o'clock clockwise, a hairline gap
between neighbours (and none around a lone slice), a dim track ring when there
is nothing to show, and a `dot` primitive for legend swatches and series
heads.

## 0.19.29

**Crew can draw now, not just spell.** Every chart in the app was assembled out
of glyphs — the gauges from `█`, the sparklines from the eighth-block ramp — so
a chart had eight height levels, one sample per column, and no way to draw a
curve, an arc or a slice at all.

There is a layer under the text now: a *paint* rectangle addressed in
fractional cells, drawn over a pane's backgrounds and under its text, blended
rather than replaced. On top of it sits a canvas whose pixels are a quarter of
a cell wide, in square units so a circle comes out round whatever shape the
cell is, with coverage-sampled edges and a run merge that turns a solid fill
into one rectangle instead of thousands.

The first thing drawn with it: the sidebar's CPU sparkline is now an **area
chart** — a smooth curve through the samples (clamped so it never spikes where
the machine did not), a gradient fill to the baseline, and a dot on the newest
reading.

## 0.19.28

**The two font corrections were double-counting, and the defaults now say so.**
Crew's stem darkening was calibrated by eye, against Terminal.app, back when it
was the only correction crew had — and back when the gamma-encoded blend was
quietly eating a quarter of every glyph's light. So the strength that looked
right was doing two jobs: its own optical darkening, and covering for a blend
nobody had measured yet.

`/gamma` corrects the blend honestly as of 0.19.25, and reaches 85% of the
outline's light on its own with no darkening at all — very nearly what the old
default reached doing both jobs. Stacked, the pair delivered **106%** of the
light the outline asks for, which is past fullness and into bloat.

`font_smooth` defaults to 70 now, where the pair lands on 99% — the outline's
own light, plus what is genuinely left of the optical darkening. The named
ladder is respaced around it (light 40, heavy 120) so the steps stay spread
either side of the default instead of bunching against the top. A config still
carrying the old default is moved once on upgrade, the same one-shot heal the
0.12.6 theme pins got; a strength you actually chose is left alone.

A test holds the pair to that contract from now on: together they must deliver
between 95% and 103% of the outline's light, and neither default can drift
without the other answering for it.

Rendered end to end against 0.19.23, the five releases of this arc put 20.7%
more linear light into a line of body text — fuller, more evenly weighted
between the round letters and the upright ones, with the counters still open.

## 0.19.27

**Small text no longer takes more darkening than it was calibrated for.** The
CoreText-style stem darkening spills a fixed fraction of a pixel. A stroke is
not fixed: it thins with the size. So the same `/smooth` number takes a larger
and larger share of the stroke as the text shrinks — measured on the embedded
font, a run of body letters gains 39% ink at 9 px against 31% at the 14 px the
ladder was tuned at.

That surplus comes out of the counters — the enclosed white in `e`, `a`, `o`,
`8` — which at 9 px are a pixel or two across to begin with. They were losing
about a third of their open area to the darkening at 11 px, against a seventh
at 32 px, which is what "small text goes muddy" looks like when you measure it.

A one-sided ramp now sheds strength below the reference size and leaves
everything above it untouched: large text is rasterized accurately and never
needed the help, and its share already falls on its own (10% ink at 48 px). The
ramp reads the size straight off the glyph's cache key, so nothing new is
plumbed, and it works in physical pixels — a Retina page at 14 pt rasterizes at
28 px and is already past the reference, which is why it read fine without it.

## 0.19.26

**The text-gamma correction now reads each run's own colours, not the theme's.**
Polarity — which way the coverage curve bends — shipped last release as a
property of the page. It is not. Crew draws dark text on bright badges inside
dark themes, bright text on dark chips inside light ones, and the cursor
inverts a cell's colours outright. A theme-wide answer gets every one of those
backwards, and backwards is worse than off: the curve then doubles the error it
was put there to cancel.

Runs already split on colour, so they now split on polarity too, taken from
each run's own fg against its own bg by WCAG relative luminance — the eye's
weighting, not the tuple's, so green ink on blue ground is correctly light ink.
A character that appears in both polarities is two atlas entries instead of one
bitmap bent whichever way it happened to be shaped first.

The prewarm was painted white-on-black no matter the theme, which under
per-run polarity would have seeded keys no bright-page frame ever looks up —
every glyph on screen paying full freight for rasterization, packing, and at
Retina sizes the atlas grow that re-uploads everything already in it. It is
painted in the page's own polarity now, with a test that fails if it drifts
back.

## 0.19.25

**Crew was throwing away 40% of its text's light, and now takes it back.**
Crew picks a non-sRGB surface on purpose, so text blends on gamma-encoded
values — the web and CoreText look. Nothing was paying that choice's bill: a
pixel at half coverage lands at half in *encoded* space, which is a fifth of
the light it should emit. Measured over the embedded font at body size, white
text on a dark page delivers about 60% of its correct linear luminance, and
reads thin for exactly that reason. Dark text on a bright page has the same
error with the sign flipped, and reads blotted.

`/gamma [off|light|medium|full|<0-255>]` bends the coverage curve back, by
polarity — up on a dark page, down on a bright one. `full` is the whole sRGB
correction: the coverage a glyph asks for is the light it gets. `medium`, the
new default, is about half of it, which puts the midtone at Apple's historical
text gamma. Both curves fix 0 and 1, so a glyph's empty pixels and its solid
interior never move; only the antialiased rim is touched, which on a small
glyph is most of it.

It rides beside `/smooth` the whole way down: the same 0–255 knob idiom, the
same named ladder shared with a Settings picker (**Text gamma**, paired with
Smoothing in the Appearance card), and the same spare cache-key bits — the
amount AND the page's polarity, so a theme switch re-keys every glyph instead
of leaving the atlas serving ink bent the wrong way.

## 0.19.24

**The stem darkening now reaches the curves too.** Crew's CoreText-style font
smoothing widened a glyph by taking, for each pixel, the brighter of its own
coverage and what its neighbours spilled into it. That operator cannot darken a
pixel whose own coverage already beats the spill — which is every pixel on the
flank of a curve or a diagonal. Measured on the embedded font at body size, `s`
took 82% of the widening `l` got and `e` took 78% of `H`'s, so the round letters
in a word read a shade lighter than the upright ones beside them.

The spill now accumulates into the room a pixel has left rather than replacing
its coverage. It never dims a pixel, never exceeds full coverage, and is
identical to the old kernel wherever a pixel starts empty — but a flank at 78%
coverage now goes up, and the gap between `s` and `l` closes to 95%. A test
measures that ratio on real rasterized glyphs and fails below 0.90.

Accumulating lays down about 1.4× the ink at the same strength, so the knob is
recalibrated to keep its promise: `/smooth 100` renders the weight 100 always
rendered. What changed is where the ink lands, not how much of it there is.

## 0.19.23

**Every test that reads the theme is now serialised against every test that
changes it.** The palette, its accent and its gradient poles are process
globals, so a test that paints cells from one of them and compares the result
against `theme()` is comparing against whatever is in force *now* — which under
a parallel runner is not necessarily what it painted with. Sixty-five tests read
the theme without the guard. Two of them had already failed this way this week:
one on the toast border, one on the welcome screen's version stamp, the latter
only on Windows CI where the runner schedules differently.

Eighty-one take it now — sixty-five that read `theme()` outright, and sixteen
more that read something the palette *derives*: a tag's colour, an agent's
colour, a measurement against the page, the accent. That second list cost one
more red run to find, because the first pass had only named the obvious four
needles. A test reads this crate's own sources and holds the rule — because a new test that compares a colour is the easiest thing in the
world to write without a guard, and it passes locally every time until it does
not. Four tests already held it through a helper, which would have deadlocked
had they taken it twice; the check knows about that.

**And the docs describe what these twenty iterations shipped**: the border
naming the command you are scrolled into and the block that failed, OSC 133,
`/blocks`, `/blame`, `/reopen`, `/leading`, `/invisibles`, the side-by-side
diff, the palette's full hand in the picker, and the picker putting a palette
on while you look at it.

This closes a run of twenty UI iterations, `0.19.3` through here. The
through-line: **crew tells you what it already knows.** Nothing in this run
needed new information — the command you are reading, the block that failed,
who touched a line, what you ran and how long it took, whether a path is
clickable, what a palette looks like: crew had all of it and was keeping most
of it to itself. The rest of the run was spent on the lists that had drifted
apart — the palette and the docs, the key maps and `/keys`, the border tokens
and each other — because two lists with nothing comparing them is how a
feature ships invisible.

## 0.19.22

**A test guard that restored half a theme.** `theme_test_guard` put the
palette back when it dropped and left the *gradient poles* wherever a test had
moved them — so a test that only reads `theme()` could still be looking at the
light somebody else turned on. Nothing had moved the poles from under it
before; the `/theme` picker's preview does, and `alert_toasts_border_in_the_
bell_color` (which reads both, and took no guard at all) started failing under
the parallel runner.

The guard restores both now, and that test takes it. Shipped one release late:
0.19.21 went out with it red, because the release check grepped for failures
and a grep that *finds* something exits 0.

## 0.19.21

**A command that failed now says so as it happens.** The border has marked
which block went wrong since OSC 133 landed, and `/blocks` lists it afterwards
— but the notification that fires the moment a long command returns to the
prompt said `✓ … finished` whether it had succeeded or exited 101.

It raises a `failed` card in the bell colour now, naming the status (`✗ cargo
test (2m14) — exit 101 failed in crew`), and a pane you were not looking at
wears `✗` as its attention marker rather than `✓`. Same event, same switch —
a failure *is* a command finishing, and splitting the preference in two would
ask you to say twice that you want to hear about commands finishing. What
differs is how loudly it is drawn: a failure drawn as quietly as a success is a
failure you scroll past.

And "the shell said nothing" is still not a success. Only a *reported* non-zero
status raises the alert; a shell with no integration keeps the notification it
always had.

## 0.19.20

**`/gradient` previews too.** Arrowing onto a named pair puts its poles on the
canvas, for the reason the theme picker wears its palette: a four-cell ramp
beside a name is not the light that pair casts under everything you are looking
at. The *level* rows — `off`, `subtle`, `lively` — are deliberately not
previewed, since they say how far the poles breathe rather than which colours
they are, which is the same distinction that keeps a rotation mode from
standing in for a palette.

Both colours are remembered whichever picker is open, so walking from a palette
row into a gradient row and then dismissing the picker restores **the pair you
had** rather than whichever one you happened to look at last.

## 0.19.19

**The `/theme` picker puts the palette on while you look at it.** Arrow onto a
named palette and the whole window wears it; arrow off, or dismiss the picker
without choosing, and the one you had comes straight back. The strip of
swatches beside each name tells you what a palette *is*; only wearing it tells
you what the screen you are actually looking at will look like — your panes,
your code, your agent's output — and those are different questions.

Three things it deliberately does not do. It does not preview a **rotation
mode**: `dark` names a pool of four, and the one crew would land on is a choice
it makes later, so showing one of them would be promising something the choice
does not. It does not **do what choosing does** — no config write, no accent
re-resolution, no CRT/glass pin clearing, no DECSET-2031 push to the programs
in the panes; a preview that did those would be a choice with an undo rather
than a look. And it does not start a **crossfade**, which at one fade per arrow
key would lag a whole step behind the selection.

The preview settles once per frame, before a single cell is built — a theme
applied halfway through a frame draws the top of the window in one palette and
the bottom in another — which also means every way the picker can go away
(Esc, a click landing elsewhere, a pane taking focus) puts the real theme back
without any of them having to know previews exist.

## 0.19.18

**`/blocks` — what you ran in this pane, and how it went.** A pane's
scrollback is one long column in which everything that ever ran is mixed
together, and the question people actually ask of it — *what did I run in here,
and which of them went wrong* — has to be answered by reading it. Crew already
knows: it records every command's output span from the foreground-process
transitions it polls, and since the last release the exit status a shell
reports. So this is a listing, not a search: newest first, how long each took,
which failed.

Each row is numbered the way `/out`'s argument is numbered, which is the point
of pairing them — `/blocks` says what you ran and `/out 2` opens the output of
the third one back. A block still running says so and counts up. A block whose
shell reported no exit status shows `·` rather than a tick: crew only knows how
a command ended when the shell says so, and drawing "no answer" as success
would be inventing one.

It opens in the file viewer like `/out` and `/diff` rather than printing into
the pane — writing a summary of a pane's history *into* that history is how a
listing becomes one more thing to scroll past. Command spans now carry the
monotonic clock as well as their buffer lines: the lines say where the output
*is*, the clock says how long it took.

## 0.19.17

**Every pane kind that answers to keys of its own now has a section in
`/keys`** — the agent pane, the file viewer, the `/far` panel, the `/todo`
list, and `/settings`. The todo list had two rows in the overlay and six more
actions that were in the manual only (delete, edit, the filter cycle, the
due-date bump); the settings form had none at all, though every field in it is
reached and changed without the mouse. `Cmd+S` also stopped claiming to be only
the broadcast toggle: it saves a focused settings form, and had since long
before this list existed.

The sections live in one list now, which the height, the width, the scrolling
and the filter all read — so adding a pane kind is one row rather than five
edits — and the parity test covers all of them.

Two more things that test needed, and both are the kind of detail that makes
the difference between a contract and a decoration: a modifier chord counts for
the key it *ends in* (`Alt+F2` for `F2`), and it counts case-insensitively,
because a table writes `Ctrl+A` while a key map matches the character `'a'`.

## 0.19.16

**The `/far` panel's whole interface is its function-key row, and `/keys` had
none of it.** F1 through F10, `Alt+F1`/`Alt+F2`, Tab, and the `!` that asks the
AI for a command were drawn along the bottom of the pane and written down in
the manual — and `/keys` is where a user looks for "what can I press here".

The overlay has a fourth section now, and the parity test from the last release
is general: it reads each pane's own key map and holds the overlay to it. Both
pane kinds are in it, so the next one is two lines rather than a rediscovery.

One more thing the generalisation needed: `F2` is reachable only as `Alt+F2`,
and a key map only knows it as `F2` — so a modifier chord in the table now
counts for the key it ends in as well as for itself.

## 0.19.15

**Every key the file viewer answers to was documented in the manual and
nowhere a user could find it.** `/keys` claimed to list "the bindings" and had
none of the viewer's: not `s`, not `r`, not `e`/`o`, not `/` and `n`/`N`, not
`]`/`[`, and not `v` — which had shipped one release earlier. It is the same
shape as `Ctrl+O` before v0.6.46: implemented, tested, and in neither list.

The overlay now has a third section, **in the file viewer**, and a test reads
`viewpane/keys.rs` itself and holds the overlay to it — because two lists with
nothing comparing them is how this happens every time.

That test earned its own lesson on the way in. Matching each key against the
overlay's whole text let `v` be "found" inside the word *viewer*, so removing
its row changed nothing; it matches the key COLUMN now, split into the
individual keys a row names — and split on the separators before `/`, or the
search key (a row spelled `/ · n / N`) is split out of existence by the very
character it is named after.

## 0.19.14

**The border sweep grew teeth, and found three things.** Nine separate
features now ride a pane card's top border — the legend, the `[-][x]` buttons,
the scroll count, the command you are scrolled into, the status glyphs, the
unread badge, the pin, the elapsed clock and the git badge — each added in a
different release, all stepping leftward through one running cursor. The sweep
that says they still fit together now also asserts what its own doc comment
had only claimed: that a token which fits at one width **never disappears at a
wider one**, and that they keep their left-to-right **order**. It also runs
with the newest of them turned on, which the previous release had left `None`.

Three fixes came out of it:

- **Nothing may sit flush against the pane's name.** The two tokens that take
  "whatever is left" were floored at the legend's last column plus *one*, while
  every other pair of neighbours on the border is separated by a cell of frame.
  A card just wide enough for the command name read `claude╶ cargo build…`.
- **The elapsed clock, the unread badge, the pin and the status glyphs had no
  floor at all** and would march straight over the legend on a narrow card
  (`claude2m14`). They all share one now — computed *before* any token is
  stamped on, which also retires the `[-][x]` workaround: the buttons are drawn
  in the legend's own colour at the far right, so the old post-hoc scan had to
  special-case them, and a token that had already eaten part of the legend made
  the scan report the shortened legend as the real one.
- **The legend gives way to the buttons**, rather than running into them. At 24
  columns the border read `claude[-][x]`. A name is already truncated on a
  narrow card; losing the close button is losing a control.

## 0.19.13

**Crew now reads OSC 133, the semantic prompt marks.** It has never needed a
shell integration to know where a command's output begins and ends — it
watches the pane's foreground process, and the two transitions it sees are the
two edges of a block. But that is the one thing polling cannot see all of: a
process crew never saw start tells it nothing about how the command *ended*,
and no amount of watching recovers an exit code.

So when a shell reports one (`ESC ] 133 ; D ; 1 ST`), crew uses it: **the
block's first row is marked on the left border in the alarm colour** — the same
tick that says "here is where this began", said about a command that went
wrong — and the name on the top border carries the status while you are
scrolled inside that block (`╶ cargo build ✗101`). A prompt mark (`A`) also
closes the block exactly, a full poll before the process watch would notice,
which is the whole reason to listen: these are precise where polling is
one-second granular.

It is an upgrade, not a requirement. A shell that says nothing keeps exactly
the blocks it had before, and a block with no reported status is deliberately
not drawn as a success — that is not the same claim. `B` (the command *line*
begins) is ignored: that is where the user is typing, and crew has nothing to
do with it.

## 0.19.12

**`v` in the viewer lays a review out side by side.** A unified diff is a
compression — the two versions of a file interleaved into one column, which is
what makes it fit in an email and what makes it hard to read. Crew's diff rung
already recovers most of what that costs: it pairs each removed line with the
added line that replaced it and dims the text the two share, so the change is
the only thing at full strength. What it cannot recover is *position*. A
removed line and its replacement occupy the same place in the file, and
stacking them says they happen one after another.

So the pairs are laid out where they belong: the old line on the left, the new
one on the right, on the same row. Everything the unified rung knows comes with
them — the pairing, the word-level refinement, the hunk headings — because both
rungs read the same paint. Each side also carries **its own file's line
numbers**, tracked from the hunk header: a unified gutter can only count rows
of the diff, and the number you quote to someone is the one in the file.

A pair wraps *both* sides at the half width and pads the shorter to the taller,
so the two versions never slide out of step exactly where the lines are long
enough to need the help; a side with no line is blank rather than a copy of its
partner, because that is where one version of the file simply has nothing.
Below two honest columns the unified rung takes it back — the toggle is a
request, not a promise the width can always keep — and it is per pane rather
than a setting, since it is a way of reading *this* review at *this* width.

## 0.19.11

**A named palette in the `/theme` picker showed one chip.** That is the same
amount of information each member of a rotation's pool gets, and it is enough
to pick a *pool* out of four and far too little to pick a *palette* out of
twelve: the dark pool's pages are all nearly black, and the accent on top is
the one colour a user is most likely to have overridden anyway.

**A named palette now shows its hand** — the ink it writes in, its accent, and
the four ANSI slots every program in a pane is about to paint with. Red and
green are the ones that carry meaning (a failure, a passing test); yellow and
blue are where two palettes sharing a page most visibly disagree. All six ride
that palette's own page, so the strip is a small picture of what the screen
will look like rather than a list of values. The settings form's theme fields
get the same strip, from the same function.

Two contracts came with it: **no two of the twelve palettes may draw the same
strip** (a new palette landing on another's colours is a row the picker cannot
tell you anything with), and **every face has to read at 3.0 against its own
page** — a chip nobody can see says the palette has no such colour, and these
are the slots programs actually paint with.

**Also:** the settings test that claimed to prove a bogus theme value draws no
chips could not have. `theme_label` resolves an unknown name to the default
before the field is ever drawn, so the renderer never sees one — and the
assertion, a difference between two whole-form chip counts, was being satisfied
by other fields' chips moving. It now asserts what is actually observable
there: which palette the drawn chips came from.

## 0.19.10

**The pointer knew nothing about links.** URLs and file references have been
drawn as links for several releases — tinted, and ruled underneath so they do
not depend on hue — and the pointer over one wore the same I-beam it wears over
every other character in the pane, on text that is one modifier away from
opening a browser or a viewer. A click target with no affordance is a secret,
which is the argument the toast stack's hover already makes and the border
buttons' before it.

Over a marked run the pointer is now a **hand** and the run goes **bold**. Bold
rather than a colour change: the run is already carrying the link colour to say
what it is, and a hover that changed that hue would be saying the second thing
in the same channel as the first. The hand is the one a border button gets —
both are "this does something when you press it".

What counts as a link is exactly what is drawn as one, which is deliberately
less than Cmd+click will open. Answering "any token that names a file on disk"
means a filesystem check, and this runs on every pointer move; the hover
promises what the drawing promised, and the click is free to find more. Row
text is reconstructed for the one row under the pointer, at pointer-move time,
never pre-scanned during layout — and a hover that changed nothing does not
cost a repaint.

## 0.19.9

**A tab-indented file opened in the viewer drew with no indentation at all.**
Not misaligned — missing. A tab has zero display width, and `place_row`, the
guard every cell surface in crew places glyphs through, skips zero-width
characters because a zero-width glyph placed at a column is overprinted by the
next one. Every Go file, Makefile and kernel-style C source anyone pointed
`/view` at came out flush left.

**Tabs now expand to the next multiple of 8 columns** — the terminal's own tab
stop, so `cat file` in a pane and `/view file` beside it agree about how far in
a line starts, which is the number `git diff` and every other tool that prints
a tab-indented file has settled on. To the next *stop*, not by a fixed width,
and measured in COLUMNS: two CJK glyphs put the cursor at column 4, and a tab
after them covers four, not eight. The expansion happens to the text before any
rung sees it, so the syntax paint, the wrap, the search and the diff pairing
all agree about which column a character is in — one of them working from
unexpanded text would put the colour where the glyph is not.

**And `/invisibles` shows the rest.** A tab wears an arrow in its first column,
trailing spaces become middle dots, and the carriage return a CRLF file leaves
at the end of every line shows its own mark: the three that cause real trouble.
They are *marked* rather than merely substituted, so a `·` genuinely in the
file is not dimmed along with them. Off by default — a diagnostic view whose
marks are noise in a file with nothing wrong with it — with a value picker and
**Settings → APPEARANCE → Reveal invisibles**.

## 0.19.8

**`/view`, `/md` and `/batch` had no path completion at all.** The bar's
ghost completion knew about exactly one command — `/dump` — and had since
before the other three existed, so the commands people type a path into most
often were the three that silently completed nothing. The palette's own
descriptions and the completion list are now held against each other by a
test: a command that says `<path>` or `<file>` and is not in the list is a
completion that quietly never happens, which is exactly how this drifted.

**And a path is now a picker, not just a ghost.** Every command with a closed
set of values already opens one — type `/theme ` and the palettes are listed
with the current one marked — while the *path* commands, whose argument is the
one you are least likely to be able to type from memory, got a single ghosted
guess and no way to see what else was there. Typing `/view ` now lists the
directory: folders first, then files by name, filtered as you type. Picking a
folder fills `<cmd> dir/` and leaves the bar open, so the next listing is what
is inside it — the same key walks into a tree and picks out of it. Hidden
entries appear only once the partial asks for them.

One `read_dir` of one directory per keystroke, never a walk — the same read
the ghost already did, bounded for the same reason: it runs on the thread
every pane is drawn from.

## 0.19.7

**`/leading` — how much air sits between rows of text.** Crew's cell height
has always been `1.25 × font_size`: a good default and a bad universal. Dense
code and long prose want different amounts of air, and the reader who finds
tight lines hard to track has had exactly one lever — the font size — which
fixes the tracking by making everything bigger.

This is the knob `/density` deliberately does *not* have. Density moves the
spaces that are genuinely empty (gutters, blank rows) on the grounds that in a
cell grid the line height *is* the cell; that reasoning holds for gutters and
not for someone who wants the same glyphs further apart. Density is how much
crew fits on the canvas; leading is how the text reads.

Only the cell's **height** takes the ratio — widening it would space the
letters of every word apart, a different typographic decision wearing the same
name, and would break the monospace contract every program in a pane draws
against. `normal` is exactly what crew has always drawn, so the setting
changes nothing until you turn it; `tight` (1.10) stops short of solid, where
a monospace face's descenders meet the ascenders below; `loose` (1.65) stops
there because the cell is also the cursor and the selection band, and past it
a highlighted row reads as a stripe with the text loose inside it.

Live and persisted, with a `/leading` value picker and **Settings → APPEARANCE
→ Line spacing**. No process-global, unlike density: the cell box is asked for
in exactly two places — the renderer, handed the ratio when the config is
adopted, and `CrewConfig::line_height`, which has the config in hand — and a
global would be a third answer to a question that already has two agreeing
ones.

## 0.19.6

**A watched pattern that matched every line of output could hide every other
notification crew had.** Toasts stack four deep and drop the oldest to make
room, so an event repeating in a burst — an agent finishing three jobs in a
second, a `/notify` pattern matching a build's every line — pushed a card per
occurrence and evicted the rest of the stack to do it.

**The same thing said twice is now one card that says `×2`.** A repeat is
matched on everything the card says — its text, its legend, and the pane it
would open — so two different events never merge into one wrong count, and it
counts up *where it is* rather than being promoted to the bottom of the
stack: sliding every other card is precisely what the pointer-hold rule
exists to prevent, and the pointer may be resting on one of them. The count
survives the hover rewrite (`waiting ×4 → open`), because the reason you are
hovering may well be that it happened four times.

## 0.19.5

**`/blame` — who last touched each line, in the viewer's gutter.** Reading a
file in a repository, the question that follows "what does this do" is "when
did it become this, and who was there", and answering it meant leaving the
pane, running `git blame` in a shell, and reading the file a second time
without any of the viewer's colouring. It is per-line data about text already
on screen, which is what a gutter is for.

Runs are collapsed — a line is labelled only when its commit differs from the
line above it — so the column reads as *boundaries* rather than as the same
sha repeated forty times. It degrades instead of truncating: `sha author`
where the pane can afford it, the sha alone where it cannot, and nothing at
all below that, never taking more than a third of the pane. `git blame` walks
a file's whole history, so the read is on a worker thread, and a failure says
why on the status line — a gutter that never appears looks exactly like one
still loading. It is a toggle, and a reload drops it: a per-line answer about
text that just changed is a wrong answer.

The column is prepended to lines that are already rendered, reading each
row's source line back out of the numbered gutter it was drawn with, so no
rung had to learn about blame. The text is wrapped at what is left after the
column, which is why turning blame on rebuilds the cache rather than
decorating it.

**Also:** the command palette's table is now three files (`Cmd`, then two
ordered groups) — it grows by a row every release, and where a row *lives* is
a question about file length while where it *ranks* is a question about the
palette. And the accent global's static initialiser dropped the red channel —
correct only because the default accent's red is zero. `pack` is a `const fn`
now, so the initialiser is the same expression every other packing goes
through, and the test that guarded it (by reading the LIVE global, which any
test that had called `set_accent` first could break) is a const assertion.

## 0.19.4

**Scroll a terminal back three pages and nothing on screen says what printed
them.** The prompt that started the output is off the top of the window; the
output is all there is. Other terminals answer this by pinning a sticky prompt
line inside the viewport, which costs a row of the program's grid — not crew's
to spend — and by asking the shell to emit OSC 133.

**The card's top border now names the command you are scrolled back into**,
beside the `⇡N` that appears under the same condition: `╶ cargo build`, in the
command ticks' own colour and wearing the same tick the left border marks a
command's first row with, so the two read as one marking rather than two
features that happen to be about commands. Crew already knew where every
command's output begins and ends — it learns that from the foreground-process
transitions it polls, no shell integration involved — so this is a lookup.

It clears at the live bottom, where the prompt is on screen answering the
question itself. Long names lose their tail and never their head, and the
badge is drawn whole or not at all: it never reaches into the legend that says
which pane you are looking at, and the width sweep that holds it to that runs
every card width from 1 to 80. What it costs the border it takes from the git
badge — the branch does not change while you scroll.

## 0.19.3

**`Cmd+W` never asked, and it never had to be asked — but the pane it took
was gone.** `/closeall` and `/only` learned to confirm; a single close is
supposed to be cheap, and confirming it would only make the common case
worse. The browser settled this argument years ago: let it be cheap, and let
it be undone.

**`/reopen` (Cmd+Shift+T) brings back the pane you just closed** — a shell in
the directory that one was standing in (its live `cd`-tracked directory, not
the one it was spawned in), the viewer back on its file, `/smith` or the todo
list back on the grid. It is a *new* pane in the same place: what died with
the PTY — the scrollback, the environment, whatever was running — is gone,
and pretending otherwise would be a worse promise than the honest one.

The entry it writes down is a `SavedPane`, the same type `/restore` reads out
of `session.toml`, replayed through the same spawn path. Undo-close and
session restore are one question asked over two timescales, so a pane kind
that learns to survive a quit gets undo-close for free — and one that cannot
be described honestly (settings, a swarm view) is skipped rather than
half-reopened. Crew remembers the last 8 closes, so undoing a `/only` or a
`/closeall` walks the grid back a pane at a time and the status line says how
many are left.

## 0.19.2

**The README's theme section described a version that no longer exists.** It
listed sixteen themes — eight of which were retired in the roster cut — the old
`random-dark` / `random-light` rotation names, and the luminous glass sheet the
flat-tube decree removed. Someone reading the front door was being told about
palettes they could not see and chrome that is not drawn.

It now says what ships: twelve palettes in four rotations, named by pool, with
the point that matters — **almost every colour in a palette is derived rather
than chosen**, each derivation guarded by a contract test — and how the picker
shows them. Two stale counts elsewhere in the docs went with it.

This closes a run of fifty-three UI iterations, `0.18.48` through here. The
through-line: **everything crew learns about a pane is drawn on the pane's own
chrome** — git state, elapsed time, unread counts, command starts, errors,
progress, position, landmarks — because a terminal's columns belong to the
program running in them; and **crew derives what other terminals ask you to
configure**, from command blocks without shell integration to a palette's
entire ANSI ladder.

## 0.19.1

**An empty file says it is empty.** Opening one drew an empty pane — which is
what a pane that failed to render looks like, and what one still loading looks
like once the "loading…" banner has gone. Three different states, one blank
rectangle.

The rung that draws a card *about* a file rather than the file itself — a PDF
with no extractor — is exempt: it has plenty to say with no text at all, and
calling it empty would be the same mistake in the other direction.

## 0.19.0

**Scrolling scales with the gesture.** A wheel tick has always been a fixed
number of lines, so crossing ten thousand lines of a build log meant the same
gesture two hundred times. Ticks that keep arriving now build a multiplier —
capped at six, so a flick crosses a long log without crossing the whole
scrollback — and a pause puts it straight back to one line a tick.

The pause is the whole design. The multiplier follows how *quickly* ticks are
arriving rather than how many have arrived, so a scroll you resume after
stopping starts slow again; without that, "start slow" means nothing and the
same wheel cannot be used for reading and for travelling.

The accumulator that lets slow trackpad ticks add up instead of rounding to
zero is untouched and still separately tested: one function per job.

## 0.18.99

**`/closeall` and `/only` ask once.** A closed pane takes its scrollback, its
running command and its agent with it, and both commands sit one fuzzy
keystroke away from `/clear` in the palette. The first run now says what it
would close; the same command again does it.

Not a dialog — crew has no modal to put one in, and a command that answers its
own question with the keystroke that asked it is one you learn without reading
anything. A different command in between replaces the question rather than
answering it (that is the case worth catching), and a question older than ten
seconds is asked again rather than answered late.

Also fixed: `config` had quietly become a private module in this branch, which
turned a `pub` method on `CrewConfig` into dead code and the workspace's
warning-free rule into a warning. The module's visibility is back, and the
lesson is that a blanket string replace on `mod config;` finds `pub mod
config;` too.

## 0.18.98

**The README says what crew has become.** Three sections written one release at
a time — git on the card, the cursor, text decorations — had grown into a list
of unrelated facts while the features underneath them turned into two coherent
ideas. They are now two sections that say those ideas: **what a pane tells you**
(everything crew knows rides the card's border, never the program's own
columns) and **reading what happened** (`/out`, `/errors`, `/errorsall`,
`/diff`, `/pin`).

No behaviour changed. A front door that describes a version from a fortnight
ago is its own kind of bug.

## 0.18.97

**The transcript's position moves to the card's border, and the turns are
marked on it.** The agent pane drew its own scrollbar down the last column of
its content — the one pane kind that did — while every other pane says where it
is on its frame. It says it on the frame now, which gives the transcript that
column back.

Beside the thumb, each of **your own** messages ticks the gutter: a long
conversation shows how many turns it holds and where you are among them before
you scroll. An agent's reply belongs to the turn above it, so only your
messages count. The rows come from the spans the renderer already records for
the fold hit-test, not from a second pass over the transcript.

Also fixed: the welcome screen's "what's new" line **dropped itself** whenever
a release headline was longer than the window — which looked like an
intermittent test failure and was really a deterministic one, tripping only on
the releases whose own headline was long. A window too narrow to hold the
version and a few words still shows nothing; a headline longer than a usable
window is now clipped, since its length is the changelog's doing rather than
the window's.

## 0.18.96

**One read of a pane's rows per frame, not three.** Marking file references,
marking URLs and scanning for errors all needed the same thing — the pane's
cells read back as rows of text — and each built its own copy from the same
cells, one after another, on every frame of every terminal pane.

The frame reads them once now and hands the same rows to all three. The
per-frame work drops by two thirds for the markers, and the three can no longer
disagree about what is on the row they are marking.

## 0.18.95

**A picker says which value you are already on.** `/theme`, `/gradient`,
`/motion`, `/density`, `/contrast`, `/shapes` and `/marks` all offer a closed
set and none of them marked the one in force, so choosing meant remembering
what you had chosen — which is the thing a picker exists to save you from.
The row you are on now says it is the current one.

For `/gradient` that answer is the **named pair** when one is pinned (`ember`),
not the level underneath it, because the name is what the picker offers. Its
eight pairs also sit under a heading of their own now, so the levels and the
colours stop reading as one list.

## 0.18.94

**A paste that would run asks first.** A terminal sends what you paste as if
you had typed it — newlines included — so a multi-line block runs line by line.
Every serious terminal asks about that, and crew did not.

It asks only when the answer matters. A program that enabled **bracketed
paste** — every modern shell, editor and agent CLI — receives the block wrapped
and decides for itself, so nothing runs and nothing is asked. Without it, the
paste is held with a count (`12 lines would run here — ⌘V again to paste`) and
the second Cmd+V sends it: the same key, which is the one you press when you
mean "yes, that one".

One trailing newline is not multi-line — copying a line out of a file takes its
terminator with it, and holding that would train you to confirm everything —
and a hold older than fifteen seconds is dropped rather than sent, because a
confirmation you have forgotten giving is not a confirmation.

## 0.18.93

**Cmd+C copies any pane, not just a terminal.** In the file viewer, a rendered
diff, an agent transcript or a todo list it did nothing at all and said nothing
about it — and those are the panes whose contents you are most likely to want
somewhere else.

Every pane draws itself as cells, so every pane can be read back: rows in
order, gaps as spaces, trailing padding trimmed. A mouse selection still wins
over the whole screen, and a pane with nothing on it now says "nothing on
screen to copy" instead of failing quietly.

## 0.18.92

**Cmd+F does something in a terminal pane.** It has opened the find bar in a
chat transcript for a long time and done *nothing at all* anywhere else —
including in the pane kind crew has most of, where the chord is the first thing
anyone reaches for. It now opens the command bar with `/find ` typed and the
caret waiting, which is the search a terminal pane has.

A key that silently misses is worse than one that is not bound: you press it,
nothing happens, and you learn that crew does not do that.

## 0.18.91

**`/theme` offers the palettes, not just the rotations.** Twelve palettes ship;
the picker listed four. The individual names have always parsed — the docs even
said so — which meant the only way to pin `crt-violet` or `harbor` was to
already know it was there. That is the opposite of what a picker is for.

The four rotations still lead, since they are what most people want, and every
palette follows under a heading, each with the swatch that shows what it
actually looks like. Typing narrows both halves at once (`/theme crt` gives the
rotation and the four tubes), and a heading with nothing left under it is
dropped rather than left standing over an empty space — the same rule the
`/keys` filter follows.

## 0.18.90

**A fourth phosphor: `crt-violet`.** Green, amber and blue are the tubes
everyone remembers; violet is the one on the vector displays, and it is the
only one of the four whose phosphor leaves room for a **warm pink alarm** — the
other three cannot separate their bell from their status by hue at all, which
is why they are the standing exemption in that contract.

It was one of the fifteen palettes retired in the roster cut, so this is a
return rather than an arrival: its name resolves to itself again instead of
being redirected to `crt-blue`, and the retired list is one shorter.

Almost none of it was chosen. A tube's ANSI ladder is one hue at six
brightnesses derived from the *peak* the palette already reaches, so the way to
raise a violet red until it clears the diff floor on a code card is to raise
the tube's brightest slot and let the derivation rebuild the rest — which is
what the parity tests print when you get it wrong.

## 0.18.89

**`/errorsall` — which pane went wrong.** `/errors` answers that one pane at a
time; with six agents running, the question you have first is *which of these*.
It counts the errors in every terminal pane's full scrollback — paged and
bounded, the same walk `/findall` uses — reports the tally per pane using the
same numbers `Cmd+1..9` do, and lands you on the first one, already scrolled to
its most recent failure. It ends somewhere useful rather than on a number.

## 0.18.88

**A clippy warning that shipped.** The command-bar sweep landed in 0.18.87 with
a `filter(..).next_back()` clippy flags as `rfind`, and the release went out
with the workspace not warning-free — which is one of this project's two hard
rules. Fixed, and worth recording: the gate was run, the count was read, and
the commit went ahead anyway.

## 0.18.87

**Two more surfaces swept, and both hold.** The width sweep that found the
invisible git badge and the overprinting sidebar rows now covers the **command
bar** and the **minimized thumbnails** as well.

Neither had a bug, which is worth saying: the bar clips its legends and its
status flash to what the frame owns, and the thumbnail drops its count before
it would land on the marker. Both are now checked at every width rather than
believed, and both checks were shown to fail when the guard they rely on is
removed — a sweep that cannot fail is a sweep that proves nothing.

The bar's check is deliberately not "one glyph per cell": a fieldset legend
draws over its own frame by design, so what it asserts is that nothing escapes
the card and that the corners are still corners once everything has been
drawn.

## 0.18.86

**The sidebar's rows overprinted themselves on a narrow nav.** Same sweep as
last release, next surface. A PANES row carries an index, a focus marker, a
title, a `[+]` restore button, an unread count and a status dot — six things,
added across several releases, each placed by hand at a fixed offset from the
row's width. On a narrow nav they landed on top of one another and on the
title, which is invisible in a screenshot because the last writer wins.

The right-hand side is now claimed from the edge inward: each item takes its
columns only if what remains still leaves the title something to say, in
priority order (the dot's slot, then the `[+]` — the row's only control — then
the count). An item that cannot fit is not drawn rather than drawn over its
neighbour, and every item keeps a column of air beside it.

## 0.18.85

**The git badge has not been drawing on any pane that has buttons** — which is
every full tile. It found its floor by scanning the top border for cells in the
legend's colour, and the `[-][x]` buttons are drawn in exactly that colour, at
the far right: so the legend's "end" came back three columns from the corner
and the badge was left a budget of nothing. It has been invisible since it
landed four releases ago, on every card that could show it.

Found by a sweep, not by eye. The top border now carries the legend, the git
badge, the elapsed clock, the pin mark, the unread count, the scroll count, the
`[-][x]` buttons and three status glyphs — each added in a different release,
all stepping leftward through one running cursor. The sweep runs every width
from 20 to 160 columns with all of them on at once and asserts that **each is
drawn whole or not at all**.

That framing is the point. `put` overwrites the cell it lands on rather than
stacking, so a collision never appears as two cells in one column — it appears
as a *fragment*: `2m1`, or a badge with its last digit gone. A test that
counted cells per column would have passed on this bug forever; the one that
looks for fragments fails the moment the fix is reverted.

## 0.18.84

**`/out 1` reaches the run before the last one.** Each pane already remembers a
few dozen command spans; `/out` only ever opened the newest. An argument counts
back through them, which is the shape of the actual problem: you ran the build,
then `ls`, then `git status`, and the output you want is three commands ago.

Commands that printed nothing are **skipped rather than counted**, so the
numbers mean what they look like — and `/out` and `/out 0` are now the same
reading, which they were not: a command that had just started and printed
nothing made plain `/out` say "nothing to show" while the output you wanted sat
one span back. Asked for a command that is not there, `/out` lists the ones
that are.

## 0.18.83

**`/copy out` copies the last command's output.** `/copy` takes the whole
scrollback, which is rarely what you are reaching for: pasting a failure into
an issue means the run that failed, not the four runs before it and the shell
prompt in between.

It is the same slice `/out` opens in the viewer — one span, two destinations —
so the two can never disagree about where a command's output starts.

## 0.18.82

**Two invisible things made visible** (and a release that forgot to say so).

A **wrapped line now says it is wrapped**: the gutter carries a `↪` where its
line number would be. A blank gutter beside a wrapped row and a blank gutter
beside a genuinely empty numbered row are the same blank, and in a wrapped file
most rows are one or the other.

And a review shows **trailing whitespace on added lines**, as middle dots in
the alarm colour. It is the nit every diff tool marks for the same reason: it
is invisible by construction, so the author did not mean it and the reviewer
cannot see it. Only added lines — what a removed line trailed with is not news
— and only past the `+` marker, so a line of pure indentation still reads as a
line.

Both shipped in the tree tagged `v0.18.81`, whose version bump and changelog
entry were lost to a failed edit: that build reports itself as `0.18.80`. This
release carries the bump the last one should have, so `/update` lands on a
binary whose version matches its tag.

## 0.18.80

**`/marks` turns the border markings off.** The last few releases taught pane
cards to draw on their own borders — a tick where each command began, a bar
beside every error line — and both are on by default because a grid that says
where the failures are without being read is most of the point.

They are still crew drawing on its own chrome about someone else's output, and
a plain frame is a reasonable thing to want. `/marks off` (or the **Card border
marks** checkbox in `/settings`) puts the borders back. `/errors` and `/out`
are unaffected: they read the same thing, they simply stop drawing it.

## 0.18.79

**The first frame says what this build brought.** Crew ships a release most
days, and every one of them writes its headline into the changelog — which is
compiled into the binary already, for `/about`. The welcome screen now shows
that headline as one dim line under the hint: `new in 0.18.79 · The first frame
says what this build brought`.

Discovery is the whole reason. A terminal that grew a `/pin`, an unread
divider, error bars and word-level diffs in a fortnight has no way to mention
any of it, and nobody reads a changelog they have to go and find. A narrow
window drops the line rather than clipping it: half a sentence is worse than
none.

## 0.18.78

**Two places that were counting instead of saying.**

The `+N` overflow tile — the one that stands in for panes the strip has no room
for — said `+3` and nothing else. Which three is the one thing anyone would
look at it to find out, so it lists them now, numbered the way `Cmd+N` numbers
them, as many as fit.

And **file references in agent replies are marked** like the ones in terminal
output. Cmd+click has resolved a path in a chat reply for as long as it has
resolved one in a shell; only the terminal said so.

## 0.18.77

**`/pin` keeps a pane on the grid.** Crew shows six full tiles and demotes the
least-recently-active pane to the strip. That rule is right about which pane
you have not touched and wrong about whether it matters: the pane you are least
likely to touch is often the agent you most want to keep watching, and it was
the first one to disappear.

A pinned pane is exempt. Its card marks itself on the top border — a pane that
behaves differently from its neighbours has to say so somewhere — and `/pin`
again hands it back to the LRU. Pins follow their panes through a close (the
index shifts with everything else, or a pin ends up holding a tile for whichever
pane inherited its number), and more pins than tiles is not an error: the oldest
pins keep their tiles, because a pin cannot make room that does not exist.

## 0.18.76

**The accent field says how it reads.** Crew measures every colour it derives —
the text ladder, the ANSI slots, the search wash, the alarm — against a floor,
and the palette suite holds every shipped accent to `3.0:1` against its page.
The one colour a person picks by hand was the only one nobody was measuring.

The Accent field now shows its contrast beside its swatch (`4.6:1`), in the
alarm colour when it falls under that floor. An accent that cannot be read is
the mistake this field makes easy, and it used to take a save and a squint to
find out.

## 0.18.75

**Two things that were saying the wrong name.**

Panes crew opens on a generated file — `/out`, `/diff`, `/about` — were
legended with the temp path the text lives in: `crew-out-3-cargo.log`,
`crew-diff-Users-me-code-crew.diff`. They are named for what they are now:
`out · cargo build`, `diff · crew`, `what's new · 0.18.75`. The file still has
to be a file; nobody should have to read its name to know what the pane is.

And the "command finished" notification now says **how long it took**: `✓ cargo
build (2m14) finished in pane 3`. A build that took six seconds and one that
took nine minutes are different events, and the notification said the same
thing for both — while the pane's own card has been showing the running clock
since the last release.

## 0.18.74

**The border shows where each command began.** The spans `/out` slices with are
good for more than slicing: every command start visible in a pane now ticks its
left border, so a screenful of scrollback shows where one thing you ran ends
and the next begins.

That is the block structure other terminals ask you to install shell
integration for — drawn as chrome, in a column that belongs to crew rather than
to the program running in the pane, and derived from what crew already watches
rather than from anything anyone has to configure.

The ticks are quieter than the error bars and drawn first, so a row that is
both the start of a command and an error reads as the error: "this failed"
outranks "this began".

## 0.18.73

**`/out` — the last command's output, on its own.** The moment a prompt comes
back, what a build just printed is buried: run together with what you ran
before it, whatever the shell printed after, and every earlier attempt. The
usual answer to this is shell integration — teaching your shell to emit OSC 133
marks — which means editing someone's `.zshrc` and hoping.

Crew does not need it. It already watches every pane's foreground process once
a second, and the two transitions it sees — idle to running, running back to
idle — *are* the two edges of a command's output. Recording the buffer's line
count at each edge gives the span, and `/out` slices exactly those lines into
the file viewer: scrollable, searchable, walkable with `]`/`[`, and left open
while the terminal carries on underneath. A command still running reports what
it has printed so far, which is the case you most want this for.

Second granularity, and the docs say so: a command that starts and finishes
between two polls leaves no span, and one still flushing when the prompt
returns can carry a line or two of the next thing. A span whose end was missed
is closed when the next command starts rather than swallowing everything after
it, and a range is clamped to what the scrollback still holds.

## 0.18.72

**Errors show up on the card.** `/errors` will walk you back to the last
failure; this is the other half — every visible line that reads as an error now
puts a red bar on its pane's **left border**, at that line's row. A failing
build shows where its failures are from across the grid, with nothing typed and
no pane focused.

On the border, not in the content: a terminal's columns belong to the program
running in it, and a marker in column zero would overwrite the first character
of the very message it points at. It is the same reading `/errors` uses, so the
marks and the jump can never disagree about what an error is.

## 0.18.71

**The card says how long it has been running.** Crew has known which command
is in the foreground of each pane for a long time, and stamped when it started
— that stamp was used for one thing, deciding whether a "finished" notification
had earned itself. The border shows it now: `9s`, `2m14`, `1h05`.

With agents in half the panes, "how long has this been going" is the question
you actually have when you look up. A build at nine seconds and a build at nine
minutes look identical without a clock, and only one of them is news.

It appears past five seconds — every command is briefly a running command, and
a clock on every `ls` is chrome — sits before the git badge, since a branch does
not change while you look away, and pads its minutes and hours so the number
does not jitter in width as it counts.

## 0.18.70

**`/errors` walks back to the last thing that went wrong.** A long build
scrolls its own failure off the screen, and getting back to it meant either
remembering a word from the message or paging up through everything that came
after. `/errors` scrolls the focused pane to the most recent line that reads as
an error and says how many are in view; repeating it steps to the one before,
the way a repeated `/find` does, because a failing build has more than one and
the one you want is rarely the last.

The definition is deliberately narrow, since a jump that lands on "errors are
handled below" teaches you not to trust the jump: an error has to announce
itself at the **start of a line** — after whatever indent, quote bar or box
edge a TUI drew around it — or right after a `file:line:col` prefix. That
covers `rustc`, `tsc`, `gcc`, `git`, Rust panics, `npm ERR!`, Python tracebacks,
`pytest`, TAP's `not ok`, and the `✗` most test runners mark a failure with,
while leaving prose, `warning:` lines and `0 errors, 2 warnings` alone.

## 0.18.69

**`/keys` filters as you type.** The overlay lists forty-odd bindings and every
key press dismissed it, so finding one meant scrolling past all the others. It
is a document, and the fastest way through a document is to say what you are
looking for — so letters now narrow the list instead of closing it, matching
the chord as well as the description (`ctrl+tab` finds it, and so does `pane`).

A section heading survives only while something under it does — a title over no
rows is a lie about where you are in the list — and a search that matches
nothing says so rather than emptying the panel, which reads as a rendering
fault. What you typed is shown where the version normally sits, because a
filter you cannot see is a list that looks broken.

**Esc** closes it, as does any key that is not a letter, a space or Backspace,
so nothing traps you in there. The filter is forgotten on the way out: one you
meet again with no memory of having set it is worse than no filter at all.

## 0.18.68

**The unread count reaches the sidebar, and scrolling counts as reading.**

The PANES list is the one view that names panes you cannot see anywhere else —
minimized, zoomed out of, behind the strip's overflow tile — so it is the place
the count was most missing. Each row carries it now, between the title and the
dot slot, and never on the row you are focused on: the pane you are looking at
cannot have unread lines. A long title gives way to the count rather than
overprinting it.

And the mark now clears when you **scroll back to the live bottom** of a pane,
not only when you type into it. Arriving at the bottom means you have been past
everything above it; a mark that survives that is a mark you have to dismiss by
hand, which is not what it is for. Only at the bottom, though — scrolling
*through* the new lines is exactly when the divider is still doing its job.

## 0.18.67

**Programs can say things for themselves now.** Crew's notifications have all
been *inferences* — a command finished, a bell rang, a pattern matched, a pane
looks blocked. Two escape sequences let a program simply say what it wants
said, and both were being dropped on the floor: `ESC ] 9 ; text ST` (iTerm2 /
ConEmu) and `ESC ] 777 ; notify ; title ; body ST` (what most Linux tooling
emits). They now raise a real notification with the program's own words and the
pane's name beside them. Since it was *requested* rather than guessed, it rides
the master `notify` switch alone.

**And progress reports draw a bar.** `ESC ] 9 ; 4 ; state ; percent ST` fills
the card's bottom border in proportion — in the alarm colour for the error and
warning states, and as a short block sweeping back and forth for the
"working, no number" state, which is more honest than parking a bar at an
arbitrary percentage. The border, not a row: a terminal's columns belong to the
program running in it.

The OSC sniffer that has been quietly reading working-directory reports since
the beginning now reads all three, from one state machine, so a sequence split
across two reads is still one sequence.

## 0.18.66

**A diff pasted into chat reads like `/diff` does.** The viewer's diff rung
pairs each removed line with the line that replaced it and draws only the run
that actually differs at full strength; a fenced diff in a chat reply or a
markdown file went through the markdown renderer instead, which colours one
line at a time and knows nothing about the line after it — so the same diff was
a wall of red and green depending on which pane it landed in.

Rather than teach the renderer about pairs, the refinement now reads the
*rendered* lines back: a line whose ink is the added or removed slot is an
added or removed line, whatever produced it. One refinement, two surfaces, and
the same two conservative rules as before — unequal runs are not paired at all,
and a pair that differs almost everywhere is left plain.

## 0.18.65

**The minimized strip says how much you missed.** A demoted thumbnail showed a
title and a dot, and the dot could only ever say "something happened". The
strip is where a pane goes when you have not touched it for a while — precisely
where the question is *how much* — so each thumbnail now carries the same
unread count the full cards do, right-aligned, with the marker keeping the left.
A thumbnail with no room for both keeps the marker, since that is the one that
says the pane is alive at all.

Thumbnails are **numbered** now too, like the full tiles: `Cmd+N` reaches a
minimized pane, and the number is how you know which N.

## 0.18.64

**The settings form shows the colours it is offering.** The theme pickers list
palette and pool names, and the accent field holds a hex code; both left you to
Save and look. They now draw the colours inside the right end of their boxes —
one chip per palette in a pool (its page with its accent across the top half),
and the accent's own hex as a block. These are exactly the chips the command
palette's `/theme` picker draws, which is the point: the same question gets the
same answer wherever it is asked.

The chips answer to the *value*, not to the field, so a picker showing
something that names no palette draws nothing rather than the last thing that
did — and a box too narrow to hold both the value and the chips keeps the
value, which is what is being edited.

## 0.18.63

**File references look like the links they already were.** `src/main.rs:42`,
`./deploy.sh`, `Cargo.toml` — Cmd+click has resolved these for a long time, and
nothing on screen said so. They are marked now, in the link colour with a
**dotted** rule where a URL wears a solid one: same colour because both are
links, a different rule because a URL leaves for the browser and a path opens
here.

The matcher is narrow on purpose. A mark on `and/or` or on `e.g.` teaches
people to ignore every mark, so a reference has to be either something with a
directory separator in it or a bare filename with a real extension (2–6
characters, not all digits) — which leaves `TCP/IP`, `10:30`, `v1.0`, `Fig.2`
and `a.b` as the prose they are.

**And `path:line` clicks work.** They never have: the position was part of the
clicked token, so the file was looked up under a name it does not have and the
click did nothing at all. The position is split off, the file opens, and the
viewer lands on that line — at the *top* of the window, because the lines after
the one you were sent to are the ones you came to read. The read is on a worker
thread, so the pane exists before there is anything to scroll; the ask waits
for the text and is forgotten once spent, which is what stops the next reload
from jumping on its own.

## 0.18.62

**The viewer's search shows itself.** Typing `/` in the file viewer was
*blind*: the needle lived in the pane's state and was drawn nowhere, so a
mistyped search and a search with no matches looked exactly alike — both of
them "nothing happened". The needle is now drawn on the pane's last row with a
caret while you type, the tally once you confirm it (`/alpha  2 lines`), and
**"no matches" in the alarm colour** when there are none, which is the one case
where that line has something to correct rather than something to report.

**And the gutter shows where the matches are.** The card's right border already
carried the landmark ticks; a live search now marks its hits over them in the
search's own colour — while you are looking for something, where the matches
are outranks where the sections are, and where you *are* (the thumb) outranks
both. A single cell painted in the highlight *background* would disappear
against the page, so the tick takes that colour's hue at ink strength.

## 0.18.61

**Where the new part starts.** A grid of panes means most of them are producing
output while you are reading one of the others, and every return to a pane
asked the same question — *what of this have I already seen?* — with scrolling
up until something looked familiar as the only answer.

Each terminal pane now remembers how many lines its buffer held when you last
read it, and draws the boundary as a **rule under the last line you had seen**.
Not a banner row: a banner covers a line of output, and what is being marked is
the *gap between* two lines — which is precisely what an underline on the row
above is. (The underline machinery that landed three releases ago turns out to
have been the right primitive for this.)

The card's top border carries the **count** beside its activity dot, capped at
`99+`. The dot has always said "something happened here"; the number is the
difference between glancing over and going back.

The mark follows the tail while you are watching a pane with nothing new below
what you have seen — so looking away marks everything up to that point as read
— and resets when you **type into** the pane, because answering is reading.

Also fixed: the previous release's documentation used `/them` as an example of
a half-typed command, and the docs-drift guard is right that a backticked
slash-token in the prose has to be a command that exists. That test was red on
0.18.60 because the docs were written after the test run rather than before it.

## 0.18.60

**The command bar colours what you type.** Every character in it was `ink`, so
the bar had nothing to say about your text until you pressed Enter and found
out. Now a leading slash command is **accent** when it resolves, muted while it
is still being typed, and drawn in the **alarm colour when nothing begins with
it** — `/them` is on its way to `/theme`, `/zzz` is on its way to nothing, and
the difference is worth knowing a keystroke earlier rather than a command
later. Flags recede; a quoted run is marked from its opening quote to its
closing one, and an unterminated quote marks to the end of the line, which is
exactly how you notice it is unterminated.

Three marks and no more. The bar is one row and the text in it is short: a
syntax highlighter's worth of colour on twelve characters is decoration, not
information.

Two viewer tests that compare rendered colours were holding no theme lock, so
they could disagree with the cells they had just built when another test
switched palettes mid-run. They hold one now — the same fix the welcome-screen
test needed last release, and the same cause.

## 0.18.59

**The viewer knows where it is, and shows it.** Two gaps closed at once.

`]` and `[` now step **markdown headings** as well as diff hunks. The markdown
renderer already decides which lines are headings, so the landmarks come from
it rather than from a second pass over the source — which also means they are
rendered rows, already wrapped, and land exactly where they are drawn. Raw mode
(`s`) has no landmarks, because it has no rendered headings: it is the escape
hatch for reading the bytes.

And the viewer's card **draws a scroll thumb at all**, which it never has. It
had the one pane kind most likely to be longer than its window and the only one
with no position indicator on it. A document's gutter appears as soon as the
content is longer than the pane rather than waiting to be scrolled — a shell's
gutter is a scrollback affordance ("there is something behind you"), a
document's is an answer to *where am I*, and that question exists at the top of
the file too.

Beside the thumb, the landmarks are drawn as **dim ticks**: a long file shows
its shape — how many sections, how far apart, where you sit among them —
before you move. Ticks that land in the same cell are one mark, and the thumb
is drawn over the tick it shares a cell with, because where you are is the
answer that wins.

## 0.18.58

**Two new palettes: `harbor` and `fern`.** The dark pool was neutral
(`paper-dark`), warm (`sepia-dark`) and violet (`nebula`); the light pool was
warm, cream and violet. Both were missing the cool, green-blue end of the room
entirely — so `harbor` is a deep blue-slate page under an azure light with its
gradient running azure into teal, and `fern` is a faint mint page under a
green-teal light, the only light palette whose accent is green and therefore
the only one that can never be mistaken for the two warm ones at a glance.

They rotate inside `dark` and `light` like every other pool member; a fresh
install on `auto` will now meet one of them.

**Almost none of their colours were chosen.** The palette system derives them:
the ramp produces the whole text ladder and all sixteen ANSI slots from the
page and the ink, the wash produces the search highlight, and the alarm
derivation produces the bell. What a new palette actually picks is its page,
its ink, its accent and its gradient poles — everything else came out of the
parity tests, which print what the derivation says when a hand-written value
disagrees with it.

Three contracts pushed back while these were being drawn, and each one was
right: `harbor`'s first azure was too quiet for the dark pool's accent band
(the pool's accents must read within 2.45× of one another, and it sat at
2.68×), `fern`'s amber status could not be told apart from a derived red alarm
on a mint page (its status is now the palette's own deep blue, which the red
separates from by hue), and the roster count is now written as a sum so adding
a palette does not read as retiring one.

## 0.18.57

**Dim text is dim.** SGR 2 is how a CLI says "this is context, not the answer",
and agent CLIs put half of what they print in it. Crew read the flag, kept it
in the grid, and drew the cell at full strength anyway — so a `claude` or
`codex` pane was a wall of equally loud lines, with no second voice at all.

A dim cell is now mixed toward the page **in linear light**, so the colour
survives the quieting (a dim red is still red — otherwise every colour-coded
secondary line turns the same grey), and then held to a **lower contrast floor
than body text**: quieter than normal, because that is what was asked for, but
never below readable. That floor matters more here than anywhere: a program
that sampled the page once and guessed wrong is already painting near the
background, and dimming *that* lands on top of it.

**SGR 8 (conceal) is honoured** as well. A program hiding what you type — a
password prompt — now has it hidden. The cell keeps its background, so a
concealed field still occupies the space it claimed, and the characters stay in
the grid for a selection to copy.

## 0.18.56

**`]` and `[` walk the review.** A diff is not read top to bottom; it is walked
file by file and hunk by hunk, and the only way to do that in the viewer was
PageDown and a careful eye. The two bracket keys now step the document's
structure: every `diff --git` header and every `@@` hunk is a landmark, named
by the function context git already puts after the range (or by the range
itself when there is none), and a renamed file is listed under the name it has
now.

Landmarks are found in the source and reported as **rendered rows** — a wrapped
line occupies several of them, and the banners above the body push every row
down — so `]` lands on the row the landmark was actually drawn at rather than
somewhere near it. That mapping is the whole reason the rung now hands back
which source line each rendered row came from, which is a thing every gutter
rung can answer and no one had asked for yet.

At either end nothing happens. A review has an end, and wrapping to the top
from it is how you lose your place.

## 0.18.55

**`/diff` is a review now, not a scrollback.** It used to run `git diff` in a
terminal pane and let git's own colours land in the scrollback: every removed
line red, every added line green, and the job of finding *what* changed inside
two nearly identical lines left to your eyes.

The diff opens in the **file viewer** instead, rendered by crew's own diff
rung. Each removed line is **paired with the added line that replaced it**, and
only the run that actually differs is drawn at full strength — what the two
lines share recedes toward the page. A change inside an identifier marks the
identifier (`foo_bar` → `foo_baz` is a different name, not a `r`), hunk
headings are set apart from the function context after them, and `+++`/`---`
file headers are headers rather than a one-line addition and removal.

Refinement is deliberately conservative, because a mark that might be wrong is
worse than no mark: runs of unequal length are **not paired at all** (there is
no honest correspondence to draw), and a pair differing almost everywhere is
left plain (marking all of both is not a mark).

The three `git` reads run **off the winit thread** — a large or
network-mounted repo takes seconds, and anything blocking that thread freezes
every pane in the grid, agents included — and the pane opens the tick they
land. The repo reviewed is the **focused pane's** directory, so `/diff` in a
pane working in another checkout reviews that checkout. A clean tree says so
rather than opening an empty pane. The viewer's scrolling, search and reload
come along for free.

## 0.18.54

**The colour pickers show the colours.** `/gradient` offers eight named pairs
and `/theme` four rotations, and both listed them as *words*: `aurora`, `moss`,
`dusk`, `dark`, `crt`. The only way to find out what one looked like was to
pick it and see — which is the single thing a picker exists to prevent.

Each `/gradient` row now draws its pair as a **four-cell ramp** from one pole to
the other, so `mono` reads as flat and `ember` reads as warm before you press
anything. Each `/theme` row draws **one chip per palette in its rotation**: the
palette's page as the cell's background with its accent across the top half.
Two colours per chip, because every page in the dark pool is nearly black and a
row of page colours would have been one smudge — the accents are what tell
`nebula` from `paper-dark`.

On a row whose whole subject is a colour, the colour outranks the sentence
describing it, so the swatch is placed before the description and the
description gives way first when the card is narrow. Every row still fits its
card: swept 1 to 80 columns, swatch and chord and all.

## 0.18.53

**The palette shows its work.** Three changes to the surface you open most.

**The matched characters are marked.** The palette has been fuzzy for a while —
`/dmp` finds `/dump` — but the row gave no sign of *why* it was a match, which
makes a fuzzy hit look like the list ignoring you. Now the letters that matched
are drawn bold, prefix run or scattered subsequence.

**The descriptions line up.** They were placed two spaces after each label, so
every row started its description somewhere else and the list could only be
scanned, never read down. There is a column now, capped at half the row so one
long command cannot push every description off the card, and a label wider than
the column pushes its own description rather than being cut in half by it.

**Commands with a chord say so.** `/clear … Cmd+K`, right-aligned. A palette
that names the shortcut is how you stop needing the palette. The five listed
are exactly the ones where the chord does *that command* — and a test reads
`chords.rs` itself to confirm each one is still handled, because a shortcut
column that teaches a key which does nothing is worse than no column.

The row lays out in a fixed order as the card narrows: the chord goes first,
then the description truncates, then the label itself — the description being
what a row is for. Swept across every width from 1 to 80 columns.

## 0.18.52

**Everything a TUI paints with was invisible.** A terminal program draws its
status line, its progress bars, its selected row and its diff blocks with
*coloured spaces* — and crew threw every blank cell away before it had even
resolved what colour the program asked for. The heuristic that decides which
program-painted backgrounds belong on a flat canvas sat directly below that
filter, judging backgrounds it never saw.

So: `fzf`'s highlighted line, a `vim` status bar, a progress bar drawn as a
run of blocks, the panel behind a TUI menu — none of them rendered. Neither did
**a selection dragged across empty space**, for exactly the same reason.

They render now. The flat-canvas rule is untouched — the near-grey the agent
CLIs paint behind your last line is still flattened, as is any low-saturation
or bright highlight background on a dark theme — but a background that carries
meaning survives, and the blank cell carrying it is drawn.

Plain blanks are still dropped, and now dropped *earlier*: the test moved above
the colour and contrast work, because a terminal is mostly empty and a
screenful of nothing must not pay a contrast floor sixty times a second.

## 0.18.51

**Every card says which branch it is on.** Crew has known how to read git
status for a while, but only ever asked about its own working directory, and
only ever showed the answer in the sidebar. Panes have their own directories —
that is the point of them — so the fleet could have an agent committing in one
worktree and a test run in another and the two cards looked identical.

Now the top border of each pane carries `main ●3 ↑2 ↓1`: branch, changed
files, ahead and behind. A clean repo is just its branch; there is no tick and
no run of zeroes to read past, because nothing to report is nothing to draw.

The border is a row already carrying a legend, a scroll count and status
glyphs, so the badge **degrades in a fixed order** rather than by eye: behind,
ahead, dirty count, then the branch truncates, and under four columns it draws
nothing at all — `m…` is not a branch name. A test sweeps every width from 0 to
60 for four repo states and asserts the badge never exceeds its budget and
never shows *less* as the card gets wider. The last time a per-field width was
decided by eye, at whatever width the pane happened to be, two fields shipped
clipped.

Queries are **off the winit thread**, one at a time across the whole fleet,
throttled per directory — and directories no pane holds any more are forgotten,
so a long session's map stays the size of the fleet rather than of its history.

## 0.18.50

**The cursor has a shape again.** Every cursor in every pane was a filled
block. Programs have been asking for other shapes for forty years — `ESC[6 q`
for a bar, `ESC[4 q` for an underline — and vim, helix, zsh's vi-mode and the
agent CLIs all use them to say which mode they are in. Crew read the escape,
kept it in the grid, and drew a block anyway, so insert mode and normal mode
looked identical.

Now the cell carries the cursor: block, bar or underline, drawn as quads on the
cell's edges. **Only the block repaints the cell it lands on** — it inverts it,
so the glyph stays readable — while the bar and the underline are rules beside
the glyph, which keeps its own colours. A bar that recoloured its neighbour
would make the character it points at change colour when the cursor arrived.

**An unfocused pane draws an outline.** The old cue was a dimmer block, and it
was the wrong cue twice over: on a canvas of panes the question is which one
takes the keys, and *dimmer* is a hard comparison to make across a grid, while
*hollow versus filled* is one you make without looking for it. It also means
the unfocused cursors are now easy to find rather than nearly invisible — the
outline's colour is floored against the page, because an outline is a fraction
of the ink a block is and the old colour was chosen for a block.

## 0.18.49

**Hyperlinks that are links.** OSC 8 is how a program attaches a URL to
arbitrary text — `ls --hyperlink` on a filename, `gh` on an issue number, a
test runner on the file that failed. Crew parsed the escape into the grid and
then had no way to show it or follow it, so those cells rendered as ordinary
words: nothing said they were clickable, and clicking scanned the visible text
for a URL and found none.

Now a hyperlinked cell is **tinted and ruled like a URL even when its text is
prose**, which is the only cue available — "see notes" looks like nothing. And
Cmd+click opens **the program's target**, resolved from the grid rather than
guessed from the text.

That target comes from whatever is writing to the pane, so it is not trusted
the way a URL you can read is. Crew opens `http://`, `https://`, `mailto:` and
`file://` links, matched case-insensitively (`HTTPS://` is a URL, and lowercase
-only matching is how `JavaScript:` gets past a filter), and refuses anything
else **by name** on the status line rather than silently. The status line names
the URL that is actually opening in every case: link text can say one thing and
point somewhere else, and the person clicking should see which.

A hyperlink spanning two words keeps the space between them, for the same
reason an underlined run does.

## 0.18.48

**Underlines, at last — the whole family.** A cell could be bold or italic and
that was the end of what it could wear. Every underline a program asked for was
read off the wire, parsed into the grid, and then dropped on the floor at the
last step, because the struct that carries a cell to the GPU had nowhere to put
it. So `rustc`'s inline diagnostics, a language server's squiggle, `git diff
--word-diff`, a spell-checker in a TUI — all of them rendered as plain text,
indistinguishable from the words around them.

Now the cell carries a decoration: **SGR 4** single, **4:2** double, **4:3**
curly, **4:4** dotted, **4:5** dashed, **SGR 9** strikethrough, and **SGR 58**'s
separate underline colour — red rule under white text, which is how an error is
marked inline.

They are drawn as quads on the pane's pixel grid, not as glyph decorations, and
**every phase is taken from the absolute x**. That is the whole design: a
squiggle spanning six columns of a diagnostic is one continuous wave rather than
six little waves that restart at each cell edge, and a dash pattern crosses a
cell boundary mid-dash. Two tests hold that line by construction — both use a
period that does not divide the cell width, because a period that does hides the
bug whether or not the phase is absolute.

A decorated *blank* now survives the empty-cell filter too. It has to: the space
between two underlined words is part of the same rule, and dropping it draws the
underline in pieces.

**URLs are underlined as well as tinted.** A link marked only by hue is not
marked at all for a reader who cannot separate that hue from the body text —
the same argument the gauges' shape cues make.

## 0.18.47

**The palette remembers what you run.** Among commands matching your query
equally well, the order was `cmddefs`'s declaration order — an order that means
something to whoever last edited that file and nothing at all to the person
typing. Type `/` on an empty bar and the list opened on whatever happened to be
at the top of the table, every time, no matter that you had run `/gradient`
forty times and `/clearlog` never.

Now the ten most recently run commands break the tie, persisted across
restarts (a shortcut that resets every launch is not one). A repeat moves its
entry rather than adding one, so the list stays a summary of your habits
instead of filling with one of them.

The rule that keeps it predictable: **recency reorders within a match-quality
band and never across one**. A prefix match still beats a fuzzy match, always,
so `/de` can never float something that does not begin with `de` above
something that does. A learned list that can reorder the *kind* of match is a
list you can no longer aim at — you would have to read it every time instead
of typing through it.

Only commands that exist are recorded: a typo is not a habit, and remembering
one would put a command that does nothing at the top of your list.

## 0.18.46

**The settings form fits the pane it is in.** Its fields were paired into
half-width boxes or given the whole row by a per-field decision taken once, by
eye, at whatever width the pane happened to be when it was written. That had
already gone wrong: `‹ sepia-light ›` is 15 columns, a half-width box holds 14
at an 80-column pane, and the Auto-dark picker shipped with its leading
chevron clipped — which reads as a rendering fault, not as a layout that ran
out of room. The fix at the time was to pin those two fields to full width
forever: correct at 80 columns and wasteful at 200.

The decision now belongs to the **width**. Each field says what it costs — the
wider of its legend and the longest value it can display, taken from the same
option lists the cycler steps through, so a new option cannot outgrow its box
without saying so — and two fields share a row only when both halves can carry
them. On a wide pane the palette pickers pair; on a narrow one they stack. At
no width is anything clipped.

Two real defects fell out of writing the contract down. `Min secs` sat at a
hard half-width and clipped its own legend below about 70 columns. And
`Patterns (one per line)` needs 29 columns with its border — wider than the
card ever gets on a narrow pane, the one legend no layout could save; it is
now **`Watch patterns`**, with the "one per line" carried by the shape of the
field, which is a text area several rows tall.

The new test sweeps every width from 40 to 240 and asserts no field is ever
narrower than what it draws. Sweeping is the point: the original bug was
invisible at the width it was written at, so a single-width test is exactly
the one that would have passed.

## 0.18.45

**Never colour alone.** WCAG 1.4.1 is one line long and easy to fail without
noticing: colour must never be the *only* thing carrying a piece of
information. About one man in twelve cannot separate red from green, every
colour cue vanishes on a monochrome CRT theme, and none of them survive a
screenshot pasted into a ticket in greyscale.

Crew mostly passed already — by accident of taste rather than by rule.
Attention markers are distinct glyphs (`!`, `⚑`, `✓`, `⊗`, `?`) that happen to
share the bell colour; broadcast is `»` and not just magenta; every toast names
itself in its legend; a busy sidebar row spins. Two places did not:

- the **load gauges**, where nominal / warning / critical was the fill colour
  and nothing else — the percentage says the number, but not which band it is
  in, which is the tier's whole job. They now mark it: `!` past 70%, `‼` past
  90%, riding the label's trailing space so the bar and the reading land on
  exactly the same columns either way.
- a **working pane in the minimized strip**, which drew the same solid `●` as
  a pane that had merely spoken recently, told apart by a brightness pulse —
  colour and motion, the two channels this is about. It now draws a
  half-filled `◐`: visibly *partial*, which is what in-progress looks like.

`/shapes [auto|off|on]`, and **Settings → APPEARANCE → Shape cues**. `auto`
follows macOS's *Accessibility → Display → Differentiate without color*, and
is off unless asked — a glyph in every gauge row is noise for a reader who can
see the colour. The rule is *never colour alone* for anyone who needs it, not
*always both* for everyone.

The gauge's colour thresholds and its marks now come from one definition, so
the two can never disagree about which band a reading is in.

## 0.18.44

**Contrast follows the operating system too.** macOS has an Accessibility →
Display → *Increase contrast* switch, and crew ignoring it was awkward
precisely because crew is otherwise careful about contrast: it already derives
its readable roles — the terminal cursor, links, selection, the warning amber,
the sparkline — by walking each colour in Oklch until it clears a measured
WCAG floor against the page it lands on. It then held those floors fixed no
matter what the user had asked the system for.

Now the switch moves the floors, and everything derived through them follows:
WCAG **AA** (4.5 text / 3.0 marks) normally, **AAA** (7.0 / 4.5) when the OS
asks. AAA rather than some number in between because it is the standard's own
next band, and because every role here is small text or a cursor, which is
what AAA was written for.

Two things that are not derived move too, for the same reason — they *spend*
contrast. The **spotlight** over unfocused panes dims text, which is the
opposite of the request; and the **gradient wash** lifts the very background
the ink sits on, with only 4–16% headroom over it. Both drop to a third of
their strength — quieted, not killed, since the spotlight is the cue that says
which pane has focus and losing it is its own accessibility loss.

New: **`/contrast [auto|normal|high]`** with a value picker, and **Settings →
APPEARANCE → Contrast**. An explicit `normal` or `high` overrules the OS, the
same way an explicit motion level does.

## 0.18.43

**The grid moves on springs.** Panes have glided to their tiles since the
reflow animation landed, but the mechanism was exponential smoothing: given
where a pane is and where it should be, cover some fraction of the difference,
every frame. That is fine for anything that starts at rest and arrives once,
and wrong for anything that can be redirected while it is still moving — which
is the whole life of a pane grid. Close a pane while the last close is still
reflowing and a smoothed rect *kinks*: it forgets it was travelling and starts
a fresh decay from wherever it happened to be.

Each pane's four edges are now critically damped springs, integrating from
position **and velocity**. Retarget one mid-flight and the motion curves
through, because it remembers it was moving — closing two panes in quick
succession now reads as one continuous rearrangement instead of two unrelated
animations. It is not a nicer curve; it is the only curve that survives
interruption, which is why every modern motion system settled on the same
primitive.

Critically damped, deliberately: no bounce. A pane rebounding off its own tile
would be reading as playful about a layout change you asked for. What the
spring buys here is the interruption behaviour and the weight of the arrival.

Tuned to land in about the quarter-second the smoothing took, so the reflow
keeps its pace and only its character changes. Motion off is still a genuine
off, an idle crew still repaints nothing, and long frames after an idle
stretch are integrated in substeps rather than trusted whole — a big `dt`
through a naive integrator sends a pane to infinity.

## 0.18.42

**Focus mode: crew stops interrupting.** `/focus`, or `Ctrl+Shift+F`.

Crew is built to get your attention — a quiet pane toasts, a pane blocked on a
prompt raises a marker and can pull focus to itself, a bell rings. That is
right almost all the time and exactly wrong for the twenty minutes you are
trying to finish a thought in one pane.

While focus mode is on: **nothing pops** (notifications still write the LOG,
still flash the bar, still raise the pane's own marker — they just do not step
onto the canvas), **nothing steals** (a waiting pane is still badged, but
focus never moves on its own), and **the rest of the canvas recedes** — the
spotlight over unfocused panes deepens from 15% to 42%, far enough to be
unmistakable and short of hiding the peripheral awareness a grid is for.

The difference from `/notify off` is that nothing is thrown away. Leaving says
what happened in one line — `3 notifications held while focused` — so the mode
costs you awareness only until you come out of it.

The input bar's legend reads `◉ focus` throughout. A mode owes you a standing
sign that it is on, or you spend the afternoon wondering why nothing pops.

## 0.18.41

**Hold a modifier and crew tells you what it does.** Rest on `Cmd` (or `Ctrl`)
for a moment and a single row of chips appears above the input bar naming what
that key reaches *from where you are* — `1…9 pane · ←↑→↓ focus · I input · T
shell · J chat`, and then `K clear · Z zoom · W close` in a pane, or `↵ send`
on the input bar. Press anything, or let go, and it is gone.

`/keys` is a manual: fifty bindings in one scrolling column, correct and
unreadable in the two seconds anyone actually has. It answers "what can crew
do", which is a question people ask once. The question they ask constantly is
narrower — *my thumb is on Cmd, what are my options right now?* — and the
answer depends on what is focused, which a static table cannot know.

Two rules keep it out of the way. It only opens on a modifier held **alone**:
someone mid-chord already knows what they are doing, and a panel that opens
during `Cmd+Shift+G` is in the way of the very thing it claims to teach. And
it waits out 450ms first, so an ordinary `Cmd+C` never flashes a panel on its
way past. `Shift` never opens it — it reaches nothing on its own and is held
through every capital letter you type.

Idle crew still repaints nothing: the opening edge asks for exactly one frame,
the closing edge for one more, and a thumb resting on a modifier that is never
held long enough asks for none at all.

## 0.18.40

**Toasts answer back.** For most of their life crew's notification cards were
something you could only watch: a 4.8-second card saying "agent-7 is waiting",
naming a pane you then had to go and find yourself.

**Rest the pointer on the stack and it holds** — every card in it, until the
pointer leaves. WCAG's *Pause, Stop, Hide* asks that auto-hiding content be
pausable, and a message on a four-second timer is unreadable to anyone who
reads slowly, is interrupted, or looks up a moment late. The whole stack holds
rather than just the card under the cursor: expiring its neighbours would
slide the stack up and move the card out from under the pointer, which is the
one thing a pause must not do. A held stack is frozen, so it asks for no
frames — an idle crew still never repaints.

**Click a card to go where it points.** A toast raised by a pane now knows
which pane, and clicking it focuses that pane — restoring it from the nav if
it had been minimized, through the same path every other focus goes. A card
that names no pane is simply dismissed. Either way the card leaves: it has
been answered, and staying on screen would say otherwise.

The card admits all this on hover — the stroke lights in the accent and the
legend reads `waiting → open`, or `note ✕` for one with nowhere to go. A click
target with no affordance is a secret.

The pane is remembered by NAME and resolved at click time, not captured as an
index: panes open and close while a card is on screen, and a four-second-old
index can easily point at a different pane, or past the end of the list. An
`exited` toast deliberately offers nothing — that pane is reaped in the same
tick.

## 0.18.39

**The canvas has a density.** `/density [compact|cozy|roomy]`, and
**Settings → APPEARANCE → Density**, decide how tightly crew packs: the gutter
between pane cards and the blank rows between chat cards move together. In a
cell grid those are the two spaces that are genuinely empty — the line height
*is* the cell, and shrinking that is the font size, which has its own knob.

`cozy` is exactly the layout crew already drew, so the setting arriving
changes nothing until you turn it. `compact` halves the gutter and drops the
chat spacer entirely (the header's coloured gutter glyph still draws the card
boundary, in ink rather than in space); `roomy` opens both up for a large
display. Compact never closes the gutter to zero — two rounded strokes that
touch read as one wide card with a line down it.

The gutter is now read from one function that render **and** hit-testing both
call, so a density can never put a click target beside the thing it draws.

## 0.18.38

**Motion follows the operating system.** macOS has a single system-wide
Accessibility → Display → *Reduce motion* switch, and crew now honors it:
`motion` gains a fourth value, `auto`, which is the new default. With the
switch off `auto` is full motion — exactly what crew did before — and with it
on, crew's `off` is a genuine off: every animation window collapses to zero,
the final state draws once, and nothing reschedules a frame.

That switch exists because vestibular disorders make sliding and zoom actively
unpleasant rather than decorative. A user who has already told their Mac they
want less of it should not have to find crew's own setting to say it again.

An explicit level still overrules the OS in **both** directions: `/motion full`
keeps crew moving under Reduce Motion, and `/motion off` stays off without it.
The Settings picker reads `auto (off)` / `auto (full)` rather than a bare
`auto`, because a deferral still owes you an answer about what it decided.

New: **`/motion [auto|off|subtle|full]`** with a value picker, so the knob is
reachable without opening Settings.

## 0.18.37

**Where you are in a buffer is a colour now.** Scroll a pane back and the
`⇡N` on its top border and the thumb down its right border both take the theme
gradient, sampled at your position: deep in the history they wear one pole, at
the live edge the other, and dragging the gutter walks them between. That is a
third reading of the same fact — the number says how far, the thumb says how
far *of how much*, and the colour says it without being read at all.

Both wear it from one function, so the border can never tell two stories about
one number. And it is the same gradient the card's own stroke runs, so the
thumb reads as part of the frame it rides rather than a widget parked on it. A
theme with no gradient keeps the flat `status_fg` both had before.

**`Ctrl+Shift+G` steps the canvas gradient** — the colour's answer to
`Ctrl+Shift+L`. It walks the eight named pairs and passes back through the
theme's own gradient once a lap, so the key that got you somewhere can always
get you home. A gradient of your own (a hex pair) joins the walk at the start
rather than being stranded off the shelf. It is in `/keys` with everything
else.

## 0.18.36

**Eight gradients you can pick by name.** `/gradient` took two hex codes,
which is a fine thing to have and a poor thing to start from. It now takes a
name: **`aurora`** (teal into violet), **`tide`** (cyan into deep blue),
**`orchid`** (violet into rose — crew's own aurora look), **`moss`** (green
into teal), **`ember`** (amber into red), **`sand`** (sand into clay),
**`dusk`** (indigo into magenta), and **`mono`** — no colour at all, the wash
in the page's own grey.

Selecting `/gradient` in the command palette lists all eight with the ladder,
so choosing the canvas's colour is arrowing through a list rather than
inventing a hex code. Anything off the shelf still works.

They are chosen for their **interval**, not their brightness — every pair is
re-lit to the active theme's own pole lightness at draw time, so a preset is
really a pair of hues, and the same eight land correctly on a near-black page
and a paper-white one without either being tuned for. A test puts every preset
on every palette and holds each pole above the WCAG 3.0 non-text floor, and a
second one checks the hue actually survives the re-lighting: `ember` must not
quietly become the theme's violet.

A pair chosen by name is stored **by name** (`gradient_poles = "ember"`), so a
preset re-tuned in a later release reaches everyone who picked it — and a
config file says what you chose rather than what it resolved to.

## 0.18.35

**`/gradient` — the canvas's colour is yours now.** Every gradient surface in
crew (the page's wash, the dot lattice, every card's stroke, the footer's
meters) runs between two poles, and until now those were the theme's and only
the theme's.

    /gradient                     what it is now
    /gradient off|subtle|lively   how far the colour breathes
    /gradient #7aa2f7 #bb9af7     a gradient of your own
    /gradient reset               back to the theme's poles

The ladder arm writes the same `gradient` key as **Settings → APPEARANCE →
Gradient colour**, so the command and the form can never disagree. Selecting
`/gradient` in the palette opens a value picker.

**Only the hue is yours.** A custom pair is re-lit to the active theme's own
pole lightness — and re-lit at *draw* time, not when you set it, so it keeps
tracking the ten-minute palette rotation and looks right on every page it
lands on. The wash lies under your text with 4-16% contrast headroom over the
page it lifts (v0.18.26): that is not headroom a colour picker gets to spend,
and `#ffffff` would erase the page. You choose the colour, crew chooses how
bright it is. Held by a test that sweeps every hue on every palette and keeps
each one above the WCAG 3.0 non-text floor.

A pair that cannot be read is no pair — the theme's own gradient stays, rather
than half of someone's.

## 0.18.34

**The page's light follows the pane you are working in.** The background wash
is two broad pools of the theme's pole light on an orbit under everything, and
that orbit was centred on the page — the same light no matter which pane had
focus. Now its centre slides toward the focused card: the page is brightest
under the thing you are typing into, and falls away from the ones you are not.
Focus the input bar and the light comes down to meet it.

It is wayfinding, not decoration. On a four-pane grid the focused frame is one
stroke among four; the wash under it is half the window, which is why this
reads from the corner of the eye when a border colour does not.

The move is partial by design (55% of the way to the card's centre). The pools
are wider than a pane, so parking them dead on it would put the falloff
*inside* the card and the light would stop reading as a page-wide field
altogether.

It travels rather than cutting: the same exponential smoothing the grid uses
to glide panes to their tiles, a little slower, so a card arrives and the
light fills in behind it. Bounded, like everything else that moves — it
settles, stops asking for frames, and an idle crew repaints nothing. At
**Motion = off** it snaps to the final state in one frame. And with nothing
focused at all the gather fades out where it stands rather than dragging a
bright field back across every pane on its way to standing still.

A settled focus is a constant, so a still frame is still a pure function of
pixel position — and a crew that has never focused anything renders the
centred wash byte for byte, which is what every headless shot test sees.

## 0.18.33

**The gradient's colour breathes now.** Every gradient surface in crew — the
page's wash, the dot lattice woven over it, every card's stroke, the footer's
meters — is drawn between the active theme's two poles, and those poles were
constants: the whole canvas could only change colour by changing theme. Now
they lean around the hue wheel over time, so the page warms and cools on its
own.

**Settings → APPEARANCE → Gradient colour** sets how far. `subtle` (the
default) leans ±16° — one colour's neighbourhood, so a violet theme visits
indigo and magenta and is never anything else. `lively` leans ±38°, far enough
that the two ends of the breath read as different lights on the same room.
`off` pins the poles to the theme's own colours, which is exactly the look
crew had before this.

It is a *breath*, not a rotation: the offset is a sine of the clock, so the
colour leans one way, comes back through the palette's exact colour, and leans
the other. A monotonic turn would eventually walk every theme through every
hue and stop being that theme.

Nothing about it can move a contrast guarantee. The turn happens in OKLCH and
moves **hue only** — lightness and chroma come back out of the conversion and
go straight back in, and a hue sRGB cannot show at that chroma loses chroma
rather than clipping a channel. Measured across all eight palettes: a pole's
contrast against its own page moves by under 8% at the widest rung, and no
offset a hand-edited config can reach takes one below the WCAG 3.0 non-text
floor (the measured minimum is 3.94).

It also costs nothing. The breath rides the same accumulator the wash's orbit
already runs on — four times slower, so a colour and the position it is drawn
at never repeat together — which means it asks for no frame of its own, holds
when the wash holds, stops dead at **Motion = off**, and is exactly zero on a
process that has never drifted. Every headless shot test still sees the
theme's own bytes.

## 0.18.32

**The background gradient drifts on its own now.** The page's wash — two broad
pools of the theme's pole light on an elliptical orbit under everything — has
always moved, but only while a pane was busy, so a quiet window sat perfectly
still and most people never saw it turn at all.

With **Settings → APPEARANCE → Drifting background** on (the default) it keeps
going when nothing is happening: one revolution every ninety seconds, fifteen
times slower than the busy drift, because idle motion is a texture and busy
motion is a signal — the two should not look alike.

It is the only animation in crew that repaints a window nothing else needed
repainted, which is a real cost on a laptop, so it is fenced on four things and
any one of them stops it: the setting, **Motion** not being `off`, a theme that
has a wash at all, and **crew holding the OS focus** — a window you are not
looking at repaints for nobody. It draws at about six frames a second rather
than the fifteen the busy path uses: at ninety seconds a revolution each frame
moves the pools about two-thirds of a degree, well under what an edge that soft
can show, so the extra frames would buy no smoothness and two and a half times
the wake-ups.

Turn it off and an idle crew goes back to drawing exactly nothing, holding its
last frame wherever the pools had reached — the static-frame determinism the CRT
trace and the gradient ring keep. The phase is still accumulated from frame
deltas rather than read off the wall clock, so the motion stays continuous
across every pause instead of teleporting after a quiet minute.

## 0.18.31

**Six colours that were picked on a dark page and never met a light one.** The
ramp and the signal roles are derived and measured; these six were not. They
were constants sitting in whichever crate happened to draw them, every one
chosen by eye on a dark theme. Measured against the page each actually lands on:

| role | dark pages | light pages |
|---|---|---|
| terminal cursor, focused | 11.2–12.3 | **1.41–1.61** |
| terminal cursor, unfocused | 2.7–3.0 | 5.7–6.6 |
| URL in a terminal | 7.7–8.4 | **2.05–2.35** |
| selection, against the text on it | 5.6–7.1 | **2.32–2.52** |
| load-average warning amber | 9.8–10.8 | **1.60–1.83** |
| load-average danger red | 5.3–5.9 | **2.95–3.39** |
| network sparkline | 10.2–11.3 | **1.53–1.76** |

Three of those are worse than faint. The cursor was **inverted**: on a light
page the pane you were typing in had the faintest cursor on the canvas, four
times fainter than every pane you were not typing in. A warning colour at 1.6 is
a warning nobody receives. A URL at 2.2 is a link you cannot read on the third
of the themes that ship light.

Each role now keeps its **hue** — a link is blue, a warning amber, an alarm red,
and those meanings do not belong to the palette — and gives up its
**lightness**, walking away from the page in Oklch until it clears WCAG's floor:
4.5 for anything read, 3.0 for a mark only seen. A colour that already clears
comes back untouched, so the dark pages are pixel-identical. The light pages go
from 1.4–2.4 to 4.5–4.9, and the focused cursor from 1.5 to 16.2. One contract
test measures all nine palettes, so this class of drift cannot return quietly.

**The `/keys` list stops having a height budget.** It did not scroll, so its
height was a hard budget: a list one row too tall was cut off in silence, and a
test failed the build whenever a binding was added. Three times in this release
alone, making room meant merging two unrelated rows and losing detail from both.
Arrows and page keys walk it now, Home/End jump its ends, and any other key
still dismisses it. The merged rows are un-merged, and this release's mouse
gestures got the rows they should have had.

## 0.18.30

Three surfaces showed you part of something and gave you no way to reach the
rest.

**The sidebar LOG scrolls.** It is five rows onto a hundred buffered entries;
the other ninety-five were reachable only through `/log`, which opens a whole
pane for something the sidebar was already showing part of. A wheel over the LOG
now moves that window, and its rule carries `⇡N` while you are back from the
live tail — a log that silently stops following looks like a log that stopped. A
line arriving mid-scroll steps the window with the buffer rather than sliding
out from under you; scroll back down to follow again.

**The scroll thumb is a gutter you can drag.** 0.18.28 gave a scrolled-back pane
a proportional thumb, and the next thing anyone tries after something says where
they are is moving it. Pressing the right border jumps there at once — a
scrollbar that only moves on the second event feels broken — and dragging
crosses ten thousand lines in one gesture, which neither the wheel nor a page key
can. It is live only while the thumb is drawn: at the live bottom there is
nothing behind it to reach. The pointer takes a row-resize arrow over it.

**Cmd+wheel resizes the font** (Ctrl+wheel off macOS). `Cmd+=` and `Cmd+-`
already did it a step at a time; the wheel is how people actually reach for it.

## 0.18.29

**The cursor says what the thing under it does.** Crew never set a cursor icon,
so the arrow looked identical over a shell's output, over the `[x]` that kills
it, and over a card that can be picked up and carried. Every one of those is a
different verb, and the pointer is where an interface says so before anything is
clicked.

An I-beam over a pane's content and the input bar; a hand over the border
buttons, the nav rows and the `+N` tile; an open hand over a card's legend row —
the handle it is carried by — closing while one is in hand; a column-resize
arrow on the sidebar's edge.

**Drag the sidebar's edge to resize it.** The nav's width was a figure in the
Settings form and nowhere else: the one dimension of the layout people actually
want to nudge, reachable only by opening a pane, typing a number and saving it.
Its edge is a visible boundary sitting right there; now it is a handle too,
clamped to the same 160–320 px the form clamps to, from one shared constant so
the two paths cannot disagree about what a legal width is. The nav is chrome and
not a pane, so the grid never changes shape — it is handed a narrower content
rect exactly as it is on a window resize.

**The `+N` overflow tile answers a click.** It was the one drawn tile with no
hit rect at all: it stood on the canvas announcing three hidden panes and
ignored every click on it. Clicking now reveals the first pane it stands for,
and clicking again walks to the next.

## 0.18.28

**Double-click selects a word.** Crew had no word selection at all. Double-click
toggled zoom instead — a gesture no terminal spends there, and the one people
reach for constantly: grab a filename, a hash, a flag, a path.

The click run now goes single (arms a drag) → word → line, capped so a fourth
click starts over rather than latching the widest gesture, and each selection
copies, the same rule releasing a drag already followed. Terminals answer
through alacritty's own semantic and line selections, so a path stays one word
and a soft-wrapped command comes back whole; every other pane kind answers the
same way over its rendered cells, so the gesture means the same thing on an
agent transcript as on a shell.

Zoom keeps the double click, on the card's **border** — where the convention
puts it anyway: a window's title bar, not its contents. Cmd+Z is unchanged.

**Drag a card onto another to swap them.** The canvas is a set of cards on a
surface and behaves like one everywhere else, so the obvious thing to try was to
drag one, and the obvious thing did nothing. Pick a card up by its top border
and the card under the pointer lights in the accent; release and the two swap
places. The legend row is the grab region because it is the one row of a card
that holds nothing to select, so a card drag and a text drag can never both be
armed.

**The scrollback gets a shape, not just a number.** `⇡200` says how far from the
bottom you are. It never said how far back there *is* — 200 lines up reads
identically in a screenful of history and in a week of it — and nothing said
where between the two you were. A proportional thumb now runs down the right
border while a pane is scrolled back: high in the buffer, low near the live
edge, short when there is a lot behind you. It rides the border rather than a
content column, because a terminal's columns belong to the program running in
it and an 80-column layout must stay 80 columns whether or not anyone scrolled.

**Fixed: the Windows build.** It has failed on every release since 0.18.18 —
`daemon::service::LABEL` has no reader on a target with no service integration,
and CI builds with `-D warnings`.

## 0.18.27

**Cmd+Arrow walks focus the way the eye does.** Pane cycling steps through panes
in *index* order, which a tiled grid does not follow: with four panes open, pane
2 sits below pane 1, not beside it. So the one gesture a tiled canvas invites —
"focus the card over there" — had no key at all.

Cmd+←↑→↓ now focuses the neighbour in that direction, chosen from the same rects
the mouse hit-tests against, so keyboard focus and the pointer can never
disagree about where a tile is. Hold Shift and the focused pane travels with the
gesture, swapping places with whichever card is that way. Neither wraps at the
edge of the grid: a wrap in a spatial gesture reads as the whole canvas jumping.
Zoomed, where there is no geometry to navigate, it falls back to stepping.

**The chrome answers the pointer.** Every click target on the canvas chrome was
silent. The `[-]` and `[x]` on a card's border were three glyphs the colour of
the legend beside them, with nothing to say the pointer had found one or which
of the two it was on; a sidebar PANES row focuses and restores its pane on a
click, and read exactly like the dead text above it.

Now the button under the cursor lights — `[-]` in the accent, `[x]` in the bell
colour, because the one control that ends a running program should say so before
it is clicked and not after. A hovered nav row lifts its ink to full contrast
rather than washing a background behind it: 0.18.26 measured the page's
remaining contrast headroom at 4–16%, so hover buys its emphasis with ink and
not with the page. Both repaint only when the target actually changes, so
sweeping the pointer across the canvas costs one frame per thing it crosses.

## 0.18.26

**The gradient reaches every card, not just the focused pane.** The sidebar, the
welcome and update cards, the command menu, the paste prompt, the composer, the
input bar, the toasts, the minimised thumbnails and every unfocused pane now
carry the theme's gradient on their border. Until now the canvas was one
coloured frame surrounded by eight flat grey ones.

The colour is free because it is paid for in hue rather than in light. A quiet
frame's gradient is re-lit to the brightness of the border colour it replaces
before it is mixed in, so it sits within a hair of where it always sat: the hue
travels around the card, the level does not move. Focus still reads first —
measured as contrast against the page rather than as brightness, because on a
light theme prominence is darker ink, not more light.

Where scaling a pole cannot reach that brightness the shortfall goes toward
white instead of clipping a channel, since a clipped channel shifts the hue and
whitening only spends saturation. Light themes live entirely in that case, which
is why the gradient reads as a pastel there and at full strength on a dark page.

Two things keep their flat colour, for the same reason: a stroke carrying a
signal is not chrome. An alert toast keeps its bell border — a warning must not
wear the command menu's skin — and the status glyphs, focus brackets and pane
legends riding the border rows keep their own colours.

The footer meters become gauges lit by the same ramp. Each eight-cell bar runs
from one pole to the other, with the unfilled tail at the same hue pulled back
toward the page, so how full a gauge is still reads as brightness and not only
as glyph density.

And the page itself could not go up. The contrast suite has always measured
every text role against the page *as declared*, which is not the page anyone
reads on — the wash lies underneath the whole canvas and mixes it toward a pole.
Measured properly for the first time, the shipped weights clear their tightest
floor by four to sixteen percent, and half again as much wash puts six of the
nine themes under it. The aurora was already calibrated to the edge of
legibility, so the new colour had to come from the chrome. A guard now pins
that, so the next attempt to turn the page up fails loudly instead of quietly
eating the ink.

## 0.18.25

**Every theme has the gradient now, in its own colours.** The ring, the dot
lattice and the wash belonged to two themes. All nine carry them — and the poles
come from each palette's own slots rather than nebula's orchid and rose copied
around. Paper-dark runs its blue into its cyan, sepia its amber into its coral,
sepia-light ochre into terracotta.

The tubes stay tubes. Their poles are two points on their *own* phosphor ramp,
so the gradient is luminance rather than hue — a single-phosphor screen that
suddenly ran purple would not be a CRT any more. Their lattice and wash run at
half strength, because the bloom and scanlines are already doing that work.

Paper and sepia keep their newsprint grain alongside the new lattice. That rule
used to say "a theme with a gradient carries no grain", which was really a
description of the two glass themes; now the exception is named rather than
inferred, and paper that lost its tooth would just be a flat page.

Under it, one rule turned out to have been copied into four places. The test
for "is this a phosphor tube" read *has a CRT style and no gradient* — true only
while gradients were a two-theme family — and three modules kept private copies
of it, even though its own comment said the distinction lived in one place. All
four now ask the question that actually separates them: whether the scanlines
are turned up. Left alone, crt-green would have quietly stopped counting as a
tube, losing its phosphor colour ladder and collapsing two project tag colours
into one.

## 0.18.24

**You can approve from your phone.** The gate could already decide and refuse;
what it could not do was *ask*, because the question had no way out of the
broker process. Now it travels in both directions — the question goes out as an
event, the answer comes back as a command, and the tool call blocks in between.

Blocking is the honest shape. The agent asked to do something that cannot be
undone; until a person says yes, nothing should happen. The wait is bounded, and
a lapse is a refusal — an unanswered question is never a quiet yes.

A conversation waiting on an approval reads the next thing you say as the
**answer**, not as new work. The agent is stopped mid-tool-call, and starting a
second task on top of it would leave the first hanging forever.

Three refusals worth knowing about. "maybe" or "later" is neither yes nor no, so
crew asks again and keeps waiting rather than guessing — the whole point of
asking is that you meant one of the two. An approval belongs to the conversation
it came from, so a "yes" from another chat cannot approve someone else's
command. And once you have answered, the conversation goes back to taking tasks.

So: text your bot a task, and if it needs to do something irreversible you get
asked first, wherever you are.

## 0.18.23

**The test guard stopped leaking a theme.** Tests that touch the process-global
theme took a shared lock so they would not race, but the lock left whichever
theme the last one published in place for everybody after it. Any test
comparing against a *derived* colour — `chatink` floors every ink for contrast
against the card background — then passed or failed depending on what ran
before it. That is what turned three markdown colour tests red overnight with
no code change at all, and why they reproduced even single-threaded.

The guard now records the active theme, pins a known one, and puts the recorded
one back on drop. Nothing user-facing changes; the point is that "the suite is
green" starts meaning something again.

## 0.18.22

**A security fix for something 0.18.21 introduced an hour earlier.** The action
gate runs inside the broker process, but *who asked* is known only to whoever
started it — and nothing carried that across the process boundary. Every broker
reported itself as a person at the keyboard, including the ones the daemon
starts for a channel conversation. So when tasks from a channel first became
routable in 0.18.21, the gate saw a local pane and allowed irreversible calls
that should have needed approval. The refusal existed only in tests.

`CREW_REQUESTER` now carries it — `pane`, `channel:<address>` or
`trigger:<name>` — set by the daemon when it opens a session for an address and
read by the broker at startup. Absent means a pane, so every broker a GUI pane
spawns behaves exactly as it always has.

Anything unrecognised is treated as a *trigger*, the most restricted kind, not
as a pane. A typo in that variable must not become a way to be trusted, so a
malformed value fails closed and is refused outright rather than quietly
granting keyboard-level trust.

## 0.18.21

**A message from a channel becomes an agent task.** Until now crew answered
three questions from a phone and apologised for everything else. Now anything
that is not `help`, `status` or `sessions` is handed to an agent session, and
the reply comes back to whoever asked.

One session per channel address, kept for the life of the daemon — a
conversation from a phone should remember the last thing you said, and opening
a fresh broker per message would throw that context away. Two different senders
never share one.

Only finished replies are forwarded. The broker also streams activity, token
stats, mid-reply deltas and task lifecycle; sending those to a phone would turn
a conversation into a debug log. A reply is delivered exactly once — a phone
buzzing twice for one answer is the most visible bug this could have — and if
the session has died, you are told so and the next message starts a fresh one
rather than writing into a pipe that will never answer.

Every tool that session calls still passes the action gate, so a task sent from
a channel cannot run an irreversible command without approval.

**Also fixed:** three markdown colour tests compared rendered cells against raw
theme slots rather than the ink the renderer actually uses — `chatink` pushes
every colour through a contrast floor first. The derived table is computed once
on first use, so which theme was live at that moment decided whether the suite
passed, which is how they went from green to red overnight with no code change.

## 0.18.20

**Messages arrive now, and the resident answers them.** Telegram shipped last
release with no way to receive: the Bot API holds a request open for 25 seconds,
so polling could never sit on the daemon's serve loop. The blocking half now
lives inside the channel — a poll thread it starts the first time the daemon
actually looks for messages, so a crew with a token but no running daemon still
talks to nobody.

The serve loop waits with a timeout instead of blocking outright, so between
requests the resident does its own work: drain the channels, answer whoever
wrote in. A daemon that only wakes when it is asked something is not a resident,
it is a server.

The vocabulary is three words on purpose — `help`, `status`, `sessions`. A
channel that answers three honest questions is worth more than one pretending to
be an agent it cannot reach yet. But anything it does not recognise still gets a
reply saying what *is* possible: silence from a remote channel is
indistinguishable from a crew that is down, which is the one thing it must never
look like.

The first time you message your bot, crew turns you away and prints your chat id
so you know what to put in `CREW_TELEGRAM_CHATS`. That notice is also how you
notice a stranger knocking.

## 0.18.19

**Telegram — the first way into crew that is not this machine's keyboard.**
Chosen over iMessage and WhatsApp for one reason: the Bot API is a documented,
stable surface with a token you create yourself. No scraping a private database,
no unofficial bridge that gets an account banned.

It ships switched off. With no `CREW_TELEGRAM_TOKEN` the channel is registered,
reports itself as not configured, and never opens a socket — so nothing changes
until you decide otherwise. `crew daemon channels` shows it either way.

**To turn it on:** message `@BotFather`, send `/newbot`, copy the token into
`CREW_TELEGRAM_TOKEN`. Then message your bot once and put your chat id in
`CREW_TELEGRAM_CHATS` — crew prints the id of any chat it turns away, so the
first rejected message tells you exactly what to paste.

Two rules carry the safety. The allowlist is empty by default and **empty means
nobody**, not everybody: an assistant with a public address is an assistant
anyone can drive, and "open until configured" is a window that stands open for
exactly as long as it takes you to notice. Crew also will not message a chat it
would refuse to hear from — a reply is itself an outbound message to a stranger.

And the read offset advances past every message seen, including refused ones.
Advancing only past accepted messages would let one stranger's message pin the
offset, so crew re-reads it — and everything behind it — on every poll forever.

## 0.18.18

**One shape for every way in.** A pane, a phone and a microphone are the same
kind of thing — somewhere a request arrives from and a reply leaves to. Writing
that once as a trait is what stops voice from being a special subsystem later:
it becomes the third implementation of an interface Telegram already forces into
existence.

An address is `kind:rest`, reusing the shape the sentinel work settled on, and
opaque to everything except the channel that owns the kind.

Three refusals carry the design. An unroutable address is an error rather than a
silent drop — a reply nobody receives looks exactly like a reply that was never
written. Two channels cannot own the same kind, or every reply becomes a coin
flip about which one delivers it. And a bare word with no kind is not an
address at all: guessing a default would send someone's reply to a stranger.

A channel that exists but has no credential is registered and *not ready*, and
sending through it fails loudly instead of appearing to work. That is the state
every real channel starts in before its token is configured — which is exactly
how the first one will arrive.

`crew daemon channels` lists the ways in. Right now it says "no channels — crew
is reachable from a pane only", which is the honest answer.

## 0.18.17

**Every tool call now passes the gate — and nothing about crew changes.** That
second half is the point. There is exactly one place every tool call in the
running broker flows through, `sys` and MCP alike, so the gate goes there rather
than into each tool. With today's only requester — a person typing into a pane
on their own machine — the gate always allows, so your crew behaves exactly as
it did yesterday.

The gate belongs in the path *before* something can put a non-human behind it,
not after. The other side of it already works and is tested: a request arriving
from a channel cannot run a shell command without approval, a trigger cannot run
one at all, and a tool from an MCP server nobody has classified is refused the
same way.

An approval that cannot be asked is refused rather than silently awaited —
nothing can carry the question yet, and saying no out loud beats hanging.

Actions that changed something are written to the ledger; reads are not. Burying
the handful of calls that touched the world under thousands of file listings
makes the ledger unreadable, which is the same as not having one.

## 0.18.16

**An append-only record of what crew did, and a `crew ledger` to read it.** An
assistant trusted with mail, money and someone's front door has to be auditable
after the fact — and the record that matters is always the one written just
before something went wrong. So the ledger is deliberately dull: one JSON object
per line, opened for append, flushed on every write, never rewritten, never
truncated.

It is not `activity.log`. That file is truncated on every process start and
skipped under test, which is right for a session log and disqualifying for an
audit trail.

It is built around two failure modes. A crash mid-append leaves half a line —
that costs one record, not the whole history, because unreadable lines are
counted and stepped over rather than aborting the read (the naive version
returns an error and the entire audit trail reads as empty). And the file is
reopened for each append rather than held open, so two writers — the daemon and
a broker child — interleave whole lines instead of overwriting each other at a
stale offset.

`crew ledger` prints it back, newest last, with `--limit`. A trail nobody can
read is not an audit trail.

## 0.18.15

**The gate: who may fire something that cannot be undone.** The last release
classified tools by reversibility. This one decides whether a call proceeds —
and the two questions come apart on one axis, which is *who asked*.

The rule is deliberately not "irreversible means ask". A person typing into a
pane on their own machine is already the approval: they can see the output and
stop it, and confirming their own keystroke is theatre. Today's behaviour is
unchanged, and that trust is a setting rather than a law, so it can be turned
off for a stricter setup.

The real rule is: irreversible **and no present human** means ask, and if
nobody answers, **deny**. A request arriving from a channel opens an approval
addressed back to the channel it came from. A trigger firing at 3am has nobody
to ask at all, so it is refused outright rather than queued — the alternative
is opening a question into an empty room and reading the silence as a yes.

Approvals are single-use, so a replayed grant finds nothing to re-fire, and ids
never repeat. An unanswered one lapses at its deadline and is recorded as
*timed out* rather than *denied*, because "they said no" and "nobody was there"
are different facts when you read them back later.

Nothing calls this yet: the gate decides, and recording then enforcement follow
in that order. No tool behaves differently in this release.

## 0.18.14

**Every tool now says whether it can be undone.** Before crew can act on
everyday life — mail, calendars, money, someone's front door — something has to
answer "can this be taken back?" before the call fires. That answer cannot live
inside each tool, because the tools that will matter most are MCP servers
written by other people. So it lives in one classification the gate reads.

The tiers are about reversibility, not danger. **Read** observes and changes
nothing. **Reversible** changes something we can put back without asking anyone.
**Irreversible** left the machine, told a person, spent money, or destroyed
something — and only that tier will ever need approval.

Two defaults are deliberate. A tool from an MCP server nobody has classified is
treated as irreversible: unknown has to mean *ask*, because the cost of that
default is one prompt and the cost of the opposite is an assistant that mailed
something on your behalf before anyone got around to writing a rule. And a test
walks the built-in `sys` surface and fails on any tool with no tier, so a fifth
built-in cannot ship unclassified.

`sys:run` is irreversible, because a shell command is a blank cheque —
`rm -rf`, `git push`, `curl`. `sys:write_file` is only reversible: a file on
your own disk is recoverable in a way that telling another human something is
not.

`CREW_SYS_MODE=readonly` now derives its block list from that same table rather
than keeping its own — they were two lists of mutating tools, and one of them
was going to be updated without the other.

## 0.18.13

**The resident can come back at login — if you ask it to.** `crew daemon
install` writes a launchd user agent on macOS or a systemd user unit on Linux;
`crew daemon install --remove` takes it away. Both are per-user: no sudo, no
system-wide daemon, nothing written outside your home directory.

Nothing else in crew installs it. Not a release, not an update, not first run.
A background service you did not ask for is what turns a bad build into a login
loop instead of an `/update`, so the command is the whole consent — and a
source-tree test now fails if any other code path reaches the installer.

The agent runs an absolute path to the binary, which matters more than it
sounds: a launchd agent starts with a minimal environment and no login PATH, so
a bare `crew` works perfectly when you test it from a terminal and silently
never starts at boot.

`crew daemon status` now also says whether the login service is installed.
"Running right now" and "comes back at login" are different questions, and the
second one is what decides whether this is a resident or just a process you
started once.

## 0.18.12

**A daemon session you can actually talk to — and whose history survives you.**
`crew daemon send` writes a line to a session's agent; `crew daemon poll` reads
its output back from a cursor. A reader thread drains the agent's stdout into a
buffer the daemon holds, so a chatty broker never blocks the resident.

The cursor is the point. Because the daemon keeps the history rather than the
client, a reader that goes away and comes back polls from the cursor it last saw
and is handed exactly what it missed. Losing the window no longer loses the work
that happened while it was gone.

That buffer is capped at 2000 lines — the daemon outlives every client, so a
history unbounded in time has to be bounded in size. Lines that fall off the
front are counted and reported, so a client returning after a long absence is
told it missed some instead of being handed a gap with no marker and quietly
drawing a false history.

The GUI still spawns its own broker, unchanged. Pointing a pane at a
daemon-owned session is the next step.

## 0.18.11

**The daemon owns the agent processes now.** A `/crew` pane spawns its own
broker child and kills it when the pane goes away — which is precisely why
closing the window ends the work. The resident keeps a session registry
instead: `crew daemon open` starts a session the daemon owns, `crew daemon
sessions` lists them, `crew daemon close <id>` stops one.

Each session is a real broker child, started the way a pane starts one and
put in its own process group, so closing a session takes the agent CLIs it
spawned along with it rather than leaving them running and spending.

Three invariants the registry is built around: a session id is never reused
after a close (a stale client still holding `s1` would otherwise close
somebody else's session); a session whose process died on its own stays
listed as dead, because hiding it would read as "never opened"; and a spawn
that fails registers nothing rather than leaving a phantom.

The GUI does not use any of this yet — panes still spawn their own broker,
unchanged. Bridging a pane onto a daemon-owned session, so a running swarm
survives the window closing, is next.

## 0.18.10

**The first piece of a crew that outlives its window.** Today the agent brain
is a child process of the GUI — the `/crew` pane re-execs this binary with
`--broker-plugin` — so closing the window ends the assistant along with it.
`crew daemon` is the resident that replaces it.

`crew daemon run` binds its own local endpoint and serves a status op;
`crew daemon status` reports pid, uptime and live session count, exiting 3
when nothing is running. Neither touches the GUI: the daemon has no window
anywhere in its startup path and answers on a box with no display.

The transport is not new — `ipc.rs` already speaks JSON-line request/reply
over a Unix socket or a Windows named pipe. The daemon binds
`crew-daemon*.sock`, deliberately outside the `crew-ipc*.sock` shape instance
discovery parses, so the resident is never listed as an askable pane by
`crew panes` or reached by `crew ask --any`. Starting a second daemon now
probes for a live one first and refuses rather than reclaiming the socket out
from under it.

Sessions still belong to the pane, so the registry is honestly empty and
reports 0. Moving them into the daemon — so a swarm survives the window
closing — is the next step.

## 0.18.9

**The usage budgets are in Settings, and nothing lives in the config file
alone any more.** These are what the footer's 5h and 7d bars are drawn
against, and they were only ever reachable as raw token counts in
`config.toml` — 5000000 and 25000000. They get a **USAGE** card of their own,
which also balances the layout: the appearance column had grown much taller
than the rest.

They are typed in **millions**, the way you would say them — `5`, `25`, `7.5`
— rather than as eight digits. That is the same trade the opacity box already
makes by taking a percentage and storing a fraction, and the footer never
shows the raw figure anyway; it draws a percentage against it.

The catch, and the reason this is more than a text box: a budget hand-set to
something the display cannot show exactly, like 5,123,456, reads as `5.12`,
and every focus move commits the field — so simply *opening* Settings and
tabbing past would have rounded it. A box still reading what is stored now
counts as no edit at all, so looking at the form never changes it.

## 0.18.8

**`auto`'s per-appearance pairing is in Settings.** These decide what `auto`
actually serves on each side — phosphor tubes at night and light paper by day
is the example the theme docs give — and until now they were reachable only by
editing the config file. **Auto dark** and **Auto light** now sit under the
Theme picker they qualify and above the day-hours, in the order the settings
answer: what each appearance serves, then when the clock calls it day.

Each offers everything the setting accepts: the built-in default, the three
rotating pools (dark, light, CRT), and all nine palettes individually. The full
list matters — a picker that could not reach `crt-green` would still have to
*show* it for anyone who had set it in the file, and a value the form can show
but not produce is one the next Save quietly drops.

With this, the only settings still living in the config file alone are the two
footer usage budgets.

## 0.18.7

**`auto`'s light hours are in Settings now.** The window that decides day from
night when your OS appearance is pinned was config-file-only. Two boxes,
**Auto day from** and **Auto day to**, sit paired under the Theme picker in the
APPEARANCE card — they are one window, and they are only read while the theme
is `auto`, so that is where they belong.

They take `HH:MM` and nothing else. A valid time is tidied up on save (`5:5`
becomes `05:05`) and an invalid one keeps what was there before rather than
guessing — coercing `25:00` to midnight would pin the theme to dark all day
from a typo. If the config file already held something unparseable, the boxes
show the window actually in effect rather than echoing the typo back.

The check that was supposed to prevent a setting reaching the config file
without reaching Settings could not: it hand-listed the fields it expected, so
anything nobody added to the list was invisible to it. It reads the config
struct itself now, and every key must either be editable or listed as
deliberately absent with a reason. That turned up four more config-only
settings — `auto`'s per-appearance theme pairing and the two footer usage
budgets — which are now known gaps rather than unknown ones.

## 0.18.6

**Three claims in the theme docs that were no longer true.** Not user-facing,
but the kind of rot that produces the bugs above. The grain field said "1.0 on
dark themes" when no palette had shipped 1.0 in months, and said nothing about
the modern family's deliberate zero — a number that did not exist, in the one
place someone adding a theme would look. The dark flag claimed to drive grain,
which it had stopped doing at the same time. And the rotation picker rested on
"every pool has at least four entries" when pools have had three since the
roster was cut to nine; the conclusion still held, but the premise had been
false for a release.

Every float a numeric field's doc names is now checked against what the
palettes actually ship. It caught two bugs in its own construction before it
caught anything else.

## 0.18.5

**Two `@projects` rendered in the same colour on a CRT theme.** A project tag
hashes its name to one of twelve palette colours, which is the whole point of
the feature — a mixed todo list reads by project at a glance. On a coloured
page those twelve are twelve hues and the closest pair is comfortably
tellable. On a phosphor tube every one of them is the *same* hue, because that
is what a tube is, so they can only separate by brightness — and they were
borrowed from the shell-output palette, where everything is bright. All twelve
sat in the top third of the range and the closest pair fell below the point
where two colours are visibly different at all.

The room was there and unused: the page is near-black, so the legible range is
about twice what those twelve were using. The tube pool is now spread evenly
across it, in the tube's own hue. The closest pair more than doubles its
separation and every project is tellable from every other. Coloured themes are
untouched.

## 0.18.4

**A tripwire under the colours nothing was watching.** No theme changes here.
The last two fixes had the same shape — a role outside the colour system's
contract, nothing measuring it, drifting until it was noticed by eye — and this
closes that class for the five signal colours (status, bell, broadcast,
activity, accent).

Worth recording what the measurement actually said, because the obvious reading
is wrong. The accent colour spans 3.7x in page contrast across the nine
palettes, which looks like chaos; almost all of it is the light/dark split, and
that split is correct. A light page reaches contrast by going dark and a
saturated dark loses it quickly, so every role sits lower on paper than on a
night page. Compared *within* an appearance the signal colours already agree to
within about 1.5x, and are now held there.

One exception, named rather than averaged away: `paper-dark` is the
high-contrast newspaper and its accent is a near-white, at more than double the
contrast of `nebula`'s orchid. Monochrome is that palette; forcing its accent
into the band would take the theme with it.

## 0.18.3

**The bell and the status line were the same colour.** They mean different
things. The status colour is progress — the git dirty dot, the input-bar
status, gauge fills. The bell colour is *needs you*: the attention glyphs a
pane wears (`!` rang, `⚑` matched, `✓` agent done, `⊗` exited, `?` waiting) and
every ERROR line in the log. On `paper-dark` and `sepia-dark` they shipped as
literally one value, and on `paper-light` and `sepia-light` within one visible
step of it. A pane that had finished looked like a pane that was still working.

`nebula` and `blossom` had already solved this by hand, and they say what the
rule is: the alarm breaks away from the hue progress is already using. So it is
derived now, taking its hue from the palette's own red — nothing invented — and
sitting at exactly the status's loudness, so the two markers are equally
readable and not the same marker. Four palettes move; the two that did it
properly are left exactly as they were.

**The three CRT tubes keep theirs, deliberately.** A phosphor has one hue, and
that hue is the entire theme; rotating `crt-green`'s bell to orange would
separate the markers and destroy the palette. On a tube the separation is the
one a real terminal used — the attention glyph blinks.

## 0.18.2

**The search highlight was invisible on light pages.** `find_hl_bg` is the one
background colour a palette ships, and the colour-system work never reached it
— that derived the *text* ladder, and this is a wash sitting behind text. So it
stayed hand-picked, and it split by appearance: the wash sat 0.106–0.134 off
the page on the three light themes and 0.173–0.246 off it on the six dark ones,
on a scale where 0.10 is one rung of the text hierarchy. `paper-light` bottomed
out at **1.25:1** — a search highlight you have to hunt for.

That is structural rather than four bad guesses. On a dark page a highlight can
gain lightness *and* colour; on a bright one the presets reached for a pale
yellow that barely moves off the paper.

The wash is derived now, with the palette still declaring what colour its
highlight is — sepia's amber, nebula's violet, the phosphor's own green — and
the system supplying only how far off the page it sits. The bar is the median
of what the nine palettes already do, applied as a floor: five are past it and
are untouched, and four move (`paper-light`, `sepia-light`, `blossom`, and
`nebula` by less than one visible step). Text on the highlight stays at 9.3:1
or better everywhere it changed.

## 0.18.1

**`auto` was stuck on dark all day on any Mac pinned to Dark.** The theme
promised "light by day, dark by night" and followed the OS appearance to
deliver it. macOS has three appearance settings, though, and the window
system only reports two: Appearance: Auto arrives as whichever side it is
currently showing, indistinguishable from someone having chosen that side
outright. So crew could see *which* appearance was active but never whether
it would ever change — and on a Mac set to Dark it never does, which made
`auto` a permanent synonym for `dark`, at noon, while the picker went on
promising otherwise.

crew asks the missing question now. While macOS switches its own appearance,
the OS still wins outright: it already encodes the day/night intent, sunset
schedule and all, and second-guessing it would be worse. Only once the
appearance is **pinned** does crew fall back to its own clock — a light-hours
window, `07:00`–`19:00` by default:

```toml
auto_light_from = "07:00"
auto_light_to   = "19:00"
```

Set them to your own hours; a window whose end is at or before its start
(`20:00`–`06:00`) wraps past midnight, for anyone whose day does. Crossing a
boundary switches the theme where you can see it happen rather than at the
next ten-minute rotation. Nothing changes on Linux or Windows, where there is
no way to tell a pinned appearance from a scheduled one: whatever the OS
reports is taken as the whole truth, exactly as before.

The window is wall-clock rather than real sunrise and sunset because crew has
no idea where it is, and a location permission prompt or a network call is a
poor trade for deciding a colour.

**And `/theme` says which clock is deciding.** "auto is dark at noon" and
"auto is dark because you pinned the OS to dark" used to be the same
sentence — a single word, `auto` — which is precisely how this shipped
broken and stayed that way. It now reads:

```
theme: auto — the OS appearance is pinned, so the clock decides: it is day,
serving light; the dark half is dark (light hours are 07:00–19:00, set with
auto_light_from / auto_light_to)
```

## 0.18.0

**Nine themes instead of twenty-four, and one colour system underneath them
all.** A minor bump rather than a patch, because fifteen palettes are gone
and every remaining colour was re-derived.

### The palettes are a system now, not 2,376 opinions

Every colour in every theme used to be picked by eye, one theme at a time —
seventeen roles plus a sixteen-slot terminal palette, times twenty-four. They
all passed the contrast checks, because the checks were run afterwards and
the failures nudged by hand. Nothing related any two of them: measured across
the themes, the *same role* landed anywhere in a **2x band**, and the accent
in a **4.9x band**. So switching themes did not move the palette to the same
place. It moved it to whatever was decided on a different afternoon.

Roles are derived now. A palette states what it is — the page, the hue and
saturation of its ink — and the ladder below that (body text, legends, hints,
placeholders, borders) follows from one shared set of levels, computed in a
perceptual colour space so that "one step dimmer" means the same thing at
every hue. Twenty of the twenty-four moved by less than half a step in the
process; rendered side by side, the largest frame changed by 1.4%.

Two deliberate exceptions, both because a rule that fits paper does not fit a
phosphor. **No theme's text goes near-white** any more: matching contrast on a
page lighter than its neighbours wanted `(250, 250, 252)` on one theme, which
is the glare every dark-mode guide warns about, so there is a ceiling. And a
**white** CRT tube keeps its brightness rather than being held to the coloured
tubes' levels — being bright is that theme's whole point.

### Shell output got the same treatment, and it needed it

The terminal palette was checked for legibility and nothing else. Three things
were wrong. ANSI black bottomed out at **1.36:1** on dark themes — very nearly
the background, so anything printed in it was invisible. Slots ranged from
4.6 to 17.3 against their own background depending on theme and colour. And
nothing ever compared the colours to *each other*: on the amber tube, ANSI
green and yellow sat closer together than eight shades of grey, which is to
say `ls --color` and `git diff` were drawing two different things in the same
colour.

All sixteen slots are derived and checked now, including the blacks and whites
nobody had ever looked at. Colour themes separate by hue, using crew's own
hues rather than an imported convention. Phosphor tubes separate by
brightness, the way a real amber monitor did — spreading them by hue would
have made them legible and destroyed them.

### Twenty-four palettes, several of which were the same palette

The two closest were **Δ 0.0209** apart, under the point where two greys stop
being distinguishable. Nine more pairs were nearly as close. The nine that
remain were picked by measuring which are genuinely most different from one
another, then making sure each family kept both its light and dark side:

- **paper** dark and light — newsprint, both ways up
- **sepia** dark and light — the warm pair
- **nebula** and **blossom** — the modern look, dark and light
- **crt-green**, **crt-amber**, **crt-blue** — the three tubes

The closest pair among them is now nearly three times further apart than the
closest pair before.

**If your config names a retired theme, it still works.** All fifteen resolve
to their nearest surviving relative, always of the same appearance — a dark
desk does not suddenly go white. `graphite` becomes `paper-dark`, `aurora`
becomes `nebula`, and so on.

### And some tube maths that had been running for nothing

The CRT pass carried barrel curvature and a corner vignette that every theme
had set to zero since the flat-tube decree. The shader was warping by an
identity and multiplying by one, per pixel, per frame. Both are gone — and
because "this changes nothing" is a checkable claim, it was checked: every
frame is pixel-identical.

## 0.17.12

**A failed update no longer removes crew.** This was worse than an update
that did not work. The library crew used to swap in the new binary does
three things in order on Windows: rename the running `crew.exe` aside, copy
itself into `%TEMP%` and *run* that copy to clean up, then move the new
build into place. The middle step executes a program out of `%TEMP%`, which
managed machines and most corporate antivirus refuse — `Access denied` — and
it happens *after* the rename, so the whole thing gives up having already
moved your `crew.exe` away and never puts anything back. If your install
looks broken after a failed update, re-running `install.ps1` restores it.

crew now does the swap itself, the same way `install.ps1` already does it:
rename aside, move the new one in, and **put the old one back if that
fails** — so a failed update leaves a working crew rather than none. No
second process, no running anything from `%TEMP%`, no administrator rights.
The download is staged next to the binary it replaces rather than in the
temp directory, which is both more likely to be permitted and the only way
the move can work at all when temp is on another drive. The superseded
binary is cleaned up on the next launch, since Windows will not delete a
program while it is running.

`crew --self-update` had the same flaw and shares the fixed path now.

**And a failed update says why.** It used to show a card in the sidebar that
cleared after a few seconds and record nothing anywhere, so the only
possible report was "it failed". Update outcomes now go to the LOG and to
`activity.log`, naming the step that failed.

## 0.17.11

**No more console flash on Windows.** Launching crew from the Start menu or
Explorer flashed a black console window every time, and nothing in the
program could stop it: Windows hands a console-subsystem program a window
before its first instruction runs, so even closing it immediately leaves it
up for the whole of startup. crew is a GUI-subsystem program now and no
console is ever created.

The console modes still work — `crew --version`, `crew ask`, `crew panes`,
`--list-fonts` all reattach to the terminal that launched them, and piped
output still goes to the pipe. One thing genuinely changes: shells do not
wait for a GUI program, so a command typed at a prompt may print *after*
the prompt comes back. There is no way to have both.

**Panes open somewhere useful again.** Double-clicking `crew.exe` in the
unzipped download made every pane open *inside that download*, so the
prompt read `PS C:\Users\me\Downloads\crew-v0.17.10-x86_64-pc-windows-msvc>`
— the exe's own folder, reported as the place you were working. Windows
sets the working directory to the exe's folder when Explorer launches it,
and crew took that at face value. It now starts at your home directory
when the directory was picked by a launcher rather than by you. The same
fix covers a macOS Dock launch, which lands at the filesystem root.

**And the path reads as a path.** Three smaller Windows bugs stacked on top
of each other: `$HOME` does not exist there, so the `~` shortening never
happened and `cd ~` did nothing; the path separator was hardcoded to `/`,
so `C:\Users\me\code` would not have shortened anyway; and Windows'
`canonicalize` returns `\\?\C:\Users\me\code`, which crew was showing
verbatim. Between them, a path that should read `~\code` was displayed in
full with four characters of prefix noise.

## 0.17.10

Two Windows bugs, both found from a photo of a running v0.17.9 screen.

**A new terminal window opened every three seconds.** `crew.exe` is a
console program — it has to be, because `crew --version`, `crew ask` and
the broker all need real stdio — but the window itself is launched
detached, which on Windows means it has no console of its own. Windows
hands a brand-new console *window* to any console program started by a
process without one, and crew checks `git status` every three seconds for
the sidebar. So: a terminal, every three seconds, forever, taking focus
each time, while the main window sat there looking perfectly fine. The
broker, the shell probe, every MCP server and the `/far` and file-viewer
helpers did the same thing on startup. Every one of them is now started
with an invisible console instead — the pipes work exactly as before.

**Text was still mangled, because the font crew ships was the last
resort.** Embedding Lilex in 0.17.9 only helped a machine that resolved
*nothing*; it went in behind `Noto Sans Mono`, so any machine carrying
that name still preferred it — and still drew a broken grid. The grid
rounds every glyph to the nearest whole cell, which is invisible for a
real terminal face and brutal for anything else: wide letters take two
cells and narrow ones (`i`, `l`, `.`, and the space) take **none**, so
`terminals.` came out as `term inals` and `commands` as `com m ands`.

The generic fallbacks are gone from the theme preference lists. Their
whole job was "resolve to something", and crew now carries a face of its
own that does that better. They are still in the picker, so `/font` can
still reach them — they are just never crew's automatic answer.

And crew now checks its own work: before a font can be used, it shapes a
probe through the real grid, at the real size and weight, and confirms
every glyph lands on exactly one cell. The old check measured the font it
found by name; this one measures the font that actually gets drawn. On
that Windows machine they were not the same font, which is how a
proportional render passed a fixed-pitch test.

## 0.17.9

crew now ships its own typeface, and a fresh Windows install finally looks
like crew.

Until this release, first launch on Windows drew the whole app in **Segoe
UI — a proportional face**. The banner and the left nav looked worst,
because a box-drawn frame shows column drift the instant it drifts.

Nothing about it was Windows-specific in the code. crew asks the text
engine for "the monospace family" and never says which one; cosmic-text
hardcodes that to `Noto Sans Mono`, which Windows does not have, so the
lookup missed and shaping fell through to the platform's general fallback
list — headed by Segoe UI. Then the grid did what it always does and
rounded every glyph advance to the nearest cell, which sends a
proportional face's narrow glyphs (`i`, `l`, `.`, `|`) to *zero* width.
Glyphs land on top of each other and every column after them slides.

Themes could not save it. Each one names the faces it would like, ending
in something that ships with an OS so a bare machine still resolves
*something* — except the tails were `Menlo`, `SF Mono`, `Noto Sans Mono`.
All three are macOS or Linux stock, and 23 of the 24 themes named nothing
a stock Windows box has. macOS was never actually immune either: its own
fallback list is headed by the system UI sans, just as proportional. It
escaped only because `Menlo` is in every list and every Mac has it.

So crew stops asking the machine. It embeds Lilex — the face the CRT
themes already led with — and registers it as the monospace family, four
weights and their italics, about 840 KB. A font you have installed still
wins: pick one with `/font`, or let the theme choose. There just is no
longer a machine where the answer is "none".

## 0.17.8

Opening something stops freezing everything. Cmd+clicking a URL, `/far`'s
Open and the browser hand-off at sign-in all went through the blocking
form of the opener, which waits for the program it launched to exit — and
they run on the thread that draws, so until that program came back, every
pane sat still. They now hand the file off and return.

Windows got the sharper end of it. Asked to open a path with no
association, Windows raises the "How do you want to open this file?"
dialog, and nothing dismisses that on a machine with no one in front of
it: CI sat on that modal for five and a half hours before anyone noticed
the build had not failed, only stopped.

The input bar can recognise a command on Windows again. Deciding whether
what you typed is worth a pane means resolving the first word against
PATH, and that search read a Unix PATH: split on `:`, which tears `C:\bin`
into `C` and `\bin`, and treating only `/` as a sign of a path, so
`C:\tools\rg.exe` looked like a bare word to go hunting for. Nothing ever
resolved, so every line you typed came back "not a command" and the bar
hinted instead of running it. Entries are split the way the platform
writes them now, a path is anything with a parent, and runnability follows
`PATHEXT` — which also means a bare `git` finds `git.exe`.

Windows CI passes for the first time, end to end: the release binary
builds and the installed app starts. A hung job now fails in 45 minutes
naming the step that hung, rather than running until the six-hour ceiling,
and a superseded run is cancelled instead of left to finish.

## 0.17.7

Three themes again. The modern family arrived as two themes of its own —
`modern` and `modern-light` — which meant the picker offered five looks
where the difference between two of them was only *which palettes*
rotate, and the Gemini/Codex pages you had to go and choose deliberately
never turned up in the rotation you actually run. They are pages like any
other, so they now rotate inside `dark` and `light`: the dark pool draws
from nine palettes (five paper, four glow), the light pool from ten (six
paper, four glow), and `crt` keeps its five phosphor tubes. `/theme`, the
settings picker and `Ctrl+Shift+L` are back to `dark → light → crt →
auto`.

A palette's own appearance decides its pool now, so nothing has to be
filed by hand: is it a phosphor tube (`ThemeId::is_crt` — which the
bloom-only `CrtStyle` the modern palettes carry for their halo
deliberately does not make it), and if not, is its page dark or light.
Every palette lands in exactly one pool, and a rotation still never
flips the page near-black↔near-white under you.

`modern` and `modern-light` keep parsing — from `/theme`, from `theme`,
and from `auto`'s `theme_dark` / `theme_light` pairing — resolving to the
pool that swallowed them, so a saved config opens on the appearance it
asked for instead of silently doing nothing.

## 0.17.6

`/theme` stops whispering. Two silences made a theme that works look like
a theme that does nothing, and both are gone.

A name this build doesn't know — `/theme modern-light` on any build from
before 0.17.5, say — changed nothing on screen and explained itself in a
three-second flash on the input bar's bottom border, which is exactly the
kind of message you miss while watching the page for a colour that never
arrives. That report is now an error: it raises a toast on the canvas and
stays in the LOG after the flash expires, and it still lists the modes
that DO exist.

`auto` now names the half it is serving. It used to report the bare word
"auto", so a pairing configured for the appearance you are not currently
in had no symptom at all: `theme_light = "modern-light"` under a dark
macOS renders identically to a config line that was ignored. `/theme`
with no argument now answers "theme: auto — OS is dark, serving dark; the
light half is modern-light (it shows when the OS turns light)", in the
input bar and in a chat pane's listing alike.

## 0.17.5

The modern family gets its light half. Daybreak, Blossom, Meadow and
Cirrus are the same Gemini/Codex look with the lights on — near-white
pages, deep slate ink, and each palette's two poles driving all three
of the family's signatures: the gradient ring on the focused frame,
the drifting wash under the page, and the dot lattice woven over it.
The poles are deepened rather than reused: a pastel that glows on
near-black is invisible on paper. They rotate as their own theme,
`modern-light`, which joins dark/light/crt/modern/auto in `/theme`,
the settings picker and the Ctrl+Shift+L cycle — so a rotation never
flips the page from near-black to near-white mid-session, and `auto`
can pair the light half by day with the dark half by night.

The halo had to be rebuilt to get there. The bloom chain only knew how
to add light, which on a near-white page blooms the PAGE and clips the
whole frame to flat white. On a light page the pass now keeps each
pixel's colourfulness instead of its brightness, and the composite
subtracts the blur: the ring lays a soft halo of its own hue on the
paper, coloured text glows the same way, and neutral body ink stays
crisp. Every palette was validated against the contrast suite before
it was written.

## 0.17.4

Done todos stop hiding. A todo pane whose items are all ticked used to
render the same "no todos" screen as an empty one — with nothing to
select, `H` couldn't even be reached — so finished work looked like no
work. That pane now says "all done · 7 in the history" and names the
way in, and any list holding ticked items grows a clickable
`[show 7 done]` button on its header (it flips to `[hide done]`). For
the keyboard: `/todo show` and `/todo hide` do the same from the
command bar, `h` still toggles on a focused list, and an empty
filtered history now says "nothing done in @home" instead of claiming
nothing was ever finished.

## 0.17.3

The modern themes get their aurora. Under the dot lattice, two broad
pools of each theme's own gradient poles now light the page from
opposite sides — Aurora's blue and violet, Nebula's violet and rose,
Graphene's greens, Cobalt's blues — so the backdrop reads as coloured
light instead of a flat fill. The pools turn slowly about the centre
while a pane is working and hold wherever they stopped when things go
quiet, so an idle crew still never repaints a frame, and Motion off
freezes them outright.

## 0.17.2

The modern themes' dot lattice pulls in tight. The grid was pitched
every 4th column and 2nd row, which read as sparse polka dots; it is
now a fine square weave — about six dots to a text row, with pin-fine
dots that never touch — so Aurora, Nebula, Graphene and Cobalt sit on
woven engineering paper instead of a widely spaced grid. It still
rides the cell metrics, so it scales with font size and DPI, and it is
still perfectly static.

## 0.17.1

The modern themes get their engineering paper. A faint lattice of soft
dots now sits behind everything on Aurora, Nebula, Graphene and Cobalt
— pitched to the text grid (every 4th column, every 2nd row, so it
scales with font size and DPI) and tinted with each theme's own
gradient poles, sliding from one to the other across the page. Purely
static — it never animates or costs a frame — and the other families
keep their newsprint grain untouched.

## 0.17.0

See what you finished, not just what's left. `/todo done` (or `H` on
the todo list, Esc to come back) opens the done history: a done-only
log grouped under day headers — today, yesterday, aug 10 — newest
tick first, each row wearing its tick time and `@project` chip.
Ticking an item now records *when* (items ticked before this release
group under "earlier"), `@project` filters the log — `/todo done
@crew` opens it pre-filtered — Space un-dones an entry back onto the
list, and `d` deletes it from history for good. Ticked something last
week and can't remember? Check the log before doing it twice.

## 0.16.13

Intel One Mono joins the font rotation — the allowlist gained it and
Cascadia Code, so `/font` roulette can land on both.

## 0.16.12

Markdown tables with CJK or emoji stay inside their lane. The table
renderer padded columns by display width but clamped rows by character
count, so a wide-glyph row could run up to twice the pane budget and
wrap mid-table. The clamp now counts display columns (and still bounds
characters, so zero-width runs can't slip through); a wide glyph
straddling the edge is dropped whole, never split.

## 0.16.11

Flip between projects without typing. `]` and `[` on the todo list
cycle the `@project` filter through your tags in usage order — "no
filter" is one stop on the ring — with the selection re-entering at
the top of each view. Combined with the per-project colors, flicking
through contexts is now a two-key habit.

## 0.16.10

Deadlines move without retyping them. `+` and `-` on a selected todo
postpone or advance its due date one calendar day — real calendar
math, so a 9:00 due stays 9:00 across DST and month ends. `+` on an
undated item starts it at tomorrow morning. A moved deadline arms its
due toast again.

## 0.16.9

The tag popup speaks in color. Completing an `@project` in the todo
composer now shows every candidate row in that project's own color —
the same one its chips wear in the list — so you can pick the tag by
hue before reading it. The command palette keeps its usual accent.

## 0.16.8

Done todos get a way back. Completing an item still hides it on the
spot, but `h` on the list now shows the done pile — sunk below the
open items, dimmed, `[x]`-boxed, newest completion first — so a
mistaken tick is one `Space` away from alive again. `h` again (or
just moving on) tucks them back out of sight.

## 0.16.7

Swarm runs leave a trail. When a `/smith` run spawns agents and its
tasks start, finish or fail, those beats now land in the LOG — and in
`/log`'s session file — as quiet lines that speak task titles
(`smith: task #2 'scan logs' → done`), with failures at the error
level. Quiet means quiet: unlike other LOG events they never flash the
input bar or raise a toast, and the high-volume token/output ticks
stay out entirely.

## 0.16.6

`/log` opens the whole story. The sidebar LOG shows only the last five
lines of activity; now every entry is also mirrored to `activity.log`
beside the config (fresh each session, errors marked `ERR`), and `/log`
opens the full trail in the file viewer — press `r` there to re-read.
A wedged or crashed session leaves its trail on disk for the next one.

## 0.16.5

The todo composer gains a real cursor. Until now the draft was
append-only — a typo at the start of a long multiline draft meant
backspacing everything after it. Now `←`/`→` move by character,
`Alt+←`/`Alt+→` hop words, `Ctrl+A`/`Ctrl+E` (or `Home`/`End`) jump to
the draft's ends, and typing, paste and forward-Delete all act at the
cursor — with the `▏` beam drawn exactly where you are, the live due
and `@tag` tints intact around a mid-string edit, and the capped card
following the cursor's line instead of the tail. On a wrapped draft
`↑`/`↓` travel the visual lines at the nearest column; only the edges
hand focus to the list.

## 0.16.4

The todo list pages from the keyboard. `PageUp`/`PageDown` hop the
selection a whole visible page of items — a page is the rows the window
actually shows, so wrapped multi-row titles count for their real height
— and `Home`/`End` jump to the first/last item. All four respect an
active `@project` filter and keep the selection scrolled into view; the
`Shift+` chords keep their app-wide pane-scroll meaning. The new keys
are listed in `/keys` and the manual.

## 0.16.3

Every `@project` gets its own color. A mixed todo list now reads by
project at a glance: the tag's name hashes to one of the theme's twelve
chromatic terminal-palette slots — nothing stored, so every pane,
restart and platform agrees, and the same project keeps the same slot
on every theme (switching themes recolors consistently instead of
reshuffling). Each color is lifted toward the theme's ink until it
clears a 3.0 contrast floor against the page, so light themes stay
readable. The row chip, the live tint while you type `@crew` in the
composer, the composer legend under an active filter, and the filter
header's `@tag` all agree on the color; due dates keep the accent.

## 0.16.2

Theme switches stop blanking the window. The old switch drew a solid
page-color wash at full opacity and faded it out — for its first frames
the screen was empty, and `/theme` read as a mini restart. Now the
renderer keeps a live snapshot of the last presented frame; when the
theme flips, the new look renders in full from its very first frame and
the old frame melts away over it (~450ms crossfade). Content is visible
at every instant — dark→light, CRT→modern, an OS appearance flip in
`auto`, all one continuous develop. With Motion off the switch is an
instant cut, and it is never a blank screen.

## 0.16.1

The todo composer wraps. A long title flows onto new rows as it fills
the card's width instead of scrolling out of sight in a single line —
the card grows with it, up to four rows.

## 0.16.0

Crew learns a modern look. A fifth theme mode, `modern`, joins dark /
light / crt / auto: four new palettes — `aurora` (Gemini blue→violet),
`nebula` (orchid→rose), `graphene` (Codex neutral with a mint accent)
and `cobalt` (electric blue→cyan) — with deep neutral pages, zero paper
grain, and a soft wide glow that runs the bloom chain with every retro
knob (curvature, scanlines, bezel) at zero: clean light, not a tube.
The focused pane's frame becomes a gradient light-ring, blending
corner-to-corner between each palette's two accent poles; it ignites
white when focus arrives, drifts slowly along the frame while the pane
is streaming, and holds perfectly still when idle. Modern themes lead
with modern typefaces (Google Sans Code, Commit Mono, Martian Mono,
Geist Mono) and sit in `/theme`, the settings picker and the
`Ctrl+Shift+L` cycle — `/theme modern` to move in.

## 0.15.1

The todo list stays a list of things left to do. Completing an item hides
it on the spot — done items no longer pile up under the open ones (they
remain in `todos.toml` as history), and the `@project` filter header
counts only what's open. Long todos now wrap instead of clipping: a
title flows onto as many full-width rows as it needs, the due/`@tag`
chips keep the first row, and clicking anywhere on a wrapped item selects
it — so the whole task is always readable at any pane width.

## 0.15.0

`/todo` opens a todo pane: one global list you type into. Enter captures
an item; a natural-language date fragment (`tomorrow`, `fri 5pm`,
`aug 15`) tints live as you type and becomes the due on save, and an
`@project` token becomes a free-form tag, autocompleted from tags already
in use. The list sorts overdue → due → undated, Space/Enter completes,
`d` deletes, `e` re-opens an item in the composer, and a lone `@tag`
filters the list to one project. Items live in `todos.toml`, every todo
pane shows the same list, and a due item raises a toast the minute it
lands.

## 0.14.4

Theme flips reach the programs inside the panes now. Terminals that enable
DECSET 2031 (neovim's TUI and other scheme-aware programs) get a
`CSI ? 997 ; Ps n` report the moment crew's light/dark scheme changes —
an OS appearance flip under `auto`, `/theme`, `Ctrl+Shift+L`, a Settings
apply — so they re-query OSC 10/11 and repaint for the new palette
mid-session instead of riding the contrast floor until restart. Crew also
answers the `CSI ? 996 n` "what scheme is it?" query and DECRQM support
probes for mode 2031, all from the same active-theme source of truth the
OSC 10/11 answers use. Programs that never opt in see nothing new.

## 0.14.3

Pair `auto` your way. Two new `config.toml` keys — `theme_dark` and
`theme_light` — re-wire what the OS-following `auto` theme serves per
appearance: each side names a rotation pool (`dark` | `light` | `crt`) or
a pinned palette, so night can be green phosphor while day stays light
paper (`theme_dark = "crt"`), or dark mode can live on one exact palette
(`theme_dark = "moss-blotter"`). Unset keys keep the built-in paper
pairing; `auto` can't serve as its own side. A Settings apply that
changes the pairing re-themes immediately — no waiting for the next OS
flip or rotation tick.

## 0.14.2

Crew follows the system now. `auto` — light by day, dark by night, riding
the OS appearance — graduates from unlisted back-compat alias to crew's
fourth first-class theme: it sits in the `/theme` picker, the Settings
form, the composer suggestions and the `Ctrl+Shift+L` cycle
(dark → light → crt → auto), and a fresh install with no saved theme now
defaults to it, so crew matches the system from the very first frame.
An appearance flip mid-session re-themes every pane live through the
develop-fade. Existing configs are untouched: a saved `dark`, `light`,
`crt`, or pinned palette keeps exactly its meaning — picking one is still
how you opt out of following, and `/theme auto` is how you opt back in.
Both startup and settings-apply now resolve the saved value through one
shared `theme_selection`, so the two paths can never disagree.

## 0.14.1

The glyph atlas breathes again. The vendored glyphon atlas was never
trimmed, so its in-use set only ever grew and LRU eviction was dead code —
every font size, family, weight, or smoothing change permanently pinned a
full ~570-glyph working set (the 0.13.7 prewarm made that a certainty),
ratcheting GPU memory until a long session could hit an AtlasFull panic.
`CellGrid::prepare` now opens each frame with `atlas.trim()`, so only
glyphs the frame actually draws stay protected and stale generations age
out under real LRU pressure. And the sidebar tells the whole truth again:
panes standing behind the minimized strip's `+N` overflow tile — visible
nowhere on the canvas — now carry the `[+]` restore marker on their PANES
rows, like every other off-screen pane.

## 0.14.0

One spacing rhythm across the canvas. Three layout-rhythm fixes close out
the 0.13.x look-and-feel loop. Legends now ellipsize instead of colliding
with the frame: a title too long for its card ends `… ─╮` — width-aware, so
emoji/CJK clip on a cell boundary — with the trailing rule always resuming
before the corner, and every card (panes, strip thumbnails, toasts, sidebar,
input bar) goes through the same `title_budget`/`clip_w` pair, replacing
three near-identical private clippers. The grid now sits exactly one gap
above the input bar at every font size: the content area's bottom edge is
derived from the bar's real cell-quantized top instead of a fixed reserve,
so the seam no longer wanders with the cell-height remainder (it ranged
2–22px, and could crush to nothing at large fonts). And a crowded minimized
strip caps its thumbnails at a readable width — the most-recently-active
minimized panes keep their cards, display order stays sorted, and a
trailing `+N` tile stands in for the rest, which remain one click away in
the sidebar's PANES list.

## 0.13.9

Font smoothing joins the Settings form. The flagship v0.12.5 stem-darkening
was `/smooth`-only — invisible unless you knew the command. The APPEARANCE
card now carries a **Smoothing** picker (`off · light · medium · heavy`,
Space/←/→ to cycle) sitting under the font fields. It reads and writes the
same `font_smooth` key as `/smooth` — the keyword ladder now lives in one
shared table, so the two surfaces cannot drift — a custom `/smooth 42`
strength shows as its number and is left alone until you actually cycle it,
and Save applies live through the same renderer path the command uses.

## 0.13.8

Three new themes. `crt-paperwhite` is the fifth phosphor — the P4 white
tube of the early Macintosh and the VT420: near-white ink with the faintest
blue-gray cast on a true black tube, fine scanlines, a modest cool halo and
the steadiest beam of the family. `moss-blotter` joins the dark papers: a
deep desaturated moss-green desk blotter with warm paper-white ink and
botanical accents — the study-lamp page. `glacier-bond` joins the light
papers: a cold blue-gray bond page, like overcast north light on cold-press
stock, with crisp near-black ink and slate-blue accents — where
coldpress-gray is strictly neutral, this one deliberately runs cold. All
three pass the palette arbiter's contrast floors and slot into the
dark/light/crt rotation pools.

## 0.13.7

Prewarmed glyph atlas. The first frames used to pay for every glyph in
sight — rasterize, smooth, pack, and (at Retina sizes) grow the 256² atlas
twice, re-uploading everything already packed each time. The mask atlas now
starts at its Retina steady state (1024², still just 1 MiB; the color/emoji
atlas stays small), and at startup — and after any font, size, weight, or
smoothing change — crew shapes one off-screen buffer of printable ASCII
plus the box-drawing, block, braille, and marker glyphs its chrome draws,
and runs it through the same smoothing-seeded path as real frames. Every
border, spinner, and keystroke thereafter finds its glyph already in the
atlas: no rasterization, no grow churn, mid-interaction.

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
