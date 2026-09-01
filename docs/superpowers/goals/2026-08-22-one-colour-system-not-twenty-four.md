# Goal — one colour system, not twenty-four hand-tuned palettes

**Status: PHASES 1–2 SHIPPED, PHASE 3 OPEN.** Every text role is derived from the page it sits on
(`crew-theme/src/ramp.rs`) and the 16-slot terminal palette is derived rather than eyeballed
(`ansi.rs`), landed across v0.18.2–v0.18.6 with parity contracts. The open question below was
answered by cutting the menu: `ALL_THEMES` is twelve, not twenty-four. Phase 3 (restraint,
measured) is carried in `2026-09-01-close-the-open-goals.md`, Pillar 5.

**Set:** 2026-08-22 by the user, after asking what 2026 actually likes.

crew has **24 themes and 2,376 hand-authored colour channels** (17 single-colour roles plus a
16-slot ANSI palette, three channels each, across `presets_*.rs` — ten files). Every one of those
numbers was chosen by eye, in isolation, against one background. They pass `contrast_thresholds`
in `crew-theme/src/lib_tests.rs` because that suite was run *after* the fact and the failures
nudged by hand. NOTHING RELATES ANY TWO OF THEM. `text_muted` in AURORA and `text_muted` in
SEPIA_DARK are both "muted" only in the sense that a person squinted at each and agreed. There is
no shared notion of how far below `ink` a muted role sits, so a theme switch does not move the
palette to the same place — it moves it to a different person's guess on a different afternoon.
That is what "the themes could look better" actually means, and adding a 25th palette makes it
worse, not better.

The 2026 research says the fix is not decoration. Four findings survive contact with a terminal
grid; the rest are for products crew is not.

**OKLCH is the one that matters.** The industry moved tonal scales to a perceptually uniform space
precisely because equal lightness in HSL/RGB is not equal *perceived* lightness across hues — so a
palette tuned on a blue accent falls apart when the same recipe is applied to amber. Tailwind,
Radix and Material all generate their 50–950 ramps by holding chroma and hue and varying only L.
crew is doing the opposite of this today. **Dark-first is now the default**, not the variant —
around 45% of newly launched SaaS ships dark-first and dev tools lead that; crew is already there
via `auto` and the dark/light pairing, so this is a confirmation, not a change. **"Mood mode"**
darks are multi-layered — deep charcoals, midnight plums, forest inks — rather than one flat
near-black, which is an argument for crew's paper/sepia/moss pages and against flattening them.
And **neon returned as micro-glow only**: focus states, small badges, outlines against dark
surfaces — never a flood. crew's bloom and ink-halo already risk the flood; this is a mandate for
restraint, not more glow.

The terminal aesthetic itself is now mainstream — monospace, fixed grids, "intentional
incompleteness", layouts that refuse to disguise their structure. **crew does not need to chase
that. crew already is that**, and the fieldset-panel decree and the flat-tube decree (v0.13.5) are
the same instinct arriving early. The risk in this goal is over-correcting into fashion. DO NOT.

### Phase 1 — every role becomes a function, not a number

A new `crew-theme/src/ramp.rs`. A palette declares intent — page lightness, one accent hue, a
chroma budget, warmth — and every role is DERIVED from it in OKLCH: `ink`, `text_muted`,
`legend_off`, `hint_fg`, `placeholder`, `dim`, `border_normal`, `border_focused`, `find_hl_bg`.
The derivation is a pure function, so it is testable without a GPU and reviewable as arithmetic
rather than as 99 opinions per theme.

THE CONTRAST SUITE STOPS BEING A CHECK AND BECOMES THE SPECIFICATION. Today
`contrast_thresholds` asserts `ink ≥ 10.0`, `text_muted ≥ 7.0`, `legend_off ≥ 3.0`,
`hint_fg ≥ 2.5`, `placeholder ≥ 2.3` against the page, and the palettes were bent until they
passed. Inverted, those same numbers become the ramp's *inputs*: solve for the L that hits the
ratio at the declared hue and chroma. A palette then cannot fail the suite, because the suite is
what produced it. The existing test stays exactly as it is — as the independent check that the
solver did what it claimed.

`Theme` KEEPS ITS FLAT `(u8, u8, u8)` FIELDS. The render path reads concrete sRGB triples on the
hot path and must not start doing colour maths per frame; the ramp runs at construction and the
struct is unchanged. This is a change to how the numbers are *authored*, not to what the renderer
consumes — which also means it can land theme-by-theme with the suite green throughout.

### Phase 2 — the ANSI palette, which is checked for legibility but not as a system

*(Corrected 2026-08-22: an earlier draft of this goal said twelve ANSI slots were unverified. That
was wrong — `lib_tests.rs:353` loops slots 1–6 and 9–14 against `term_bg` at ≥ 3.0. What follows
is what is actually missing.)*

Three real gaps. **Slots 0, 7, 8 and 15 are skipped outright** — the blacks and whites, deliberately
excluded because they sit near the background, which is exactly why "near" needs a number rather
than a comment. **The floor is 3.0**, a legibility minimum, and nothing above it: a palette can
pass with every hue crowded into the same corner. And **nothing checks the slots against each
other** — red and yellow may be a hair apart in perceived lightness and the suite will not notice,
though a user reading `git diff` will.

The same ramp fixes all three by construction: the eight base hues get one shared chroma and
lightness per theme, spaced by hue rather than by eye, with the bright half a fixed lightness step
above — the way every respected scheme (Catppuccin, Tokyo Night, Monokai Pro) is built. The suite
then grows a *pairwise* assertion — minimum perceptual distance between slots — which is the check
that does not exist today in any form.

### Phase 3 — restraint, measured

One pass over the effects, with the micro-glow finding as the rule: bloom radius and amplitude,
the gradient light-ring, the dot lattice, grain. Each one gets a defensible number per pool rather
than a per-theme feel. THE TEST IS WHETHER A SCREENSHOT STILL READS AT A GLANCE, and the
`crew-render` screenshot harness (`cargo run --example screenshot -p crew-render`) already renders
all 24 themes to PNG, so this is verifiable rather than argued.

### The open question — 24 is a menu, not a design

Every serious 2026 source frames colour as a *system* that adapts, not a catalogue to browse. Nine
distinct palettes with a real point of view beat twenty-four where several are a hue rotation of
each other. **This is the user's call, not mine, and Phase 1 does not depend on it** — the ramp
makes cutting easy later and makes keeping all 24 defensible now, because they would finally share
a spine. Worth deciding before Phase 3, since restraint tuning is per-pool work that gets cheaper
with fewer members.

### What this goal is not

NOT a new theme. NOT a new effect. NOT a redesign of the panel language — the fieldset card with a
legend on its top border stays, and so does the flat tube. NOT a font change: `fonts.rs` pairs a
typeface to each theme and that pairing is already deliberate. If this goal ends with crew looking
*different*, it has failed. It ends with crew looking like it was drawn by one hand.

---

**Sources consulted (2026-08-22):**
[Evil Martians on OKLCH](https://evilmartians.com/chronicles/oklch-in-css-why-quit-rgb-hsl) ·
[OKLCH colour space guide](https://colorarchive.org/guides/oklch-color-space-guide/) ·
[UI colour trends 2026](https://www.recursion.agency/blog/ui-color-trends-2026) ·
[Dark mode best practices 2026](https://www.tech-rz.com/blog/dark-mode-design-best-practices-in-2026/) ·
[UI/UX trends 2026 (data-backed)](https://www.index.dev/blog/ui-ux-design-trends) ·
[Aesthetics in the AI era](https://medium.com/design-bootcamp/aesthetics-in-the-ai-era-visual-web-design-trends-for-2026-5a0f75a10e98) ·
[Terminal themes round-up](https://terminalcolors.com/)
