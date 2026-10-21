# Goal — Hindi, and every script that joins its letters, drawn whole

**Status: WRITTEN, NOT STARTED.** Set 2026-10-21.

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
