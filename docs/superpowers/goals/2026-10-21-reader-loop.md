# Loop — the mail reader, five rounds

**Status: RUNNING.** Set 2026-10-21 by the user: *"write a loop and execute it 5 time to improve
the UI of email reading pane / detail pane - improve everything, how image render, how font
render and how action button appear, navigation should be simple and improved"*.

## One round

1. **Shoot.** The reader as `inboxreadshot_tests` (a thread of three mails, pictures),
   `inboxmailshot_tests` (the corpus of real-world mail) and `inboxsplitshot_tests` (a wide
   pane) draw it, on the Ink Reader light page and on glass.
2. **Survey** the shots through four lenses:
   - **Pictures:** sharpness, size, placement, and the boxes of pictures still loading or
     failed.
   - **Type:** the measure (line length), spacing, headings, the message header, and how a
     long thread reads.
   - **Actions:** the reader's own actions (reply, archive, delete, next) and a mail's buttons.
   - **Navigation:** knowing where you are (in the list, in the mail), and the fewest steps
     to the next thing.
3. **Pick** the two to four changes that a reader would notice most.
4. **Build** them with tests, and shoot again to see them.
5. **Release** (`scripts/release.sh`) and log the round below: what the survey found, what
   changed, and the version.

Rules that hold every round:

- Every surface is judged on both pages.
- Every file stays under the 200-line cap.
- The foot keeps every key visible, as the user asked on 2026-10-21.
- Nothing fetched that was not fetched before.

## The plan, from the first survey

1. **Navigation and actions.** The foot was 17 keys in three equal lines. Each key becomes a
   chip you can click, in groups (move, act, view), with the most used first. The reader's
   header says which thread of the list this is (`3 of 25`), with `‹` and `›` you can click.
2. **Type.** Lines are capped at a reading measure on wide panes. A long thread shows its
   older mails folded, as Gmail does, and each message header gets air and a quieter date.
3. **Pictures.** Pictures are drawn as textures at their own resolution with smooth scaling,
   not as a mosaic of coloured cells. A picture still loading shows a quiet box.
4. **A mail's buttons.** They look like buttons: a filled capsule, its words on it, and the
   hand over it.
5. **Polish.** A fresh survey of everything, including the wide split view and glass.

## Log

**Round 1 (v0.27.51): navigation and actions.**

- **The survey.** The foot was 17 keys in three lines of one weight, as loud as the mail
  above it, and none of them could be clicked. Nothing in an open thread said where it was in
  the list.
- **The foot as keycaps.** Each key is drawn on a keycap (the capsule inline code wears), what
  it does in muted ink beside it, in groups with a wider gap between them: move (`esc`, `n`,
  `p`), act (`r`, `e`, `d`, `z`, `s`, `m`), view (`q`, `i`, `v`, `w`) and more (`a`, `t`,
  `u`, `o`). The list has groups of its own. Every keycap is a button: a click does the key,
  `esc` included, and the pointer is a hand over it. The tables are in
  `crates/crew-app/src/inbox/footkeys.rs`; drawing and clicks read one layout (`foot::placed`).
- **Where you are.** The subject's row ends with `‹ 3 of 25 ›` (`+` when Gmail has more), and
  the arrows open the previous and the next thread, as `p` and `n` do
  (`crates/crew-app/src/inbox/readpos.rs`). The subject wraps short of it.
- **Tests:** a keycap's click is its key (`n` opens the next thread, `esc` goes back); the
  counter's arrows step through the list; every key still shows in order.

