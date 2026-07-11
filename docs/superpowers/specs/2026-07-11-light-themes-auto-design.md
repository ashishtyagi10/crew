# Light Themes, Random Split & Auto Day/Night — Design

**Date:** 2026-07-11
**Status:** Approved

## Goal

Four new newspaper-look light themes; `random` splits into `random-dark` /
`random-light` pools; `/theme auto` follows the OS appearance — dark pool at
night (OS dark), light pool by day (OS light) — rotating within the pool
every 10 minutes.

## 1. New light presets — `crew-theme/src/presets_paper.rs` (+ a new `presets_paper_light.rs` if the file grows unwieldy)

All four: `dark: false`, `grain: 3.0`, full 16-slot ANSI palettes in the
paper's tone, and paper-light's contrast discipline (ink ≥ 16:1 on the page,
text_muted ≥ 11:1, every ANSI slot ≥ 4.5:1 — enforced by extending the
existing lib_tests contrast checks to iterate ALL light themes).

| id | page | ink concept |
|---|---|---|
| `sepia-light` | warm aged-newsprint cream | deep brown-black; light twin of sepia-dark |
| `salmon-broadsheet` | FT-pink salmon | cool near-black; financial broadsheet |
| `coldpress-gray` | cool pale gray | near-black; light twin of graphite, lowest glare |
| `ivory-ledger` | slightly yellow ivory | green-black ink, accounting-ledger accents |

`ThemeId` gains four variants; `ALL_THEMES` grows 9→13 (grouped: paper
family, then CRTs); `as_u8`/`from_u8` extended (new ids append — persisted
u8s stay stable); `describe()` lines added.

## 2. Random pools — `crew-theme/src/lib.rs`

- `random_pick(current, seed, dark: bool)` — pool = themes where
  `is_dark() == dark`, minus `current`. Light pool has 5 entries (paper-light
  + 4 new), dark pool 8 — both comfortably non-empty.
- Mode state replaces the `RANDOM: AtomicBool` with an `AtomicU8` mode:
  `Off | RandomDark | RandomLight | Auto`. `set_random_mode(mode, now_ms)`
  switches immediately to a pick from the mode's pool (auto: pool chosen by
  the OS-appearance atomic) and starts the clock; `tick_random(now_ms)`
  rotates within the active pool.
- `/theme` names: `random-dark`, `random-light`, `auto`; `random` stays as an
  alias for `random-dark` (no breakage for saved configs / muscle memory).
  The `/theme` list line shows all three modes with the active one marked.

## 3. Auto (OS appearance) — app ↔ crew-theme

- crew-theme: `set_os_dark(bool)` + `os_dark()` on an `AtomicBool`
  (default true — before the first report, auto behaves like random-dark).
- crew-app: on window creation read `window.theme()` (winit) and on
  `WindowEvent::ThemeChanged` call `crew_theme::set_os_dark(...)`. When auto
  mode is active and the appearance flips, switch immediately to a pick from
  the new pool (and reset the 10-minute clock) — the existing
  `tick_random`-returns-`true` → repaint + re-apply accent path already
  handles the redraw; the ThemeChanged handler triggers the same path.
- Platforms where winit reports no theme (some Linux setups): `window.theme()`
  is `None` → the default (dark) stands until a ThemeChanged arrives.

## 4. Persistence & cycle

- Config `theme` string accepts the two pool names and `auto` exactly like
  theme names today (`spawn.rs` startup path); saved and restored verbatim.
- `Ctrl+Shift+L` cycle: 13 fixed themes, then `random-dark`, `random-light`,
  `auto`, wrap. `cycle_next` returns the mode label for the status line.

## Testing

- Preset contrast: extend the existing theme contrast tests to all light
  themes (ink/muted/ANSI floors above).
- `random_pick` pool purity: light pool never returns a dark theme and vice
  versa; never returns `current`.
- Mode transitions: enabling each mode switches immediately within the right
  pool; `tick_random` honors the pool; auto follows a flipped `set_os_dark`
  on the next call; alias `random` == `random-dark`.
- `/theme` parse: new names, alias, unknown-name echo lists them.
- u8 round-trip for the four new ids.

## Out of scope (YAGNI)

Sunrise/sunset by location, per-side pinned themes, transition animations,
custom user themes.
