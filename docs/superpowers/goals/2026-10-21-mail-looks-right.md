# Goal — mail that looks like mail: pictures and formatting on real-world email in `/inbox`

**Status: IN PROGRESS.** Set 2026-10-21; looping until done.

**Set:** 2026-10-21 by the user, after the mail reader goal was marked complete: *"bug: email
rendering still pre-historic like only text are appearing, no image or markdown. Can we improve
that via goal and keep looping until it is perfect."*

## Why the finished reader still reads as text

The mail reader goal (`docs/superpowers/goals/2026-10-20-mail-reader.md`, v0.27.34–36) was
checked against an 11-fixture corpus of mostly *semantic* HTML: real `<h1>`, `<b>`, `<ul>`,
`<blockquote>`. Real inbox mail is mostly something else. It is built with table layouts and
styled entirely with CSS, by Mailchimp, Klaviyo, Braze and SendGrid templates, receipts, and
GitHub and Google notifications. Read against that kind of mail (investigated 2026-10-21; the
user runs v0.27.37, so it is not an old build):

- **No pictures.** Every `http(s)` picture is held back until `i`
  (`crates/crew-app/src/inbox/html/image.rs:20-29`): it turns into ` ▣ alt ` words, or nothing
  when the alt is empty. Almost every picture in commercial mail is from the web. The only clue,
  `▣ N pictures from the web · i shows them`, comes after the whole body
  (`crates/crew-app/src/inbox/readmail.rs:63-70`), below the fold of any long mail. Gmail shows
  these pictures by default.
- **No formatting.** `styled` in `crates/crew-app/src/inbox/html/walk.rs:81-99` knows only
  `font-weight:bold` (or 700–900) and italics. A `<td style="font-size:28px">` headline reads as
  a plain paragraph, and a `font-weight:600` subhead as plain text.
- **Layout noise.** A logo inside a link comes out as `[▣ Acme](…)`. A two-column product grid
  repeats each caption, once as the picture's alt and once as its text. A height set in
  `style="height:300px"` is ignored. `data:` pictures are never drawn.

## Done means

**Phase 0: a corpus that looks like a real inbox, and a score.**

1. **Real-world mail as fixtures.** Add, under the mail reader's fixture directory, HTML from
   open-source email templates whose licences allow it (MIT or similar), each wrapped as Gmail
   would send it: `format=full` JSON, `multipart/alternative` with both parts, base64url. Cover
   at least these shapes:
   - a marketing newsletter (hero picture, CSS headline, buttons, a two-column grid, a footer);
   - a receipt (logo, a table of items, totals);
   - a transactional notice (a "reset your password" button);
   - a GitHub-style notification;
   - a digest (a list of articles, each with a thumbnail);
   - a calendar invite;
   - a personal mail sent from Gmail (`<div dir="ltr">`, a quoted reply).

   Each fixture names where it came from and under which licence.
2. **A written expectation per fixture, checked by a test.** What a reader must see:
   - which lines are headings;
   - which words are bold;
   - which pictures are drawn, at roughly what size;
   - which links are buttons;
   - that the table stays a table;
   - that no line is repeated;
   - that no `▣` or raw markup is left.

   The scorecard test fails on any miss, and it prints every fixture's misses in one go.
3. **Shots of every fixture** on the light Ink page and on glass, written by the existing
   off-screen shot harness (`crates/crew-app/src/inboxreadshot_tests.rs`), so each loop is
   judged by its pixels as well as by the scorecard.

**Phase 1: pictures shown.**

4. **Web pictures load by default**, as in Gmail. Tracking pixels are never fetched (declared
   2×2 or smaller, spacer files, and anything that decodes that small, as today). `i` hides a
   thread's web pictures and shows them again, and the foot says which state it is in. The
   privacy cost is said once, where the setting lives: a fetched picture tells its sender when
   the mail was opened and from which address, and Gmail's own proxy, which hides the address,
   is not reachable through the API.
5. **Pictures sized the way the mail sizes them.** Width and height come from the attributes,
   then from `style` (`width:`, `height:`, `max-width:`). A logo stays logo-sized and is not
   stretched to the measure, and a hero picture takes the measure. `data:` pictures decode in
   place, under the same `image::Limits`.
6. **A picture inside a link is a picture you can click.** It is drawn, not shown as
   `[▣ alt](…)`, and Cmd+click on it opens the link.
7. **No echoes.** An alt text that repeats the caption next to it is dropped.

**Phase 2: typography from CSS.**

8. **Headings by size.** A block whose `font-size` is 24px or more becomes `#`, 20px or more
   `##`, and 17px or more and bold `###`. The same goes for `<font size>` 5 and above. One line
   inside a header table is a heading, not a paragraph.
9. **Weight and emphasis.** `font-weight` of 600 or more, `semibold` and `<strong>` in any
   spelling are bold. `font-style:italic` and `<i>` are italic. `text-decoration:line-through`
   is struck through.
10. **Buttons are buttons.** A link styled as one (its own background, padding or border, alone
    in its cell) is drawn as a capsule (`CellView::mark`, as chat draws inline code) that opens
    on Cmd+click and shows up in `Cmd+E` hints.
11. **Quiet small print.** The footer (unsubscribe, the postal address, "you are receiving
    this because") is drawn in muted ink when it is marked by a small `font-size` (11px or
    less) or `color` close to grey.

**Phase 3: layout that reads in order.**

12. **Columns in reading order.** A row of cells is read left to right, top to bottom, with
    a blank line between cells that hold a block, not a run-on line. A grid of products is a
    list of products: picture, name, price, button.
13. **No layout leftovers.** Spacer cells and rows (empty, or holding only `&nbsp;` or a
    1×N picture), `<br>` runs and nested wrapper tables leave no blank lines beyond one, and
    no stray `|` or `---`.
14. **Rules where the mail draws them.** A block with `border-top` or `border-bottom` and no
    words, or an `<hr>`, is a rule.

**Phase 4: the reader around the mail.**

15. **The open thread reads like the list now looks.** The subject is a heading under a row of
    air. Each message opens with the sender's name in bright ink and its time in muted ink at
    the right, and messages are divided by air, not rules. The attachments are rows under
    their message, as today.

**The loop:** each round runs the scorecard and the shots and fixes the fixture that reads
worst. It ships as a release with its shots in the log at the end of this document. The goal
is done when:

- every fixture's expectation passes;
- the shots show nothing a reader would call broken (a picture cut off, words overlapping, an
  empty band, raw markup), on both themes;
- the user has opened their own inbox on the release and said so.

## Light weight

- The in-house HTML reader stays. No browser engine, no CSS cascade beyond inline `style` and
  the few `<style>` rules a mail puts on a tag or a class it uses.
- Pictures stay in memory with the open thread, as today, with the same caps: 10 s and 5 MB a
  picture, 20 a message, and `image::Limits`.
- Every file stays under the 200-line cap. `html/walk.rs` gets a typography module beside it
  instead of growing.

## Not in this goal

Background colours and colour bands from the mail (a terminal page keeps its own); web fonts;
animated GIFs beyond their first frame; rendering a mail as Gmail's web page would (the reader
stays crew's markdown look); a per-sender "always show pictures" list (web pictures are on by
default).

## Where it stands

One entry per round: what was fixed, what the corpus showed before and after, and the release.

**Round 1 (v0.27.40): the biggest losses.**

- **Fixtures.** Five MIT-licensed templates joined the corpus as fixtures, with their sources and
  notices in `crates/crew-app/src/inbox/html/corpus/SOURCES.md`:
  - Cerberus's hybrid newsletter (`marketing`);
  - leemunroe's notice (`notice`);
  - Postmark's receipt, password reset and comment notification (`pm_receipt`, `pm_reset`,
    `pm_comment`).

  `crates/crew-app/src/inboxmailshot_tests.rs` shoots them in the reader. Each picture is a
  stand-in of its own shape, at several depths, on both themes.
- **Whole sections were missing.** Before this round, the newsletter lost both of its column
  sections and its thumbnail rows. `hide.rs` read `font-size:0` as hidden and dropped the
  subtree, but that is the standard way to close the gaps between inline-block columns, which
  Cerberus and every MJML-built mail use. A size is now inherited, as in a browser
  (`crates/crew-app/src/inbox/html/font.rs`). Words directly under a zero size stay unseen, so
  an injection hidden there still never reaches agent smith, and columns that set their own
  size read. The newsletter went from 2 pictures and 9 paragraphs to 11 pictures, 3 headings
  and every paragraph.
- **Typography from CSS (items 8 and 9).** A short line set at 24px or more is `#`, at 20px
  or more `##`, and bold at 17px or more `###`, and `<font size>` counts too. A weight of 600
  or more is bold. A long block in a large type stays body text.
- **Web pictures load as a thread opens (item 4).** `i` hides them and shows them again, and
  when they are hidden a line under the sender says so.
- **Buttons (item 10).** A link with its own fill or border, or a link inside a table or cell
  classed `btn` or `button`, is a padded capsule that is the link. The lying-link check still
  runs on buttons.
- **Small fixes.** A link worded as a file name (`spec.pdf`) is no longer named as a lie.

Still to do (the shots show them):

- A wide hero picture with no height gets the default 12-row box, so it is drawn smaller
  than the measure.
- A heading on the light page is only bold.
- The footer's small print is not muted.
- The capsule's underline runs under its padding.
- The reader's own frame is not done yet (item 15).

**Round 2 (v0.27.41): pictures at their own size, and the reader's frame.**

- **Picture sizes (item 5).** The mail's height now counts when it is set in `style` as well
  as in the attribute, at 16 px a row (a cell is about 8 by 16 of a mail's px), so a 40 px
  logo takes 3 rows.
- **Pictures with no height.** These are written `#rows=12&fit=W`, where `W` is the width the
  mail asks for, or 0. When the picture lands, its real size goes to the open thread
  (`Pics::sizes`, `InboxPane::sync_sizes`), and `crates/crew-app/src/inbox/readfit.rs` gives
  the box the rows its shape needs. It is as wide as the mail asks, or the picture's own
  width, and never wider than the text. A hero now takes the measure and a logo stays a logo.
  The words under the picture move once, when it lands, as they do in Gmail.
- **The reader's frame (item 15).** It now has:
  - a row of air above the subject;
  - each message's sender bright and bold, their address muted when it fits, and the date at
    the right edge of the text;
  - a row of air under that line;
  - two blank rows between messages.
- **Capsules lose their underline.** A link drawn as a capsule (a mail's button) is no longer
  underlined, because the capsule already marks it (`crates/crew-app/src/chatspan.rs`). This
  holds in chat too.

Still to do:

- Muted small print in footers.
- A heading on the light page is only bold.
- Echoed captions when pictures are hidden (item 7).
- Columns read in order (item 12) holds in the corpus; it needs a product-grid fixture to
  prove it.

**Round 3 (v0.27.42): template-built mail, and what is around a mail's words.**

- **More fixtures.** Six Colorlib templates (MIT) joined the corpus: a product grid, a cart
  reminder, an order receipt, an editorial newsletter, a product launch and a dark newsletter.
  The grid is the fixture item 12 asked for, and its products read as picture, name, words and
  button, in order.
- **Classes in a stylesheet set type (item 8).** MJML writes a headline as
  `<span class="h1">` and puts its 38px in a `<style>` rule.
  `crates/crew-app/src/inbox/html/css.rs` reads plain class rules (`.h1`, `span.h1`, outside
  `@media`) for size, weight and style only; it never reads anything that hides. A block that
  holds a single span is a heading by the span's type. A heading must hold words: a price set
  large (`$120`) is bold, not a heading, and nothing inside a heading is wrapped in `**`.
- **Links that sit side by side** with only CSS between them (a menu, footer links, social
  marks, each in its own cell) read on one line, joined by `·`. A picture at icon size (32 px
  or less) is its alt word, so a row of social logos reads `Twitter · Facebook · Instagram`.
- **Tables with no header row.** A receipt's item rows used to promote the first item to a
  header. Now, with no `th` row, the header is empty, and the markdown renderer leaves an
  empty header out along with its rule (`crates/crew-app/src/md/table.rs`).
- **Small print (item 11).** A run of blocks set at 12px or less that ends the mail (and is
  no more than its second half) is its footer, drawn in muted ink: the address, why you got
  it, unsubscribe. This happens in five fixtures, and no body text is caught in it.
- **A first box from the width.** A picture with only a width gets a box guessed at a photo's
  shape until it lands, not the default 12 rows.

Still to do:

- A heading on the light page is still only bold.
- Echoed captions when pictures are hidden (item 7).
- `data:` pictures.
- Not tried yet: a calendar invite's buttons and a Gmail personal mail with a signature
  picture.

**Round 4 (v0.27.45): the user's own mail.** The user said *"go ahead and test against my own
mails"*. `crates/crew-app/src/inboxlive_tests.rs` has three `#[ignore]`d steps, each run by hand:

1. One saves recent threads as Gmail sends them (`format=full`) outside the repo. It uses the
   user's grant file, never prints it, marks nothing read and fetches no picture.
2. One reads them as the reader does.
3. One draws them with stand-in pictures.

Forty threads were read: inbox, promotions and updates, in English and Hindi. Almost nothing
was wrong at the text level: no raw markup, two pictures left as words, and the long lines were
long paragraphs. The shots showed these, now fixed:

- **`↗ open the link` under linked pictures (16 times).**
  - A picture inside a link is now the link (item 6). The link rides in the picture's source
    (`&go=`), and `inbox::links` opens it by Cmd+click anywhere on the picture, or by its
    `Cmd+E` hint.
  - A link around blocks (a news digest's headline and summary in `<div>`s) makes every
    paragraph begun inside it the link (`Out::link`, `block`).
  - A link around a tracking pixel leaves nothing behind.
- **Addresses run together** (`HUNTERSVILLENC28078`). A span with horizontal padding or
  margin, or `display:inline-block`, is now a word of its own.
- **A button in a large type became a heading.** A block holding only a button is the button.
- **Too many top-level headings.** A promotion set 33 lines as headings, every offer a
  headline. A mail with more than four `#` headings has them all a step down, so the page is
  not a wall of accent headlines.

Found but not the reader's to fix: Devanagari (a Hindi news digest) is drawn a cell per
character, so its vowel signs and conjuncts come apart. That is shaping in crew's cell
renderer, crew-wide, and needs a goal of its own.
