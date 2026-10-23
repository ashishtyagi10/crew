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
