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
