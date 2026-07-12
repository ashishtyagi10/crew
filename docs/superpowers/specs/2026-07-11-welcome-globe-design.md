# Welcome Screen: Rotating ASCII Globe — Design

**Date:** 2026-07-11
**Status:** Approved (user: replace everything; procedural shaded globe)

## Goal

The empty-screen welcome becomes a procedurally rendered, smoothly rotating
ASCII globe — continents shaded with density characters — with the tagline
and keyboard hint beneath it. The figlet CREW banner, per-column shimmer, and
the dev-at-terminal figure all go.

## Rendering — `welcomeglobe.rs` (new; replaces `welcomeart.rs`)

Pure function, cell-based like today:

```rust
/// Render a WxH-cell globe frame at rotation `phase` (radians).
pub fn globe(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16,
             phase: f32, land: (u8,u8,u8), sea: (u8,u8,u8), bg: (u8,u8,u8))
```

- For each cell in the WxH box: treat it as a point on the projected disc
  (account for the ~0.5 cell aspect: a cell is twice as tall as wide, so the
  disc spans `w` cols × `h` rows with `h ≈ w/2` for a circle). Points outside
  the disc emit nothing (page shows through).
- Back-project to sphere coordinates, add `phase` to the longitude, and look
  up an embedded 1-bit Earth map (a small `const [u64; 24]`-style bitmap,
  ~48×24 lon×lat — coarse continents are exactly the retro look wanted).
- Shade: land pixels pick from `#%*+=` by illumination (a fixed light from
  the upper-left: dot product with the surface normal); sea pixels use
  `·` / `.` / space by the same illumination so the sphere reads as a ball,
  not a flat map. Land draws in `land` color (theme ink), sea in `sea`
  (text_muted) — theme-driven, no hardcoded colors.
- Rotation: `phase = tick as f32 * 0.05` at the existing ~20 fps
  (`ANIM_DIV = 3`) → one revolution ≈ 6 s. No new timers; the existing
  welcome redraw cadence drives it.

Default size 44×22 cells; when the pane is smaller, scale down to the
largest fitting even size ≥ 16×8, else fall back to the existing spaced
single-line "CREW" branch (kept verbatim).

## Layout — `welcome.rs`

Top-down centred stack: globe, blank row, TAGLINE, HINT — banner constants,
`col_style` shimmer, and the `welcomeart::scene` call are deleted
(`welcomeart.rs` removed; its box-bounds test concept moves to the globe).
Version stamp bottom-right unchanged. `anim_should_redraw`/`ANIM_DIV`
unchanged.

## Testing

- Disc bounds: every emitted cell within the given box; nothing outside the
  ellipse (corner cells empty).
- Rotation: frames at phase 0 and 0.5 differ; same phase twice is identical
  (pure/deterministic).
- Shading chars come only from the fixed land/sea sets; land cells use the
  `land` color, sea cells `sea`.
- Continents visible: at phase 0 the visible hemisphere contains BOTH land
  and sea cells (map lookup wired correctly).
- welcome.rs: existing bounds/hint/version/tiny-size tests adapted (banner
  tests deleted); the stack centres and the globe row range sits above the
  tagline.

## Out of scope (YAGNI)

City lights/day-night terminator, mouse interaction, configurable speed or
size, keeping the old banner behind a flag, colored oceans per theme beyond
the two theme colors.
