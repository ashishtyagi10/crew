# Goal — todo pane: full keyboard navigation (list traversal + a composer cursor)

**Status: SET 2026-08-10** by the user: the todos should be fully navigable from the
keyboard.

**SHIPPED v0.16.4–v0.16.6** (night loop, 2026-08-11): `PageUp`/`PageDown` page the
selection by real visible height and `Home`/`End`/`Ctrl+A`/`Ctrl+E` jump to first/last
(`todopane/keys.rs`), and the composer grew a real cursor with word motion — both respecting an
active `@project` filter.

**What exists today** (shipped 0.15.0–0.16.1, `crew-app/src/todopane/keys.rs`): Up/Down
step the selection one item, Tab/Esc hop between composer and list, Space/Enter toggle
done, `d`/Backspace/forward-Delete delete, `e` edits in place — and that is the whole
vocabulary. There is no paging or first/last jump, and the composer is append/pop only:
no cursor, so a typo at the start of a long (now multiline, v0.16.1) draft means
backspacing everything after it. This goal closes both gaps.

## Proposed decisions (defaults taken while drafting — veto before work starts)

- **Printable keys keep jumping to the composer.** Today any printable typed while the
  list is focused (except `d`/`e`) refocuses the composer and types — the pane is
  capture-first. That reflex stays, which rules out vim `j`/`k`/`g`/`G` in v1 (stretch
  behind a toggle, maybe). Navigation uses named keys only.
- **`Shift+PageUp/Down/Home/End` stay app-level.** They are the global pane-scroll
  chords (`keys.rs:26-41`); the todo pane takes the *unmodified* keys, like `/far` does.
- **Cmd-chords are off the table.** App routing only forwards non-super keys to panes
  (`keys.rs:162`), so line-start/line-end travel uses `Ctrl+A`/`Ctrl+E` (the terminal
  idiom — crew is a terminal) plus bare `Home`/`End` when the composer has text.
- **The composer cursor lands here first, not in chat.** The chat composer is also
  cursor-less (`chatkeys.rs:125`). The todo composer is the smaller, purer seam to
  prove the pattern; graduating it into `chatinput.rs` is stretch, not scope.

## What already exists (assembly, not invention)

- **List-nav shape: `farpane/keys.rs:94-97`.** `move_sel(p, ±PAGE)` for PageUp/Down,
  `set_sel(0 / usize::MAX)` for Home/End, typing-vs-list arbitration per key. The todo
  version must be *row-sum aware*: items are variable-height (wrapped titles,
  `render::item_h`), so a "page" is the items spanning `list_height` rows, not a count.
- **Selection visibility is already solved.** `ensure_visible`/`clamp_scroll`
  (`todopane/mod.rs`) do row-sum window math for any selection move — new nav keys just
  call them, as arrows do today.
- **Modifier plumbing is one signature.** `TodoPane::on_key(event, cols, rows)` gets no
  mods (`keys.rs:195-197`); chat receives `shift`+`ctrl`, far receives `alt` at the same
  call site. Thread `ctrl` and `alt` through the same way (`Alt+Left/Right` = word jump;
  note macOS Alt produces option-glyphs in `Key::Character`, so word-jump must key off
  the named arrow + alt flag, not the character).
- **Cursor↔row/col mapping is free.** v0.16.1's `wrap_ranges` (`todopane/render.rs`)
  already yields the composer's wrapped line ranges as char indices; a cursor is an index
  into the same `Vec<char>`, so its (line, column) falls out of the ranges the renderer
  already computes. The date/tag tint spans are recomputed per frame from char indices
  and survive mid-string edits untouched.
- **The `▏` cursor cell and tail-follow logic** (`composer_cells`) become cursor-follow:
  same cells, the anchor changes from "end of text" to "the cursor's line".

## The contract (definition of done)

1. **The list is fully traversable.** PageUp/PageDown move the selection by one visible
   page of items (row-sum against `list_height`, minimum one item); Home/End jump to the
   first/last visible item. All four keep the selection on-screen via `ensure_visible`,
   respect the active `@project` filter, and do nothing surprising on an empty list.
   Unmodified keys only — the Shift chords still scroll panes app-wide.
2. **The composer gains a real cursor.** A `cursor: usize` char index on `TodoPane`:
   Left/Right move by char, `Alt+Left/Right` by word, `Ctrl+A`/`Home` and `Ctrl+E`/`End`
   to the draft's ends; typing inserts *at* the cursor, Backspace deletes before it,
   forward-Delete deletes at it (today forward-Delete is ignored in the composer — no
   binding is displaced). Submit/Esc/tag-accept reset it exactly where `reset_input`
   lives today. Paste inserts at the cursor. Every mutation goes through one seam so
   `sync_menu` (tag popup) keeps firing.
3. **The cursor is wrap-aware and visible.** `▏` renders at the cursor, not the text
   end; on a multiline draft Up/Down move the cursor one wrapped line (nearest column),
   and the capped card tail-follow becomes cursor-follow — the cursor's line is always
   one of the visible interior rows. Up on the first line exits to the list (bottom
   item, nearest the composer — today's spatial reflex); Down on the last line enters
   the list at the top. Tint spans (due fragment, `@tags`) render correctly around a
   mid-string cursor.
4. **The keys are discoverable.** The todo bindings join the help overlay's table
   (`help.rs`) under a todo section — one line per zone (list / composer), no new UI.

## Stretch (ranked, separate iterations)

1. **A way back for done items** — they auto-hide with *no UI path back* (0.15.1);
   a show-done toggle key would make them reachable and un-done-able again.
2. **Due bump on selection** — `+`/`-` postpone/advance the selected item's due a day.
3. **Project-filter cycling** — `[`/`]` step through known tags without typing `@tag`.
4. **Graduate the cursor to `chatinput.rs`** — the chat composer wants the same seam.
5. **Vim keys behind a toggle** — only if the capture-first reflex can be preserved.

## Verification

- Unit, with RED transcripts before green (standing rule — assert concrete values, not
  shapes): page math over variable-height items (wrapped titles at narrow widths, filter
  on); cursor table tests (insert/delete/word-jump/line-ends over multibyte + wide
  chars); wrap-line Up/Down column memory at the cap boundary; the composer↔list
  handoffs at both edges. Mutation spot-check the cursor arithmetic — off-by-ones there
  are exactly the mutants that survive vacuous tests.
- Live (`.claude/skills/verify` harness, isolated HOME — still blocked by macOS perms,
  same checklist debt as the todo-pane goal): type a 3-line draft, arrow to line one,
  fix a typo mid-word, watch `▏` and the due tint; PageDown through 30 items; Home/End.
- Idle invariant: a resting todo pane with a mid-text cursor requests no frames
  (`wants_animation_frame` gains no new term — the 0.8.0 rule).
