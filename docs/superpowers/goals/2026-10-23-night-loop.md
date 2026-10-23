# Loop — UI, crew AI, inbox and todo, through the night

**Status: RUNNING (from v0.28.24).** Set by the user at 01:00 on 2026-10-23, after loop 15
closed (`2026-10-22-loop-15.md`, v0.28.9–v0.28.23): *"keep running UI / Crew AI / Inbox /
todo loop for rest of the night autonomously"*.

## Rules that hold every round

The rules of loop 15 carry over unchanged:

- **One round, one release**, through `scripts/release.sh`. Rounds alternate UI → AI →
  inbox/todo, so no lane waits on another, and keep going until morning.
- **UI** rounds follow the shot survey: everything but `/inbox` and `/todo`, judged on dark,
  light, neon and the glass family. Nothing new on screen without a shot.
- **AI** rounds follow `/improve` (`.superpowers/sdd/ai-loop-2026-09-14.md`): one harness
  feature each, smallest honest version, the changelog naming the tool the idea came from, a
  live probe where one is possible and "unit tests only" said plainly where it is not.
- **Inbox and todo** rounds follow the inbox loops' rules (`2026-10-22-inbox-loop-7.md`): no
  feature the user did not ask for; a round fixes, simplifies or polishes; nothing a test does
  opens a browser, reads the real Downloads, the real grant or the real Reminders.
- Every file stays under the 200-line cap and no file in `line-cap-debt.txt` grows; a test
  fails without each fix; no scratch file stays in the tree while `release.sh` runs (it
  commits with `git add -A`); `release.sh` prints `GATE FAILED` but exits 0, so its log is
  read for `RELEASED` before the next round starts.

## The survey (once, at the start)

Three at once, as in loop 15: the UI on every palette through the headless shot tests; the
AI harness against loop 15's open list; the inbox and todo code, newest first.

## Log

**Round A1 (v0.28.24): an agent's edit can name which of two identical lines it means.**

- **The survey** (the probe logs of 2026-10-20 and 2026-10-22): 27 of 59 failed `sys:edit`
  calls were "`old` appears 2 times", and qwen-max sent `"line": N` with them thirty times,
  which `systools::edit` never read; one turn sent the same failing pair six times.
- **`editpick.rs`**: the refusal names each copy's line, and `"line"` picks the exact match
  whose lines hold it, or is refused with the list again. In a batch a line means the file as
  read (`Shift`), and the report and the shown rows follow the copy that changed. `edit()` left
  `systools.rs` (388 → 356) for `editargs.rs`; the report for `editreport.rs`.
- **Live, qwen-max, two identical `let floor = 4.5;` blocks, "change only the second"**:
  before, 2 refusals a run, 26–42 s, and 1 of 3 ended with `"all": true` changing both; after,
  1 refusal, the next call `"line": 15`, 3 of 3 right.

**Round I1 (v0.28.25): a change from the phone no longer moves the highlight to another todo.**

- **The survey** (the inbox/todo audit, finding 2): the todo pane's `poll` re-read the store
  keeping the selection as a row NUMBER, and since the Reminders sync a phone moves the store
  unasked. A reminder ticked or added there re-sorted the rows; `d` (no undo) deleted the todo
  that slid under the highlight, and the next pass deleted it on the phone too.
- **`follow`** (`todopane/mutate.rs`): a write from elsewhere keeps the highlight on its todo's
  id; a gone todo leaves it on the row. Keys still move by row. Two of the three tests fail
  without it.

**Round U1 (v0.28.26): `/theme` describes blossom and nebula as they are now.**

- **The survey** (every shot on every palette; 128 shot tests, 483 PNGs): the theme picker
  called blossom "violet→rose on warm white (modern light)", two weeks after it became a dark
  rose smoke with white words, and nebula named the retired "modern" family.
- **Two lines rewritten** (`ThemeId::describe`), and a test that no dark palette's line says
  light or warm and none says modern. Shot of the picker checked.

**Round I2 (v0.28.27): `/todo reminders` keeps the pairs.**

- **The survey** (finding 1): `/todo reminders` reset `reminders.toml` whole, list and pairs,
  so after the sync was off (or refused access, or the command was typed again to check) the
  next pass paired by title alone. Since I2 of loop 15 a title pairs only with one as done as
  it is, so a todo ticked or deleted in crew meanwhile came back as a new todo.
- **`turned_on`** keeps the list and the pairs and sets `remake`, which lets a deleted `crew`
  list be made again once (`pick`); the pass takes it and saves.

**Round U2 (v0.28.28): the frame light and the gloss stop at a pane's title.**

- **The survey** (finding 2): on neon an unfocused pane's title read 8:1 above the rule and
  2.5:1 below it, where its lower half stands inside the sheet on the frame's light. Its
  suggested fix (scale by the veil) would not have reached it: the veil is only the half above
  the rule. Measured in the shader's terms, the band was two things: neon's holographic bleed
  and the gloss lip every glass palette has.
- **Both stop at the frosted patch behind a title** (`glass.wgsl`, `* (1.0 - thick)`), the
  way the rim stops at its notch. Lower half over a dark desktop: neon 2.5 → 5.4:1,
  glass-clear 5.2 → 7.3, glass-night 5.2 → 7.3, nebula 4.6 → 6.5, blossom 5.2 → 6.9; over a
  white one 2.3–3.5 → 4.6–4.9. Shots compared before and after; a source guard test.

**Round I3 (v0.28.29): delete your last few synced todos and they stay deleted.**

- **The survey** (findings 3 and 10): the sync's guard read "every paired todo gone at once"
  as a `todos.toml` that would not load, so deleting the last two or more quickly brought them
  all back; and a file that would not load was read as empty and written over at the first
  change (`fs::write` truncates first, the very cut-off it feared).
- **The store tells the two apart** (`read_at`, `take_unreadable`): a broken file is kept as
  `todos.toml.bad` and the pass forgets the pairs once, so the open reminders come back; the
  guard keeps only its Reminders half. Writes go through `todos.toml.tmp` and a rename.

**Round A2 (v0.28.30): a turn of calls ending in a `sh` block of the next command still runs.**

- **The survey** (open item 2 of loop 15): a ```` ```sh ```` fence under the calls counted as
  prose, and with one line saying what comes next the reply went past the two lines a call
  may be followed by; both relay turns of a5w-run2 ended on four edits, unrun.
- **`around::opens_shell`**: a fence opening a shell block is wrapping; the command inside
  still counts, so several commands or another language still leave the calls as text.
- **Live, unit tests only in effect**: four runs of the a5w compile-error task (two each side)
  never met the shape; they ended on the 8-round budget, or on edits described with no calls.
  Seen on the way, left open: after a budget stop the "not run" note says the turn ended
  before any call ran, which names the wrong cause (`toolround.rs`).

**Round U3 (v0.28.31): `/theme` on the light page shows every palette's colours.**

- **The survey** (finding 4): a glass palette's chips are drawn with no page under them (smoke
  cells on glass went missing in patches, D-L10), so on the light page each one's first chip,
  its near-white ink, read 1.1:1. Its suggested fix (every glass chip on its page there) made
  the picker a column of dark slabs, since every mode is see-through now, the CRTs included.
- **Only a chip that would not read on the page it is drawn on gets its own page**
  (`swatch::chip_of`, under 1.5:1): the glass inks and the brightest phosphor faces. Nebula's
  survey item (its chips washed by a bloom twice glass-clear's) is left for later: the fix is
  either its bloom (its look) or deeper fills (the mark floor), both bigger than a round.

**Round I4 (v0.28.32): `t` on an overdue notice keeps its date in the past.**

- **The survey** (findings 4 and 5): a month and day in a mail was read as the next one, so
  "was due by Oct 15" in a mail of Oct 20 filed a todo for next October, its reason line
  hiding the year; "by" and "due" matched inside words ("lobby at 3pm" set a 3pm due); and a
  todo filed from mail with a passed due toasted a minute later.
- **`tododue::nearest`**: a month and day is the one nearest the mail's day; the keys must
  start a word; the reason line names a year that is not this one; `add_todo` files a passed
  due as told about, as the composer does.

**Round U4 (v0.28.33): `/usage` with nothing used shows its TOKENS ring.**

- **The survey** (finding 7): the empty ring, drawn so an idle crew and a broken widget differ,
  was half strength in the frame colour: 1.04:1 on glass over white, 1.6:1 on paper.
- **Drawn whole** (`plot/pie.rs`): 1.39:1, 2.98:1 and 3.02:1, the frames' own weight.

**Round I5 (v0.28.34): answering an invite never marks the next one answered.**

- **The survey** (finding 6): the answer from Google set "you accepted" on whatever thread was
  open when it landed, a second or two after `y y`, and `e` moves on at once.
- **Only the invite with the answer's UID is marked** (`icsanswer.rs`). The test fails
  without it.

**Round A3 (v0.28.35): a reply whose stream closed early is picked up, not passed off as whole.**

- **The survey** (loop 15's open item): a stream that ended cleanly with no `[DONE]` and no
  finish reason was taken as the whole reply; the resume of 0.28.17 only heard streams that
  failed. Checked on a loopback server: both providers returned `Ok "…the two bran"`.
- **Built in a worktree lane** (`feat/a3-stream-end`, an implementer agent) and reviewed: a
  stream is finished only when it says so (`[DONE]`, a finish reason, usage; `message_stop`),
  and one that closes before that with text or a call under way is a transient error, so the
  resume picks it up. `openai_http.rs` shrank 601 → 538 (`sseframe.rs`).
- **Live, through a proxy that ends the stream cleanly after N chunks** (qwen-max, reply
  route): before, 189 characters ending "This process involves", no error; after, resumed to
  892 characters (N=9) and 683 (N=14), joined cleanly.

**Round U5 (v0.28.36): `/disk` names keep the same air from their tile's edge.**

- **The survey** (finding 6): a name went one column past its tile's edge floored, so `crates`
  and `.git` sat a tenth of a cell from their borders and `target` most of a cell.
- **Half a cell at least, each side** (`diskdraw.rs`); a test sweeping 40–90 columns failed
  at 42 without it. Shot compared.

**Round A4 (v0.28.37): when the tool budget stops a turn, the answer says so.**

- **Found tracing A2's probes**: a turn's last reply had five fenced edits, a line, then a test
  run. Only the trailing call is the reply's call, so the edits stayed in the answer when the
  budget refused everything, under a note that the turn "ended before any call in it ran";
  the reply judge read that as a turn that did nothing.
- **`budget_answer` names the budget** for calls left in its answer, and `never_ran` adds no
  second note. Replayed through `Broker::run`; unit tests only.
- **Open, and bigger**: calls followed by prose and then more calls run only the last group,
  budget or not (`parse.rs`, `call_ending` needs the calls to end the text above them).

**Round I6 (v0.28.38): `r` answers the right mail when someone's address holds yours.**

- **The survey** (also noted): which mail a reply answers was decided by a substring, so
  signed in as ann@, joann@'s mail was "yours" and `r` answered the one before it.
- **Your whole address, any case** (`draft::answered`), as the rest of the inbox compares.

**Round U6 (v0.28.39): a pop-up's edge words stand on its backdrop, not its shadow.**

- **The survey** (finding 5): the `/keys` hint on the pop-up's bottom rule had the card's own
  shadow through its middle on glass over a light desktop: pop-ups have no sheet, so the veil
  a pane's title gets never applied to them.
- **`popupback.rs`**: the overlay backdrop (moved out of `scene.rs`, 556 → 537) reaches a cell
  high behind each word run on its rules, and the shader's shadow stops under a legend's outer
  half on every card. The first shot showed a pill per word: a legend's spaces sit on the rule
  cells and `notch::runs` read them as rule; now the later cell wins, as drawing does, which
  also makes a pane title's rim gap one gap.
