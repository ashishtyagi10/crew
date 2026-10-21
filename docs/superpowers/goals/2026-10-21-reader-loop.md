# Loop — the mail reader, five rounds

**Status: DONE (v0.27.51–55).** Set 2026-10-21 by the user: *"write a loop and execute it 5 time to improve
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

**Round 2 (v0.27.52): type and the shape of a thread.**

- **The survey.** On a wide pane a mail's lines ran the width of the screen, 140 columns and
  more. A thread of three mails or more showed every one in full, the newest at the bottom of
  a long scroll.
- **A reading measure.** The reader sets no line wider than 88 columns
  (`reader::MEASURE`), about the 600 px a mail is designed for. The header, pictures and
  `‹ 3 of 25 ›` follow the measure.
- **Older mails fold.** In a thread of three or more, every mail but the newest folds to one
  line (`crates/crew-app/src/inbox/readfold.rs`): the sender in bold, how the mail starts in
  plain words (no markdown marks, table bars or picture marks), and its date at the right.
  The names share one column, so the words line up. A click opens a folded mail, and the
  pointer is a hand over it. `x` opens every mail, or folds the older ones again; it joins
  the foot's view group and `/keys`.
- **Tests:** a thread of three folds two and opens on a click and on `x`; a folded mail's
  words are plain; the measure caps the text.

**Round 3 (v0.27.53): pictures as pictures.**

- **The survey.** Every picture was a mosaic of coloured quads, about 2 px each and point
  sampled, so a photo was blocky on a Retina screen and the edge of anything with
  transparency was ragged. A picture still loading was the words `loading picture…` in an
  empty band.
- **A texture per picture.** crew-render draws pictures (`crew_render::Picture`,
  `Pixels`) as textures (`crates/crew-render/src/images.rs`, `images.wgsl`), one textured
  rectangle per picture, linearly filtered at the display's resolution, over the cells'
  backgrounds and under their text. Pixels are uploaded once per picture (named by an id of
  their own) and kept while drawn; textures unused for 600 frames are let go. Clipping is
  done on the CPU, texture coordinates and all (`picture::place`). Pixels are premultiplied,
  so a transparent edge does not pull in a dark fringe. The open mail's pictures go this way
  (`InboxPane::pictures`, `panepics`, the mail window, and the shot harness's `shot_pics`);
  the viewer, chat and the terminal keep the mosaic for now.
- **Sharper.** A decoded picture keeps up to 1024 samples a side, up from 640, so a hero
  across the measure stays sharp on a Retina screen.
- **Loading.** A picture still coming is a quiet box of its own size under its words.
- **Tests:** a headless GPU test draws a four-colour picture and reads it back: each
  quarter its own colour, nothing outside its box, nothing past its clip. Placement and clip
  maths are unit-tested.

**Round 4 (v0.27.54): a mail's buttons, and links you can click.**

- **The survey.** A mail's buttons ("Shop now", an invite's Yes / Maybe / No) were drawn as
  inline code is, a grey chip with link-blue words: quieter than the call to action the mail
  meant, and no different from code. Every link and button in the reader needed Cmd+click,
  which no mail reader asks for.
- **Buttons are filled.** A link around a padded code span, the converter's button, is
  drawn as a capsule filled with the link's colour, its words in the page colour or the ink,
  whichever reads hardest on it (`crates/crew-app/src/chatbutton.rs`): white on blue on the
  light page, dark on light blue on glass.
- **A plain click opens a link or a button** in the open mail, as in Gmail (a file's row saves
  and opens it). The pointer is a hand over every one. Cmd+click still works.
- **Tests:** a padded code link is a filled button whose words read on it, and plain inline
  code is not; a plain click on a link and on a button opens each one's address.

**Round 5 (v0.27.55): reply where you read.** The plan's fifth round was polish. The user
asked mid-round: *"'r' should open a reply box rather than agent typing the response in the
smith pane, we should have reply functionality within email detail panel"*, so this round
is the reply box.

- **`r` opens a reply box** under the open mail (`crates/crew-app/src/inbox/compose.rs`,
  drawn by `composedraw.rs`).
  - It is a crew card with `reply to Ann Lee` on its edge, addressed as the old smith draft
    was: To the last other sender (their `Reply-To`), `Re:` the subject, threaded with
    `In-Reply-To` and `References`.
  - The mail above gives it the rows it needs, three of text at least and up to half the
    pane (`reader_rows`, read by drawing, pictures, links and scrolling).
  - While it is open every key is the box's: letters, Enter for a new line, the arrows,
    Home, End, Backspace, Delete, and a paste. Off a Mac the Ctrl keys a typed line keeps
    stay with it.
- **Sending.** Cmd+Enter (Ctrl+Enter off a Mac, or a click on the `⌘↩ send` keycap) sends it
  through Gmail's `messages.send` in the thread (`gmail.modify` covers it). The box shows
  `sending to …` and stays until Gmail says the mail went, so a failed send keeps every word.
  Then the thread opens again with the reply in it.
- **Closing.** Esc closes the box: at once when it is empty, and after a second Esc when it
  holds words.
- **agent smith, inside the box.** Tab asks smith to write a draft, which lands in the box for
  you to change before sending. Asked for with no box open, as before, its draft becomes a
  Gmail draft.
- **The foot** shows only what works while you write: send, close, and smith.
- **Tests:** `r` opens a box whose keys are its own (`d` is a letter there); Esc twice lets the
  words go; an empty reply is not sent; a reply is sent in its thread, kept through a failure,
  and closed once Gmail says it went; Tab's draft and a paste land at the caret.

## The loop, closed

Five releases, v0.27.51 to v0.27.55:

- **Navigation:** keycaps you can click, `‹ 3 of 25 ›`, `n` and `p`, folded threads.
- **Type:** a reading measure.
- **Pictures:** real textures.
- **Actions:** filled buttons, links that open on a plain click, and a reply box.

Left for a later round:

- The viewer, chat and terminal pictures could take the texture path too.
- No undo for a sent mail (Gmail's API has no unsend).

**After the loop (v0.27.56), the user: *"add cc and forward to the reply box"*.** The box has
a `to` and a `cc` line (`crates/crew-app/src/inbox/composehead.rs`, `field.rs`, `addr.rs`):
Tab or Enter goes on to the next, the arrows move between them and the words, a click puts
the caret on the line clicked, and a bad address is refused with the caret on it. `R`
replies to everyone the mail went to. `f` forwards the newest mail
(`crates/crew-app/src/inbox/forward.rs`, `forwardmail.rs`): your note, Gmail's
`---------- Forwarded message ---------` block, then the mail's own HTML with the pictures
it draws by `cid:` and its plain words, and every file it carried (`mime.rs`), sent
through Gmail's upload address (`upload.rs`); past Gmail's 25 MB it says so and sends
nothing. Every body now goes base64, so a long paragraph never breaks the 998-byte line
limit.

