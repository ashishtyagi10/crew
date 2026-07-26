# Smith Pane Semantic Palette — Design

**Status:** approved 2026-07-26
**Scope:** chat-card markdown rendering (`crew-app`) + one theme test (`crew-theme`)

## 0. The problem

Every message body in the smith pane draws in essentially one colour. `chatmd::
span_style` hands the message's `fg` to almost everything. Only three things
differ today: code chrome and rules are `text_muted`, `h1`/`h2` are `ink` + bold,
and links are `linkhl::LINK_FG` + bold. A fenced Rust block reads as body text on
a faintly darker card (`code_bg()` = `page_bg` lerped 8% toward `ink`) — code is
not visually distinct from prose.

The user asked for code to be a different colour from normal text. Agreed scope:
a **full semantic palette** — code, headings, blockquotes, list markers and rules
each get a colour identity. Explicitly **not** syntax highlighting: one colour
meaning "this is code", not five colours inside the code. No tokenizer, no
`syntect`, no `tree-sitter` (neither is in the workspace).

## 1. Colour source: the per-theme ANSI palette

**Decision: read semantic colours from the active theme's `ansi` slots. Do not
add colour fields to `Theme`, and do not derive them by lerping `accent`/`ink`.**

There are 13 presets (`presets_paper` 5, `presets_paper_light` 4, `presets_crt`
4). Every new `Theme` field is 13 hand-tuned values to keep in sync forever.

The originally-proposed alternative — derive from `accent_default`/`status_fg` +
lerp — was rejected on evidence: `PAPER_DARK.accent_default` is `(240, 240, 240)`,
i.e. white. On the default theme, an accent-derived code colour is grey on a grey
page, which is the exact complaint being fixed.

Each preset already ships 16 hand-tuned ANSI colours, chosen against that
theme's own page. Two properties make them the right source:

- **They are already theme-correct in all 13 presets, for free.**
- **They preserve monochrome themes automatically.** `CRT_GREEN`'s "cyan" is
  `(0, 255, 200)` and its "yellow" is `(200, 255, 80)` — still green. A
  single-phosphor tube does not sprout a foreign hue.

Measured WCAG contrast of each candidate slot against its own `page_bg`, worst
case across all 13 presets:

| slot | worst preset | ratio |
| --- | --- | --- |
| `ansi[2]` green | SEPIA_LIGHT | 6.06 |
| `ansi[3]` yellow | IVORY_LEDGER | 4.64 |
| `ansi[4]` blue | SEPIA_LIGHT | 6.97 |
| `ansi[6]` cyan | SEPIA_LIGHT | 5.85 |

Every candidate clears 4.5:1 everywhere — comfortably past the 3.0 floor
precedent set in crew-term's fg/bg flooring. The two slots this design uses are
locked in by a test (§5).

## 2. The palette

Chosen assignment — cool code, warm structure, so the two never compete, and
code sits far from the existing link blue:

| Element | Colour | Weight |
| --- | --- | --- |
| Fenced code content | `ansi[6]` cyan, on `code_bg()` | normal |
| Inline `` `code` `` span | `ansi[6]` cyan, on `code_bg()` | normal |
| List bullet / ordered marker | `ansi[3]` yellow | normal |
| Blockquote `▎` bar | `ansi[3]` yellow | normal |
| Blockquote text | `text_muted` | as authored |
| Heading, **all** levels h1–h6 | `ink` | bold |
| Code chrome (`╭─ lang`, `╰─`) | `text_muted` | unchanged |
| Horizontal rule | `text_muted` | unchanged |
| Link | `linkhl::LINK_FG` | bold, unchanged |
| Body prose | message `fg` | unchanged |

Headings change from "`ink` at h1/h2, bold-only at h3+" to "`ink` + bold at every
level" — a unification, not a new hue.

## 3. Architecture

### 3.1 `crew-app/src/chatpal.rs` (new)

One home for chat colour derivation. `code_bg()` and `link_color()` move here
verbatim from `chatmd.rs`; four siblings join them:

```rust
pub(crate) fn code_fg() -> Color     // theme().ansi[6]
pub(crate) fn marker_fg() -> Color   // theme().ansi[3]
pub(crate) fn quote_fg() -> Color    // theme().text_muted
pub(crate) fn heading_fg() -> Color  // theme().ink
```

Each reads `crew_theme::theme()` at call time, so a live `/theme` switch repaints
correctly with no cache to invalidate — the same contract `code_bg()` already has.

### 3.2 The md model gains two distinctions it currently discards

`md::LineKind` is `Body | CodeHeader | Code | CodeFooter | Rule | Blank`.
Blockquotes and list items both flatten into `Body` before `chatmd` sees them,
even though `md::parse` models them properly (`Block::BlockQuote`,
`Block::List`/`ListItem`). Two additions:

- **`LineKind::Quote`** — a line belonging to a blockquote.
- **`MdStyle.marker: bool`** — this span is a structural marker glyph (a list
  bullet or a quote bar), not content. `MdStyle` is `Copy + Default`, so the
  field defaults to `false`; only 3 `MdStyle` literals exist in the tree.

Both are additive. No existing variant or field changes meaning.

### 3.3 `md/layout.rs` marks spans it already builds separately

`list_lines` and `quote_lines` already construct the marker text as its own
`plain_span`, so marking is surgical:

- `list_lines`: the bullet/ordinal prefix span gets `marker = true`. Continuation
  lines (`" ".repeat(prefix_len)`) do **not** — they are padding, not a marker.
- `quote_lines`: the `▎ ` bar span gets `marker = true`, and each inner line is
  rewritten `Body → Quote`.

**Invariant:** `quote_lines` rewrites the kind of `Body` lines only. A fenced
block inside a blockquote keeps `Code`/`CodeHeader`/`CodeFooter` and renders as a
code card; a `Rule` inside a quote stays a rule; `Blank` is skipped as today.

### 3.4 `chatmd.rs` maps kind → colour

`Body` and `Quote` share one styling function that differs only in base colour —
prose `fg` for `Body`, `quote_fg()` for `Quote` — so bold, italic, inline code and
links behave identically inside and outside a quote. Precedence within that
function, highest first:

1. `marker` → `marker_fg()`
2. link → `link_color()` + bold
3. `heading >= 1` → `heading_fg()` + bold
4. `style.code` → `code_fg()` + `code_bg()` background
5. otherwise → the base colour

A span cannot be both a marker and content: markers are `plain_span`s with a
default `MdStyle`, so rules 2–4 cannot fire on them.

`LineKind::Code` moves from `fg` to `code_fg()`, keeping `code_bg()`.
`CodeHeader`, `CodeFooter`, `Rule` and `Blank` are untouched.

`map_lines`'s `line_fg` — the colour of the one-column indent cell and of empty
lines — becomes `quote_fg()` for `Quote` lines, matching the existing treatment
where chrome lines take `muted`.

## 4. Blast radius

- **`/md` viewer** (`mdcache.rs:65`) calls the same `chatmd::map_lines`, so it
  inherits the palette. Intended: one look for markdown everywhere.
- **Streaming tail** (`chattail.rs`) already overwrites every cell's `fg` with
  `text_muted` after rendering, so it stays uniformly dim. No change. (It does
  not clear `bg`, so a code tint can still show through — pre-existing, out of
  scope.)
- **Source mode** (Ctrl+Shift+M, `chatbody::source_lines`) builds all-plain cells
  and never enters `map_lines`. Raw text stays flat — required.
- **Terminal panes** are untouched; this is chat-card rendering only.

## 5. Testing

**crew-theme — the guard that matters.** Extend `contrast_thresholds`
(`lib_tests.rs`) with `ansi[3]` and `ansi[6]` ≥ 4.5:1 against `page_bg`, asserted
across `ALL_THEMES`. This is what stops a future preset from shipping an
unreadable code or marker colour; the palette's correctness rests on it.

**md layout** — structure, not colour:
- a blockquote's lines come back as `LineKind::Quote`
- a fenced block inside a blockquote still comes back as `Code` with its
  `CodeHeader`/`CodeFooter`
- a list's bullet span has `marker == true`; the item's text span does not
- a wrapped list item's continuation line has no `marker` span

**chatmd** — actual cell colours, read against the active theme:
- a fenced code line's cells are `ansi[6]` with `code_bg()`
- an inline code span's cells are `ansi[6]` with `code_bg()`; the prose around it
  is not
- a bullet cell is `ansi[3]`; the item text is the message `fg`
- a quote bar cell is `ansi[3]`; the quoted text is `text_muted`
- an inline code span *inside* a blockquote is still `ansi[6]`
- an `h3` heading's cells are `ink` + bold
- a link inside a list item is still the link colour, not the marker colour

**chatbody** — source mode produces no coloured cells for the same input.

## 6. Non-goals

- Syntax highlighting of any kind.
- New `Theme` colour fields.
- Per-language colour variation.
- Re-colouring terminal panes or the streaming tail.
- Table cell colouring beyond what heading/bold already give.
