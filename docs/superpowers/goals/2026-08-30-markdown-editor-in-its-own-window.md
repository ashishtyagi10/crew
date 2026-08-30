# Goal — a markdown file opens in its own crew window, already rendered, and you type into the render

**Set:** 2026-08-30 by the user: *"open Markdown viewer into another window of crew. Make it a
Markdown editor in preview mode — the user should immediately see markdown editor properly
rendered."*

**Extends:** `docs/superpowers/goals/2026-07-28-file-viewer-and-markdown-editing.md`. Phase 1 (the
viewer) shipped 2026-07-31. Phase 2 (edit in the render) was specified there and never started;
this goal takes it up, adds the thing that goal did not have — **a second window** — and settles the
question that goal left implicit: what you see the *instant* the file opens.

Three sentences of state of the world:

1. **Crew has exactly one window.** `CrewApp` holds `window: Option<Arc<Window>>` and
   `renderer: Option<Renderer>` — one each, both `Option` because they exist only after
   `resumed()`. `window_event` takes a `WindowId` and **discards it** (`_id`,
   `handler.rs:168`), so every event from every window would land on the same app state.
2. **The markdown rung is read-only.** `/view` renders markdown properly today; `e` shells out to
   `$EDITOR` in a terminal pane and reloads on exit. There is no cursor in the render, and
   `md::parse::Block` carries no source provenance, so nothing could be written back if there were.
3. **A picture is no longer a chip.** The 2026-07-28 goal declined in-cell image decoding and made
   `![alt](src)` a selectable placeholder. As of 0.19.80–81 crew draws real pictures on the paint
   layer, in the viewer and in a terminal pane. **An image in an edited document is the image.**

THE GOAL, in one line: **`/view README.md` puts the file in a window of its own, rendered, with a
cursor already in it — you type and what you typed is what you see, and the file on disk stays a
file a human wrote.**

---

### Pillar 1 — a window is a second canvas, not a second app

A crew window is *the whole canvas*: a grid of panes, a nav, an input bar. A second window is
another one of those, in the same process, sharing one broker, one theme, one config, one font
atlas — and owning its own grid, its own focus, and its own surface.

What has to become plural, named exactly:

- **`Renderer` per window.** `Gpu::new(window)` builds an instance, adapter, device, queue and
  surface per call. The device and queue are the expensive half and are shareable; the surface,
  its configuration and the CRT/bloom targets are per-window and are not. The split is
  `Gpu` → `GpuShared { instance, adapter, device, queue }` + `WindowGpu { surface, config, format }`,
  which is also what makes a second window cost a surface rather than a second GPU context.
- **`WindowId` stops being discarded.** `window_event`'s `_id` becomes the key into a
  `Vec<CrewWindow>`; the handler routes, the window state acts. This is the change that has to
  happen first and the one that touches the most call sites — every `self.window`/`self.renderer`
  read in `crew-app` is a read of "the window this event was for".
- **What is per-window and what is per-app** has to be decided once, written down, and enforced by
  where the field lives. Per-window: panes, grid, focus, zoom, input bar, sidebar, toasts, hint
  mode, the glide clock. Per-app: config, theme, palette, the broker connection, the todo store,
  the usage ledger, the font database. A field in the wrong half is a bug that only shows up with
  two windows open, which is the whole reason to write the list before the code.
- **Closing the last window quits; closing any other does not.** Session restore learns to restore
  *windows*, each with its panes, or it silently collapses two windows into one on the next launch.

The auto-tiling guardrail is untouched: each window tiles its own panes into its own near-square
grid. This adds no layout system — it adds a second canvas for the one layout system there is.

### Pillar 2 — the document window: rendered on arrival, cursor already in it

`/view` on a markdown file (and `/md`, Cmd+click on a `.md` path, `/far` F3 on one) opens a **new
window** containing that document, maximized to its own canvas, with the rendered document filling
it. Not a split, not a source pane, not a preview toggle: **the render is the document**, and the
cursor is in it before you touch anything. "Preview mode" as the user asked for it means exactly
this — the state you arrive in is the finished-looking one, and editing does not change the view.

- **No markers on screen, ever.** No `**`, no `#`, no `](`. Cmd+B on a selection sets bold; the
  heading level is a property of the block, not a prefix you type. The markers are a
  *serialization*, produced on save.
- **Images are drawn.** `![alt](src)` resolves against the document's directory and draws through
  the same paint-layer path the viewer's image rung uses, decoded on a worker. A picture in a
  README is a picture while you edit around it.
- **The window says what the cursor is inside.** A link's URL is invisible in a render; the status
  line names it while the cursor is in one, and Cmd+K edits it.
- **Esc closes the window** (and Cmd+W closes the pane, as everywhere else), after the same
  unsaved-changes gate any editor owes you.
- A non-markdown file still opens the way it does today, in a pane of the current window. This
  goal is not "every file gets a window".

### Pillar 3 — the buffer is the parsed document, and untouched bytes never move

Carried forward from the 2026-07-28 goal because it is still the decision that makes this
buildable, and still the one that makes it survive contact with a git repo:

- **The buffer is the parsed document, not the bytes.** The cursor is a document position; bold is
  a span attribute; markdown is produced on save. Inverting every edit back into byte offsets is
  where the character-level ambiguity and the weeks of work live.
- **Every block carries the source byte range it came from**, which pulldown-cmark hands over for
  free through `Parser::into_offset_iter()` in place of the plain `Parser` that `md/parse.rs` folds
  today. On save, only blocks whose node changed are re-serialized; everything else is spliced back
  **verbatim**.
- **The test that proves it:** open a 400-line spec, change one sentence, save — `git diff` shows
  one changed line. A save that rewrites `*` bullets to `-`, setext headings to ATX and rewraps
  every paragraph is a hostile diff from a coding tool, and it is what every naive round-trip does.
- Marker conventions are sniffed from the file on load (a document already writing `*italic*` keeps
  getting `*italic*`), defaulting to `**`/`_`.
- A fenced code block has nothing to invert: it edits as plain monospace text, in its own field.

### What "done" looks like

1. Two crew windows, each with its own panes and focus, one process, one broker; events go to the
   window they came from; closing one leaves the other running; a restored session brings both back.
2. `/view notes.md` opens a window showing the rendered document, cursor in it, no markers on
   screen, images drawn.
3. Typing, Enter, Backspace, selection, Cmd+B/I, link editing and Tab-through-table-cells all act
   on the render and read correctly.
4. Cmd+S writes markdown whose diff against the original touches only what was edited — asserted by
   a test over a real repo document, not a fixture written for the purpose.
5. Everything above holds at `/motion off`, on a light theme, and in a window narrow enough that
   the document has to wrap.

### The ways this ships half-done

- **The `WindowId` refactor is skipped** "for now" by opening the document in a pane instead. That
  is a different feature; it is the one crew already has.
- **Per-window state is left global.** Two windows then share a focus, a zoom or a toast, and the
  bug is reported as "crew is haunted".
- **The editor ships without the byte-range splice.** It will look finished and produce diffs
  nobody can review — worse than the read-only viewer it replaced.
- **Phase 2 starts before the window exists.** The window is the smaller, more separable half and
  it is what the user actually asked for first; a WYSIWYG editor inside the existing pane grid
  answers a question that was not asked.

---

### Decision taken in implementation (2026-08-30, v0.19.89) — the buffer

Pillar 3 above says the buffer is the parsed document and markdown is a
serialization produced on save. **Built the other way round, and here is why.**

The reason Pillar 3 gives for its model is sound — inverting a rendered edit
back into bytes is where the ambiguity lives — but it needs source byte ranges
on every block *anyway*, to splice untouched blocks back verbatim. Once the
provenance exists, the simpler arrangement is available: **the buffer is the
source text, and the cursor is a byte offset in it.** Then

* a typed character is a splice at that offset — untouched bytes do not merely
  *tend* not to move, they cannot move, and the minimal-diff guarantee is
  structural rather than something the serializer has to keep promising;
* there is no serializer at all, so there is no marker-convention sniffing, no
  `*` vs `-` bullet drift, no re-wrapping;
* what the cursor cannot do is exactly what the render cannot account for,
  which turns out to be the right rule rather than a limitation (below).

What is carried is one field: `MdSpan.src` / `CardCell.src`, the byte a
rendered character came from, split with the span when a line wraps. It is
`None` wherever the render is **not** a verbatim copy of the file — an entity
(`&amp;` is one character from five bytes), an escape, the space CommonMark
puts where a soft break was, and every glyph the renderer invented (a bullet,
a table rule, a code field's border). A caret cannot stand in those places,
which is correct: there is nothing there to type into. Claiming an offset that
is four bytes out would be far worse than admitting there is none.

The invariant is asserted directly, at three widths over the whole grammar:
for every rendered character, the byte it claims holds that character.

Pillar 3's other half stands unchanged: on save, only what was edited moves.
