# Smith Pane Semantic Palette Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give chat-card markdown a semantic palette — code, list markers, blockquote bars and headings each read as themselves instead of every message body drawing in one colour.

**Architecture:** Colours come from the active theme's existing 16-slot ANSI palette, read at call time through one new module (`crew-app/src/chatink.rs`) — no new `Theme` fields, so all 13 presets stay correct with zero hand-tuning. The markdown layout layer currently flattens blockquotes and list items into `LineKind::Body`; it grows one variant (`LineKind::Quote`) and one span flag (`MdStyle.marker`) so `chatmd` can tell structure from prose.

**Tech Stack:** Rust, `crew-app` (winit/wgpu GUI, binary crate), `crew-theme` (no-dependency colour crate), `pulldown-cmark` (already vendored via the `md/` engine).

**Spec:** `docs/superpowers/specs/2026-07-26-smith-semantic-palette-design.md`

## Global Constraints

- **NEVER run `cargo build --release` or `cargo clean`** — disk is tight on this machine. `cargo test` / `cargo clippy` (dev profile) only.
- **No new dependencies.** No `syntect`, no `tree-sitter`, no tokenizer. This is not syntax highlighting.
- **No new `Theme` fields.** Colours are derived from existing slots. Adding a field means 13 presets to hand-tune.
- **Colours are read at call time** from `crew_theme::theme()`, never cached in a `static`/`OnceLock` — a live `/theme` switch must repaint correctly.
- **Tests compare against `crew_theme::theme()`'s own slots**, never hardcoded RGB triples. The active theme is global mutable state and tests run in parallel; a hardcoded `(140, 220, 110)` passes or fails depending on which test set the theme last.
- **Keep source files under ~200 lines.** When a file approaches it, move tests to a sibling `<name>_tests.rs` included via `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;` — the established pattern in this repo.
- **`cargo clippy --workspace --all-targets -- -D warnings` must be green**, with no `#[allow(...)]` suppressions added to make it so.
- **Never commit keys** — the crew repo is public.
- Exact ANSI slots: **code = `ansi[6]`** (cyan), **markers = `ansi[3]`** (yellow). Do not substitute other slots; the contrast test in Task 1 covers only these two.

---

### Task 1: Theme contrast guard for the two palette slots

The palette's whole correctness argument is "the ANSI slots are already readable in every preset." That must be a test, not a claim — otherwise a future preset ships an unreadable code colour and nobody notices. This task lands first so the colour source is locked before anything consumes it.

**Files:**
- Modify: `crates/crew-theme/src/lib_tests.rs` (inside the existing `contrast_thresholds` test)

**Interfaces:**
- Consumes: nothing.
- Produces: nothing consumed by later tasks — this is a standalone guard. Later tasks rely on the *fact* it asserts: `theme().ansi[6]` and `theme().ansi[3]` are readable on `page_bg` in all 13 presets.

- [ ] **Step 1: Read the existing test**

Open `crates/crew-theme/src/lib_tests.rs` and find `fn contrast_thresholds()`. It loops `for id in ALL_THEMES` with `let name = id.as_str(); let t = id.theme(); let bg = t.page_bg;` and a local `let cr = contrast_ratio;`, then asserts `ink >= 10.0`, `text_muted >= 7.0`, `legend_off >= 3.0`, and so on. You are adding two assertions in the same style, inside the same loop.

- [ ] **Step 2: Add the two assertions**

Add these at the end of the loop body, after the existing assertions:

```rust
        // The chat markdown palette (crew-app `chatink`) draws code from
        // ansi[6] and structural markers (list bullets, quote bars) from
        // ansi[3], both on the page. Their readability is therefore a
        // theme-level promise, not a crew-app detail: a new preset with a
        // washed-out cyan or yellow breaks chat rendering, and this is where
        // that gets caught. Measured worst cases when written: ansi[6] 5.85
        // (SEPIA_LIGHT), ansi[3] 4.64 (IVORY_LEDGER).
        assert!(
            cr(t.ansi[6], bg) >= 4.5,
            "{name}: ansi[6] (chat code) vs page_bg = {:.3} (need >= 4.5)",
            cr(t.ansi[6], bg)
        );
        assert!(
            cr(t.ansi[3], bg) >= 4.5,
            "{name}: ansi[3] (chat marker) vs page_bg = {:.3} (need >= 4.5)",
            cr(t.ansi[3], bg)
        );
```

- [ ] **Step 3: Prove the assertions actually run (mutation check)**

An assertion that passes on day one proves nothing — it might not be reached. Temporarily change both `>= 4.5` thresholds to `>= 40.0` (impossible — the max achievable ratio is 21).

Run: `cargo test -p crew-theme contrast_thresholds`
Expected: FAIL, with a message naming a specific theme, e.g. `CRT_GREEN: ansi[6] (chat code) vs page_bg = 15.664 (need >= 40.0)`.

If it passes, the loop isn't reaching your code — fix that before continuing.

- [ ] **Step 4: Restore the real thresholds and confirm green**

Change both `40.0` back to `4.5`.

Run: `cargo test -p crew-theme`
Expected: PASS, all tests.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-theme/src/lib_tests.rs
git commit -m "test(theme): hold ansi[3]/ansi[6] at 4.5:1 on the page for the chat palette"
```

---

### Task 2: `chatink` module + code and heading colours

Delivers the user's core ask on its own: code stops looking like prose. Needs no markdown-layout changes, so it can land and be judged independently of Task 3.

**Files:**
- Create: `crates/crew-app/src/chatink.rs`
- Create: `crates/crew-app/src/chatmd_tests.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod chatink;`)
- Modify: `crates/crew-app/src/chatmd.rs` (remove the two local colour fns, restructure `span_style`, attach the test module)
- Modify: `crates/crew-app/src/chatbody.rs` (one added test in the existing inline `mod tests`)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces, all `pub(crate)` in `crate::chatink`, each returning `crate::chatbody::Color` (= `(u8, u8, u8)`):
  - `code_bg() -> Color`, `code_fg() -> Color`, `marker_fg() -> Color`, `quote_fg() -> Color`, `heading_fg() -> Color`, `link_color() -> Color`
  - Task 4 calls `marker_fg()` and `quote_fg()`; nothing else calls them yet, which is expected and must not be "fixed" by deleting them.
- Produces in `crate::chatmd`: `fn body_span_style(span: &MdSpan, base: Color) -> (Color, bool, bool, Option<Color>, Option<Arc<str>>)` — Task 4 extends it with a marker branch and calls it a second time for quote lines.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/chatmd_tests.rs`:

```rust
use super::*;

/// The characters of one card line, in order.
fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// Render `text` through the same path a chat message body takes.
fn lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    map_lines(crate::md::render_chat(text, width), width, fg)
}

#[test]
fn fenced_code_takes_the_code_colour() {
    let out = lines("```rust\nfn x() {}\n```", 40, (9, 9, 9));
    // 0 = "╭─ rust" chrome, 1 = code content, 2 = "╰─" chrome.
    assert_eq!(row_text(&out[1]), " fn x() {}");
    let cell = &out[1][1];
    assert_eq!(cell.fg, crew_theme::theme().ansi[6]);
    assert_eq!(cell.bg, Some(crate::chatink::code_bg()));
}

#[test]
fn code_chrome_stays_muted() {
    let out = lines("```rust\nfn x() {}\n```", 40, (9, 9, 9));
    assert_eq!(row_text(&out[0]), " ╭─ rust");
    assert_eq!(out[0][1].fg, crew_theme::theme().text_muted);
    assert_eq!(out[0][1].bg, None);
}

#[test]
fn inline_code_is_coloured_but_surrounding_prose_is_not() {
    let fg = (9, 9, 9);
    let out = lines("use `let` now", 40, fg);
    // " use let now" — index 0 is the indent cell, 5..8 is "let".
    assert_eq!(row_text(&out[0]), " use let now");
    assert_eq!(out[0][5].fg, crew_theme::theme().ansi[6]);
    assert_eq!(out[0][5].bg, Some(crate::chatink::code_bg()));
    assert_eq!(out[0][1].fg, fg);
    assert_eq!(out[0][1].bg, None);
}

#[test]
fn headings_are_ink_and_bold_at_every_level() {
    for src in ["# One", "## Two", "### Three", "###### Six"] {
        let out = lines(src, 40, (9, 9, 9));
        let cell = &out[0][1];
        assert_eq!(cell.fg, crew_theme::theme().ink, "{src}");
        assert!(cell.bold, "{src}");
    }
}

#[test]
fn links_keep_the_link_colour() {
    let out = lines("go to [site](https://s.io) now", 60, (9, 9, 9));
    let cell = out[0].iter().find(|c| c.link.is_some()).expect("a link cell");
    assert_eq!(cell.fg, crate::chatink::link_color());
    assert!(cell.bold);
}
```

Attach it at the bottom of `crates/crew-app/src/chatmd.rs`:

```rust
#[cfg(test)]
#[path = "chatmd_tests.rs"]
mod tests;
```

And add one test to the existing `mod tests` at the bottom of `crates/crew-app/src/chatbody.rs`:

```rust
    #[test]
    fn source_mode_stays_flat() {
        let fg = (9, 9, 9);
        let lines = body_lines("see `this`:\n```rust\nfn x() {}\n```", 40, fg, true);
        for line in &lines {
            for cell in line {
                assert_eq!(cell.fg, fg, "source mode must not colour cells");
                assert_eq!(cell.bg, None, "source mode must not tint cells");
            }
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app chatmd`
Expected: compile error — `crate::chatink` does not exist yet. That is the failure for this step.

- [ ] **Step 3: Create the `chatink` module**

Create `crates/crew-app/src/chatink.rs`:

```rust
//! Where chat markdown gets its colours. Every helper reads the ACTIVE theme
//! at call time, so a live `/theme` switch repaints with nothing to
//! invalidate.
//!
//! Code and marker colours come from the theme's own 16-slot ANSI palette
//! rather than from new `Theme` fields. Two reasons: all 13 presets already
//! tune those slots (a new field would be 13 values to hand-maintain), and
//! single-phosphor CRT presets keep their hue for free — CRT_GREEN's "cyan"
//! is (0, 255, 200), still green, so a green tube never sprouts a foreign
//! colour. `crew-theme`'s `contrast_thresholds` test holds both slots at
//! >= 4.5:1 against the page in every preset.
//!
//! Named `chatink`, not `chatpal`: `chatpalette.rs` is the slash-command
//! palette UI and the two are easy to confuse.
use crate::chatbody::Color;

/// The code card's background: the page nudged toward the ink colour, so the
/// block reads as a card in every theme without a dedicated theme slot.
pub(crate) fn code_bg() -> Color {
    let t = crew_theme::theme();
    crate::anim::lerp_rgb(t.page_bg, t.ink, 0.08)
}

/// Code text — fenced blocks and inline spans alike.
pub(crate) fn code_fg() -> Color {
    crew_theme::theme().ansi[6]
}

/// Structural marker glyphs: list bullets/ordinals and the blockquote bar.
pub(crate) fn marker_fg() -> Color {
    crew_theme::theme().ansi[3]
}

/// Quoted prose — one step back from body text.
pub(crate) fn quote_fg() -> Color {
    crew_theme::theme().text_muted
}

/// Headings, at every level.
pub(crate) fn heading_fg() -> Color {
    crew_theme::theme().ink
}

/// Link tint: reuse the terminal pane's own URL-highlight colour (`linkhl`)
/// so a link reads the same whether it's in a pane or a chat card.
pub(crate) fn link_color() -> Color {
    crate::linkhl::LINK_FG
}
```

Register it in `crates/crew-app/src/main.rs`. The `mod` list is alphabetical; insert between `mod chathdr;` and `mod chatinput;`:

```rust
mod chatink;
```

- [ ] **Step 4: Rewrite `chatmd.rs` to use it**

In `crates/crew-app/src/chatmd.rs`:

Delete the local `code_bg()` and `link_color()` functions entirely (they now live in `chatink`), and add `use crate::chatink;` alongside the existing imports.

Replace `span_style` with the version below, and add `body_span_style` after it. `span_style` keeps its current signature and its `LineKind::CodeHeader | CodeFooter | Rule`, `Code` and `Blank` arms; only the `Body` arm changes, delegating to the new function:

```rust
fn span_style(
    span: &MdSpan,
    kind: LineKind,
    fg: Color,
    muted: Color,
) -> (Color, bool, bool, Option<Color>, Option<Arc<str>>) {
    match kind {
        LineKind::CodeHeader | LineKind::CodeFooter | LineKind::Rule => {
            (muted, false, false, None, None)
        }
        LineKind::Code => (
            chatink::code_fg(),
            false,
            false,
            Some(chatink::code_bg()),
            None,
        ),
        LineKind::Blank => (fg, false, false, None, None),
        LineKind::Body => body_span_style(span, fg),
    }
}

/// Styles one prose span over `base` — the colour its plain text draws in.
/// Precedence, highest first: link, heading, inline code, then `base`. A span
/// can carry several of these at once (`# A [link](u)`), so the order is what
/// decides; it is checked top-down rather than accumulated, so each branch
/// states its whole result.
fn body_span_style(
    span: &MdSpan,
    base: Color,
) -> (Color, bool, bool, Option<Color>, Option<Arc<str>>) {
    let style = span.style;
    // Inline code inside a link keeps the code tint, as it did before.
    let code_bg = if style.code {
        Some(chatink::code_bg())
    } else {
        None
    };
    if let Some(url) = &span.link {
        return (
            chatink::link_color(),
            true,
            style.italic,
            code_bg,
            Some(Arc::from(url.as_str())),
        );
    }
    if style.heading >= 1 {
        return (chatink::heading_fg(), true, style.italic, code_bg, None);
    }
    if style.code {
        return (chatink::code_fg(), style.bold, style.italic, code_bg, None);
    }
    (base, style.bold, style.italic, None, None)
}
```

Note the behaviour change this encodes deliberately: headings were `ink` at h1/h2 and bold-only at h3+; now every level is `ink` + bold.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: PASS. Existing `chatbody` tests that assert heading or code styling may now be asserting the old colours — if one fails, it is asserting superseded behaviour: update it to the new expectation (compare against `crew_theme::theme()` slots, never a hardcoded triple). Do not weaken an assertion to `!= ` or delete it.

- [ ] **Step 6: Lint**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean. `marker_fg` and `quote_fg` have no callers until Task 4; if `dead_code` fires on them, do NOT add `#[allow(dead_code)]` and do NOT delete them — fold Task 4 into this commit instead, and say so in the commit message.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-app/src/chatink.rs crates/crew-app/src/chatmd.rs \
        crates/crew-app/src/chatmd_tests.rs crates/crew-app/src/chatbody.rs \
        crates/crew-app/src/main.rs
git commit -m "feat(chat): colour code and headings from the theme's own palette"
```

---

### Task 3: `LineKind::Quote` and `MdStyle.marker` in the md engine

Pure markdown-layout work: teach the engine to keep the two distinctions it currently discards. No colour here — Task 4 consumes this.

**Files:**
- Modify: `crates/crew-app/src/md/mod.rs` (add `LineKind::Quote`, add `MdStyle.marker`)
- Modify: `crates/crew-app/src/md/inline.rs` (the one `MdStyle { .. }` literal needs the new field)
- Modify: `crates/crew-app/src/md/wrap.rs` (add `marker_span`)
- Modify: `crates/crew-app/src/md/layout.rs` (`list_lines`, `quote_lines`)
- Modify: `crates/crew-app/src/chatmd.rs` (one arm, so the match stays exhaustive)
- Modify: `crates/crew-app/src/md/layout_tests.rs` (new tests)

**Interfaces:**
- Consumes: nothing from Tasks 1–2.
- Produces, for Task 4:
  - `md::LineKind::Quote` — a blockquote line.
  - `md::MdStyle.marker: bool` — `true` on a list bullet/ordinal span or a blockquote bar span, `false` on everything else.
  - `md::layout::wrap::marker_span(text: String) -> MdSpan` — `pub(super)`, internal to `md`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/md/layout_tests.rs` (its enclosing module already has `use super::*;`, which brings in `LineKind` and the `#[cfg(test)]` import of `render`):

```rust
#[test]
fn blockquote_lines_are_marked_as_quotes() {
    let out = render("> hi there", 40);
    assert_eq!(out[0].kind, LineKind::Quote);
    assert_eq!(out[0].spans[0].text, "▎ ");
    assert!(out[0].spans[0].style.marker, "the bar is a marker");
    assert!(
        out[0].spans[1..].iter().all(|s| !s.style.marker),
        "quoted text is not a marker"
    );
}

#[test]
fn fenced_code_inside_a_quote_keeps_its_code_kinds() {
    let out = render("> ```\n> x = 1\n> ```", 40);
    let kinds: Vec<LineKind> = out.iter().map(|l| l.kind).collect();
    assert!(kinds.contains(&LineKind::CodeHeader), "{kinds:?}");
    assert!(kinds.contains(&LineKind::Code), "{kinds:?}");
    assert!(kinds.contains(&LineKind::CodeFooter), "{kinds:?}");
    assert!(
        !kinds.contains(&LineKind::Quote),
        "a quoted code block is all code lines, no prose: {kinds:?}"
    );
}

#[test]
fn list_bullet_is_a_marker_but_its_text_is_not() {
    let out = render("- one", 40);
    assert_eq!(out[0].spans[0].text, "• ");
    assert!(out[0].spans[0].style.marker);
    assert!(out[0].spans[1..].iter().all(|s| !s.style.marker));
}

#[test]
fn wrapped_list_continuation_carries_no_marker() {
    // Bullet "• " is 2 cols, leaving 10: "aaaa bbbb" then "cccc dddd".
    let out = render("- aaaa bbbb cccc dddd", 12);
    assert!(out.len() > 1, "expected the item to wrap: {out:?}");
    assert!(
        out[1].spans.iter().all(|s| !s.style.marker),
        "continuation padding is not a marker"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app md::layout`
Expected: compile error — no `LineKind::Quote` variant and no `marker` field on `MdStyle`.

- [ ] **Step 3: Add the two model additions**

In `crates/crew-app/src/md/mod.rs`, add the field to `MdStyle`:

```rust
pub(crate) struct MdStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,  // inline code span
    pub heading: u8, // 0 = body text, 1..=6 = heading level
    /// A structural marker glyph — a list bullet/ordinal or a blockquote bar
    /// — rather than authored content. The chat renderer colours markers
    /// separately from the text they introduce.
    pub marker: bool,
}
```

and the variant to `LineKind`, directly after `Body`:

```rust
    Body,
    Quote,      // a line of blockquote prose (bar + quoted text)
```

In `crates/crew-app/src/md/inline.rs`, the `fn style(&self) -> MdStyle` literal now needs the field:

```rust
        MdStyle {
            bold: self.bold > 0,
            italic: self.italic > 0,
            code: false,
            heading: 0,
            marker: false,
        }
```

(The `MdStyle::default()` uses elsewhere need no change — `bool` defaults to `false`.)

- [ ] **Step 4: Add `marker_span`**

In `crates/crew-app/src/md/wrap.rs`, next to `plain_span`:

```rust
/// A structural marker glyph (list bullet/ordinal, blockquote bar) rather
/// than authored content — `chatmd` colours these separately.
pub(super) fn marker_span(text: String) -> MdSpan {
    MdSpan {
        text,
        style: MdStyle {
            marker: true,
            ..MdStyle::default()
        },
        link: None,
    }
}
```

- [ ] **Step 5: Mark the spans in `layout.rs`**

In `crates/crew-app/src/md/layout.rs`, extend the `wrap::` import to bring in the new helper:

```rust
use wrap::{marker_span, plain_span, split_hardbreaks, wrap_group};
```

In `list_lines`, the prefix span becomes a marker only on an item's first line — continuation lines get plain padding, which is not a marker:

```rust
                let mut spans = vec![if first {
                    marker_span(prefix.clone())
                } else {
                    plain_span(" ".repeat(prefix_len))
                }];
```

Replace `quote_lines` wholesale:

```rust
fn quote_lines(inner: Vec<Block>, cols: usize) -> Vec<MdLine> {
    const PREFIX: &str = "▎ ";
    let prefix_len = PREFIX.chars().count();
    let inner_cols = cols.saturating_sub(prefix_len).max(1);
    let mut sub = lines(inner, inner_cols);
    for line in sub.iter_mut() {
        if line.kind == LineKind::Blank {
            continue;
        }
        let mut spans = vec![marker_span(PREFIX.to_string())];
        spans.append(&mut line.spans);
        line.spans = spans;
        // ONLY prose becomes Quote. A fenced block inside a quote keeps its
        // Code/CodeHeader/CodeFooter kind so it still renders as a code card,
        // and a rule stays a rule — the bar is prepended to all of them, but
        // the kind is what decides how the line is drawn.
        if line.kind == LineKind::Body {
            line.kind = LineKind::Quote;
        }
    }
    sub
}
```

- [ ] **Step 6: Keep `chatmd`'s match exhaustive**

The new variant breaks the `match kind` in `crates/crew-app/src/chatmd.rs`. Give it the same treatment `Body` gets for now — Task 4 gives it its own colour:

```rust
        LineKind::Body | LineKind::Quote => body_span_style(span, fg),
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: PASS, including the four new tests. If an existing md test constructs `MdStyle { .. }` positionally or asserts on a full `MdLine` equality, update it to include `marker: false` — that is a mechanical fix, not a behaviour change.

- [ ] **Step 8: Lint**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-app/src/md crates/crew-app/src/chatmd.rs
git commit -m "feat(md): keep blockquote and list-marker structure through layout"
```

---

### Task 4: Colour quotes and markers

Consumes Task 3's plumbing to finish the palette.

**Files:**
- Modify: `crates/crew-app/src/chatmd.rs` (`map_lines` line colour, `span_style` quote arm, `body_span_style` marker branch)
- Modify: `crates/crew-app/src/chatmd_tests.rs` (new tests)

**Interfaces:**
- Consumes: `md::LineKind::Quote` and `md::MdStyle.marker` (Task 3); `chatink::marker_fg()` and `chatink::quote_fg()` (Task 2); `chatmd::body_span_style(span, base)` (Task 2).
- Produces: nothing — this is the last task.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/chatmd_tests.rs` (its helpers `row_text` and `lines` are already defined at the top of that file from Task 2):

```rust
#[test]
fn list_bullet_is_the_marker_colour_and_its_text_is_not() {
    let fg = (9, 9, 9);
    let out = lines("- one", 40, fg);
    assert_eq!(row_text(&out[0]), " • one");
    assert_eq!(out[0][1].fg, crew_theme::theme().ansi[3], "the bullet");
    assert_eq!(out[0][3].fg, fg, "the item text");
}

#[test]
fn quote_bar_is_the_marker_colour_and_quoted_text_is_muted() {
    let out = lines("> hi there", 40, (9, 9, 9));
    assert_eq!(row_text(&out[0]), " ▎ hi there");
    assert_eq!(out[0][1].fg, crew_theme::theme().ansi[3], "the bar");
    assert_eq!(out[0][3].fg, crew_theme::theme().text_muted, "quoted text");
}

#[test]
fn inline_code_inside_a_quote_is_still_code_coloured() {
    let out = lines("> use `let` here", 40, (9, 9, 9));
    let theme = crew_theme::theme();
    let coded: Vec<_> = out[0].iter().filter(|c| c.bg == Some(crate::chatink::code_bg())).collect();
    assert_eq!(coded.len(), 3, "l-e-t: {}", row_text(&out[0]));
    assert!(coded.iter().all(|c| c.fg == theme.ansi[6]));
}

#[test]
fn fenced_code_inside_a_quote_still_renders_as_code() {
    let out = lines("> ```\n> x = 1\n> ```", 40, (9, 9, 9));
    let theme = crew_theme::theme();
    let code_cells: Vec<_> = out
        .iter()
        .flatten()
        .filter(|c| c.bg == Some(crate::chatink::code_bg()))
        .collect();
    assert!(!code_cells.is_empty(), "a quoted fence still gets a code card");
    assert!(code_cells.iter().all(|c| c.fg == theme.ansi[6]));
}

#[test]
fn the_bar_of_a_quoted_fence_is_a_marker_not_code() {
    let out = lines("> ```\n> x = 1\n> ```", 40, (9, 9, 9));
    // Every line of the quoted block carries the bar, code lines included.
    let bar = out
        .iter()
        .find(|l| row_text(l).starts_with(" ▎"))
        .expect("a barred line");
    assert_eq!(bar[1].fg, crew_theme::theme().ansi[3], "the bar stays a marker");
    assert_eq!(bar[1].bg, None, "the bar takes no code tint");
}

#[test]
fn a_link_inside_a_list_item_keeps_the_link_colour() {
    let out = lines("- see [site](https://s.io)", 60, (9, 9, 9));
    let cell = out[0].iter().find(|c| c.link.is_some()).expect("a link cell");
    assert_eq!(cell.fg, crate::chatink::link_color());
    assert_ne!(cell.fg, crew_theme::theme().ansi[3], "not the marker colour");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app chatmd`
Expected: FAIL — the bullet and quote-bar cells come back as the prose `fg`, not `ansi[3]`, because nothing reads `MdStyle.marker` yet.

- [ ] **Step 3: Give markers and quotes their colours**

In `crates/crew-app/src/chatmd.rs`:

Split the combined arm from Task 3 so a quote line draws over the quote colour:

```rust
        LineKind::Body => body_span_style(span, fg),
        LineKind::Quote => body_span_style(span, chatink::quote_fg()),
```

Add the marker check at the very top of `span_style`, ABOVE the `match kind` — not inside `body_span_style`. A marker is chrome: it must win regardless of the line's kind, so that the `▎` bar prefixed to a fenced block inside a blockquote draws as a marker rather than picking up the `LineKind::Code` arm's code colour and background tint:

```rust
fn span_style(
    span: &MdSpan,
    kind: LineKind,
    fg: Color,
    muted: Color,
) -> (Color, bool, bool, Option<Color>, Option<Arc<str>>) {
    // Checked before `kind`: the quote bar is prefixed to EVERY line of a
    // quote, including the Code lines of a fenced block inside it, and a bar
    // drawn in code colour on a code tint would read as part of the code.
    if span.style.marker {
        return (chatink::marker_fg(), false, false, None, None);
    }
    match kind {
        // ... unchanged arms ...
    }
}
```

And in `map_lines`, give quote lines' indent/blank cells the quote colour, matching how chrome lines already take `muted`:

```rust
        let line_fg = match line.kind {
            LineKind::CodeHeader | LineKind::CodeFooter | LineKind::Rule => muted,
            LineKind::Quote => chatink::quote_fg(),
            _ => fg,
        };
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: PASS, all tests.

- [ ] **Step 5: Lint the whole workspace**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean, with no `#[allow]` added anywhere in this branch.

- [ ] **Step 6: Check the file budget**

Run: `wc -l crates/crew-app/src/chatmd.rs crates/crew-app/src/chatink.rs crates/crew-app/src/chatmd_tests.rs crates/crew-app/src/md/layout.rs`
Expected: each under ~200 lines. If `chatmd.rs` has gone over, move `body_span_style` and `span_cells` into a sibling `chatmdspan.rs` rather than leaving the file over budget.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-app/src/chatmd.rs crates/crew-app/src/chatmd_tests.rs
git commit -m "feat(chat): colour blockquotes and list markers"
```

---

## Verification (after all four tasks)

- [ ] `cargo test --workspace` — green.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — green.
- [ ] Launch `target/debug/crew` (a dev launch spawns ITSELF as the broker via `current_exe()`; never overwrite `~/.local/bin/crew`, which is the user's working install) and send `/smith` a prompt that returns a fenced code block, a list, a blockquote and a heading. Confirm code reads cyan-ish, bullets and quote bars read yellow-ish, headings read as ink.
- [ ] Rotate themes with `Ctrl+Shift+L` through several presets, including a CRT one. Confirm the palette shifts with the theme, that a CRT preset stays monochrome, and that no colour becomes unreadable.
- [ ] Toggle source mode (`Ctrl+Shift+M`) and confirm raw text is flat.
- [ ] Open `/md` on a markdown file and confirm it picked up the same palette.
