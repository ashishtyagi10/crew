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
| 2 | feat/soft-dial-mould | unreleased | soft: a dial's face is moulded out of the card (`plot/dialrelief.rs`) | porcelain, sand, mist, slate: flat 6% ink disc + stroked bezel arc → the card's own colour raised by a light and a dark shadow, 0 strokes round the face; 16 other palettes byte-identical |
| 3 | feat/soft-scroll-slot | unreleased | soft: the scroll thumb is a slot in the card (`cardpaint.rs`) | porcelain, sand, mist, slate: opaque bar (alpha 0.95) riding a frame that is not drawn → the reading's colour at 0.45 with a lit far lip; 16 other palettes byte-identical |
| 4 | feat/tab-number-is-text | unreleased | every palette: a background tab's number is text (`tabcard.rs`) | number vs page: 2.0:1 on 16 palettes, 2.4 on the tubes, 3.0 on the Ink pages (`border_normal`) → `text_muted`, the colour its name wears; all 20 changed |

### Round 1 — the load history reads on glass
Behaviour: while the window is see-through, the nav's 5- and 15-minute load
averages are held to the secondary floor over the frost on any desktop, not
stepped back toward a page that is no longer what they stand on. Seen:
`theme_nav_shot`, crt-green and crt-amber over lavender (dark green on grey
→ pale green). Measured: `the_history_reads_on_sheer_glass_over_any_desktop`.
Rows: none. Opaque windows: byte-identical. Harness added: `theme_nav_shot`.

### Round 2 — soft themes: a dial is a knob
Behaviour: on the soft themes a gauge dial (the nav's SYSTEM trio, the
dashboard's) drops its grey plate and its stroked bezel; the face is the
card's colour, raised off it by the same two shadows that raise the card,
lit from the same side. Seen: `relief_nav_shot`, porcelain and slate.
Measured: `the_face_is_the_page_and_the_two_rims_say_where_it_ends` (lit rim
brighter than the page, far rim darker). Rows: none. Idea: neumorphism.io's
raised round button; the open item of the first loop.

### Round 3 — soft themes: the scroll thumb is a slot
Behaviour: a pane's scroll thumb rides the card's right stroke; a soft card
strokes none, so the thumb was a lone dark line down the card, the hardest
on the page. On the soft themes it is drawn as a slot: its colour thinned
to 0.45 and a one-pixel lit lip on the side away from the light. Seen:
`glass_work_shot`, porcelain and slate. Measured:
`a_relief_card_cuts_the_thumb_as_a_slot`. Rows: none.

### Round 4 — a background tab's number is text
Behaviour: in the nav's tab cards the tab number took the card frame's
colour. Lit (the tab on screen, a tab that needs you) that is a colour; on a
quiet tab it is the rule's grey, and on a soft page the rule is not even
drawn, so `2` was the faintest thing in the nav. A quiet tab's number wears
`text_muted` like its name, still bold. Seen: `relief_nav_shot` (slate).
Measured: `a_quiet_tabs_number_is_as_readable_as_its_name`, all 20. The
rail's divider already did this. Rows: none.

## Pending release

- **Soft themes: the side panel is one panel again.** On `porcelain`,
  `sand`, `mist` and `slate` every section of the nav sits in one raised
  panel, parted by a fine engraved line, instead of each being its own
  pill. A tab's card is pressed into the panel.
- The side panel's 5- and 15-minute load averages can be read on a
  see-through window. They were faded toward the theme's page colour, which
  a see-through window does not show; on the green and amber tubes over a
  light desktop they were close to invisible.
- Soft themes: the gauge dials are raised out of the card like round
  knobs, instead of grey discs with a ring drawn round them.
- Soft themes: a pane's scroll marker is a soft slot in the card instead
  of a hard dark line down its edge.
- In the side panel's tab cards, the number of a tab that is not on screen
  can be read: it was drawn in the faint colour of the card's outline.
