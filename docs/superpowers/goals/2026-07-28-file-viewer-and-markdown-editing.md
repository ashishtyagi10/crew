# Goal — read any file where you already are; write markdown as it looks

**Set:** 2026-07-28 by the user.
**Phase 1 spec:** `docs/superpowers/specs/2026-07-28-file-viewer-pane-design.md`

Every file crew shows you today it shows you badly or not at all. `/md <path>` opens ANY readable
file as a source|preview split — so a `.rs` gets a correct source half and a nonsense markdown
preview beside it — and `/far`'s F3 View and F4 Edit are both a lie: they call `open` and hand the
file to the OS (`farpane/keys.rs:127`), throwing you out of the app you were working in. The value
here is not rendering. It is that a file opens NEXT TO THE AGENT THAT MENTIONED IT, on the same
keyboard, in the same theme, inside the grid you were already reading.
NOT EVERY FORMAT DESERVES A PANE, and saying so is half the design. A PDF or a `.docx` rendered
into a monospace cell grid is a strictly worse Preview.app — no fonts, no images, no pagination.
Those rungs get TEXT EXTRACTION, labelled honestly as an extract, with `o` to open the real app one
key away. macOS ships `/usr/bin/textutil` (docx/doc/rtf/odt) for free; PDF has no built-in CLI, so
poppler's `pdftotext` is PROBED, never required — a missing tool DEGRADES A RUNG, it never errors.

### Phase 1 — the viewer (ships first, alone)
`PaneContent::View`, opened zoomed like `/md` and closed with Esc back to the prior focus and zoom.
ONE enum, ONE match, each rung honest about what it is: Code (line numbers + `md/syntax.rs`),
Markdown (RENDERED ONLY — see below), Data (json/yaml/toml), Csv (via `md/table.rs`), Diff,
Extract (banner + `o`), Opaque (metadata card + `o`). `MdPane` FOLDS IN as one rung rather than
living alongside — two half-viewers is how this rots. Entry points all route to one function:
`/view <path>` (`/md` an alias), `/far` F3, Cmd+click a path in any pane, and agent-cited paths in
a /smith reply. `e` spawns `$EDITOR` in a normal term pane and the viewer RELOADS when it exits.
THE SOURCE HALF IS DELETED. Displaying markdown source next to its render is a dev tool wearing a
reading experience's clothes. Markdown renders, full stop; `s` toggles a read-only raw view for the
times a table misbehaves.

### Phase 2 — markdown edits in the render, markers never visible
The cursor lives in rendered text. No `**` on screen, ever; Cmd+B sets bold; the file is markdown
only when it is written. THE SUBSTITUTION THAT MAKES THIS BUILDABLE: do NOT keep bytes as the
buffer and invert every edit back into them — that is where the character-level marker ambiguity
and the weeks of work live. The BUFFER IS THE PARSED DOCUMENT (ProseMirror's model): cursor is a
document position, bold is a span attribute, markdown is a SERIALIZATION produced on save.
THE INVARIANT THAT KEEPS IT USABLE IN A GIT REPO: untouched bytes never move. `Block` today carries
NO source provenance (`Paragraph(Vec<MdSpan>)`, no range anywhere) — it gains the source BYTE RANGE
it came from, which pulldown-cmark hands over for free via `Parser::into_offset_iter()` in place of
the plain `Parser` `md/parse.rs` folds today. On save, ONLY blocks whose node changed are
re-serialized, the rest
spliced back VERBATIM. Edit one sentence in a 400-line spec, get a one-sentence diff. The same
field the cursor needs to find its way home is what makes saving non-destructive; one field, two
jobs. Without it, opening and saving a README rewrites `*` bullets to `-`, setext headings to ATX,
and rewraps every paragraph — a hostile diff, from a coding tool.
WHAT WYSIWYG DEMANDS THAT A RENDERER DOES NOT, each of which is a way to ship this half-done:
a link's URL is INVISIBLE in the render, so it shows in the status bar while the cursor is inside
one and Cmd+K edits it (the attach-popup card pattern); a code fence has nothing to invert and
edits as plain monospace text; tables edit cell-wise with Tab; `![alt](src)` is a selectable chip
(in-cell image decode was considered and declined). Marker conventions are SNIFFED from the file on
load — a doc already writing `*italic*` keeps getting `*italic*` — defaulting to `**`/`_`. Typing
at a style boundary inherits the run to its LEFT. Undo is a bounded ring of document snapshots with
typing runs coalesced; Cmd+S saves, the legend carries the dirty marker, closing dirty asks.
NON-NEGOTIABLE, and the one that bites hardest: every read, every `stat`, every extractor
subprocess and every SAVE stays off the winit thread (the blocking rule). A 40 MB log or a
`pdftotext` over 300 pages on the main thread freezes EVERY pane in the grid, agents included — so
the pane owns a real Loading/Ready/Failed state and registers with `wants_animation_frame` while it
waits. Second: code files stay READ-ONLY with the `$EDITOR` handoff even after the editing core
exists — markdown is prose and prose wants rendering, but code editing wants vim, LSP and
multi-cursor, none of which this pane will ever have. Building a worse IDE is the failure mode
here. Third: nothing pretends to be a page renderer, and Phase 2 does not start before Phase 1 has
shipped.
