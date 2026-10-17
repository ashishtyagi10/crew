# Goal — the light and dark themes read like an e-reader: Ink Reader

**Status: SET 2026-10-16, DELIVERED 2026-10-17** (see "What shipped" at the end).

**Set:** 2026-10-16 by the user: *"write a goal to change light theme into Ink Reader like
look and feel. Apple kindle app does have that look. white background with black lines and
change the dark theme to dark Ink Reader look — once the goal written execute it."*

## What the two themes are today

The paper family (`crates/crew-theme/src/presets_paper.rs`) is the pair the user means by
"light" and "dark". Since the all-glass redesign of 2026-10-09 both are see-through glass with
white words: `paper-dark` a deep warm smoke, `paper-light` a warm paper-toned smoke, each with
a bloom halo round the focused frame, gradient poles on the meters and frames, and the glass's
text shadow. Neither reads as paper any more. The other six palettes (nebula, blossom,
glass-clear, glass-night and the two CRT tubes) stay glass. This goal is the two paper themes
only.

## The look: what the Kindle app's reading themes are

- **Ink Reader (light):** a white page, black words, black lines. Nothing glows, nothing is
  tinted, nothing shows through. Grey is the only second ink, for what is quieter. Colour is kept
  where it carries meaning (the terminal's ANSI output, a diff's red and green) and printed
  rather than lit: deep, desaturated inks that read on white.
- **Ink Reader (dark):** the same page inverted, as the Kindle app's black theme is: a black page,
  white-grey words, grey lines, the focused frame white.

## Done means

1. `paper-light` is an opaque white page (no glass, no window see-through): page `#FFFFFF`-ish,
   ink near-black, every frame line black or a grey of black, `dark: false`.
2. `paper-dark` is its inverse: an opaque near-black page, white-grey ink, grey lines, the
   focused frame white.
3. No glow, bloom, text shadow, scanline or gradient pole on either: no `crt`, no `modern`, no
   `liquid`. The focused frame is a plain, heavier line, the way an e-reader marks where you are.
4. Colour only where it means something: ANSI output and the semantic roles (status, bell,
   broadcast, activity, find) are printed inks that read on the page. Every one of the theme
   system's contrast guards holds for both, unchanged.
5. The rest of crew draws them correctly. Nothing assumes every theme is glass, the window is
   opaque under them, and the six glass palettes are untouched.
6. Shot and checked: the work shot and the survey shot cover both themes, and their pixels are
   measured (ink against page, lines against page), not judged by eye alone.
7. Documented: README and `docs/CREW.md` say what light and dark look like now, and the
   changelog says it in the project's voice.

## Not in this goal

Fonts (crew stays monospace; the Kindle app's serif is not the point), motion (the motion setting
already quiets the pane), and the theme names (`paper-light`/`paper-dark` stay, so nobody's saved
theme changes under them).

## What shipped (2026-10-17)

1. `paper-light`: page `(250, 250, 248)`, ink `(28, 28, 28)`, `dark: false`, no `liquid`,
   `crt` or `modern`. `paper-dark`: page `(10, 9, 8)`, ink `(232, 232, 232)`. `glass::print`
   gives both a sheet with no tint, gloss, rim or shadow, and `tubesheer` keeps the window opaque.
2. Lines: the focused frame is the ink (18.1:1 light, 16.2:1 dark). Unfocused frames are a grey of
   it at 3:1, from a ramp ladder of their own (`HOUSE_INK`: the paper ladder with
   `border_normal` at 3.0 instead of 2.0). "Heavier" is carried by tone, not stroke width: card
   corners are drawn at the light weight (`crew-render/src/corners.rs`), and a heavy rule
   meeting a light arc would look broken.
3. `dark` and `light` are one page each, no longer rotations. `nebula` and `blossom` moved to
   the `glass` rotation, which now has four palettes. A one-page pool's rotation tick returns
   `false`, so nothing repaints.
4. Fallbacks that were lit on a page with no gradient now use a grey of the ink: the scroll
   thumb and `⇡N` (`panescroll::position_fg`), next to the meters (`summarymeter`). `/gradient`
   on an Ink page is kept for the glass themes, and the status line says so.
5. The title bar wears the page on an Ink page (`titlebar::wear_for`), because the native bar
   follows the Mac (a dark strip across the white page on a dark-mode Mac). Not screenshotted:
   the session had no Screen Recording permission, so it needs a look in the live app.
6. Shots measured (1280×720 at 1×): ink 16.3:1 / 16.2:1 on the page, unfocused lines
   `(146, 146, 145)` / `(94, 93, 91)` at 3:1, the thumb `text_muted`. Both the work shot and the
   survey shot now cover every palette, the Ink pages included.

Left alone on purpose: text weight. Dark-on-white words draw thinner than white-on-black
(about 8% of a text row is ink against 19%), because `textgamma` delivers each glyph's exact
outline light on both polarities. `/smooth` is the knob for anyone who wants it heavier.
