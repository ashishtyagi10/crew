# Goal — the inbox list, redesigned: how `/inbox` looks before a thread is opened

**Status: IN PROGRESS.** Set 2026-10-20; the user answered its questions on 2026-10-21 and said
go ahead. Phases 0 and 1 shipped in v0.27.37, phase 2 in v0.27.38, phase 3 after it, pared
back to a quiet list at the user's word (see "Where it stands" at the end). Phases 4 and 5 are
not started.

**Set:** 2026-10-20 by the user, while the mail reader goal was being built: *"write a goal to
update the look and feel of listing mode as well for `/inbox`"*.

## What the list looks like today

One row per thread, drawn by `row` in `crates/crew-app/src/inbox/draw.rs` from what
`crates/crew-app/src/inbox/wire.rs` keeps (sender, subject, snippet, unread, starred, time,
message count). Seen in the `inboxshot_tests` render on both themes:

- **The selected row barely shows.** Only a `›` in the first column marks it; the row itself
  has no highlight.
- **The markers collide.** `●` (unread) and `★` (starred) share a two-column gutter, so a
  starred unread thread reads `●★` crammed against the sender.
- **The sender column wastes space.** It is a fixed width, so a short name like "GitHub"
  leaves a wide empty gap before the subject.
- **One line holds too much.** Subject and snippet share it and are cut with `…`, often in the
  middle of the subject on a narrow pane.
- **Read and unread differ only in weight.** An unread thread is bold; a read one is the same
  ink, not bold. The inbox has no rhythm: no gaps or headers by day.
- **The row says little.** Nothing shows attachments, importance, labels, a draft, who else is
  in the thread, or that you replied last.
- **The foot is cut off.** The key hint ends in `…` on every normal width.
- **Empty and waiting states are bare.** They are one muted word (`loading…`,
  `nothing in the inbox`) in a large blank pane.

## What crew's other lists already do (so this matches them)

- `/todo` groups rows under day headers (`crates/crew-app/src/todopane/group.rs`, drawn by
  `crates/crew-app/src/todopane/headrow.rs`, which also owns the empty-list hint).
- Its row chips drop by priority as the tile narrows
  (`crates/crew-app/src/todopane/rowchips.rs`).
- Chips are capsules through `CellView::mark`, as inline code is in chat.
- The theme has a selection colour (`selection_bg`) that every theme is checked against.
- The user wants **bright white text on glass**, legible, and not argued about. Every colour
  here comes from theme tokens, and every surface is shot on the light Ink page and on glass.

## Done means

**Phase 0: check what the metadata already gives.**

1. Confirm from Gmail's reference and one live list which fields `threads.get?format=metadata`
   returns for **every** message: `labelIds` (`IMPORTANT`, `DRAFT`, `SENT`, `CATEGORY_*`, user
   labels), the `From` header of each, and whether `payload.mimeType` is there (it would say
   `multipart/mixed`, meaning an attachment). Anything that needs a second call per thread is
   dropped from this goal. The only new call allowed is one `labels.list` per pane open (label
   names and colours), and it is cached.

**Phase 1: the row, rebuilt.**

2. **Two-line rows when there is room.**
   - Line 1: the senders, the message count as a quiet chip, and the time right-aligned.
   - Line 2: the subject, then the snippet in muted ink.
   - The list falls back to one-line rows when fewer than 8 two-line rows fit. `L` switches
     between the two, and the choice is remembered for the session.
3. **The selection is unmistakable.** The selected row gets a full-width wash in
   `selection_bg` and an accent bar at its left edge (`▍`), on both lines of a two-line row.
   The `›` goes. A mouse hover gets a lighter wash.
4. **Markers that never collide.**
   - `●` unread stays in the gutter, in accent.
   - `★` moves to the right edge beside the time, where an attachment glyph also goes.
   - Each marker has its own cell, and is dropped before it is cramped.
5. **Read mail recedes.** Unread threads are bright ink and bold; read threads are muted ink.
   On glass, unread is bright white. The snippet is always muted.
6. **Senders the way Gmail writes them.** "Ann, Bo, me (4)" from the `From` of every message,
   newest last; `me` for the signed-in account. Sized to the names on screen (capped at about
   24 columns), never a fixed gap.
7. **A width ladder.** As the pane narrows, the snippet goes first, then the count chip, then
   participants shorten to the newest sender, then the time goes compact. Asserted as cell tests
   at 40, 60, 80 and 120 columns.

**Phase 2: the list's shape.**

8. **Day headers** in muted ink between rows: Today, Yesterday, the weekday names this week,
   Last week, then month names, then years. Drawn the way `/todo`'s done history bands its
   days, for the inbox and for a search.
9. **Empty, loading and error states that look finished.**
   - While a page loads: skeleton rows (muted bars on the paint layer, the shape of real rows).
   - An empty inbox: a drawn mark with "Inbox zero" and when it was last checked.
   - A failure: what went wrong and the key that fixes it, never a raw error string.
10. **A foot that never ends in `…`.** It shows the keys that fit, most-used first. `?` opens
    the full key card as a pop-up, the way crew's `/help` card opens.

**Phase 3: what a row can say (no new scopes, at most one new call).**

11. **Attachment** glyph by the time, if phase 0 finds `payload.mimeType` in the metadata. The
    glyph is chosen by a real charmap check, as crew picks every icon.
12. **Labels as chips.** Gmail's Important marker; user labels as small capsules in their own
    Gmail colours; a `Draft` chip when the thread holds one. All dropped first by the width
    ladder.
13. **You replied last:** a quiet `↩` when the newest message is the user's own (`SENT`).
14. **An initial chip** before the sender in two-line mode: the sender's initial on a colour
    derived from their address, as Gmail does. Nothing is fetched (no avatars from the web).

**Phase 4: mouse and motion, crew's way.**

15. **Mouse.** Clicking a row selects it; clicking the selected row opens it. Clicking the `★`
    toggles it. Hover gets the wash from item 3, and the wheel already scrolls.
16. **Motion.**
    - An archived or deleted row folds away instead of vanishing.
    - New mail arriving at the top gets a brief accent pulse.
    - The selection wash glides to the next row.
    - All of it follows crew's motion setting: off means instant.

**Phase 5: the list and the thread side by side (the user's answer, 2026-10-21).**

17. **A reading pane on a wide pane.** At 140 columns or more, an open thread reads on the right
    and the list stays on the left (two fifths of the pane, 48 to 64 columns), with a rule
    between them. Narrower, the thread takes the pane as it does today.
18. **Every way into the reader still works in the half it has.** Scrolling, the picture boxes,
    `Cmd+click` and `Cmd+E` on links, and the attachment rows all measure the right-hand side,
    not the whole pane. `J` and `K` open the next and the previous thread without going back to
    the list, on any width.

**Not in this goal:** category tabs (Primary, Promotions, Social); selecting several threads to file at once; sorting
other than newest first; any new Gmail scope.

## Light weight (measurable)

- No extra Gmail call per refresh; at most one `labels.list` per pane open.
- The row layout is one function of (thread, width, density, selected), tested by its cells at
  four widths. Shots on the light Ink page and on glass, in both densities, joining the existing
  `crates/crew-app/src/inboxshot_tests.rs`.
- Every file under the 200-line cap; `crates/crew-app/src/inbox/draw.rs` splits along row,
  bands and states as it grows.

## Answered by the user (2026-10-21)

*"two-line rows, side-by-side on wide panes, no tabs — go ahead"*

- **Density:** two-line rows when there is room (item 2, as proposed).
- **Wide panes:** yes, the list and the open thread side by side (phase 5).
- **Category tabs:** no.

## Where it stands

**Phase 0, checked 2026-10-21 against Gmail's reference (no live list was read):**
`format=metadata` returns "only email message ID, labels, and email headers". So `labelIds` and
the `From` of every message come with the list as it is fetched today. The reference does not
list `payload.mimeType` among them, so the attachment glyph (item 11) reads the `Content-Type`
header instead, asked for beside `From`, `Subject` and `Date` in the same call: `multipart/mixed`
means a file is attached (a guess Gmail's own clip is surer of). `labels.list` gives each label's
`id`, `name` and `type` but not its colour, which takes a `labels.get` per label. So item 12's
chips take crew's own tag colours (`crew_theme::tag_color`), not Gmail's.

**Phase 1, shipped in v0.27.37:** items 2 to 7. A row is laid out by `crates/crew-app/src/inbox/row.rs` into a
whole-width line buffer (`crates/crew-app/src/inbox/line.rs`), so a selected row carries the
wash (`selection_bg`) on every column and `bgruns` rounds and frosts it as it does every
highlight; the count chip is a `mark` capsule over it, and every colour on the wash is lifted
to the text floor against it on all eight themes. The accent bar is paint. Two-line rows are
divided by a faint rule above each thread's names line: at the row boundary itself the rule
read as an underline of the subject. Senders come from `crates/crew-app/src/inbox/who.rs`
(each person once, newest last, `me` by `SENT` or the account's address). `L` picks the density
for the session (`crates/crew-app/src/inbox/list.rs`). The ladder's widths: the snippet below 64
columns, the chip below 52, all but the newest sender below 44, a compact time below 36, two-line
rows climbing it 16 columns later. The `›` marker is gone.

**Phase 2, shipped in v0.27.38:** items 8 to 10. `crates/crew-app/src/inbox/layout.rs` lays the list out as
bands and threads, and drawing, paint and (next) clicks all read it; the list scrolls as little
as keeps the selected thread on screen, with its day's band over the first thread shown. The
bands (`crates/crew-app/src/inbox/days.rs`) are lowercase, as `/todo`'s done history writes its
days: `today`, `yesterday`, the weekday, `last week`, `earlier this month`, the month, the
year. A rule runs out from each band's name. `crates/crew-app/src/inbox/states.rs` draws the
empty list: outline rows on the paint layer while a page loads (static, so a slow list costs no
frames), a ring and tick in the accent over `inbox zero` and when Gmail was last asked (a check
that finds nothing new now says so: `Got::Checked`), the way back from a search that found
nothing, and a failed list as words and a key (`R tries again`; a refused sign-in names
`/inbox signout`). With rows already on screen a failure is still a note at the foot. The foot
(`crates/crew-app/src/inbox/foot.rs`) fits whole keys, most used first, and ends `? keys` when
it drops any; `?` opens `/keys` on a new section, `in the /inbox pane`
(`crates/crew-app/src/helpinbox.rs`), which the key-map parity test now reads against the
inbox's own key files.

**Phase 3, built and then pared back (2026-10-21).** Items 11 to 14 were built: label chips
(with one `labels.list` per pane open), `»` for Important, a paperclip, `draft`, `↩` for "you
wrote last", and an initial chip. The user looked at the list as shipped in v0.27.38 and said it
was *"still very dense. it should be minimal and looks good"*. So the release that carries
phase 3 is a quiet list, and it replaces items 3 and 12 to 14:

- **What a row keeps:** the unread dot in the gutter, the names, the message count as a plain
  muted number (no capsule), Gmail's red `draft`, and the time at the right with the star and
  the paperclip (item 11, still read from the `Content-Type` header asked for beside the other
  three) beside it.
- **What went:** label chips and the `labels.list` call, the Important mark, `↩`, the initial
  chip, the accent bar on the selection (the wash is enough), and every rule. The rules
  between threads and out from each day's name gave way to air: a row of it under the pane's
  top edge and over the foot, a blank row between two-line threads (none at crew's `compact`
  density), and a blank row before each new day. A day's name is a quiet word in muted ink
  (`crates/crew-app/src/inbox/layout.rs`).
- **Ink:** names and subject are bright on every row, so a glass theme reads white (the user's
  standing ask). Unread mail is the dot, bold names and a bright time. The snippet is muted
  and follows the subject after two spaces, with no dash.
- **Density:** two-line rows need six threads, each with its blank row, to fit.

**The foot, changed at the user's ask (2026-10-21):** *"all shortcut should be visible at the
bottom of email detail"*. The foot (`crates/crew-app/src/inbox/foot.rs`) now lists every key the
pane answers to, wrapped whole onto as many lines as they take (four at most). Each key is
bright and what it does is muted. A note such as `deleted "…" · z undoes` takes a line of its
own above the keys instead of replacing them. Only a pane too short to spare the lines ends
`? keys`. The list, the reader and the setup page draw in the rows above the foot
(`body_rows`).

**Keys, changed at the user's ask (2026-10-21):** *"use `z` for undo - not sure what quote
mean and use `u` for unsubscribe action in email detail panel"*.

- `z` undoes, in the list and in the reader.
- Folding quoted replies moved from `z` to `q`. Its fold reads `··· N lines quoted from
  earlier replies · q shows them`.
- `u` in the reader unsubscribes (`crates/crew-app/src/inbox/unsub.rs`), from the mail's
  `List-Unsubscribe`. Where the mail also sends `List-Unsubscribe-Post`, that is one RFC 8058
  POST to its https address; otherwise a mail goes to the list's address through
  `messages.send`, which `gmail.modify` covers (checked on the method's reference page,
  2026-10-21). Either is done only for mail whose `Authentication-Results` say `dkim=pass`,
  and never for an IP or a local host. Any other mail can only have its page opened in the
  browser. A second `u` confirms.
