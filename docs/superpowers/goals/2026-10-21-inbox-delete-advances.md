# Goal — delete in the reader opens the next thread

**Status: COMPLETE (v0.27.43).** Set 2026-10-21. The user said go ahead the same day,
adding *"archive too"*.

**Set:** 2026-10-21 by the user: *"write a goal that if I delete an email in the detail pane, it
should automatically open the next email detail instead of taking me back to list."*

## What happens today

`d` in an open thread (the reader, `crates/crew-app/src/inbox/reader.rs`) goes through
`InboxPane::triage` to `delete` in `crates/crew-app/src/inbox/triage.rs`. That sends the
thread to Trash and calls `take_row`, which removes the row from the list and sets
`self.reading = None`. So every delete drops you back on the list, and reading the next thread
takes `Enter` again. `e` (archive) goes through the same `take_row`, so it behaves the same way.

Gmail calls the fix *auto-advance*: after you delete a conversation, it opens the next one. Its
default direction is the next older conversation, which in a newest-first list is the row
below.

## Done means

1. **`d` in the reader opens the next thread.**
   - The thread goes to Trash as it does today, with the same `deleted "…" · u undoes` note at
     the foot.
   - The reader then opens the thread that moves up into the deleted row's place: the next
     older one.
   - It opens exactly as `Enter` opens a thread. Gmail is asked for it (`Ask::Open`), it is
     marked read as it opens (as Gmail's auto-advance does), the picture store is cleared, and
     the reader starts at the top.
   - The list's selection moves to that thread too, so `Esc` lands on it.
2. **The edges.**
   - Deleting the last thread in the loaded list opens the newer one above it.
   - Deleting the only thread goes back to the list, which shows its empty state (inbox zero,
     or "no mail matches" for a search).
   - In search results, the next thread is the next result, because the list holds the
     search.
   - The advance never asks Gmail for another page; the next page loads as it does today.
3. **No flash of the wrong mail.** While the next thread loads, the reader shows that
   thread's header from its row and a loading line, never the deleted thread's body. An
   `Opened` that arrives late for the deleted thread is dropped. `take.rs` already matches on
   the id; a test holds this.
4. **Undo comes back to it.** `u` straight after an advance puts the deleted thread back in its
   place and opens it again in the reader. The thread you were advanced to is not marked
   unread again.
5. **The `w` window follows.** The mail window draws the same pane
   (`crates/crew-app/src/inbox/mailwin.rs`), so a delete there advances the same way. A test
   presses `d` through the window's key path.
6. **One way to open a thread.** `Enter`, the advance and undo's reopen all call one helper
   (`open_at(index)`). Phase 5 of the list-look goal (item 18: `J` and `K` open the next and
   previous thread from the reader) uses the same helper.

**Light weight:** no new Gmail calls (the advance is the same `threads.get` that `Enter`
sends), no setting, and every file stays under the 200-line cap. `triage.rs` is near it, so the
helper goes in its own file.

**Tests** (in `crates/crew-app/src/inbox/triage_tests.rs`):

- `d` while reading row 2 of 5 leaves the reader open on what was row 3, now row 2, selected
  and marked read, with an `Open` ask sent for it;
- the same on the last row opens the row above it;
- on the only row, the reader closes on an empty list;
- a late `Opened` for the deleted id leaves the new reader empty;
- `u` reopens the deleted thread.

## Answered by the user (2026-10-21)

- **Should `e` (archive) advance the same way?** *"archive too"*. `take_row` itself advances,
  so both keys behave the same.

## Where it stands

**Shipped in v0.27.43.**

- `crates/crew-app/src/inbox/advance.rs` holds `open_at(i)`, the one way a thread opens.
  `Enter`, the advance and undo's reopen all call it, and it is ready for phase 5's `J` and `K`.
- `advance(at)` opens the row that slid into the gone thread's place. At the end of the list it
  opens the row above, and with no row left it stays on the list.
- `take_row` in `crates/crew-app/src/inbox/triage.rs` calls `advance(at)` whenever the reader
  was open. `Undo` moved to `crates/crew-app/src/inbox/undo.rs` and carries `reopen`, so `z`
  puts the thread back and opens it again.
- A late `Opened` for the deleted thread is dropped by the id check `take.rs` already had, and
  a test holds this.
- The `w` window sends its keys through the same `on_key` and `press` as the tile, so it
  advances too.
- Tests are in `crates/crew-app/src/inbox/advance_tests.rs`. They cover:
  - `d` in the middle of the list;
  - `e` at the end and on the only row;
  - the late answer;
  - `z` reopening the thread;
  - a delete from the list staying on the list.

**Not in this goal:** a setting for the direction (Gmail offers older, newer, or back to the
list); loading the next thread's body before you delete; advancing after `m` or `s`, which
leave the thread in the list.
