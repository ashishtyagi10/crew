# Motion loop ×3 — started 2026-11-21 (from v0.30.20, ui lane)

Three animation rounds the user asked for, one per round, each merged to main
untagged (the 8 PM job releases).

**Correction to the recommendation that started this loop.** It said panes
"just appear" and "just disappear" and that tab switches cut. The tree says
otherwise: `spawn_timeline`/`assemble_t` draws a new card in, `ghost.rs`
collapses a closed or minimized card, and `switch_grid` re-stamps `born_ms`
so a tab's cards assemble on arrival. Rounds 2 and 3 are re-scoped to the
real gaps below.

## Rounds

### R1 — scroll glides (smooth scrolling)
- Behaviour: a mouse-wheel notch, Shift+PageUp/PageDown and Shift+Home/End
  no longer jump; the scroll offset travels there on a critically damped
  spring (`spring.rs`, the grid glide's own primitive), a whole row per step.
- Scope cut: pixel-level offsets need the renderer to clip cell quads (only
  text clips today) and the terminal to hand over one row beyond the viewport.
  Left as the next row. Trackpad (`PixelDelta`) stays direct — it has its
  own momentum, and a spring on top is lag.
- Measured: unit tests on the spring's line stepping; motion off = today's jump.
- Rows: + `scrollglide.rs`; nothing on screen added.

### R2 — a new pane slides out of its parent
- Gap (re-scoped): open already assembles and close already collapses
  (`ghost.rs`); what was missing is WHERE a new card comes from — it was set
  straight onto its tile while its neighbours glided aside.
- Behaviour: the single fresh pane starts at the pane that opened it (the one
  focus just left, or the focused one for a background open) and rides the
  grid springs to its tile. Position only: a size animation would resize a
  new shell's pty on every frame.
- Measured: `build_frame` test — first frame at the parent, size constant,
  settles on its own tile. Rows: + `paneentry.rs`.

## Pending release
- **Scroll glides** (R1): a wheel notch, Shift+PageUp/PageDown and
  Shift+Home/End moved scrollback in one frame, so the eye lost its place.
  The offset now travels on a critically damped spring, a whole row per step:
  a notch lands in ~150 ms, a page in ~300 ms, and notches in a row become
  one motion. Typing during a glide cancels it; the trackpad stays direct.
  Motion off keeps the jump, and an idle pane costs no frames. (From
  browsers' and WezTerm/Warp's smooth scrolling.)
- **New panes come out of their parent** (R2): Cmd+D's new card used to be
  set straight onto its tile while its neighbours glided aside. It now starts
  where the pane that opened it sits and slides to its place at its final
  size, so the split reads as one pane becoming two. Panes born together (a
  restored session) still just assemble. (From macOS window zoom-from-origin
  and zellij's split animation.)

## Log
- **R1 (unreleased) — scroll glides.** `scrollglide.rs` (`ScrollGlide` on
  `Pane.glide`, preserved across reseed/snap), stepped in `render.rs` on the
  grid glide's clock, kept alive by `pane_animating`. Bug caught by its own
  test: a reverse notch that cancelled the trip to zero read as "done" with
  rows already handed out — `moving()` now also checks `applied` and the
  spring. Mutations caught: end-of-buffer stop, snap-to-bottom cancel. Gate:
  fmt, check 0 warnings, 41 suites green. Not driven live (no new pixels;
  the timing is unit-tested).
- **R2 (unreleased) — new panes come out of their parent.** `paneentry.rs::entry`
  picks the ONE fresh pane in `placed.full` and its parent (focus_prev when
  focus moved to the new pane, else focused), seeded into `pane.rect` in the
  glide loop so the existing reseed→spring path carries it. Mutation caught:
  dropping the seed fails the `build_frame` end-to-end test. Gate green.
