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
| 2 | 5700ff9e | unreleased | dark pages draw hued code at its own lightness (`hueapart.rs`, `apca.rs`) | APCA Lc on the field, 14 dark non-tube palettes: code 52-55 → 56-60, strings 41-42 → 56-60; light + tubes unchanged |
| 3 | 6f7a00ce | unreleased | a dark page's comment reads (`commentread.rs`) | APCA Lc of comments, 14 dark palettes: 32-40 → 45-46, code ≥ 1.24× above; light + tubes unchanged |
| 4 | MERGE4 | unreleased | the underline and strike sit where the face says (`rules.rs`) | strike 2.5-4 px above the lower case's middle → on it; underline at `loose` 3-4 px further off its word → same gap as at normal; Comic faces' underline 6.5 px under the baseline → 0.12 em |

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

Not done: comments stay at Lc 32-40 on dark pages (light: 56-61) — round 3;
the tubes' code is Lc 44.

### Round 3 — a dark page's comment reads (`commentread.rs`)
Behaviour: the ladder sets a comment back from the ink by a ratio, and three
steps under near-white is far dimmer to the eye than three over near-black.
On a dark non-tube page the comment (and the `#[attr]` rung that shares it)
is lifted at its own hue and chroma to APCA Lc 45 on the field, stopping
1.2:1 under the code. Measured: comment Lc on 14 dark palettes 32-40 →
45.0-46.0, code still 1.24-1.32× above; light pages 56-61 and tubes 26-27
unchanged. Seen: `chat_shot_themes` (tokyo-night added), the `// signed`
comment before/after. Pinned: `a_dark_pages_comment_reads_on_its_field`
(fails with the lift removed). Rows: none.

### Round 4 — the underline and strike sit where the face says (`rules.rs`)
Behaviour: the rules were placed on the cell (underline two rule-widths
up from its floor, strike at its middle), not on the text. Measured against
eight faces' own `post`/`OS/2` tables at 14 pt Retina: at the 1.25 cell the
underline matched most of them within a pixel, but sat 6.5 px under the
Comic faces' baseline and fell 3-4 px further from its word at `loose`; the
strike ran 2.5-4 px above the middle of the lower case on every face. Now
both are measured from cosmic-text's own baseline: the face's underline
(0.12 em under where the table is empty, as both Comic faces' are), never
lower than before; the face's strike while it lands in the middle half of
the x-height, else that middle. A double underline grows down from a single
where the cell has room. Seen: `term_shot_width_sweep` attrs, before/after
(strike through `trike`, not over it). Pinned: `rules_tests.rs` (8, one
through `FontParams`; it fails with the face ignored). Rows: none.

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
- **Comments on a dark page are readable.** A comment is set back from the
  text by a fixed ratio, and on a dark page that ratio is a much bigger
  drop to the eye: dark palettes drew comments at APCA Lc 32-40, light ones
  at 56-61. On a dark page a comment is now lifted, in its own scheme's tint,
  to Lc 45, and always stays a step under the code around it. Light pages
  and the green and amber tubes are unchanged. Costs nothing per frame.
  Idea: APCA's levels for secondary text; GitHub Dark's comment grey.
- **Strikethrough runs through the letters, underlines stay with the
  word.** Both rules were placed on the cell instead of on the text: the
  strike crossed the tops of `a` and `e` rather than their middles, and
  every step of `/leading` let the underline fall further from its word —
  it hung 6.5 px under Comic Mono's line even at the normal spacing. Each
  is now measured from the baseline the face is drawn on, where the font's
  own tables put it, with a common fallback for faces whose tables are
  empty. Costs one metrics read when the face or size changes. Idea:
  Ghostty, kitty and Alacritty place both rules from the font's
  `post`/`OS/2` metrics.
