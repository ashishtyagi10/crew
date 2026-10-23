# Loop — UI, crew AI, inbox and todo, through the night

**Status: DONE — v0.28.24 to v0.28.97, 74 releases, 01:00 to 07:55.** Set by the user at
01:00 on 2026-10-23, after loop 15
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

**Round U13 (v0.28.66): chat find and history search show a hit's start again.**

- **The UI regression survey** (tonight's own changes, every palette; R2): U12's path rule took
  chat find's and history search's rows for paths and cut their fronts. A path now has no spaces.

**Round U14 (v0.28.67): pop-up titles and hints sit on soft frost, not a dark tab.**

- **The UI regression survey** (R1): U6's patch behind a pop-up's rule words was a box a cell
  high, a hard tab on glass over white and a notch in the tubes' glow. The quads are gone; the
  shader lays the page in under a sheetless card's rule words in the veil's feathered shape.

**Round U15 (v0.28.68): faint theme chips are drawn paler, not boxed in black.**

- **The UI regression survey** (R3): U3 and U7's page cells became 12 px black capsules butting
  into the next chip. A faint glass face is now walked to 2:1 on the page and left bare.

**Round I18 (v0.28.69): a due agent smith writes with a `T` is kept.**

- **The second inbox/todo audit** (also noted): smith's `2026-10-23T17:00` on the todo card was
  no date `/todo` reads and was dropped. `todofill::untee` reads it; a zoned one stays out.

**Round U16 (v0.28.70): `/theme` in a narrow tile, every row says what sets it apart.**

- **The UI regression survey** (N1): cut to fit, `dark` and `light` both read "Ink Reader — an…"
  and three glass rows "iPhone liquid…". The descriptions lead with what differs now.

**Round I19 (v0.28.71): Tab while agent smith works says what it is working on.**

- **The second inbox/todo audit** (also noted): the busy line named the key's job, so Tab on the
  card while a draft came said "still filling the card". `smith_busy` names smith's own job.

**Round U17 (v0.28.72): on the CRT tubes, swatches keep their colours.**

- **The UI regression survey** (N2): the tubes' filament turned every 6 px chip white-hot. A
  coloured chip is drawn just under the filament's floor on a tube that burns; greys are left.

**Round I20 (v0.28.73): an edit to a todo deleted meanwhile is kept.**

- **The second inbox/todo audit** (also noted): Enter on a todo deleted elsewhere while it was
  open for editing dropped the words silently; they are added as a todo of their own now.

**Round I21 (v0.28.74): an invite you are not a guest of says so, not "was told".**

- **The inbox audits' open item**: answering on Google Calendar with no guest who is you
  changed nothing and said "was told". `mark_mine` reports it, and the answer says why not.

**Round I22 (v0.28.75): unread mail a further page brings in says it went on top.**

- **The inbox audits' open item** (#8): a next page's unread went above the reader, unsaid, by
  the "unread on top" rule. The rule stands; the foot says `N more unread on top`.

**Round I23 (v0.28.76): a sign-in Google no longer honours reads as that.**

- **The inbox audits' open item**: `invalid_grant` on a Calendar call showed raw; it reads as a
  turned-down sign-in now (`states::known`).

**Round A14 (v0.28.77): when a run fails, a second model reads it and advises.**

- **The advisor parked last night** (`feat/a5-advisor`, no gain then) was ported onto main in a
  worktree lane (`feat/a14-advisor`): on a failed `sys:run`, at most twice a turn, a second
  model's three lines join the next prompt; read before the stop check, never after `/stop`.
- **Live, the compile-error task** (qwen-max engineer, qwen3.8-max advisor): with it 1 of 2
  solved in 27 s (the advice named the uncommitted `f64` change; the agent reverted it), without
  it 0 of 2, and 0 of 9 earlier tonight. Small numbers, and the first solve of the night.

**Round I24 (v0.28.78): retyping a todo's date with `e` changes the date, not the title.**

- **The third inbox/todo audit** (finding 2) caught I14's edge: the due before every title put
  it where Backspace from the end no longer reached it. It is last again, before the title only
  when the title ends on a lead-in word.

**Round I25 (v0.28.79): "due by May 15" in an October mail is next May.**

- **The third inbox/todo audit** (finding 5) caught I4's edge: "nearest" flipped any date over
  half a year ahead to last year's. Only an overdue notice ("was due") or a date weeks back
  reads as passed now.

**Round I26 (v0.28.80): "send deck fri by 5pm" is due Friday at five.**

- **The third inbox/todo audit** (finding 1) caught I11's edge: only `at` could sit between a
  date and its time, so `fri by 5pm` was due today and `fri` stayed in the title. Any one
  lead-in word goes with the date now, and `in 2 days at 5pm` fits the date reader's window.

**Round A15 (v0.28.81): a missed edit says where `old` parts, or that `new` is already there.**

- **AI survey 2** (#2, from Aider's failure report): misses were 14 of the a14 probes' 60 rounds,
  most of them answered only "the rest does not match". New `editmiss.rs` names the line `old`
  parts at, the places a glued `old` came from, or that `new` is already in the file.
- **Live**: the compile task ×3 on this build solved on its first edit each time and never
  missed, so the new text never showed. Unit tests replay the captured shapes.

**Round I27 (v0.28.82): an invite answered by mail is not called on Google Calendar.**

- **The third inbox/todo audit** (finding 4): an answer to an invite Google does not have went by
  mail, yet the card said "on your Google Calendar" and `c` refused to add it. Only an event
  Google has takes Google's answer now.

**Round I28 (v0.28.83): "by EOD Friday" in a mail is due Friday at five.**

- **The third inbox/todo audit** (finding 3): an end-of-day word made the mail's own day the
  due, whatever day followed it, and "next Friday at 5pm" lost its time to an older branch the
  grammar now covers.

**Round I29 (v0.28.84): "By the way, Friday works for me" is not a deadline.**

- **The third inbox/todo audit** (finding 6): a date anywhere in the four words after a key
  counted. It must start right after it now; "by noon Friday" goes blank (noon is not a time the
  grammar reads), said in the changelog.

**Round I30 (v0.28.85): a todo filed with a passed due says it has passed.**

- **The third inbox/todo audit** (also noted): the add note read "(due Fri Oct 23)" for a
  deadline already late. It says ", passed" and the year when not this one; `handoff`'s doc line
  is back where I19 moved it from.

**Round A16 (v0.28.86): a budget stop runs its last request, and a revision starts from the tools' record.**

- **AI survey 2** (#1, from Claude Code `/goal` and Codex goals): 22 of 22 budget stops asked
  for one more batch, which was dropped, and the judge then asked for the run it never saw. The
  batch runs now, unread, and the revision turn is given the last turn's tool lines.
- **Live** (compile task, advisor off, ×3): all 6 budget stops ran their last batch, and the
  judge's gap named the test result; the old build dropped it in both of its runs. Unsolved in
  both builds.

**Round I31 (v0.28.87): in a long smith session, Tab's draft still arrives.**

- **The second inbox audit's open item** (#5): a fold in smith's 500-message pane put the answer
  before the place the inbox looked from, and Tab waited ten minutes for "held no draft". The
  place now counts folded messages.

**Round U18 (v0.28.88): a value picker's ghost text is the highlighted row.**

- **The fourth UI survey** (regressions: U13–U17 all hold; finding 1): arrowed to `light`, the
  bar read `/theme dark`. It ghosts the row Tab fills now (shot: paper-light and glass-clear).

**Round U19 (v0.28.89): a path cut to fit keeps its file name.**

- **The fourth UI survey** (finding 2): text alone could not tell a path with a space, a folder
  row or a root file from words. The pickers mark their path rows now (`MenuItem::path`).

**Round A17 (v0.28.90): an edit off only in its indentation lands, matched to the file's.**

- **AI survey 2** (#4, from Codex `apply_patch` and Aider): after an exact miss, an `old` of two
  or more lines matching one place with one uniform leading-whitespace shift lands there, `new`
  shifted the same; the report says so. Replay: 14 of 14 indentation-only captures land, 16 of
  16 reflowed stay refused. Live: one two-block batch landed in one call.

**Round A18 (v0.28.91): crew's own tools are described whole on the text path.**

- **AI survey 2** (runner-up #8, Claude Code sends whole descriptions): clipped at 100, sys:edit
  lost `edits`, `all` and `line`. Live on the two-copies task, the first edit carried `line` in
  2 of 3 runs, against 0 of 3 before; the third put it beside `edits` and was refused once.

**Round A19 (v0.28.92): `line` beside a one-edit list applies to that edit.**

- **A18's third live run** sent `line` beside `edits` and was refused as if it had not. A list of
  one takes it now; a longer list is refused saying where it goes. Unit tests only.

**Round A20 (v0.28.93): a cut build quotes each error's line of code.**

- **AI survey 2** (#3, from Aider and SWE-agent): under the line of places, each rustc error
  comes back as its code row and rustc's word on it, grouped by file (all 25 of the test build
  in ~2.9 KB). Live, advisor off: 1 of 3 solved in 23 s by fixing the source function, against
  0 of 6 earlier tonight; the other two were no better on lookups or misses.

**Round I32 (v0.28.94): "by Friday, October 30" is October 30 again.**

- **The fourth inbox/todo audit** (I24–I31: seven hold, I29 broken): I29's "date right after
  its key" read "by Friday, October 30" a week early and "Due date: Oct 30" as nothing. A
  weekday may lead its date and "is"/"date" may follow the key now; `end_of` moved to
  `todoeod.rs` to keep the cap.

**Round I33 (v0.28.95): "by the end of the day" and "EOD Oct 30" read whole.**

- **The fourth inbox/todo audit** (finding 2): "the" was dropped after the words were counted,
  and only one word after "EOD" was read. Both are read whole now.

**Round I34 (v0.28.96): "due Friday by 5pm" is Friday at five.**

- **The fourth inbox/todo audit** (finding 1, inbox3's open sibling): a day just before "by"
  goes with the time after it now; a full stop between them still separates them.

**Round I35 (v0.28.97): an answer by mail outlives the add to Google Calendar.**

- **The fourth inbox/todo audit** (finding 3): Google's "needs action" on the added copy
  overrode the answer you had mailed. It yields to yours now.

## Close (07:55)

- **74 releases:** 19 UI (U1–U19), 20 AI (A1–A20), 35 inbox/todo (I1–I35). Four surveys ran
  behind them: three UI shot surveys (the last found U13–U17 all holding) and four inbox/todo
  audits (the last found I29 broken, fixed as I32).
- **Regressions of tonight's own making, all fixed the same night:** U3/U7 chip slabs (U15), U4
  ring on CRT (U8), U6 hard tabs (U14), U12 find rows (U13), I4 last-May (I25), I14 backspace
  (I24), I29 weekday-led dates (I32). The lesson held: a regression survey after every batch.
- **The AI lane's live wins:** sys:edit `line` (A1, 3/3), the advisor (A14, the night's first
  solve), the budget batch runs (A16, 6/6 stops), indentation-only misses land (A17, 14/14
  replayed), whole tool descriptions (A18, `line` on the first edit 2/3 vs 0/3), the quoted
  build digest (A20, 1/3 solved with the advisor off vs 0/6).
- **Left open, for the owner:**
  - nebula's bloom (ui.md #3), and the four glass palettes' look-alike strips (ui4 #3: their
    smokes lifted to the page do not separate);
  - "book table for friday by 3pm" filing "book table for" (the old "for <day>" rule, now past
    `at`), "2 PM candidates" read as a time, and a group invite's answer;
  - `feat/a5-advisor`, superseded by A14, for the user to delete.
- **Left open, small:** I24's title ending on a lead-in opens with the due in front; I25's
  "was" look-back crosses a sentence; "by noon Friday" sets no due (noon is not in the
  grammar); the popup shot harness (ui.md #8) and `bar_popup_shot`'s sel mismatch; AI survey 2
  #5 (a line of intent per round, risky) and runners-up #6, #7, #9, #10.

