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
| 1 | MERGE1 | unreleased | one apparent size whichever face draws (`sizeadjust.rs`) | x-height spread 1.22× → 1.12× over nine faces; 3 faces wider than the cell → 0; JetBrains/Lilex/Plex/Geist 1.0 exactly |

### Round 1 — one apparent size whichever face draws (`sizeadjust.rs`)
Behaviour: each face is shaped at a factor of the font size: a face with a
short lower case grows toward a 0.53-em x-height, and no face is drawn wider
than its cell (CSS `font-size-adjust`, and WezTerm/kitty fitting a face to
the cell). The cell never moves. Seen: `proof_shot` with `CREW_SHOT_FONT`,
Comic Shanns and MonoLisa before/after. Measured: x-height spread across nine
measured faces 0.45-0.55 em (1.22×) → 0.491-0.55 em (1.12×); advance over
the cell 0.545-0.64 em → none past 0.6 (`sizeadjust_tests.rs`). Rows: none.

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
