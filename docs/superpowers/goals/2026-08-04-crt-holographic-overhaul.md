# Goal — CRT stops being a bright border: holographic glow, real transparency, TRON/JARVIS overhaul

**Set:** 2026-08-04 by the user.

The verdict from daily use: paper-light and paper-dark are fine; **CRT is just a bright border**.
Pick any of the four phosphors (`crates/crew-theme/src/presets_crt.rs` — green, amber, violet,
blue) and what you actually see is a black page, monochrome text, and a hot 2.5px focused frame.
The parts that were supposed to sell the fantasy are all tuned to be polite:

- **The glow is a whisper.** The "bloom" is a cheap two-ring neighbour sample in `crt.wgsl`
  (1.5px inner + 3.5px outer at 0.45 weight, `crates/crew-render/src/crt.rs:17-24`) — the halo
  dies ~8px out. Real phosphor, TRON light-traces, and JARVIS HUDs all bleed light *far* beyond
  the stroke; ours reads as slightly-soft antialiasing. And every knob (`SCANLINE=0.18`,
  `GLOW=0.55`, `CURVATURE=0`, `CORNER=0`) is a **compile-time constant** — the four phosphors
  cannot even differ from each other, and `Theme.crt` is a single `bool`.
- **The transparency is doctrine-forbidden.** `glass.rs:113-117` deliberately makes CRT glass
  "the faintest of the family" and a test (`crt_glass_is_restrained`, `glass.rs:227`) *pins*
  CRT to fainter-than-dark, zero noise, zero shadow. That was the right call for "a laptop LCD
  wearing scanlines"; it is exactly wrong for a holographic HUD, where panels are luminous
  translucent sheets floating over depth.
- **The panes are cards, not light constructs.** Borders are flat opaque strokes; legends sit
  on them like any paper theme. Nothing about a CRT pane says *drawn in light* — no edge-lit
  frame, no corner emphasis, no interior tint, no sense that the border IS the light source.

THE GOAL, in one line: the CRT family graduates from "dark theme with scanlines" to a
**holographic terminal** — TRON's edge-lit light-trace geometry meets JARVIS's luminous layered
HUD — where glow is generous and physical, panels are translucent sheets of tinted light, and
the whole frame feels like it is projected, not painted. Paper light/dark are untouched.

### Pillar 1 — real bloom, per-theme knobs
The two-ring neighbour sample is replaced (or augmented) by a proper wide bloom — the classic
downsample/blur/upsample chain, or a separable two-pass gaussian over a half-res bright-pass
target — so hot pixels throw a soft halo tens of pixels wide with a believable falloff, and a
focused border actually *radiates*. `Theme.crt: bool` becomes a small `CrtStyle` struct (glow
strength/radius, scanline weight, tint, flicker character…), so green can run hot-phosphor
while blue runs cold-TRON-edge and violet runs JARVIS-orchid — the phosphors get personalities
instead of sharing four global constants. Budget rule: the extra passes work on downsampled
targets and the whole chain stays static when idle (same discipline `update_uniform`'s
`flicker=0` already follows) — this is a winit-main-thread app and the CRT pass owns every
frame, so the bloom must be O(half-res), not O(full-res × taps).

### Pillar 2 — transparency is the point, not a garnish
The "CRT glass stays faintest" doctrine is **repealed**. CRT panes become luminous translucent
sheets: a visible interior tint of the phosphor colour (brighter than paper-dark's glass, not
fainter), inner edge-glow bleeding from the border into the pane body, and content that reads
as printed *on* the light sheet. The `crt_glass_is_restrained` test flips from pinning
restraint to pinning the new contract. Beyond in-app glass, CRT should compose with the
existing window-alpha path (Glass vs Opacity are separate paths — keep them separate) so a
translucent-window CRT actually looks like a HUD floating over the desktop instead of a black
slab with holes. Contrast stays non-negotiable: the 3.0 WCAG terminal fg/bg floor and
`contrast_thresholds` still arbitrate — glow and tint may never cost legibility.

### Pillar 3 — TRON/JARVIS design language for the chrome
The border stops being a stroke and becomes a **light-trace**: focused panes get the full
treatment (outer halo from the new bloom, inner bleed, brighter corner nodes or bracket
emphasis in the fieldset-legend corners), unfocused panes dim to a thin quiet trace — the
focus-led hierarchy the presets already comment about, now expressed in light instead of RGB
steps. The legend reads as etched into the light. Motion joins in through the existing
`wants_animation_frame` registry (the JARVIS-motion instinct, already proven): a slow breathing
of the focused frame, an activity-driven surge when a pane streams (the flicker input already
exists per-frame), maybe a one-shot ignition sweep when a pane gains focus. Idle must still
converge to a static frame — animation is an accent, not a heartbeat.

### Pillar 4 — proven on pixels, not vibes
Every visual claim gets a measurement. The headless GPU test
(`crates/crew-render/tests/crt_headless.rs`) grows assertions for the new look: halo width
(sample a horizontal line through a lit border and assert luminance above threshold ≥ N px from
the stroke), interior tint (pane-body pixels measurably above page_bg), and scanline pitch
still ≥3px (2px aliases — known lesson). The sRGB/linear boundary rules apply to every new
blend (`target_rgba` keyed off `format.is_srgb()`, amplitudes space-calibrated); the live-app
verify harness (screenshot + pixel sampling) signs off each phosphor in both the opaque and
translucent-window configurations. A before/after shot per phosphor lands in the PR.

### Done means
1. `Theme.crt` is a per-theme style struct, all four phosphors ship distinct tunings, and the
   old global constants in `crt.rs` are gone or demoted to defaults.
2. The bloom chain produces a halo whose luminance stays above a pinned threshold ≥16px from a
   focused border stroke (headless test asserts it; the old two-ring pass died at ~8px).
3. CRT glass alpha exceeds paper-dark's (the inverted `crt_glass_is_restrained` contract), pane
   interiors carry a measurable phosphor tint, and a translucent-window CRT screenshot shows
   the desktop reading through the sheet.
4. Focused vs unfocused panes are distinguishable by glow alone in a grayscale screenshot —
   the hierarchy lives in light, not just in colour steps.
5. Idle CRT renders a byte-identical frame twice in a row (no perpetual animation), and a
   4-pane streaming session holds 60fps on the dev machine.
6. Terminal fg/bg contrast never drops below the 3.0 floor in any phosphor under the new tint
   (existing arbiter tests extended to the CRT family).
7. Paper light and paper dark screenshots are pixel-identical before/after — the overhaul is
   CRT-scoped.
