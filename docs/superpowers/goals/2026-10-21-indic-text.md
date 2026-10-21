# Goal — Hindi, and every script that joins its letters, drawn whole

**Status: BUILT (v0.27.47), waiting on the user's check on their own machine.** Set and
started 2026-10-21 (*"go ahead with the hindi goal"*). The record is the last section.

**Set:** 2026-10-21 by the user, after the inbox was read against their own mail and a Hindi
news digest came out letter by letter: *"write a goal for the hindi rendering"*.

## What is wrong

In every crew surface, Devanagari is drawn one character to a cell, with its marks either
pushed apart or lost. Those surfaces are the terminal panes, chat, the `/inbox` reader, the
input bar, and anything else drawn through crew-render's cell grid. So `भारत` reads
`भ ा र त`, and a conjunct like `क्ष` reads `कष`. There are two faults, both in shared code
(investigated 2026-10-21):

1. **The marks that sit beside a letter are pushed into cells of their own.**
   - crew-render already shapes a pane's text in runs:
     `crates/crew-render/src/celltext.rs` builds one string per pane and calls
     `set_rich_text` with `Shaping::Advanced`, and cosmic-text 0.18.2 (harfrust) shapes a
     Devanagari cluster correctly.
   - Two later steps undo it. `set_monospace_width` rounds every glyph's advance to a whole
     cell, so a spacing vowel sign (`ा`, `ि`) becomes 0 cells or 1.
   - The correction pass (`detect_corrections`, `cells_for` in the same file) then treats
     every character `unicode-width` calls 1 wide as a one-cell glyph, and spaces it out to
     fill its cell.
2. **The marks that sit on a letter never reach the renderer.**
   - A zero-width mark (virama `्`, anusvara `ं`, nukta `़`, the `े` matra) is dropped in
     chat and the reader, where `crates/crew-app/src/chatplace.rs` skips width-0
     characters.
   - It is also dropped in the terminal: `crates/crew-term/src/modelcells.rs` copies a
     cell's one character, and alacritty_terminal keeps the marks in `Cell::zerowidth()`.
   - Underneath both, a `CellView` (`crates/crew-render/src/cellgrid.rs`) holds a single
     `char`.

Widths are not the problem. Every crate in the tree, alacritty_terminal and ratatui included,
uses unicode-width 0.2, which gives a base letter and a spacing sign 1 cell and a combining
mark 0. Summing those gives the cells programs and alacritty already assume (`भारत` 4, `क्ष`
2, `हिंदी` 4), so cursor and column maths stay as they are.

Fonts are not the problem either. crew embeds only Lilex, which has no Devanagari
(`crates/crew-render/src/embedfont.rs`). cosmic-text's own fallback reaches Devanagari Sangam MN
on macOS, Nirmala UI on Windows and Noto Sans Devanagari on Linux. The check that refuses a
font that does not snap to cells (`crates/crew-render/src/fontverify.rs`) tests only the primary
family, so it never sees these.

## What other terminals do

WezTerm and Kitty shape a line's runs of one style with HarfBuzz and split the result into
grapheme clusters. Each cluster gets the cells its characters' widths sum to and is drawn from
its first cell at its natural advance, centred or squeezed to fit. They never snap a cluster
glyph by glyph.

## Done means

**Phase 0: see it fail.**

1. **A shot of the failure, kept.** These go through the off-screen harness
   (`crates/crew-app/src/shotgpu_tests.rs`), on the light page and on glass:
   - `भारत`, `क्ष`, `हिंदी`, `श्री`, `द्वारा`, `कृपया`;
   - a sentence of Hindi news;
   - a line mixing Hindi and English.

   These are drawn in a chat reply, the `/inbox` reader, a terminal running
   `printf 'भारत क्ष हिंदी\n'`, and the input bar. They are kept beside the fixed shots in the
   goal's record.

**Phase 1: keep every mark.**

2. **A cell carries its marks.** A `CellView` gains the combining marks that follow its
   character, as a small inline tail that costs nothing when it is empty, which is almost
   always. Every place that builds cells keeps them on the cell before them instead of
   dropping them:
   - `chatplace::line_cells`;
   - the input bar (`crates/crew-app/src/inputbar_render.rs`);
   - crew-term's `modelcells` (`Cell::zerowidth()`);
   - every other width-0 skip, each found by a search and listed in the record.
3. **Copy keeps them too.** A selection copied or a block yanked has its marks, so `क्ष`
   pastes as `क्ष`.

**Phase 2: draw a cluster whole.**

4. **Runs of complex script are shaped and placed as runs.**
   - In `fill_rich_text`, a maximal run of grapheme clusters from the Indic and Southeast
     Asian blocks (U+0900–U+0DFF, plus Thai, Lao, Myanmar and Khmer) is left as spaces in
     the snapped main buffer, so the grid stays aligned.
   - Each run is drawn as its own text area (`crates/crew-render/src/textprep.rs`) at its
     first cell's x, shaped at its natural advance with no monospace width. It is cached by
     (text, style, size).
   - The run is scaled down only when its natural width is more than the cells its
     characters' widths give it.
   - The correction pass skips these runs.
5. **One style at a time.** A run splits where bold, colour or a link changes, so a bold Hindi
   headline in a reply or a mail is bold and a Hindi link is underlined.
6. **The grid does not move.**
   - Cursor, selection, hit-testing, the find bar's marks, backgrounds and decorations all
     keep using the cells the widths give. Backgrounds and decorations already take a cell's
     width from `scene::cell_cols` (`crates/crew-render/src/scene.rs`).
   - A Hindi word in a terminal takes the columns the program thinks it does, so a TUI drawn
     around it (a table, a box) still lines up.

**Phase 3: proof.**

7. **Shots.** The phase-0 set again on both themes, plus Bengali, Tamil, Gujarati and Thai
   lines. A test reads the pixels and passes when two things hold:
   - no blank pixel column falls inside a word;
   - no word's ink spills past its cells.
8. **The record.** The goal's last section holds before and after shots of the reader on
   the Hindi digest that showed the bug, and a note on what Windows and Linux were checked
   with. A check by the user on their own machine closes it.

## Light weight

- Latin text pays nothing. The new path runs only for runs of complex-script clusters, and
  the cell tail is empty for every cell without marks.
- A run is shaped once and cached. A full pane of Hindi costs one text area per run, measured
  in the record against a pane of English.
- Every file stays under the 200-line cap. `celltext.rs` gets a sibling for the run path
  instead of growing.

## Not in this goal

- Right-to-left scripts (Arabic, Hebrew), which need bidi reordering as well as shaping, and a
  goal of their own.
- Emoji presentation and ZWJ sequences, which crew draws through its own path.
- A bundled Devanagari font, since the systems' own fallbacks are there.
- Hindi input methods.

## Record

**v0.27.47 (2026-10-21).** Phases 0 to 3 are built. Item 8 waits on the user's own check, and
Windows and Linux were not run (see the end of this section).

### What was built

- **Phase 0.** `crates/crew-app/src/indicshot_tests.rs` shoots the samples on a chat reply, the
  `/inbox` reader, a terminal (`printf`, the sentence, the mixed line, and a box table drawn
  around Hindi), and the input bar, on PaperLight and GlassNight. The before shots showed every
  sample letter by letter (`भ ा र त`). `क्ष` was drawn `कष`, `हिंदी` lost its nasal dot, and
  Tamil and Bengali were scattered across their rows.
- **Item 2, a cell carries its marks.** `crew_theme::marks::Marks` holds up to four marks
  inline (16 bytes, no allocation). It is `CellView::marks` and `RenderCell::marks`.
  - Terminal: `modelcells` copies alacritty's `Cell::zerowidth()`.
  - `chatwidth::place_row` holds each character until the marks after it are read, and hands
    them to its callback. All 37 callers pass them on, through `CellView::marked`.
  - Also converted: `chatplace::line_cells` (chat, the stream tail, the inbox reader),
    `chatlayout`'s legacy rows, the inbox list (`inbox::line::Line`), the composer's ghost
    suggestion and the minimized-pane preview (both gave a mark its own cell on top of the next
    letter), and the ratatui bridge (`tui::split`). ratatui keeps a grapheme such as `हि`
    whole in one cell and blanks the next; the bridge now gives each character with a width a
    cell of its own, in the columns ratatui reserved.
- **Item 3, copy.** `copytext` rebuilds a row with every mark, and without the space the old
  copy put after a wide character (`日本` used to paste as `日 本`). It is used by selection
  copy (`gridsel`), Cmd+C with no selection (terminal and in-app), and `/copy` scrollback
  (`dump::grid_row`). Terminal mouse selection already read alacritty's own text.
- **Item 4, runs.** `crates/crew-render/src/scriptrun.rs` collects runs of joining script (the
  blocks the goal names) and leaves their cells blank in the snapped buffer.
  `scriptshape.rs` shapes each run on its own and caches it. `scriptplace.rs` places it, and
  each run is drawn as its own text area (`scriptrun::areas`, used by `textprep` and
  `presmooth`). The correction pass never sees a run. A cell that carries marks also takes no
  letter-spacing, since spacing lands on the marks too.
- **Item 5, style.** A run splits where the colour, weight, slant or ink polarity changes, so
  a bold headline is bold and a link is in the link's colour.
- **Item 6, the grid.** Cells, backgrounds, the cursor, selection and find marks are unchanged.
  A terminal table drawn around Hindi stays square (see the terminal shot).

### Where it differs from the plan

The plan drew each run at its first cell and shrank it only when wider than its cells. Shots
showed two faults with that:

1. **Gaps.** Ink is rarely as wide as the widths say. `हिंदी` is four cells and under three of
   ink, and every word put all its slack after it, so a sentence read `भारत   ने आज`.
2. **Tiny words.** Others are wider than their cells: `आज` is two cells and nearly three of
   ink, and Bengali and Tamil words reach 1.5×. Shrunk to fit, they came out at 60–70%.

So the placement is now a **phrase at a time**. A phrase is the runs on one row, each one blank
after the last.

- Its words are drawn at their natural size, from the phrase's first cell, with equal gaps of
  ½ to 1½ cells.
- The gaps end the phrase at its last cell when they can. A word's ink stays within a cell or
  two of its cells, so the cursor, a selection and a find mark still sit on it.
- A word may spill into the blanks after it, at most its own width again, keeping half a cell
  clear (`RunSpec::room`).
- Only a phrase too tight for the ½-cell gap falls back to each word in its own room, shrunk
  if it must be.

Five further changes came from the shots:

- **Punctuation.** Punctuation straight after a word joins its run (`भेजा,`, `दिल्ली:`), so it
  is not left behind on its grid cell. `|` and `/` are left out, since programs draw columns
  with them.
- **Bold.** macOS falls back to Kohinoor Devanagari at the body's weight 500 but to Devanagari
  Sangam MN at 700, so bold Hindi was another typeface and hardly heavier. Bold now comes from
  the family that drew the run at the body weight (`weight_in_family`).
- **Underlines and strikes.** These are drawn per cell, and in a phrase drawn off its cells a
  link's underline ran past its last word. `scriptplace::Warp` now maps them onto the ink:
  across a word, across the gap to the next word of its phrase, and along with a word for the
  blanks after it. Cursor and backgrounds stay on the grid.
- **Baselines.** Each run sits on its row's baseline as the snapped buffer laid it out, so
  Hindi beside English shares the English line.
- **The proof's second test.** It checks ink against the row's cells plus the last word's
  room, not the cells alone, because a word may spill into the blank after it.

### Width-0 skips left as they were

- `navtext::put_centred` (dial labels) and the sites that place fixed ASCII one char a cell:
  `popupchrome`, `chatscroll`, `help`, `panecard`, `keycard`, `hints`, `welcomeart`,
  `gauges`, `toosmall`, `updatecard`. Also `git.rs` (the branch name), `minstrip` (pane
  names) and `cmdhead` (the running command), whose text could hold a mark but rarely does.
- `viewpane::caret` stops: on a line ending in a mark, the end stop sits before the mark.
- `viewpane::lspdeco`: a char-index span can land one mark off.
- `chatprobe::token_at` (the Cmd+click path) indexes chars by display column.
- `hintact`'s copy reads `grid_lines` (URLs and hashes, which carry no marks).
- The chat composer's beam is drawn on the cell whose index is the caret, so with the caret
  on a mark it is not drawn.
- The markdown engine wraps by chars, not width, so Hindi lines wrap a little early.

### Proof

- **Shots.** `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew indic_ -- --ignored`
  writes the phase-0 set, plus Bengali, Tamil, Gujarati and Thai, on both pages. No PNG is
  kept in the repo; the command makes them.
- **Pixels.** `indicpixel_tests` (same command) draws Hindi rows and a mixed row on both
  pages and reads the pixels:
  - each Devanagari word is one unbroken span of ink;
  - every row's ink starts at its first cell and ends within its last word's room;
  - Latin after Hindi starts on its own cell.

  With the run path switched off it fails: 9 spans of ink for 6 words.
- **The digest.** The Hindi news digest that showed the bug (thread `1a15d247` of the user's
  saved mail) was drawn by `inbox_live_shot`, before (v0.27.46) and after. Every headline
  link reads whole, and its underline ends with its words. The shots are of the user's own
  mail, so they stay outside the repo.
- **Unit tests.**
  - `scriptrun_tests`: collection, punctuation, room.
  - `scriptplace_tests`: phrase layout, the tight fallback, shrinking, bold's family, the
    warp.
  - `crew-term` `modelmarks_tests`, `crew-theme` `marks_tests`, `crew-app`
    `markcells_tests` (every producer and selection copy) and `copytext_tests`.

### Light weight

Measured with `cargo test --release -p crew-render scriptrun_cost -- --ignored --nocapture`,
on a 120×40 pane. A warm English pane is the baseline:

| Pane | Grid | Runs |
|---|---|---|
| English, warm | 29 ms | none |
| Hindi, cold | 72 ms | 3.5 ms (1,200 runs) |
| Hindi, run cache warm | 72 ms | 0.08 ms |
| Hindi, before this goal | 782 ms cold, 391 ms warm | none |

- A pane of plain spaces also costs 72 ms, so the grid's share is cosmic-text's cost for long
  runs of blanks, not the run path. Neither a no-break space nor a merged span changed it.
- The old path paid for a correction measured and rebuilt on every Devanagari glyph.
- Latin pays nothing new: no runs, and empty marks.
- A pane is only rebuilt when its content changes.

### Windows and Linux

Not run. On macOS the fallbacks found are Kohinoor Devanagari (body) and Devanagari Sangam MN,
Bangla Sangam MN, InaiMathi (Tamil), Gujarati Sangam MN and Ayuthaya (Thai). The goal names
Nirmala UI on Windows and Noto Sans Devanagari on Linux; nothing here depends on which family
answers. The shaping tests skip, saying so, on a machine with no face for the script
(`scriptrun::tests::drawable`), so CI on a runner without Indic fonts skips rather than fails.

