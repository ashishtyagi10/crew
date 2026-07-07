# Markdown Phase 1 — Engine + Chat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A markdown engine (`crates/crew-app/src/md/`) and Chat-pane integration: messages render as full markdown, Ctrl+Shift+M toggles raw source, Cmd/Ctrl+click opens links.

**Architecture:** `pulldown-cmark` parses to an event stream; `md/parse.rs` folds events into blocks; `md/layout.rs` wraps blocks into styled `MdLine`s at a column budget. Chat maps `MdLine`→`CardLine` (existing cell pipeline; `CellView` already supports bold/italic — chat merely stops hardcoding `italic: false`). Link hit-testing is stateless: on modified click, re-derive the pane's placed lines and look up the span. Spec: `docs/superpowers/specs/2026-07-06-markdown-renderer-design.md`.

**Tech Stack:** Rust, pulldown-cmark (only new dependency), existing crew cell/theme pipeline.

## Global Constraints

- Source files ≤200 lines (tests files may exceed; several do). Prefer new focused modules over growing files.
- Only new dependency: `pulldown-cmark = "0.13"` in `[workspace.dependencies]`, consumed by crew-app. Nothing else.
- TDD every task: failing test first, watch it fail for the right reason, implement, watch it pass. `cargo fmt` before commit (pre-commit runs fmt check + cargo check).
- Test command: `cargo test -p crew-app <filter>`.
- `md::render` is pure (no theme access); colors are applied only in chat mapping code.
- Code-block chrome must stay visually identical to today: ` ╭─ <lang>` header (muted), verbatim hard-wrapped lines on `code_bg()` (page lerped 8% toward ink), ` ╰─` footer, one-column indent (see `chatbody.rs:57-84`).
- Existing behavior that must not regress: terminal-pane Cmd+click (`clickopen.rs`), chat scroll/input, all 547 current crew-app tests.

---

### Task 1: `md` model + parser (`md/mod.rs`, `md/parse.rs`)

**Files:**
- Modify: `/Cargo.toml` (workspace `[workspace.dependencies]`: add `pulldown-cmark = "0.13"`), `crates/crew-app/Cargo.toml` (add `pulldown-cmark.workspace = true`)
- Create: `crates/crew-app/src/md/mod.rs`, `crates/crew-app/src/md/parse.rs`
- Modify: `crates/crew-app/src/main.rs` or the module root that declares mods (grep `mod chatbody;` and add `mod md;` beside it)
- Test: `crates/crew-app/src/md/parse_tests.rs` (`#[cfg(test)] #[path = "parse_tests.rs"] mod tests;` at the bottom of parse.rs — repo pattern)

**Interfaces (Produces — later tasks depend on these exact shapes):**

```rust
// md/mod.rs
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) struct MdStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,     // inline code span
    pub heading: u8,    // 0 = body text, 1..=6 = heading level
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct MdSpan {
    pub text: String,
    pub style: MdStyle,
    pub link: Option<String>, // absolute URL this span links to
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LineKind {
    Body,
    CodeHeader, // "╭─ lang" chrome line (chat draws it muted, no bg)
    Code,       // verbatim code content (chat draws it on code_bg)
    CodeFooter, // "╰─"
    Rule,       // horizontal rule
    Blank,      // paragraph separator
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct MdLine {
    pub spans: Vec<MdSpan>,
    pub kind: LineKind,
}

pub(crate) fn render(text: &str, cols: usize) -> Vec<MdLine>; // parse + layout (layout lands Task 2; until then mod.rs exposes parse-only internals)
```

```rust
// md/parse.rs — internal block model consumed by layout.rs (Task 2)
#[derive(Debug, PartialEq)]
pub(super) enum Block {
    Paragraph(Vec<MdSpan>),               // inline-styled spans, no wrapping yet
    Heading(u8, Vec<MdSpan>),
    CodeBlock { lang: String, lines: Vec<String> },
    List(Vec<ListItem>),                  // ListItem { ordered_idx: Option<u64>, depth: u8, spans: Vec<MdSpan> }
    BlockQuote(Vec<Block>),
    Table { header: Vec<Vec<MdSpan>>, rows: Vec<Vec<Vec<MdSpan>>> },
    Rule,
}
pub(super) fn parse(text: &str) -> Vec<Block>;
```

Parser requirements (implementer designs the event-fold internals; these tests are the contract):
- `Parser::new_ext` with `Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH`.
- Inline styles nest (`**bold _italic_**` → span with bold+italic). Strikethrough → `italic: true` on the span (cells have no strikethrough; spec'd fallback).
- `[text](url)` → span(s) with `link: Some(url)`. Bare `http(s)://…` in prose also become link spans (pulldown autolink is off for bare URLs — post-process paragraph spans with the same span-detection logic as `openurl::url_spans`, reimplemented on `&str` in parse.rs; do NOT import openurl, keep md/ self-contained).
- Soft breaks → space; hard breaks → separate paragraph lines are handled in layout (parse keeps `\n` markers by splitting into separate Paragraph spans lists is NOT required — represent a hard break as a dedicated `MdSpan { text: "\n", .. }` and let layout split on it).
- Never panics on any input.

- [ ] **Step 1: Write failing tests** — `parse_tests.rs`, complete battery (write exactly these, plus any you find necessary):

```rust
use super::*;

#[test]
fn paragraph_inline_styles_nest() {
    let blocks = parse("plain **bold _both_** `code`");
    let Block::Paragraph(spans) = &blocks[0] else { panic!("not a paragraph: {blocks:?}") };
    let texts: Vec<(&str, bool, bool, bool)> = spans.iter()
        .map(|s| (s.text.as_str(), s.style.bold, s.style.italic, s.style.code)).collect();
    assert!(texts.contains(&("bold ", true, false, false)), "{texts:?}");
    assert!(texts.contains(&("both", true, true, false)), "{texts:?}");
    assert!(texts.contains(&("code", false, false, true)), "{texts:?}");
}

#[test]
fn heading_levels_carry_through() {
    let blocks = parse("## Two\n\ntext");
    assert!(matches!(&blocks[0], Block::Heading(2, s) if s[0].text == "Two"));
}

#[test]
fn fenced_code_keeps_verbatim_lines_and_lang() {
    let blocks = parse("```rust\nfn x() {}\n  indented\n```");
    assert_eq!(blocks[0], Block::CodeBlock {
        lang: "rust".into(),
        lines: vec!["fn x() {}".into(), "  indented".into()],
    });
}

#[test]
fn markdown_link_and_bare_url_become_link_spans() {
    let blocks = parse("see [docs](https://ex.am/d) and https://ex.am/raw now");
    let Block::Paragraph(spans) = &blocks[0] else { panic!() };
    let links: Vec<(&str, &str)> = spans.iter()
        .filter_map(|s| s.link.as_deref().map(|u| (s.text.as_str(), u))).collect();
    assert_eq!(links, vec![("docs", "https://ex.am/d"), ("https://ex.am/raw", "https://ex.am/raw")]);
}

#[test]
fn nested_and_ordered_lists_carry_depth_and_index() {
    let blocks = parse("- a\n  - b\n1. one");
    let items: Vec<(Option<u64>, u8, String)> = blocks.iter().flat_map(|b| match b {
        Block::List(items) => items.iter()
            .map(|i| (i.ordered_idx, i.depth, i.spans.iter().map(|s| s.text.clone()).collect::<String>()))
            .collect::<Vec<_>>(),
        _ => vec![],
    }).collect();
    assert!(items.contains(&(None, 0, "a".into())), "{items:?}");
    assert!(items.contains(&(None, 1, "b".into())), "{items:?}");
    assert!(items.contains(&(Some(1), 0, "one".into())), "{items:?}");
}

#[test]
fn blockquote_wraps_inner_blocks() {
    let blocks = parse("> quoted");
    assert!(matches!(&blocks[0], Block::BlockQuote(inner)
        if matches!(&inner[0], Block::Paragraph(s) if s[0].text == "quoted")));
}

#[test]
fn table_splits_header_and_rows() {
    let blocks = parse("| a | b |\n|---|---|\n| 1 | 2 |");
    let Block::Table { header, rows } = &blocks[0] else { panic!("{blocks:?}") };
    assert_eq!(header.len(), 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][1][0].text, "2");
}

#[test]
fn rule_and_strikethrough() {
    let blocks = parse("---\n\n~~gone~~");
    assert!(matches!(blocks[0], Block::Rule));
    assert!(matches!(&blocks[1], Block::Paragraph(s) if s[0].style.italic));
}

#[test]
fn hard_break_becomes_newline_span() {
    let blocks = parse("a  \nb"); // two trailing spaces = hard break
    let Block::Paragraph(spans) = &blocks[0] else { panic!() };
    assert!(spans.iter().any(|s| s.text == "\n"), "{spans:?}");
}

#[test]
fn garbage_never_panics() {
    for s in ["", "``", "**", "[a](", "|", ">>>", "#".repeat(300).as_str(),
              "\u{0}\u{fffd}*_`[", "- \n- \n  1. \n```"] {
        let _ = parse(s);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p crew-app md::parse` → compile error (module absent). Expected RED.
- [ ] **Step 3: Implement** — add the dependency lines; `md/mod.rs` with the model above and `mod parse;` (a temporary `render` that returns `Vec::new()` is acceptable this task ONLY if Task 2's brief replaces it — instead prefer: mod.rs declares the model + `pub(super) use` and NO render yet; add `#[allow(dead_code)]` where needed to keep clippy/fmt clean); `md/parse.rs` folding the pulldown-cmark events. Register `mod md;` alongside the other mods.
- [ ] **Step 4: Verify green** — `cargo test -p crew-app md::parse` all pass; `cargo test -p crew-app` still green; `cargo tree -p crew-app | grep pulldown` shows exactly one new crate (plus its `unicase`/`memchr`-class transitive deps; no `getopts` — do not enable pulldown's binary features).
- [ ] **Step 5: Commit** — `git add Cargo.toml Cargo.lock crates/crew-app/Cargo.toml crates/crew-app/src/md/ <mod-root-file>` ; message `feat(crew): markdown parser — pulldown-cmark event fold to styled blocks`

---

### Task 2: layout — `md::render(text, cols)` (`md/layout.rs`)

**Files:**
- Create: `crates/crew-app/src/md/layout.rs`; Modify: `crates/crew-app/src/md/mod.rs` (add `mod layout;` + the real `render`)
- Test: `crates/crew-app/src/md/layout_tests.rs`

**Interfaces:**
- Consumes: Task 1's `Block`/`ListItem`/`MdSpan`/`MdStyle`.
- Produces: `pub(crate) fn render(text: &str, cols: usize) -> Vec<MdLine>` in `md/mod.rs` delegating `parse` → `layout::lines(blocks, cols)`.

Layout rules (the tests below are the contract; implementer designs internals):
- Prose word-wraps at `cols` preserving span styles across wrap points; a span may split mid-way. Wrap on spaces (same semantics as `chatlayout::wrap_indices`: break at last space that fits, hard-split a word longer than the budget). Hard-break spans (`"\n"`) force a line break.
- Headings: single logical line (wrapped if long), every span `heading: level, bold: true`.
- Lists: `• ` bullet (or `1. ` etc. for ordered) at `depth*2` indent; continuation lines get hanging indent aligned under the text start.
- Blockquote: each inner line prefixed with a `▎ ` span (style default); nesting stacks prefixes.
- Code blocks: `CodeHeader` line with spans `[MdSpan("╭─ <lang or "code">")]`, `Code` lines verbatim (hard-chunked at `cols` — no character dropped; chunk by char count, layout is width-naive here because chat re-chunks with `chatwidth::fit_end`; NO — keep it simple and single-source: layout hard-chunks by char count and chat consumes as-is; wide-glyph over-run is accepted for phase 1 and noted), `CodeFooter` `[MdSpan("╰─")]`.
- Tables: column widths = max cell char-width per column (header included), cells space-padded, ` │ ` separators; header line spans bold; a `Rule`-kind line of `─` under the header; table wider than `cols` → each line hard-truncated at `cols` (no wrap).
- Rules: `LineKind::Rule` with one span of `─` × cols.
- Blank `LineKind::Blank` between blocks (exactly one; none leading/trailing).
- `cols < 4` → degrade: plain unwrapped truncation, never panic.

- [ ] **Step 1: Failing tests** (`layout_tests.rs` — write exactly these plus what you need):

```rust
use super::*;

fn flat(l: &MdLine) -> String { l.spans.iter().map(|s| s.text.as_str()).collect() }

#[test]
fn prose_wraps_and_styles_survive_the_wrap() {
    let lines = render("**bold text that must wrap across lines**", 12);
    assert!(lines.len() >= 2);
    assert!(lines.iter().filter(|l| l.kind == LineKind::Body)
        .all(|l| l.spans.iter().all(|s| s.style.bold)), "style lost at wrap");
    assert!(lines.iter().all(|l| flat(l).chars().count() <= 12));
}

#[test]
fn heading_line_is_marked_and_bold() {
    let lines = render("# Title", 40);
    assert_eq!(lines[0].spans[0].style.heading, 1);
    assert!(lines[0].spans[0].style.bold);
    assert_eq!(flat(&lines[0]), "Title");
}

#[test]
fn code_block_chrome_lines() {
    let lines = render("```rust\nfn x() {}\n```", 40);
    let kinds: Vec<LineKind> = lines.iter().map(|l| l.kind).collect();
    assert_eq!(kinds, vec![LineKind::CodeHeader, LineKind::Code, LineKind::CodeFooter]);
    assert_eq!(flat(&lines[0]), "\u{256d}\u{2500} rust");
    assert_eq!(flat(&lines[1]), "fn x() {}");
    assert_eq!(flat(&lines[2]), "\u{2570}\u{2500}");
}

#[test]
fn code_hard_chunks_verbatim() {
    let lines = render("```\nlet a = 1;\n```", 6);
    let code: String = lines.iter().filter(|l| l.kind == LineKind::Code).map(flat).collect();
    assert_eq!(code, "let a = 1;");
}

#[test]
fn lists_indent_and_number() {
    let lines = render("- a\n  - b\n\n1. one", 40);
    let texts: Vec<String> = lines.iter().filter(|l| l.kind == LineKind::Body).map(|l| flat(l)).collect();
    assert!(texts.contains(&"• a".to_string()), "{texts:?}");
    assert!(texts.contains(&"  • b".to_string()), "{texts:?}");
    assert!(texts.contains(&"1. one".to_string()), "{texts:?}");
}

#[test]
fn blockquote_prefixes() {
    let lines = render("> quoted words", 40);
    assert!(flat(&lines[0]).starts_with("\u{258e} "), "{}", flat(&lines[0]));
}

#[test]
fn table_aligns_and_bolds_header() {
    let lines = render("| a | bb |\n|---|---|\n| ccc | d |", 40);
    let texts: Vec<String> = lines.iter().map(|l| flat(l)).collect();
    assert_eq!(texts[0], "a   \u{2502} bb");
    assert!(lines[0].spans.iter().any(|s| s.style.bold));
    assert!(texts[1].starts_with('\u{2500}'));
    assert_eq!(texts[2], "ccc \u{2502} d ");
}

#[test]
fn link_spans_survive_layout() {
    let lines = render("go to [site](https://s.io) now", 40);
    let link: Vec<&MdSpan> = lines[0].spans.iter().filter(|s| s.link.is_some()).collect();
    assert_eq!(link[0].text, "site");
    assert_eq!(link[0].link.as_deref(), Some("https://s.io"));
}

#[test]
fn blank_lines_separate_blocks_exactly_once() {
    let lines = render("a\n\nb", 40);
    let kinds: Vec<LineKind> = lines.iter().map(|l| l.kind).collect();
    assert_eq!(kinds, vec![LineKind::Body, LineKind::Blank, LineKind::Body]);
}

#[test]
fn byte_soup_never_panics_and_respects_cols() {
    let soups = ["\u{0}*[`|>#-~", "𓀀𓀁𓀂 **𓀃** https://𓀄", "a".repeat(10_000)];
    for s in soups.iter() {
        for cols in [1usize, 4, 13, 80] {
            for l in render(s, cols) {
                if l.kind != LineKind::Code { // code is verbatim-chunked by chars
                    assert!(flat(&l).chars().count() <= cols.max(1));
                }
            }
        }
    }
}
```

- [ ] **Step 2: RED** — `cargo test -p crew-app md::layout` → compile error (no `render`/`layout`).
- [ ] **Step 3: Implement** `layout.rs` (+ wire `render` in mod.rs). Keep each fn small; if layout.rs would exceed 200 lines, split tables into `md/table.rs`.
- [ ] **Step 4: GREEN** — `cargo test -p crew-app md` all pass; full `cargo test -p crew-app` green.
- [ ] **Step 5: Commit** — `feat(crew): markdown layout — styled blocks to wrapped MdLines`

---

### Task 3: CardCell italic+link; placed-lines refactor in chatmsgs

**Files:**
- Modify: `crates/crew-app/src/chatbody.rs:10-29` (struct + `plain`), `crates/crew-app/src/chatmsgs.rs:112-151` (`message_cells`)
- Test: extend `chatmsgs_tests.rs` / `chatbody` inline tests

**Interfaces:**
- Produces: `CardCell { c, fg, bold, italic: bool, bg, link: Option<std::sync::Arc<str>> }` (new fields default `false`/`None`; update `plain()` accordingly — all existing construction sites compile via `plain`/struct literals you update in this task).
- Produces: `chatmsgs::placed_lines(pane: &ChatPane, cols: u16, rows_budget: u16) -> Vec<(u16 /*abs row*/, CardLine)>` — the scroll-windowed placement that `message_cells` currently computes inline; `message_cells` becomes a thin map over it (same output as today — pin with a test), and Task 6's link hit-test reads it.
- `chatmsgs.rs:145`'s `italic: false` becomes `italic: cell.italic`.

- [ ] **Step 1: Failing tests** — (a) a CardCell with `italic: true` produces a `CellView` with `italic: true` through `message_cells` (construct a minimal ChatPane with one message — mirror existing chatmsgs_tests setup); (b) `placed_lines` × thin-mapper equivalence: for a pane with 3 messages, `message_cells` output equals mapping `placed_lines` cells (write the equivalence as: every CellView emitted has a (row, col) present in placed_lines' coverage and vice versa).
- [ ] **Step 2: RED** — `cargo test -p crew-app chatmsgs` → compile error (no `italic` field / no `placed_lines`).
- [ ] **Step 3: Implement** — struct fields; refactor; thread italic. Every `CardCell{...}` literal in chatbody.rs/chatmsgs.rs gains `italic: false, link: None` except where meaningful.
- [ ] **Step 4: GREEN** — `cargo test -p crew-app` fully green (547+ tests — this touches shared types; the compiler finds every site).
- [ ] **Step 5: Commit** — `feat(crew): CardCell carries italic and link; chat placement extracted`

---

### Task 4: chat preview renders through md::render

**Files:**
- Modify: `crates/crew-app/src/chatbody.rs` (`body_lines` internals), delete `crates/crew-app/src/chatcode.rs` + its `mod` declaration
- Test: chatbody inline tests (port the 4 existing ones — they must still pass byte-for-byte on the new path) + new ones

**Interfaces:**
- Consumes: `md::render`, Task 3's CardCell.
- `body_lines(text, cols, fg)` signature unchanged (callers untouched). Internals: `md::render(text, width)` then map `MdLine`→`CardLine`: one-column indent cell first (as today); `LineKind::CodeHeader/CodeFooter` → muted fg, no bg; `Code` → `code_bg()` bg; `Body` spans → fg from style (heading 1-2 → `theme().ink` + bold; heading ≥3 → bold only; default → `fg` param), `bold`/`italic` from MdStyle, inline-code spans get `code_bg()` bg; link spans → bold + fg `theme().link` if the theme has a link slot, else `lerp_rgb(fg, (66,133,244), 0.6)` — check `crew_theme::Theme` fields first and use an existing link/accent slot if present (grep `linkhl` for how terminal link tint gets its color; reuse that source); `link` Arc set from `MdSpan.link`; `Rule` → muted `─` line; `Blank` → empty line.

- [ ] **Step 1: Failing tests first** — port the four `chatbody` tests verbatim (they pin chrome parity: `newlines_split_prose_into_lines`, `code_block_gets_borders_language_tag_and_bg`, `untagged_fence_is_labelled_code`, `long_code_lines_hard_wrap_verbatim` — note the last one asserts `l.len() <= 6` per line; the md path must chunk to the same budget) plus new: `bold_survives_to_cardcells` (`**hi**` → cells bold), `heading_is_bold`, `link_cells_carry_url` (cells for "site" have `link.as_deref() == Some("https://s.io")`), `bullet_list_renders`. RED: run `cargo test -p crew-app chatbody` — the four ported tests currently PASS on the old path; the new ones FAIL (no md path yet). That is the correct RED shape: new tests red, parity tests green before AND after.
- [ ] **Step 2: Implement** the mapping; delete chatcode.rs (its parsing tests died with it — parity moved to md::parse's fence test + chatbody's ported tests).
- [ ] **Step 3: GREEN** — `cargo test -p crew-app` fully green. Manually sanity-diff one message: `cargo run --release` optional, skip in CI.
- [ ] **Step 4: Commit** — `feat(crew): chat messages render full markdown via md engine`

---

### Task 5: Ctrl+Shift+M source toggle

**Files:**
- Modify: `crates/crew-app/src/chat.rs` (ChatPane field), `crates/crew-app/src/chatbody.rs` or `chatmsgs.rs` call site (source branch), `crates/crew-app/src/keys.rs:78-100` (chord), pane title (`pane.rs:63-87` chat arm)
- Test: keys/chat tests

**Interfaces:**
- `ChatPane.show_source: bool` (default false, not persisted).
- keys.rs, directly under the Ctrl+Shift+L block (same shape, `keys.rs:81-88`):

```rust
        // Ctrl+Shift+M toggles markdown source view on the focused chat pane.
        if event.state.is_pressed()
            && mstate.control_key()
            && mstate.shift_key()
            && matches!(&event.logical_key, Key::Character(s) if s.eq_ignore_ascii_case("m"))
        {
            if let Some(p) = self.focused_pane_mut() {
                if let PaneContent::Chat(c) = &mut p.content {
                    c.show_source = !c.show_source;
                }
            }
            self.redraw();
            return;
        }
```

(If `focused_pane_mut` doesn't exist, use the same focused-pane access `toggle_zoom`/key routing uses — grep `self.focus` in keys.rs:141-155.)
- Source branch: where message bodies are built (the `body_lines` call in chatmsgs.rs), `if pane.show_source` → plain verbatim word-wrapped lines (reuse the old prose path: newline-split + `wrap_indices`, all cells `plain(c, fg, false)`), else md path.
- Title: chat arm of `title_text` appends `" · source"` when `show_source`.

- [ ] **Step 1: Failing tests** — (a) chat pane with a `**bold**` message: `show_source=false` → some cell bold; `show_source=true` → NO cell bold and the literal `**bold**` chars appear in cells; (b) title suffix test.
- [ ] **Step 2: RED**, **Step 3: implement**, **Step 4: GREEN** (`cargo test -p crew-app`), **Step 5: Commit** — `feat(crew): Ctrl+Shift+M flips chat between markdown preview and raw source`

---

### Task 6: Cmd/Ctrl+click opens chat links

**Files:**
- Modify: `crates/crew-app/src/events.rs:30-34`, `crates/crew-app/src/clickopen.rs`, `crates/crew-app/src/chatview.rs` (or chatmsgs) — `link_at`
- Test: `clickopen`/chatview tests

**Interfaces:**
- Platform modifier helper in events.rs (or a small `fn open_modifier(mods) -> bool`): `cfg!(target_os = "macos")` → `super_key()`, else `control_key()`. Terminal-pane behavior keeps its existing macOS `super_key()` path — on non-mac, `control_key()` now also drives it (this is the user-requested Windows behavior; note it in the commit message).
- `chatview::link_at(pane: &ChatPane, cols: u16, rows: u16, row: u16, col: u16) -> Option<String>` — derive `chatmsgs::placed_lines(...)` (Task 3) with the same geometry `chatview::cells` uses for the message area (same `top_rows`/scroll math — extract shared placement geometry rather than duplicating constants), index the CardLine at `row`, return `cells[col].link` as String.
- `clickopen::cmd_click_at_cursor` grows a chat branch: after the terminal `cursor_cell()` miss, resolve pane at cursor + (row, col) via the same rect math as `openurl::cursor_cell` (`openurl.rs:51-71`) but for Chat panes (extract the pixel→(row,col) math into a helper `cursor_rowcol(&self, i) -> Option<(i32, i32)>` shared by both) → `chatview::link_at` → `open::that(url)` + status line, return true.
- events.rs:31 changes from `self.mods.state().super_key()` to the platform helper.

- [ ] **Step 1: Failing tests** — (a) unit: pane with message `see [d](https://x.io/p)` at known geometry → `link_at` on the link's (row,col) returns the url; off-link (row,col) returns None; (b) scrolled pane still resolves (set `pane.scroll` > 0, assert the visible link resolves at its shifted row). Test geometry: build the pane, call `chatview::cells(pane, cols, rows)` and locate the link text's row/col from the returned CellViews (search cells for the char run "d"), then assert `link_at` at that position — this keeps the test independent of layout constants.
- [ ] **Step 2: RED** — no `link_at`. **Step 3: implement** (helper extraction + branch). **Step 4: GREEN** — `cargo test -p crew-app` full green (openurl/clickopen tests must not regress). **Step 5: Commit** — `feat(crew): cmd/ctrl+click opens markdown links in chat panes`

---

### Task 7: E2E sanity + install (ops-leaning; controller may run inline)

- [ ] `cargo test --workspace` — everything green.
- [ ] `cargo build --release`; `cp target/release/crew ~/.local/bin/crew`.
- [ ] Manual smoke note for the user: restart app → any agent reply with markdown renders styled; Ctrl+Shift+M flips it; Cmd+click a link in chat opens the browser. (GUI harness screenshot optional: `osascript` + `screencapture` per repo memory.)

## Self-Review Notes

- Spec §1→Tasks 1-2; §2→Tasks 3-5; §3→Task 6; §5 error/no-panic→T1 `garbage_never_panics` + T2 `byte_soup_never_panics_and_respects_cols`; §6 testing map is embedded per task. §4 (viewer) is phase 2 — not in this plan.
- Type consistency: `MdStyle/MdSpan/MdLine/LineKind` (T1) consumed T2/T4; `CardCell.italic/.link` (T3) consumed T4/T6; `placed_lines` (T3) consumed T6; `render` signature identical T2/T4.
- Known intentional deviations from "complete code": md/parse.rs + md/layout.rs internals are specified by contract + full test batteries instead of verbatim code (the plan author cannot transcribe correct pulldown-cmark event-fold code without running it; the tests are the binding spec). Dispatch those implementers on a standard model, not the cheapest tier.
