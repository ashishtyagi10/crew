# Goal — crew follows the system appearance, automatically

**Status: SHIPPED v0.14.2–v0.14.4** — `auto` is the fourth theme and the fresh-install default,
with `theme_dark`/`theme_light` pairing and a DECSET 2031 scheme push to child TUIs. **Live verify
outstanding** on the standing macOS-permissions debt (`2026-09-01-close-the-open-goals.md`,
Pillar 5).

**Set:** 2026-08-06 by the user: crew's theme should sync with the OS automatically — light mode
gets a light crew, dark mode gets a dark crew, and a mid-session flip (manual toggle, or macOS
auto-switching at sunset) re-themes crew without anyone typing anything.

## What already exists (most of the machinery, none of the visibility)

The plumbing shipped quietly over 0.12.x–0.13.x and works today — for anyone who knows the secret
word:

- **`RandomMode::Auto`** (`crew-theme/src/lib.rs`): a rotation mode whose pool is the dark or
  light *paper* pool depending on the OS appearance. Parses from `/theme auto` — but it is
  deliberately **unlisted back-compat**: `THEME_MODES` is `[Dark, Light, Crt]`, so the `/theme`
  picker, the Settings form cycle (`settingspane/cycle.rs`), `Ctrl+Shift+L`, the composer
  suggestions (`suggestvalues.rs`) and the startup status line all pretend it doesn't exist.
- **Live OS tracking**: winit's `ThemeChanged` feeds `set_os_dark` and re-applies the selection
  immediately when auto is active (`events.rs:188`); startup seeds from `window.theme()`
  (`handler.rs:90`). The 0.13.3 develop-fade veil already makes the flip cinematic — "an OS
  appearance flip" is literally one of its named trigger paths.
- **Terminal side**: crew-term answers OSC 10/11 and DSR from the live theme, exports COLORFGBG,
  and floors fg/bg contrast at 3.0 WCAG — so pane content stays readable across a flip even
  though CLIs sample the theme once at startup.

The gap is not machinery. It is that **following the OS is a hidden cheat code instead of the
obvious out-of-box behavior.**

## The contract (definition of done) — SHIPPED in 0.14.2 (2026-08-07)

All five core items landed; the stretch items below remain open. The shared
resolution lives in `CrewConfig::theme_selection` (configread.rs); live
verification of the OS flip is still worth a pass with the GUI harness.

1. **Auto is a first-class theme.** It joins `THEME_MODES` (listed last: dark, light, crt,
   auto) and therefore every surface that derives from it: `/theme` value picker, Settings form
   cycle, `Ctrl+Shift+L` rotation, composer suggestions, status labels. Its description already
   says the right thing: "light by day, dark by night — follows the OS."
2. **Fresh installs follow the OS.** A config with no `theme` key resolves to auto, not
   `PaperDark` (`configread.rs::theme_id` / `spawn.rs::apply_config` fallback). First launch on a
   light-mode Mac must come up light. **Migration rule: existing configs are untouched** — any
   persisted `theme` value, palette or mode, is an explicit choice and keeps exactly its current
   meaning. No one-shot heal rewrites it (contrast with the 0.12.6 pin-clear, which fixed configs
   that were *broken*; a pinned dark theme is not broken).
3. **Choosing stays a statement of intent** (the 0.12.6 contract, extended): picking `dark`,
   `light`, `crt`, or a fixed palette opts *out* of following the OS — no surprise re-theming
   under a user who said what they want. Picking `auto` opts back in. The Settings form and
   `/theme` listing should make the tradeoff legible in the one-line descriptions.
4. **A flip lands everywhere a theme switch lands.** OS flip while auto ≡ `/theme` switch:
   develop-fade veil, accent re-derivation (`palette::set_accent`), theme font
   (`tick_theme_font`), CRT/glass derivation, and the OSC 10/11/COLORFGBG answers all read the
   new theme on the next query. Most of this is already routed through `apply_selection` — the
   work is proving it, not building it (see verification).
5. **The dark-by-default guess is documented, not silent.** `OS_DARK` starts `true`; on a
   platform where winit never reports an appearance, auto behaves as dark. Fine — but the doc
   comment and the `/theme` description should say so.

## Stretch (ranked, separate iterations)

1. **Per-appearance pairing** — SHIPPED in 0.14.3: `theme_dark` / `theme_light` config keys pair
   each appearance with a pool (`dark`|`light`|`crt`) or a pinned palette
   (`crew_theme::set_auto_pools` / `auto_side`; clamped() carry-through regression-tested).
2. **Tell live CLIs about the flip** — SHIPPED in 0.14.4: crew-term sniffs DECSET 2031 per pane
   (`schemenotify.rs`), answers `CSI ? 996 n` and DECRQM, and the app pushes `CSI ? 997 ; Ps n`
   on every darkness flip (`schemepush.rs`). The contrast floor remains the fallback for programs
   that never opt in. Still worth a live check against a real neovim ≥ 0.11 session.

## Verification

- Unit: `THEME_MODES`-derived surfaces list auto (picker rows, cycle order, suggestions);
  fresh-config resolution lands on `Mode(Auto)`; an existing `theme = "paper-dark"` config still
  pins `PaperDark` after upgrade.
- Live (`.claude/skills/verify` harness, isolated HOME): launch the dev instance, flip macOS
  appearance via `osascript -e 'tell app "System Events" to tell appearance preferences to set
  dark mode to not dark mode'`, screenshot before/after — page background must cross the
  light/dark boundary within one veil fade, sidebar/input-bar/toasts included, and flip back.
- The idle invariant holds: auto adds no per-frame work — it is event-driven (`ThemeChanged`)
  plus the existing 10-minute rotation tick, and `wants_animation_frame` gains no new term.
