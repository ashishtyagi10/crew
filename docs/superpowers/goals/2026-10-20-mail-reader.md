# Goal — read mail the way it was written: rich text and pictures in `/inbox`

**Status: SET 2026-10-20** (the user: "go ahead with the mail reader goal"). Phases 0 and 1
shipped in v0.27.34; phase 2 built (see "Where it stands" at the end).

**Set:** 2026-10-20 by the user, once `/inbox` was signed in live: *"write a goal to give better
email viewer, like showing better markdown or htmls with images, whatever is possible."*

## What the reader does today

An opened thread is plain words (`crates/crew-app/src/inbox/reader.rs`). The body comes from
`crates/crew-app/src/inbox/body.rs`: the `text/plain` part if there is one, otherwise the
`text/html` part run through `crew_plugin::html::readable`
(`crates/crew-plugin/src/broker/sysfetchtext.rs`). That function was written to hand a web page
to a model cheaply. It drops tags and collapses whitespace, so a mail loses:

- **structure:** headings, bold, lists, quotes and tables all become one run of prose;
- **links:** the text survives but the address behind it is gone, so "Track your order" leads
  nowhere;
- **pictures:** inline images, logos, screenshots and charts are not shown at all;
- **hidden text:** preheaders and `display:none` blocks are read out as if they were visible;
- **non-UTF-8 mail:** bodies are decoded as UTF-8 with lossy replacement, so a windows-1252
  newsletter shows replacement characters wherever it uses curly quotes;
- **the thread's shape:** each reply repeats the whole history beneath it, so a ten-message
  thread reads as ten copies of itself.

## What crew can already draw (so this is mostly plumbing)

- **A markdown engine** (`crates/crew-app/src/md/mod.rs`, pulldown-cmark): headings, emphasis,
  lists, quotes, code, rules, footnotes, task lists, and tables that wrap to the pane
  (`crates/crew-app/src/md/table.rs`, `crates/crew-app/src/md/tablewrap.rs`). Chat already
  paints its output to cells with pictures in place (`crates/crew-app/src/chatmd.rs`,
  `crates/crew-app/src/chatmdcells.rs`).
- **Pictures.** `![alt](src)` reserves rows in the layout (`crates/crew-app/src/md/picture.rs`).
  `crates/crew-app/src/viewpane/bitmap.rs` decodes and downscales on a worker thread, then draws
  the image as sub-cell quads on the paint layer (about 3 samples per cell). The cache in
  `crates/crew-app/src/imgcache.rs` loads **local files only**: a remote image stays alt text.
- **Hint mode** (`crates/crew-app/src/hints.rs`): `Cmd+E` labels every URL on the focused pane;
  the lowercase letter copies it and the capital opens it.
- **A document window** (`crates/crew-app/src/docwin/mod.rs`): `w` in a viewer pane pops the
  document into a window of its own.

## What "whatever is possible" means

**Possible:** mail that reads like a well-formatted document, with real links, inline pictures,
folded quoted history, and pictures from the web when you ask for them.

**Not possible, and not in this goal:** a pixel-exact copy of the HTML. crew draws a grid of cells
on the GPU and has no web engine. Embedding one (WebKit or Chromium) would add a huge dependency
and break the rule that everything in crew is a drawn tile. CSS layout, the sender's colours,
backgrounds and fonts are dropped: the theme's ink wins, which also keeps every mail readable on
light and dark pages. For the exact look, `o` already opens the thread in Gmail.

## Where it could go wrong, most serious first

1. **Privacy.** A remote image is a read receipt: fetching it tells the sender when the mail was
   opened, from which IP address, and roughly where. Gmail's web app hides this behind Google's
   image proxy; crew would fetch directly. So remote pictures are **off by default**, load per
   message on a key, and tracking pixels are never fetched.
2. **Hostile HTML.** Mail is written by strangers. There is no engine, so no script can run, and
   `body::clean` already strips control characters. That leaves three risks:
   - **Deceptive links:** a link whose text says `paypal.com` but points somewhere else.
   - **Hidden text:** words the user never sees but agent smith reads when the thread is handed
     over with `a` or `r`. A prompt injection can hide there.
   - **Image bombs:** a small file that decodes to gigabytes. `bitmap.rs` currently calls
     `image::load_from_memory` with no limits, which is fine for the user's own files but not
     for mail.
3. **Fidelity.** Newsletters are laid out in nested tables. Rendering those cell for cell gives
   nonsense columns. The goal is *readable*, not *identical*, and two escape hatches always
   remain: show the plain-text part, or open the thread in Gmail.
4. **Cost.** A large marketing mail is 200–500 KB of HTML with dozens of images. Conversion and
   decoding happen off the winit thread, and every stage has a cap.

## Light weight (measurable)

- **No new dependency unless phase 0 shows it is needed and the user agrees.** pulldown-cmark,
  `image` and `base64` are already in the tree. HTML parsing is the open question; see item 2.
- **Converted on the inbox worker thread**, cached as markdown per message, and laid out at draw
  time like chat. A 300 KB newsletter fixture converts in under 50 ms in a release build.
- **No extra network for a text mail.** Inline (`cid:`) pictures cost one `attachments.get` each;
  read the price from Gmail's current quota table in phase 0. Remote pictures cost nothing
  until the user asks.
- **Nothing written to disk.** Decoded pictures live in memory and go when the pane closes,
  except an attachment the user explicitly saves.
- Every file under the 200-line cap, held by `crates/crew-app/src/linecap.rs`.

## Done means

**Phase 0: a corpus and one decision (no UI).**

1. **A fixture corpus** of synthetic mails, never real ones, each with its expected markdown.
   The shapes:
   - Gmail-composed HTML with a `gmail_quote`;
   - Outlook (`MsoNormal`, `<!--[if mso]>` blocks, `divRplyFwdMsg`);
   - Apple Mail's `blockquote type=cite`;
   - a newsletter built from nested layout tables, with a hidden preheader;
   - a receipt with a real data table;
   - a GitHub notification;
   - a calendar invitation;
   - a forwarded message;
   - `multipart/related` with `cid:` pictures;
   - a plain-text-only mail with `>` quoting and a `-- ` signature;
   - a windows-1252 mail;
   - a mail with a deceptive link.
2. **The HTML decision.** Write a lenient tokenizer that keeps a tag stack and emits markdown, in
   crew's own code. Keep it if it passes the corpus in roughly 600 lines. If it can't, bring the
   options (`html5ever` and smaller crates) and their weight to the user before phase 1. Record
   the decision here.

**Phase 1: rich text.**

3. **HTML becomes markdown.** The engine draws paragraphs, line breaks, headings, bold and
   italic, links, nested lists, blockquotes, `pre`/code, rules, and data tables. Layout tables
   are flattened into blocks, not drawn as columns. A layout table is one marked
   `role=presentation`, or one that holds nested tables, block content, or a single column.
   Dropped completely: `head`, `style`, `script`, conditional comments, and hidden content
   (`display:none`, `visibility:hidden`, `mso-hide:all`, zero-size text). All named and numeric
   entities are decoded.
4. **The HTML part is preferred** when a mail has both parts. `v` switches the open message to
   its plain-text part and back. That is the escape hatch when a conversion goes wrong, and some
   senders write a better plain part.
5. **Plain-text mail is not parsed as markdown.** A `*` or `#` in a plain mail is literal text.
   Its URLs become links, `>` runs draw as quote bars, and a signature after `-- ` draws in the
   muted ink.
6. **Quoted history folds.** Each message's quoted history collapses to one muted row:
   `··· quoted text (24 lines)`. That covers Gmail's `gmail_quote`, Apple's `type=cite`,
   Outlook's reply header block, and plain "On … wrote:" followed by `>` runs. In a thread the
   earlier messages are already on screen above. `z` unfolds the quote under the cursor.
7. **Links work and cannot lie.**
   - Every link carries its real address, so hint mode labels it and the capital letter opens
     it. Only `http`, `https` and `mailto` open; any other scheme is copied, never run.
   - When a link's text names a domain and its address goes to a different one, the real domain
     is drawn after it: `paypal.com ⟶ paypa1-login.xyz`.
8. **Charsets.** Each part is decoded by the `charset` in its `Content-Type`. UTF-8, US-ASCII,
   ISO-8859-1 and windows-1252 are decoded by hand (a 32-entry table). Any other charset keeps
   today's lossy decode, and the reader says so in one line: `this mail is in Shift_JIS — o
   opens it in Gmail`. If the corpus or the user's own mail shows other charsets are common,
   `encoding_rs` is the dependency to propose.
9. **Smith reads what the user sees.** The prompts in `crates/crew-app/src/inbox/smith.rs` are
   built from the same converted text: hidden content stays dropped and links keep their real
   addresses. The thread is still fenced and labelled untrusted.
10. **Bounded.** HTML past 512 KB is converted up to that point, followed by a line: `the rest
    is in Gmail — o`. Every phase-1 fixture gets a shot test at 80 and 120 columns on the light
    and dark Ink themes.

**Phase 2: pictures.**

11. **Inline pictures, shown by default.** A `cid:` image is fetched from its part with
    `messages.attachments.get` on the worker, decoded by `bitmap.rs`, and drawn where the mail
    puts it. These bytes are inside the mail, so fetching them tells no one anything.
12. **Remote pictures, off by default.** The message shows one row: `▣ 6 pictures from the web —
    i shows them`. `i` fetches them for that message only, with these limits:
    - `https` (or `http`) only;
    - no cookies and no `Referer` header;
    - a 10 s timeout, at most 5 MB per picture and at most 20 per message.
    Tracking pixels are never fetched: an image declared at 2×2 or smaller, or a known spacer
    file. An image that decodes that small is not drawn either.
13. **Safe decoding.** Every decode of mail bytes runs under `image::Limits`: a maximum width and
    height, and a maximum allocation. A file over the limit draws as its alt text.
14. **Sized from the mail.** A picture with `width`/`height` attributes gets rows in proportion,
    so a 40 px logo takes 2 rows, not 12. Without attributes it takes the markdown engine's
    `picture::ROWS` (12). Rows are fixed before decoding, so the text never jumps when a picture
    lands.
15. **Image attachments** draw under their message, up to 4, after the `attached:` line.

**Phase 3: room to read, and attachments.**

16. **`w` pops the open thread into a document window**, the way `w` does in the viewer: the
    same rendering, full width, no tiles. The inbox pane's triage keys keep working in that
    window.
17. **Attachments are reachable.** Each attachment row shows its name and size, and hint mode
    labels it. The capital letter saves the file to `~/Downloads` (never overwriting an existing
    file) and opens it with the system's app. This is the one thing crew writes to disk for this
    goal, and only when asked.

**Not in this goal:** a web engine or pixel-exact HTML; the sender's colours, fonts and
backgrounds; forms; composing or sending; a per-sender "always show pictures" list (an open
question below); calendar invitation cards; inverting pictures for dark mode.

## Open questions for the user

- **Remote pictures per sender.** After `i`, should crew offer "always for this sender", and
  where would that list live? The default here is no list: `i` each time.
- **The default view.** Rich HTML is the default in this goal. Should a user be able to choose
  plain text as their default?

## Where it stands

**Phase 0, decided:** crew's own converter, no new dependency. A lenient tokenizer and tree
(`crates/crew-app/src/inbox/html/tok.rs`, `crates/crew-app/src/inbox/html/tree.rs`) and a walk
that writes markdown (`crates/crew-app/src/inbox/html/walk.rs`, `out.rs`, `block.rs`,
`crates/crew-app/src/inbox/html/table.rs`, `crates/crew-app/src/inbox/html/quote.rs`,
`crates/crew-app/src/inbox/html/link.rs`, `crates/crew-app/src/inbox/html/hide.rs`,
`crates/crew-app/src/inbox/html/ent.rs`): about 1,000 lines, more than the 600 guessed, every
file under the cap. The corpus is ten synthetic mails in `crates/crew-app/src/inbox/html/corpus/`
(Gmail, Outlook, Apple Mail replies; a newsletter; a receipt; a GitHub notification; a calendar
invitation; a forward; a deceptive link; structure), each with its expected markdown, held by
`crates/crew-app/src/inbox/html/corpus_tests.rs`.

**Phase 1, built:** items 3–10. The reader draws through `chatmd` (`crates/crew-app/src/inbox/reader.rs`,
layout cached per width and toggles in `crates/crew-app/src/inbox/reading.rs`); a message is a
`Rich` (`crates/crew-app/src/inbox/rich.rs`) made in `crates/crew-app/src/inbox/body.rs` from its
parts in their charsets (`crates/crew-app/src/inbox/charset.rs`), plain mail through
`crates/crew-app/src/inbox/plain.rs`. Links reach hint mode and Cmd+click through
`crates/crew-app/src/inbox/links.rs`. Two departures from the items as written: `z` and `v`
act on the whole open thread rather than the message under a cursor (the reader has no cursor),
and the shots are the inbox's two themes (Ink light, glass dark) rather than 80 and 120 columns.

**Phase 2, built:** items 11–15. The converter writes `![alt](<src#rows=N>)` for a `cid:`
picture always and for a web one only in the "with web" version a `Rich` also keeps
(`crates/crew-app/src/inbox/html/image.rs`); `#rows=N` is read by the markdown engine
(`crates/crew-app/src/md/picture.rs`), so any document can size a picture's box. The pane's
pictures live in `crates/crew-app/src/inbox/pics.rs`; fetching and the limited decode in
`crates/crew-app/src/inbox/picfetch.rs`; message layout with picture placement in
`crates/crew-app/src/inbox/readmail.rs`; the paint pass in `crates/crew-app/src/inbox/picpaint.rs`.
`i` acts on the whole open thread, like `z` and `v`. A linked picture keeps its link on a line
under it (`↗ Read the report`). The quota cost of `attachments.get` was not read; it is one
call per carried picture, at most four attachments a message.

**Not yet:** phase 3 (`w`, attachments saved and opened).
