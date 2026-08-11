# Goal — todo pane: every `@project` gets its own color

**Status: SET 2026-08-10** by the user: each project tag in the todo pane should render
in its own distinct color, so a mixed list reads by project at a glance. Not started.

**What exists today:** every `@tag` renders in the single user accent —
the row chip (`todopane/render.rs::place_right`), the live composer tint
(`tag_spans` → the accent branch of the style closure), and the composer legend when a
filter is active. The filter header row (`@crew · 3 items`) is flat `text_muted`. With
one accent, `@home` and `@crew` items are indistinguishable without reading.

## Proposed decisions (defaults taken while drafting — veto before work starts)

- **Stateless hash, not stored assignment.** The color comes from hashing the tag name —
  no `color` field in `todos.toml`, nothing to migrate, every pane and restart agrees
  for free. Tradeoff: two tags can collide on a slot (fine at typical tag counts with
  ~8 chromatic slots); user-pinned colors are stretch, and the hash leaves room for
  them later. Hash the **lowercased** name — tags already dedupe case-insensitively
  (`tagmenu::known_tags`), so `@Crew` and `@crew` must share a color.
- **The pool is derived from `Theme.ansi`, not hand-tuned per preset.** `Theme` already
  ships a 16-slot terminal palette per theme (`crew-theme/src/lib.rs:52`) — the 8
  chromatic slots are a categorical pool the theme author already balanced. Deriving
  avoids adding a field to every preset file (there are 8+ presets now and counting)
  and keeps [[project-theme-system]]'s rule: `contrast_thresholds` is the arbiter, no
  hardcoded colors anywhere.
- **Same project = same slot on every theme.** The hash picks a slot index; the theme
  supplies the slot's color. Switching themes recolors every tag consistently instead
  of reshuffling which tag is which.
- **The due-date fragment keeps the plain accent.** Tags leaving the accent actually
  *adds* contrast between the two live tints in the composer — today both are accent
  and only bold separates them.

## What already exists (assembly, not invention)

- **Deterministic hashing is house style.** `charrain.rs:24` — a SplitMix-style integer
  hash, "the deterministic stand-in for RNG". Same shape here: fold the lowercased
  bytes, take `% pool.len()`. No `DefaultHasher` (its seed is process-random on some
  platforms — the color must survive restarts).
- **Contrast machinery is in place.** `crew_theme::contrast_ratio` (`lib.rs:100`) and
  the `contrast_thresholds` test suite (`lib_tests.rs:300`) that sweeps `ALL_THEMES`
  asserting per-field floors — the new pool gets a case in the same suite. Precedent
  for *flooring* rather than rejecting: crew-term already lifts fg/bg answers to a 3.0
  WCAG minimum ([[project-term-query-replies]]).
- **Every tag render site is already tinted through one closure or one call.** The row
  chip is one `place_right(&chip, …, accent, false)` argument; the composer tint is the
  `in_tag` branch of one style closure (`composer_cells`); the legend and filter header
  are single `format!` rows. No new rendering machinery — just the color argument.
- **The tag popup is shared UI.** `cmdmenu::menu_card` styles rows itself
  (`suggest::MenuItem` has `dim`/`header` flags, no color field) — coloring popup rows
  touches the shared command-palette path, so it is **stretch, not scope**.

## The contract (definition of done)

1. **A pure, tested `tag_color(name, &theme) -> (u8, u8, u8)`.** Lowercase → SplitMix
   fold → slot in the theme-derived pool. Deterministic across restarts, panes and
   platforms; case-insensitive; total (any non-empty string maps somewhere; the empty
   string never reaches it — tags are ≥ 1 char by construction).
2. **The pool passes the arbiter on every theme.** Built per `Theme` from the chromatic
   `ansi` slots, each entry lifted to ≥ 3.0 contrast against `page_bg` (the crew-term
   floor shape) — lifted, not dropped, so the pool size (and thus every tag's slot) is
   identical on every theme. A `contrast_thresholds`-suite case asserts every pool
   entry ≥ 3.0 vs `page_bg` for `ALL_THEMES` — concrete ratios in the failure message,
   not a boolean (the vacuous-band lesson from the CRT overhaul).
3. **Every user-facing tag render uses it.** The row chip, the composer's live `@tag`
   tint, the composer legend under an active filter, and the `@tag` part of the filter
   header row (the ` · 3 items` tail stays `text_muted`). The due fragment stays
   accent. Done-hidden items cost nothing (they don't render at all).
4. **Zero per-frame weight and no new animation term.** The hash is a handful of
   integer ops per visible tag per redraw — no cache, no state, and
   `wants_animation_frame` gains no new term (the 0.8.0 rule).

## Stretch (ranked, separate iterations)

1. **Colored rows in the tag popup** — give `suggest::MenuItem` an optional label
   color; the command palette ignores it, `/todo`'s popup uses it.
2. **User-pinned colors** — an optional per-tag override (`@crew=#7fd`-style, or a
   `colors` table in `todos.toml`); the hash remains the fallback.
3. **The filter chip in due toasts** — a due toast for a tagged item carries the tag
   in its color.
4. **Cross-surface consistency** — chat `@mentions` of the same names pick up the same
   slots (`chatmention.rs` spans), one visual language across panes.

## Verification

- Unit, RED transcripts before green (standing rule — concrete values, not shapes):
  hash stability table (name → expected slot, incl. mixed-case pairs mapping equal and
  a CJK/emoji tag not panicking); pool-derivation table per theme (slot count identical
  across `ALL_THEMES`); the contrast sweep with printed ratios; a render test asserting
  two different tags on one list produce two different fg triples on their chip cells,
  and that composer tint + row chip agree for the same tag.
- Mutation spot-check the hash fold and the slot modulo — the classic survivors of
  vacuous color tests are "always slot 0" mutants.
- Live (`.claude/skills/verify` harness — still blocked by macOS perms, same checklist
  debt): create `a @crew`, `b @home`, `c @crew`; screenshot: crew≠home colors,
  crew==crew, tint visible while typing; switch theme (`/theme`) and re-shoot — colors
  change, pairing holds, light themes stay readable.
