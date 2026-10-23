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

**Round A5 (v0.28.40): your own commands can check or block an agent's tool calls.**

- **The survey** (item 4): crew had no hooks; every tool call passes `SessionTools::call`.
- **Built in a worktree lane** (`feat/a5-hooks`) and reviewed: pre hooks after the approval
  gate block by exit 2 or a JSON decision, the reason read by the agent as its result; post
  hooks' output is added under the result; only the user's own `hooks.json` is read; a broken
  file blocks; `CREW_HOOKS=0`. `session.rs` shrank 660 → 633 (`sessionask.rs`).
- **Live, through a scratch HOME** (qwen-max): a guard refusing `rm` blocked it twice with its
  reason, and the agent's next call took the `mv` the reason named; the post hook's line came
  back under each of three reads; nothing was deleted.

**Round I7 (v0.28.41): a proposed new time is not an invite.**

- **The survey** (also noted): only `METHOD:REPLY` was "not an invite", so a proposed new time
  (`COUNTER`) offered `c` (adding the proposal as the event) and `y y` (an answer to yourself).
- **Every method but an invite's own** says what it is and offers no keys (`ics.rs`).

**Round I8 (v0.28.42): a failed invite answer says it did not go.**

- **The survey** (finding 8): Google Calendar's failures were worded as Gmail's, then the
  inbox's known words replaced the whole line, `not answered:` included, so a failed answer
  read "Gmail can't be reached from here" over an unchanged card.
- **`told.rs`** keeps the `not …:` head and Google Calendar's name; `fetch` names the service
  by the address asked.

**Round U7 (v0.28.43): the faint theme chips no longer run into a black bar.**

- **The second UI survey** (finding 1) caught U3's side effect: the page cells it gave six
  faint chips sat one above another, and the renderer merged them into one dark bar.
- **Each is its own capsule** (`cmdrow::spans`, the mark the fuzzy-match highlight uses).

**Round A6 (v0.28.44): every group of tool calls a reply ends with runs.**

- **Found tracing A4**: a call above the reply's last ran only with blank lines alone between,
  so fenced edits over "then run the tests" and a `sys:run`, or two fenced groups, ran only the
  last group; the edits stayed text and the agent went on as if they were made.
- **`call_ending`** takes a call above when the gap under it passes the reply-tail test
  (`ends_the_reply`): fences, at most two short lines, no retraction. The old test that held one
  prose line as quoting now holds a three-line explanation or a retraction.
- **Live, inconclusive**: two runs of the compile-error task never finish in the budget either
  way; they changed 12 and 3 files against 4–7 before. Unit tests carry it.

**Round I9 (v0.28.45): `-` on an overdue row no longer toasts it again.**

- **The survey** (also noted): any `+`/`-` re-armed the due toast, so moving an overdue row
  further back toasted it again. Now a due still past counts as told, as the composer has it.

**Round U8 (v0.28.46): the empty TOKENS ring is quiet again on the tubes and the glass.**

- **The second UI survey** (finding 2) caught U4's side effect: solid, the ring was lifted by
  the tubes and the glass past the frame (7.7:1 on crt-green) and read as a full ring, and
  over a white desktop it was still under the empty hours' dots.
- **`heatmap::track_ink`**: solid on paper, the idle dot's wash elsewhere. crt-green 2.5:1,
  glass 2.8–3.0 over dark and 1.7 over white, paper 3.0.

**Round A7 (v0.28.47): a tool call written as `<parameter name="…">` tags runs.**

- **The survey** (item 5, loop 15's open item 3): qwen3.8-max's sixth call shape, the name and
  the arguments each in a `<parameter name="…">` tag inside `<tool_call>`, was text.
- **`paramcall.rs`** reads it, and the per-argument form Anthropic's XML calls use. Unit tests
  only (one capture; about one turn in forty).

**Round I10 (v0.28.48): "call bob next friday" is "call bob", due next Friday.**

- **The survey** (finding 7): the composer's grammar had no `next`/`this`, so only the weekday
  parsed and the word stayed in the title; the inbox read "next friday" as next week's and the
  todo card refused it.
- **`nextday.rs`**: `next WD` is the inbox's reading, `this WD` the coming one; `duedate.rs`
  shrank 252 → 240 (its debt row with it).

**Round U9 (v0.28.49): a blurred `/far` pane's divider is the frame's colour.**

- **The second UI survey** (finding 3): the line between the lists was the muted text ink, 7–10:1
  in a box drawn at about 2:1; 0.27.13 had moved the panel edges to the frame colour and missed
  it. Now `border_normal` (`merge_divider`); shots match the frame to the pixel.

**Round A8 (v0.28.50): a missed edit is shown the line it probably meant.**

- **From tonight's probes**: of 154 `sys:edit` calls 69 failed, 36 as "not in the file", and 31
  of those were never in the file as written: the agent retyped the line from memory.
- **`editnear.rs`**: the miss names the nearest line (difflib's ratio, 0.6 or more) and shows
  it numbered, as Aider's failed edits do. Replayed on the 31: 22 shown the line they meant,
  9 nothing (mostly a line in another file). Unit tests and the replay; no live run.

**Round U10 (v0.28.51): a settings picker never loses its left chevron.**

- **The second UI survey** (finding 4): the fit that pairs short settings counted a picker's
  chevrons but not their spaces, so Motion beside Glass read `auto (full) ›`.
- **Counted** (`fit::min_cols`, widest + 2); Motion now takes its own row, whole.

**Round A9 (v0.28.52): an edit that misses on whitespace is shown the lines as they are.**

- **From the same probe data as A8**: 14 of 69 failed edits were "the same text with different
  whitespace", answered only with "read the file".
- **`editnear::spaced`** shows the lines that hold the text, numbered, to copy. Unit tests only.

**Round U11 (v0.28.53): document windows keep a column of air inside their frame.**

- **The second UI survey** (finding 5): a document window drew from the frame's first inside
  column, so its sticky heading band ran 4 px from the strokes. `docwin::PAD` gives it a pane's
  gutter in the resize, the click mapping and the drawing. The band's tightness under the
  legend, vertically, is left as it was.

**Round I11 (v0.28.54): "standup tomorrow at 9am" is due tomorrow at 9.**

- **The second inbox/todo audit** (finding 1): a date, "at", a time parsed as the time alone,
  due today with the date left in the title; a mail's "by Friday at 5pm" came out due the day
  it was sent. `leadin::dated` lets the date before a time keep its "at".

**Round U12 (v0.28.55): the `@` and `/view` pickers keep a long path's file name.**

- **The second UI survey** (finding 6): a long path was cut at its end, so two files read
  `@crates/crew-app/src/viewpane/render_…`. A path row now keeps its tail, cut on a separator
  (`cmdlabel.rs`, out of `cmdrow.rs`, which shrank 198 → 189); match marks follow their letters.

**Round I12 (v0.28.56): `t` right after you answer names who asked, not you.**

- **The second inbox/todo audit** (finding 2): the card's fallback title named the last mail's
  sender, you, after your own reply. `smith::todo_title` takes the newest mail someone else
  sent; a corpus case holds it.

**Round I13 (v0.28.57): `#Émile` keeps one band, like any other name.**

- **The second inbox/todo audit** (finding 6): the bands matched names ignoring ASCII case
  only, so `#Émile` missed its own band and the list opened a header at every row. Names now
  match lowercased, in the bands, the filter and the tag menu.

**Round A10 (v0.28.58): a build too long to show names every error's place at the top.**

- **From tonight's probes**: the compile task's `cargo test` ran 54 times, every result cut to
  its tail, two of about 25 errors in view; no run passed.
- **`rundigest.rs`**: a cut run says `N errors at path:line,line · …` under its exit line, from
  the whole output (`runfit.rs` 199 → 191, `failed` moved with it).
- **Live, twice**: the line came with every cut run and named library files the agents had not
  looked at; the longer run went 25 → 23 → 26 → 25 → 19 → 20 → 14 errors. Neither finished.

**Round I14 (v0.28.59): `e` then Enter leaves a todo as it was.**

- **The second inbox/todo audit** (finding 4): a todo reloaded for editing and sent back
  unchanged lost a title's last "on" to the due's lead-in, and a title's own `@word` became
  its project. `edit_at` writes the tags first and the due before the title.

**Round A11 (v0.28.60): a failed edit sent again is answered, not run.**

- **From the probe data again**: 24 of 28 repeated edits repeated one that had just failed, 23
  in the same turn; an edit is all or nothing, so it could only fail the same way.
- **`Seen::missed`** remembers a failed edit until anything is written, and the relay answers
  the repeat with the first failure and its reason. Unit tests only.

**Round I15 (v0.28.61): "e archives this mail" only over the mail you unsubscribed from.**

- **The second inbox/todo audit** (finding 7): an unsubscribe answered after `n` offered `e`
  over the next thread, and `e` archived it. The offer now needs the sender's mail open.

**Round I16 (v0.28.62): declining one moved meeting no longer declines the series.**

- **The second inbox/todo audit** (finding 3): an invite for one occurrence lost its
  `RECURRENCE-ID` when read, so the reply answered the series, and the Calendar path patched the
  series event. `Invite.occurrence` keeps it as written; the reply names it; an occurrence goes
  by mail. The Calendar half is from the code, not run against Google.

**Round A12 (v0.28.63): a swarm worker that resends a failed edit is told too.**

- **Parity with A11**: the native tool loop (`apiagent::pending`) shares `Seen` and now answers
  a failed edit sent again from its first failure. Unit test; the relay's live probes stand.

**Round I17 (v0.28.64): "call mom at 5 pm" is due at five.**

- **The second inbox/todo audit** (also noted): `5 pm` with a space parsed as nothing.
  `clockword::joined` makes a clock and its am/pm one token, in every reader of a due.

**Round A13 (v0.28.65): `/doctor` says what your tool hooks are.**

- **A5's follow-up**: a `hooks.json` that will not load blocks every call, and nothing said so.
  `/doctor` has a `hooks` line: off, none, how many, or ✗ with the file's error.
