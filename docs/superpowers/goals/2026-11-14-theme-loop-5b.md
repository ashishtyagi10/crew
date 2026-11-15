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
| 0 | 63efb0e6 | 0.30.15 | soft: the nav is one raised panel, grooves between sections, tab cards as wells (`navrelief.rs`) | 10 raised tiles + a tile per tab → 1 panel, 9 grooves, a well per tab |
| 1 | c84248d9 | 0.30.15 | LOAD's 5- and 15-minute figures read on see-through glass (`load.rs` `on_frost`) | worst contrast over a black/white desktop, sheer: 1.27-2.09:1 on 11 palettes (3.0-3.4 on neon, prism) → ≥ 3.5:1, or the 1-minute colour where that has no headroom; 13 palettes changed |
| 2 | c22bb00d | 0.30.15 | soft: a dial's face is moulded out of the card (`plot/dialrelief.rs`) | porcelain, sand, mist, slate: flat 6% ink disc + stroked bezel arc → the card's own colour raised by a light and a dark shadow, 0 strokes round the face; 16 other palettes byte-identical |
| 3 | e3415563 | 0.30.15 | soft: the scroll thumb is a slot in the card (`cardpaint.rs`) | porcelain, sand, mist, slate: opaque bar (alpha 0.95) riding a frame that is not drawn → the reading's colour at 0.45 with a lit far lip; 16 other palettes byte-identical |
| 4 | f57b3b02 | 0.30.15 | every palette: a background tab's number is text (`tabcard.rs`) | number vs page: 2.0:1 on 16 palettes, 2.4 on the tubes, 3.0 on the Ink pages (`border_normal`) → `text_muted`, the colour its name wears; all 20 changed |
| 5 | 413d1c03 | 0.30.15 | glass: a reply's list bullets read on a see-through window, and an audit holds every drawn glyph there (`chatink::marker_fg`, `glassaudit_tests.rs`) | bullets over the frost: 2.58 blossom, 2.52 glass-clear, 2.59 glass-night → ≥ 3.0; glyphs under the floor across the nav, a tools window and smith + a shell on 16 glass palettes: 3 colours → 0 |

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

### Round 5 — an audit of what a see-through window draws
Behaviour: round 1's defect was a class, a colour derived against the page
by the widget that draws it. `glassaudit_tests.rs` builds the nav, a window
of tools (dashboard, to-do, file panels) and smith mid-turn beside a shell
on every glass palette and measures each drawn glyph over the frost on a
black desktop and a white one (fills against themselves; the settings
swatches exempt). It found one more: the bullets of a list in a reply,
which now take the mark floor over the frost while the window is sheer.
Not covered: the settings form, whose scrolled-off group fades on purpose.
Rows: none. Opaque windows: byte-identical.

## Close

Five rounds and the nav panel that started the evening, all merged and
pushed, released together on the user's ask. Honest scope: rounds 1 and 5
change the see-through palettes (13 and 3 of them), round 4 all 20, rounds
0, 2 and 3 the four soft palettes. The two contact sheets showed nothing
to change on the Ink Reader pages beyond round 4.

Open: on the soft themes the active tab's well is no deeper than the
others; `Save`/`Cancel` and the status chips are flat; `/far`'s selected
row is a square grey bar in a round well; the glass audit does not cover
the inbox, the file viewer or pop-ups.

## Pending release

Nothing: all of the above went out as v0.30.15.
