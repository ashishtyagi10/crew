# Markdown renderer — chat preview + file viewer — design

2026-07-06. A full Markdown renderer for the crew GUI, on two surfaces: the
Chat pane renders agent/broker messages as markdown (with a raw-source
toggle), and a new `/md <path>` viewer pane shows a markdown file as
side-by-side source | preview. Cmd+click (macOS) / Ctrl+click (Windows/Linux)
on a rendered link opens it in the system browser. The viewer pane opens
full-width/full-height (zoomed).

Decisions fixed with the user: both surfaces; links open in the system
browser (the OS/browser owns window sizing — best-effort, `open::that`);
side-by-side split in the file viewer only — chat is preview-first with a
whole-pane source toggle.

Grounding: everything in crew renders as a flat `Vec<CellView>` per pane, and
`CellView` already carries `bold`/`italic` (crew-render `celltext.rs:125-133`
maps them to cosmic-text Weight/Style; advances are cell-snapped so styling
never shifts the grid). So this feature is two cell-producing layers plus a
parser — zero crew-render changes.

## 1. Parser & engine — new `crates/crew-app/src/md/` module

**Dependency:** `pulldown-cmark` (pinned in `[workspace.dependencies]`,
default features minus SIMD as needed), added to crew-app only. First
markdown crate in the workspace; hand-rolling full CommonMark was rejected
(unbounded edge-case tail), as was extending the ad-hoc `chatcode.rs`
(not "full markdown").

Module layout (≤200 lines per file):

- `md/mod.rs` — public model + entry point:
  - `MdStyle { bold: bool, italic: bool, code: bool, heading: u8 /*0=body*/ }`
  - `MdSpan { text: String, style: MdStyle, link: Option<String> }`
  - `MdLine { spans: Vec<MdSpan>, kind: LineKind }` where
    `LineKind ∈ { Body, CodeBlock { lang_header: bool }, Rule, Blank }`
  - `pub(crate) fn render(text: &str, cols: usize) -> Vec<MdLine>` — parse +
    wrap in one call; pure, unit-testable, no theme/color knowledge.
- `md/parse.rs` — pulldown-cmark `Parser` event stream → block tree
  (headings 1–6, paragraphs, bullet/ordered lists with nesting, blockquotes,
  fenced/indented code with language, tables, rules, inline
  bold/italic/code/links/strikethrough — strikethrough renders dim-italic
  since cells have no strikethrough).
- `md/layout.rs` — blocks → wrapped `MdLine`s at a column budget: prose
  word-wraps (reuse `chatlayout::wrap_indices` semantics); list bullets
  (`•`, `1.`) with hanging indent (2 spaces per nest level); blockquote `▎ `
  prefix; code blocks verbatim hard-wrapped; tables rendered as
  column-aligned text (header row bold, `─` separator), overwide tables
  hard-wrapped; headings single-line, `#`-free.

Color is applied at the consumer (chat/viewer), mapping `MdStyle` +
`crew_theme::theme()`: headings bold (+H1/H2 use `ink` at full strength,
lower levels muted), inline code + code blocks reuse chat's existing dimmed
`code_bg` treatment (`chatbody.rs:33-36` today), links tinted like the
terminal link tint (`linkhl`) and underlined-equivalent (bold+tint — cells
have no underline flag).

## 2. Chat pane: markdown preview + source toggle

- `CardCell` (`chatbody.rs:7-19`) gains `italic: bool`; `chatmsgs.rs:145`
  threads it into `CellView` instead of the hardcoded `false`.
- `chatbody::body_lines` re-routes: **preview mode** feeds the whole message
  body through `md::render(text, cols)` and maps `MdLine`→`CardLine`
  (spans→cells, theme colors applied here); code blocks keep today's chrome
  (`╭─ lang` header, dimmed bg, `╰─` footer) emitted by the mapping layer so
  the visual language is unchanged. `chatcode.rs` becomes dead for preview
  and is removed once parity tests pass (its fence tests port to md/).
  **Source mode** shows the raw text verbatim, word-wrapped, no styling.
- Toggle: `ChatPane.show_source: bool` (default false). Global chord
  **Ctrl+Shift+M** in `keys.rs` flips it on the focused Chat pane (pattern:
  the Ctrl+Shift+L theme chord, `keys.rs:81-88`). Echoed in the pane title
  (`· source` suffix) so state is visible.
- Only message bodies render as markdown; header lines (`▍sender · time`)
  and system chrome are untouched.

## 3. Links: Cmd/Ctrl+click in chat and viewer

- Modifier is platform-split: `super_key()` on macOS, `control_key()`
  elsewhere (matches user ask; terminal panes keep their existing behavior).
- Hit-testing is **stateless, on demand** (mirrors `openurl.rs`'s
  read-the-grid approach): on modified click over a Chat/Md pane,
  re-derive that pane's current lines (same code path as `cells()`), find
  the span under (row, col), and if it carries `link: Some(url)` →
  `open::that(url)` (`clickopen.rs:41` precedent). No cached link maps, no
  frame-state invalidation bugs.
- `events.rs:25-52` gains the dispatch: existing terminal branch first,
  then Chat/Md branch. Non-link modified clicks fall through to today's
  behavior (selection/focus).
- Both markdown link syntax (`[text](url)`) and bare `http(s)://` URLs in
  prose are clickable (bare URLs get auto-link spans in `md/parse.rs`).

## 4. Viewer pane: `/md <path>` side-by-side source | preview

- New `PaneContent::Markdown(MdPane)` variant (`pane.rs:29-35`) + arms in
  the three variant matches (`title_text`, `cells`, key routing).
- `MdPane { path: PathBuf, source: String, active: Side, scroll_src: usize,
  scroll_prev: usize }` — skeleton mirrors FarPane's two-panel split
  (`farpane/mod.rs:87-99`): left half raw source (line numbers, verbatim,
  hard-wrapped), right half rendered preview (`md::render` at half-width
  cols), `│` divider column, Tab switches the active side, wheel/keys scroll
  the active side independently. Cells are hand-built `CellView`s (chat
  path), not the ratatui bridge — same styled-span mapping code as chat.
- Spawn: `spawn_md_pane(path)` in the spawn module — reads via
  `std::fs::read_to_string` with status-bar error on failure
  (`spawn_batch_pane` precedent, `spawn.rs:189-215`), pushes the pane, and
  sets `self.zoomed = true` so the viewer opens **full width/height** (the
  user's sizing ask; Ctrl+Shift+F/double-click un-zooms as usual). Re-read
  on demand: `r` while focused reloads the file.
- Command surface: `/md <path>` arm in `dispatch.rs` (strip_prefix pattern,
  `dispatch.rs:43-61`), `suggest::COMMANDS` entry with trailing-space fill
  (arg-expanding, `suggest.rs:81-86`). Relative paths resolve against
  `self.cwd`.

## 5. Errors & edge cases

- Unreadable/binary/non-UTF-8 file → status-bar error, no pane spawned.
- Pathological markdown must never panic: `md::render` is pure and fuzzed by
  a property-style test (arbitrary byte-soup strings → no panic, output
  lines ≤ cols). pulldown-cmark carries the CommonMark edge cases.
- Zero-width panes (cols < 4): render degrades to truncated plain text.
- Huge files: viewer renders lazily per visible rows from precomputed lines;
  initial parse of multi-MB files is accepted (one-shot, done at spawn —
  winit-thread budget note: parse of a 1 MB doc with pulldown-cmark is
  single-digit ms; anything larger is out of scope).

## 6. Testing

- `md/` unit tests: every block type, inline nesting (`**bold _italic_**`),
  link extraction (markdown + bare URL), wrap correctness at narrow cols,
  table alignment, no-panic property test.
- Chat: parity tests — a fenced-code message renders the same chrome as the
  old `chatcode` path (port its tests); italic threading test
  (CardCell→CellView); source-toggle test (same message, raw vs rendered);
  link hit-test unit test (row/col → url).
- Viewer: spawn test (file → pane, zoomed set), split layout test (line
  budgets per side at a given grid), scroll clamp tests, Tab side-switch
  test, reload test, error-path test (missing file → no pane + status
  message).
- GUI smoke (manual, harness): `/md README.md` screenshot + Cmd+click a
  link.

## 7. Phasing — two plans

1. **Phase 1 — engine + chat**: `md/` module + dependency, CardCell italic,
   chat preview rendering, Ctrl+Shift+M source toggle, Cmd/Ctrl+click links
   in chat.
2. **Phase 2 — viewer**: `PaneContent::Markdown`, `/md` command + suggest
   entry, side-by-side split, zoomed spawn, scrolling/Tab/reload, viewer
   link clicks.

Each phase lands as its own reviewed branch; phase 2 consumes only `md/`'s
public model from phase 1.
