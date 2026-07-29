# The file viewer pane (Phase 1) — design

**Date:** 2026-07-28
**Goal:** One zoomed pane that opens *any* file where you already are — code,
markdown, data, csv, diffs — with an honest floor under the formats a cell grid
has no business rendering. Read-only; editing hands off to `$EDITOR`.

Phase 2 (markdown edited in the render, markers never visible) is specified
separately and **must not start before this ships**.

## Today

- **`/md <path>`** (`spawnmd.rs`) opens any UTF-8-readable file as a
  **source | preview split**, zoomed, and focuses it. A `.rs` file therefore
  gets a correct source half and a nonsense markdown preview beside it.
  Non-UTF-8 is refused with a status line and no pane.
- **`MdPane`** (`mdpane.rs`, `mdpane_view.rs`, `mdcache.rs`, `mdkeys.rs`) is the
  only file pane. `MdAction` is `{ Close, Status(String) }`; `Side`/Tab toggles
  which half takes scroll.
- **`/far` F3 View and F4 Edit both call the OS.** `open_selected`
  (`farpane/keys.rs:385`) returns `FarAction::Open(path)` for both keys, and a
  remote file downloads first. Nothing is ever shown inside crew.
- **Cmd/Ctrl+click** on a file token in a terminal pane opens `$EDITOR`
  (`clickopen.rs`, reusing `/edit`); in a chat pane it only follows markdown
  link URLs.
- **No format awareness exists.** `md/syntax.rs` highlights comment/string/
  keyword, but only inside fenced code blocks in markdown, and only line-wise
  (documented: a string spanning lines highlights on its opening line only).
- **Session v2** (`sessionsave.rs`) persists kinds `shell` | `far` | `crew`.
  File panes are not restored.

## Decision

One pane kind, one format ladder, markdown rendered only.

### 1. Module layout

A `viewpane/` directory mirroring `farpane/`'s shape, files kept small and each
with a sibling `*_tests.rs` as the crate does throughout:

| File | Owns |
|---|---|
| `viewpane/mod.rs` | `ViewPane` model, `LoadState`, scroll offsets |
| `viewpane/detect.rs` | `path + bytes → Format`. **Pure.** |
| `viewpane/load.rs` | worker-thread load + extractor argv. Argv is **pure**. |
| `viewpane/render.rs` | `cells()` per rung |
| `viewpane/keys.rs` | `reduce() → ViewAction` |
| `viewpane/md.rs` | the Markdown rung — `MdCache`'s preview side, full width |

The plan splits this further (`pane.rs` for the model, `lines.rs` for rung →
`CardLine`, `render.rs` for `CardLine` → `CellView`, `csv.rs`, `search.rs`) to
stay under the ~200-line ceiling the crate keeps. Six files or nine, the
boundaries above are the ones that matter.

`PaneContent::Markdown(MdPane)` becomes `PaneContent::View(ViewPane)`.
`MdPane`'s split geometry, `Side`, Tab and the numbered source half are
**deleted**; its preview layout and `MdCache` move into `viewpane/md.rs`
unchanged, including the invariants that half already honours (display-width
walks via `chatwidth::char_w`, `RefCell` cache keyed on `cols`, stored offsets
clamped by `clamp_scrolls` and not merely the view).

Call sites to update, complete list: `pane.rs:39,114,134`, `keys.rs:188`,
`scroll.rs:62`, `clickopen.rs:99`, `clipboard.rs:90`, `poll.rs:193`,
`windowtitle.rs:23`, `askbar.rs:205`, `spawnmd.rs`.

### 2. The format ladder

```rust
pub(crate) enum Format {
    Code { lang: &'static str },   // rs, ts, py, go, c, sh, …
    Markdown,                      // md, markdown, mdx
    Data { lang: &'static str },   // json, yaml, toml, ini
    Csv { delim: char },           // csv, tsv
    Diff,                          // diff, patch
    Extract { via: Extractor },    // docx/doc/rtf/odt → textutil; pdf → pdftotext
    Opaque { why: Opaque },        // Binary | NoExtractor | NotUtf8
}
```

Detection, in priority order:

1. **Extension table** — the common case, a static lookup.
2. **Content sniff** for extensionless or unknown files: a NUL byte in the
   first 8 KB → `Opaque::Binary`; `#!` → `Code`; a `diff --git` or `@@ -` head
   → `Diff`.
3. **UTF-8 validity** — invalid → `Opaque::NotUtf8`.

The binary sniff **outranks the extension**, and only for the binary verdict: a
file named `.md` that is actually a JPEG is `Opaque`, but a `.md` full of odd
text stays `Markdown`.

`Extract` is chosen by extension, then **downgraded to `Opaque::NoExtractor`**
if the tool is not on `PATH`. macOS ships `/usr/bin/textutil`, so docx/doc/rtf/
odt work with nothing installed; PDF has no built-in CLI, so `pdftotext`
(poppler) is probed and its absence names what to install. **A missing tool
degrades a rung; it never errors.**

### 3. Loading, off the winit thread

This is the invariant everything else is arranged around. A 40 MB log, a `stat`
on a stalled network mount, or `pdftotext` over 300 pages executed on the winit
thread freezes **every pane in the grid**, agents included.

```rust
pub(crate) enum LoadState {
    Loading { since_ms: u64, rx: Receiver<LoadResult> },
    Ready(Content),
    Failed(String),
}
```

`load::start(path) -> Receiver<LoadDone>` spawns a thread exactly as
`farpane/run.rs::start` does; `poll_panes` drains it each tick alongside the
Far drains. The pane opens **immediately** in `Loading` with a skeleton, so a
slow file never blocks the keystroke that asked for it.

**Detection runs on that worker too**, which is why `start` takes only a path.
Classifying a file needs the head of its bytes, and reading those bytes is I/O
like any other; deciding the rung on the winit thread would reintroduce exactly
the freeze this section exists to prevent. `LoadDone` therefore carries the
`Format` back alongside the text.

Extractor argv is pure and unit-tested with nothing installed, following
`farpane/rclone.rs`'s split of argv-construction from execution:

```rust
fn argv_textutil(p: &Path)  -> Vec<String> // -convert txt -stdout <p>
fn argv_pdftotext(p: &Path) -> Vec<String> // -layout <p> -
```

**`poll.rs:25` gains an arm.** `wants_animation_frame` is an asserted invariant
("an idle crew never repaints", 0.9.0) — a `Loading` pane must appear there to
animate its skeleton, and must stop the moment it goes `Ready`. A test pins
both directions.

### 4. Rendering

`ViewPane::cells(cols, rows)`, zero-size-guarded like `MdPane::cells` is today.
Shared across rungs: a line-number gutter (`Code`, `Data`, `Csv`, `Diff` only —
not `Markdown`, `Extract`, or the `Opaque` card), a **banner row** when the
content is an extract or truncated, display-width wrapping, and scroll offsets
clamped to content.

| Rung | Drawn as | Reuses |
|---|---|---|
| `Code`, `Data` | gutter + comment/string/keyword ink | `md/syntax.rs` |
| `Markdown` | today's preview half at **full width** | `viewpane/md.rs`, `md/` |
| `Csv` | column-aligned table, header ruled | `md/table.rs::lines` |
| `Diff` | `+`/`−` ink from the theme | `crew_theme::theme()` |
| `Extract` | plain text under `text extract — o opens <app>` | — |
| `Opaque` | metadata card: size, kind, mtime, and what `o` will do | — |

Colour comes from `crew_theme::theme()` (`ink`, `text_muted`, `page_bg`) as
`mdpane_view.rs` already does — **no new colours**, per the standing rule.

Two reuse details that are not free calls. `md/table.rs::lines` is `pub(super)`
and takes markdown AST (`&[Vec<MdSpan>]`), so the `Csv` rung parses rows and
wraps each cell as a single plain `MdSpan`, and `md` re-exports `lines` to the
viewer — the column-width, padding and header-rule logic is what is being
reused, not a CSV renderer that does not exist. `md/syntax.rs`'s line-wise
limitation carries over unchanged and is not worth fixing here: a multi-line
string highlights on its opening line only.

### 5. Size cap that says what it did

`MAX_VIEW_BYTES = 8 MB`. Over it the pane shows the **first 8 MB** under a
banner naming the real size and offering `o` — it does not refuse. Capping what
is *shown* rather than what may be *opened* is the same call made for `@file`
line ranges (ledger iteration 55), and every cap owes the user a visible answer
(iteration 56).

### 6. Keys

```rust
pub(crate) enum ViewAction {
    Close,                    // Esc — restores prior focus and zoom
    Status(String),
    Edit(PathBuf),            // e — $EDITOR in a term pane
    OpenExternal(PathBuf),    // o — the OS default app
    Reload,                   // r
}
```

Arrows / PgUp / PgDn / Home / End scroll. `/` opens an in-pane search with
`n`/`N` (matching is a pure `find_matches(content, needle)`), `g`/`G` jump to
top/bottom. `s` toggles a **read-only raw source** view — the escape hatch for
the times the render is the thing you are debugging.

### 7. The `$EDITOR` handoff, and the reload that closes the loop

`e` spawns a normal crew terminal pane running `$EDITOR <path>` and records the
originating viewer. `poll_panes` already tracks each `TermPane`'s foreground
`cmd` (via `procname`); when the editor pane's command ends or its pane closes,
the viewer **reloads if it is still open on that path**. Without that, the
handoff silently shows stale content and reads as a bug.

The spawned pane goes through the existing run-pane wrapper — which is
`/bin/bash`, deliberately, because `zsh -c 'set -m; …'` creates a process group
without ever `tcsetpgrp`-ing it, and a full-screen editor reading the tty would
stop instantly.

### 8. Entry points — one function, four doors

`CrewApp::open_view(path)` is the single entry; every door routes to it.

| Door | Change |
|---|---|
| `/view <path>` | new `Cmd` in `cmddefs.rs` |
| `/md <path>` | kept as an alias; description becomes "view a file"; markdown still renders |
| `/far` **F3** | `open_selected` returns a new `FarAction::View(path)` instead of `Open` |
| `/far` **F4** | returns `FarAction::Edit(path)` → `$EDITOR` in a term pane, no longer `open` |
| Cmd+click | a file token in a **terminal or chat** pane opens the viewer instead of `$EDITOR` |

Directories keep their current behaviour on every door. A remote Far file still
downloads first, then views.

Agent-cited paths in a `/smith` reply are handled by the **same click path**,
resolved lazily: `clickopen::token_at` already extracts the token and checks
existence at click time, so nothing is `stat`ed per render. Cmd+click gaining a
new meaning is an accepted change to existing muscle memory — `e` is one key
from the viewer.

### 9. Errors

A pre-open failure (missing, unreadable, no permission) sets the status line
naming the file and the reason, and opens **no pane** — today's behaviour. A
failure *after* the pane exists is `LoadState::Failed` drawn in the pane
itself, with `o` and `r` still live; a pane that is already on screen must not
report its failure only to a status line the user may never look at.

### 10. Session persistence

`SavedPane` gains kind `view` with its `path`. `valid()` requires the path to
still exist, so a viewer on a deleted file is dropped rather than restored
empty. Restoring re-runs the loader off-thread like any other open. This is the
**last task and the droppable one** — the pane works without it.

## Testing

- `detect`: table-driven over extensions, sniffs, the binary-outranks-extension
  rule, and the extractor downgrade with the tool absent.
- `load`: argv purity (no tools installed anywhere in CI), truncation banner
  text, failure surfacing.
- `render`: per-rung cell assertions, the zero-size guard, `clamp_scrolls`
  after a jump to the end.
- `keys`: `reduce()` table.
- `poll`: a `Loading` pane makes `wants_animation_frame` true; a `Ready` pane
  does not.
- routing: `/far` F3/F4 and Cmd+click each land in `open_view`.
- The GUI `verify` skill for the live pane, against the real app.

## Non-goals

Editing of any kind (Phase 2 covers markdown, and markdown only). Page
rendering of PDF or Word. In-cell image decode. `syntect` or `tree-sitter`.
Code editing in the pane — ever: code editing wants vim, LSP and multi-cursor,
and building a worse IDE is this feature's named failure mode.
