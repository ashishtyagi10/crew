# Theme loop ×5, second run — 2026-11-14

The user, 2026-11-14 (evening, after the soft nav became one panel): "run
the UI enhancement loop to further fine tune the look and feel of each theme
5 time and do the release after 5 iteration complete".

Lane: `ui`. Rounds merge to main and push; the release is cut once, after
round 5, on the user's explicit ask (v0.30.14 already went out today).

## Where the backlog came from

Two contact sheets of all 20 palettes, sheer over a lavender desktop:
`glass_work_shot` (smith + a shell) and the new `theme_nav_shot` (the open
nav in tabs beside the dashboard and the settings form). Read for what each
family DRAWS, since the first loop found every derived role in one band.

## Rounds

| # | merge | version | what | before → after |
|---|---|---|---|---|
| 0 | 63efb0e6 | unreleased | soft: the nav is one raised panel, grooves between sections, tab cards as wells (`navrelief.rs`) | 10 raised tiles + a tile per tab → 1 panel, 9 grooves, a well per tab |
| 1 | (this merge) | unreleased | LOAD's 5- and 15-minute figures read on see-through glass (`load.rs` `on_frost`) | worst contrast over a black/white desktop, sheer: 1.27-2.09:1 on 11 palettes (3.0-3.4 on neon, prism) → ≥ 3.5:1, or the 1-minute colour where that has no headroom; 13 palettes changed |

### Round 1 — the load history reads on glass
Behaviour: while the window is see-through, the nav's 5- and 15-minute load
averages are held to the secondary floor over the frost on any desktop, not
stepped back toward a page that is no longer what they stand on. Seen:
`theme_nav_shot`, crt-green and crt-amber over lavender (dark green on grey
→ pale green). Measured: `the_history_reads_on_sheer_glass_over_any_desktop`.
Rows: none. Opaque windows: byte-identical. Harness added: `theme_nav_shot`.

## Pending release

- **Soft themes: the side panel is one panel again.** On `porcelain`,
  `sand`, `mist` and `slate` every section of the nav sits in one raised
  panel, parted by a fine engraved line, instead of each being its own
  pill. A tab's card is pressed into the panel.
- The side panel's 5- and 15-minute load averages can be read on a
  see-through window. They were faded toward the theme's page colour, which
  a see-through window does not show; on the green and amber tubes over a
  light desktop they were close to invisible.
