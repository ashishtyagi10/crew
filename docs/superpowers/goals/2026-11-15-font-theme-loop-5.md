# Font + theme loop ×5 — 2026-11-15

The user, 2026-11-15: "run 5 iteration to fine tune font and each theme,
use best practices".

Lane: `ui`. Rounds merge to main and push, untagged; the 8 PM job cuts the
day's release.

## Where the backlog came from

- The faces crew rotates through, measured with CoreText (`CTFontGetXHeight`,
  the advance of `0`): x-height 0.45 em (Comic Shanns Mono) to 0.55
  (JetBrains Mono, MonoLisa); advance 0.545 (Comic Mono) to 0.64 (MonoLisa),
  against a fixed 0.6-em cell.
- The open list of the two theme loops of 2026-11-14.

## Rounds

| # | merge | version | what | before → after |
|---|---|---|---|---|
| 1 | 8d0f6eeb | unreleased | one apparent size whichever face draws (`sizeadjust.rs`) | x-height spread 1.22× → 1.12× over nine faces; 3 faces wider than the cell → 0; JetBrains/Lilex/Plex/Geist 1.0 exactly |
| 2 | MERGE2 | unreleased | dark pages draw hued code at its own lightness (`hueapart.rs`, `apca.rs`) | APCA Lc on the field, 14 dark non-tube palettes: code 52-55 → 56-60, strings 41-42 → 56-60; light + tubes unchanged |

### Round 1 — one apparent size whichever face draws (`sizeadjust.rs`)
Behaviour: each face is shaped at a factor of the font size: a face with a
short lower case grows toward a 0.53-em x-height, and no face is drawn wider
than its cell (CSS `font-size-adjust`, and WezTerm/kitty fitting a face to
the cell). The cell never moves. Seen: `proof_shot` with `CREW_SHOT_FONT`,
Comic Shanns and MonoLisa before/after. Measured: x-height spread across nine
measured faces 0.45-0.55 em (1.22×) → 0.491-0.55 em (1.12×); advance over
the cell 0.545-0.64 em → none past 0.6 (`sizeadjust_tests.rs`). Rows: none.

### Round 2 — a dark page's hued code reads as a light page's (`hueapart.rs`)
Behaviour: code, strings, markers and the hued syntax classes were pulled
1.8:1 from prose in LUMINANCE, which cannot see hue: on every dark page that
dimmed cyan code toward the page. A class whose hue sits ≥ 0.06 from the ink
in OKLab's a-b plane is now apart from prose already and keeps its own
lightness; grey classes (quotes, comments) and the tubes walk as before.
Prism's strings take its bright green, since jade sat 0.047 from its
ice-mint keywords once neither was dimmed. Measured in APCA Lc (new
`crew_theme::apca`, checked against the published reference pairs) on
`chatink::derive`: dark pages' code 52-55 → 56-60, strings 41-42 → 56-60,
markers 58-62 → 62-68; light pages and tubes unchanged. Seen:
`md_shot_themes` (tokyo-night added). Pinned:
`hued_code_reads_on_a_dark_field_as_on_a_light_one` (Lc ≥ 55, fails on the
old walk). Rows: none.

Not done: comments stay at Lc 32-40 on dark pages (light: 56-61) — they are
the class meant to recede, but the gap is the next thing to look at; the
tubes' code is Lc 44.

## Pending release

- **Every face reads at one size.** A theme step that changed the font also
  changed how big the text looked: Comic Shanns' lower case stood a fifth
  shorter than JetBrains Mono's, and MonoLisa, SF Mono and Fira Code, wider
  than crew's cell, were drawn into it with their letters touching. Each
  face is now shaped at its own factor — a short face grows toward the
  pack's x-height, no face is drawn wider than the cell — and JetBrains
  Mono, Lilex, IBM Plex and Geist stay exactly as they were. Costs one font
  lookup when the family or weight changes. Idea: CSS `font-size-adjust`;
  fitting the face to the cell is what WezTerm and kitty do.
- **Code on a dark page is as bright as its colour.** Code, strings and
  markers were held apart from prose by lightness alone, a ratio that cannot
  see hue, so every dark palette dimmed its cyan code and green strings
  toward the page — APCA Lc 52 and 41 there, where a light page drew 62-67.
  A class whose hue already sets it apart from the text keeps its own
  lightness now: 56-60 on every dark palette. Grey classes and the
  single-phosphor tubes, which have only lightness to work with, are
  unchanged, and prism's strings take its bright green to stay apart from
  its keywords. Costs nothing per frame (the colours are derived once per
  palette). Idea: APCA, the WCAG 3 draft's contrast model, which Chrome
  DevTools and the Ghostty theme discussions use for dark themes.
