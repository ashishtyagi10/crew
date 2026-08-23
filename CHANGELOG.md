# Changelog

Notable changes per release, newest first. Versions are the workspace version
in `Cargo.toml`; every tag builds a release the app picks up through
`/update`.

The top entry must always name the current version — `changelog_covers_the_
current_version` in `crew-app` asserts it, so a release cannot ship without a
line saying what it was.

## 0.18.3

**The bell and the status line were the same colour.** They mean different
things. The status colour is progress — the git dirty dot, the input-bar
status, gauge fills. The bell colour is *needs you*: the attention glyphs a
pane wears (`!` rang, `⚑` matched, `✓` agent done, `⊗` exited, `?` waiting) and
every ERROR line in the log. On `paper-dark` and `sepia-dark` they shipped as
literally one value, and on `paper-light` and `sepia-light` within one visible
step of it. A pane that had finished looked like a pane that was still working.

`nebula` and `blossom` had already solved this by hand, and they say what the
rule is: the alarm breaks away from the hue progress is already using. So it is
derived now, taking its hue from the palette's own red — nothing invented — and
sitting at exactly the status's loudness, so the two markers are equally
readable and not the same marker. Four palettes move; the two that did it
properly are left exactly as they were.

**The three CRT tubes keep theirs, deliberately.** A phosphor has one hue, and
that hue is the entire theme; rotating `crt-green`'s bell to orange would
separate the markers and destroy the palette. On a tube the separation is the
one a real terminal used — the attention glyph blinks.

## 0.18.2

**The search highlight was invisible on light pages.** `find_hl_bg` is the one
background colour a palette ships, and the colour-system work never reached it
— that derived the *text* ladder, and this is a wash sitting behind text. So it
stayed hand-picked, and it split by appearance: the wash sat 0.106–0.134 off
the page on the three light themes and 0.173–0.246 off it on the six dark ones,
on a scale where 0.10 is one rung of the text hierarchy. `paper-light` bottomed
out at **1.25:1** — a search highlight you have to hunt for.

That is structural rather than four bad guesses. On a dark page a highlight can
gain lightness *and* colour; on a bright one the presets reached for a pale
yellow that barely moves off the paper.

The wash is derived now, with the palette still declaring what colour its
highlight is — sepia's amber, nebula's violet, the phosphor's own green — and
the system supplying only how far off the page it sits. The bar is the median
of what the nine palettes already do, applied as a floor: five are past it and
are untouched, and four move (`paper-light`, `sepia-light`, `blossom`, and
`nebula` by less than one visible step). Text on the highlight stays at 9.3:1
or better everywhere it changed.

## 0.18.1

**`auto` was stuck on dark all day on any Mac pinned to Dark.** The theme
promised "light by day, dark by night" and followed the OS appearance to
deliver it. macOS has three appearance settings, though, and the window
system only reports two: Appearance: Auto arrives as whichever side it is
currently showing, indistinguishable from someone having chosen that side
outright. So crew could see *which* appearance was active but never whether
it would ever change — and on a Mac set to Dark it never does, which made
`auto` a permanent synonym for `dark`, at noon, while the picker went on
promising otherwise.

crew asks the missing question now. While macOS switches its own appearance,
the OS still wins outright: it already encodes the day/night intent, sunset
schedule and all, and second-guessing it would be worse. Only once the
appearance is **pinned** does crew fall back to its own clock — a light-hours
window, `07:00`–`19:00` by default:

```toml
auto_light_from = "07:00"
auto_light_to   = "19:00"
```

Set them to your own hours; a window whose end is at or before its start
(`20:00`–`06:00`) wraps past midnight, for anyone whose day does. Crossing a
boundary switches the theme where you can see it happen rather than at the
next ten-minute rotation. Nothing changes on Linux or Windows, where there is
no way to tell a pinned appearance from a scheduled one: whatever the OS
reports is taken as the whole truth, exactly as before.

The window is wall-clock rather than real sunrise and sunset because crew has
no idea where it is, and a location permission prompt or a network call is a
poor trade for deciding a colour.

**And `/theme` says which clock is deciding.** "auto is dark at noon" and
"auto is dark because you pinned the OS to dark" used to be the same
sentence — a single word, `auto` — which is precisely how this shipped
broken and stayed that way. It now reads:

```
theme: auto — the OS appearance is pinned, so the clock decides: it is day,
serving light; the dark half is dark (light hours are 07:00–19:00, set with
auto_light_from / auto_light_to)
```

## 0.18.0

**Nine themes instead of twenty-four, and one colour system underneath them
all.** A minor bump rather than a patch, because fifteen palettes are gone
and every remaining colour was re-derived.

### The palettes are a system now, not 2,376 opinions

Every colour in every theme used to be picked by eye, one theme at a time —
seventeen roles plus a sixteen-slot terminal palette, times twenty-four. They
all passed the contrast checks, because the checks were run afterwards and
the failures nudged by hand. Nothing related any two of them: measured across
the themes, the *same role* landed anywhere in a **2x band**, and the accent
in a **4.9x band**. So switching themes did not move the palette to the same
place. It moved it to whatever was decided on a different afternoon.

Roles are derived now. A palette states what it is — the page, the hue and
saturation of its ink — and the ladder below that (body text, legends, hints,
placeholders, borders) follows from one shared set of levels, computed in a
perceptual colour space so that "one step dimmer" means the same thing at
every hue. Twenty of the twenty-four moved by less than half a step in the
process; rendered side by side, the largest frame changed by 1.4%.

Two deliberate exceptions, both because a rule that fits paper does not fit a
phosphor. **No theme's text goes near-white** any more: matching contrast on a
page lighter than its neighbours wanted `(250, 250, 252)` on one theme, which
is the glare every dark-mode guide warns about, so there is a ceiling. And a
**white** CRT tube keeps its brightness rather than being held to the coloured
tubes' levels — being bright is that theme's whole point.

### Shell output got the same treatment, and it needed it

The terminal palette was checked for legibility and nothing else. Three things
were wrong. ANSI black bottomed out at **1.36:1** on dark themes — very nearly
the background, so anything printed in it was invisible. Slots ranged from
4.6 to 17.3 against their own background depending on theme and colour. And
nothing ever compared the colours to *each other*: on the amber tube, ANSI
green and yellow sat closer together than eight shades of grey, which is to
say `ls --color` and `git diff` were drawing two different things in the same
colour.

All sixteen slots are derived and checked now, including the blacks and whites
nobody had ever looked at. Colour themes separate by hue, using crew's own
hues rather than an imported convention. Phosphor tubes separate by
brightness, the way a real amber monitor did — spreading them by hue would
have made them legible and destroyed them.

### Twenty-four palettes, several of which were the same palette

The two closest were **Δ 0.0209** apart, under the point where two greys stop
being distinguishable. Nine more pairs were nearly as close. The nine that
remain were picked by measuring which are genuinely most different from one
another, then making sure each family kept both its light and dark side:

- **paper** dark and light — newsprint, both ways up
- **sepia** dark and light — the warm pair
- **nebula** and **blossom** — the modern look, dark and light
- **crt-green**, **crt-amber**, **crt-blue** — the three tubes

The closest pair among them is now nearly three times further apart than the
closest pair before.

**If your config names a retired theme, it still works.** All fifteen resolve
to their nearest surviving relative, always of the same appearance — a dark
desk does not suddenly go white. `graphite` becomes `paper-dark`, `aurora`
becomes `nebula`, and so on.

### And some tube maths that had been running for nothing

The CRT pass carried barrel curvature and a corner vignette that every theme
had set to zero since the flat-tube decree. The shader was warping by an
identity and multiplying by one, per pixel, per frame. Both are gone — and
because "this changes nothing" is a checkable claim, it was checked: every
frame is pixel-identical.

## 0.17.12

**A failed update no longer removes crew.** This was worse than an update
that did not work. The library crew used to swap in the new binary does
three things in order on Windows: rename the running `crew.exe` aside, copy
itself into `%TEMP%` and *run* that copy to clean up, then move the new
build into place. The middle step executes a program out of `%TEMP%`, which
managed machines and most corporate antivirus refuse — `Access denied` — and
it happens *after* the rename, so the whole thing gives up having already
moved your `crew.exe` away and never puts anything back. If your install
looks broken after a failed update, re-running `install.ps1` restores it.

crew now does the swap itself, the same way `install.ps1` already does it:
rename aside, move the new one in, and **put the old one back if that
fails** — so a failed update leaves a working crew rather than none. No
second process, no running anything from `%TEMP%`, no administrator rights.
The download is staged next to the binary it replaces rather than in the
temp directory, which is both more likely to be permitted and the only way
the move can work at all when temp is on another drive. The superseded
binary is cleaned up on the next launch, since Windows will not delete a
program while it is running.

`crew --self-update` had the same flaw and shares the fixed path now.

**And a failed update says why.** It used to show a card in the sidebar that
cleared after a few seconds and record nothing anywhere, so the only
possible report was "it failed". Update outcomes now go to the LOG and to
`activity.log`, naming the step that failed.

## 0.17.11

**No more console flash on Windows.** Launching crew from the Start menu or
Explorer flashed a black console window every time, and nothing in the
program could stop it: Windows hands a console-subsystem program a window
before its first instruction runs, so even closing it immediately leaves it
up for the whole of startup. crew is a GUI-subsystem program now and no
console is ever created.

The console modes still work — `crew --version`, `crew ask`, `crew panes`,
`--list-fonts` all reattach to the terminal that launched them, and piped
output still goes to the pipe. One thing genuinely changes: shells do not
wait for a GUI program, so a command typed at a prompt may print *after*
the prompt comes back. There is no way to have both.

**Panes open somewhere useful again.** Double-clicking `crew.exe` in the
unzipped download made every pane open *inside that download*, so the
prompt read `PS C:\Users\me\Downloads\crew-v0.17.10-x86_64-pc-windows-msvc>`
— the exe's own folder, reported as the place you were working. Windows
sets the working directory to the exe's folder when Explorer launches it,
and crew took that at face value. It now starts at your home directory
when the directory was picked by a launcher rather than by you. The same
fix covers a macOS Dock launch, which lands at the filesystem root.

**And the path reads as a path.** Three smaller Windows bugs stacked on top
of each other: `$HOME` does not exist there, so the `~` shortening never
happened and `cd ~` did nothing; the path separator was hardcoded to `/`,
so `C:\Users\me\code` would not have shortened anyway; and Windows'
`canonicalize` returns `\\?\C:\Users\me\code`, which crew was showing
verbatim. Between them, a path that should read `~\code` was displayed in
full with four characters of prefix noise.

## 0.17.10

Two Windows bugs, both found from a photo of a running v0.17.9 screen.

**A new terminal window opened every three seconds.** `crew.exe` is a
console program — it has to be, because `crew --version`, `crew ask` and
the broker all need real stdio — but the window itself is launched
detached, which on Windows means it has no console of its own. Windows
hands a brand-new console *window* to any console program started by a
process without one, and crew checks `git status` every three seconds for
the sidebar. So: a terminal, every three seconds, forever, taking focus
each time, while the main window sat there looking perfectly fine. The
broker, the shell probe, every MCP server and the `/far` and file-viewer
helpers did the same thing on startup. Every one of them is now started
with an invisible console instead — the pipes work exactly as before.

**Text was still mangled, because the font crew ships was the last
resort.** Embedding Lilex in 0.17.9 only helped a machine that resolved
*nothing*; it went in behind `Noto Sans Mono`, so any machine carrying
that name still preferred it — and still drew a broken grid. The grid
rounds every glyph to the nearest whole cell, which is invisible for a
real terminal face and brutal for anything else: wide letters take two
cells and narrow ones (`i`, `l`, `.`, and the space) take **none**, so
`terminals.` came out as `term inals` and `commands` as `com m ands`.

The generic fallbacks are gone from the theme preference lists. Their
whole job was "resolve to something", and crew now carries a face of its
own that does that better. They are still in the picker, so `/font` can
still reach them — they are just never crew's automatic answer.

And crew now checks its own work: before a font can be used, it shapes a
probe through the real grid, at the real size and weight, and confirms
every glyph lands on exactly one cell. The old check measured the font it
found by name; this one measures the font that actually gets drawn. On
that Windows machine they were not the same font, which is how a
proportional render passed a fixed-pitch test.

## 0.17.9

crew now ships its own typeface, and a fresh Windows install finally looks
like crew.

Until this release, first launch on Windows drew the whole app in **Segoe
UI — a proportional face**. The banner and the left nav looked worst,
because a box-drawn frame shows column drift the instant it drifts.

Nothing about it was Windows-specific in the code. crew asks the text
engine for "the monospace family" and never says which one; cosmic-text
hardcodes that to `Noto Sans Mono`, which Windows does not have, so the
lookup missed and shaping fell through to the platform's general fallback
list — headed by Segoe UI. Then the grid did what it always does and
rounded every glyph advance to the nearest cell, which sends a
proportional face's narrow glyphs (`i`, `l`, `.`, `|`) to *zero* width.
Glyphs land on top of each other and every column after them slides.

Themes could not save it. Each one names the faces it would like, ending
in something that ships with an OS so a bare machine still resolves
*something* — except the tails were `Menlo`, `SF Mono`, `Noto Sans Mono`.
All three are macOS or Linux stock, and 23 of the 24 themes named nothing
a stock Windows box has. macOS was never actually immune either: its own
fallback list is headed by the system UI sans, just as proportional. It
escaped only because `Menlo` is in every list and every Mac has it.

So crew stops asking the machine. It embeds Lilex — the face the CRT
themes already led with — and registers it as the monospace family, four
weights and their italics, about 840 KB. A font you have installed still
wins: pick one with `/font`, or let the theme choose. There just is no
longer a machine where the answer is "none".

## 0.17.8

Opening something stops freezing everything. Cmd+clicking a URL, `/far`'s
Open and the browser hand-off at sign-in all went through the blocking
form of the opener, which waits for the program it launched to exit — and
they run on the thread that draws, so until that program came back, every
pane sat still. They now hand the file off and return.

Windows got the sharper end of it. Asked to open a path with no
association, Windows raises the "How do you want to open this file?"
dialog, and nothing dismisses that on a machine with no one in front of
it: CI sat on that modal for five and a half hours before anyone noticed
the build had not failed, only stopped.

The input bar can recognise a command on Windows again. Deciding whether
what you typed is worth a pane means resolving the first word against
PATH, and that search read a Unix PATH: split on `:`, which tears `C:\bin`
into `C` and `\bin`, and treating only `/` as a sign of a path, so
`C:\tools\rg.exe` looked like a bare word to go hunting for. Nothing ever
resolved, so every line you typed came back "not a command" and the bar
hinted instead of running it. Entries are split the way the platform
writes them now, a path is anything with a parent, and runnability follows
`PATHEXT` — which also means a bare `git` finds `git.exe`.

Windows CI passes for the first time, end to end: the release binary
builds and the installed app starts. A hung job now fails in 45 minutes
naming the step that hung, rather than running until the six-hour ceiling,
and a superseded run is cancelled instead of left to finish.

## 0.17.7

Three themes again. The modern family arrived as two themes of its own —
`modern` and `modern-light` — which meant the picker offered five looks
where the difference between two of them was only *which palettes*
rotate, and the Gemini/Codex pages you had to go and choose deliberately
never turned up in the rotation you actually run. They are pages like any
other, so they now rotate inside `dark` and `light`: the dark pool draws
from nine palettes (five paper, four glow), the light pool from ten (six
paper, four glow), and `crt` keeps its five phosphor tubes. `/theme`, the
settings picker and `Ctrl+Shift+L` are back to `dark → light → crt →
auto`.

A palette's own appearance decides its pool now, so nothing has to be
filed by hand: is it a phosphor tube (`ThemeId::is_crt` — which the
bloom-only `CrtStyle` the modern palettes carry for their halo
deliberately does not make it), and if not, is its page dark or light.
Every palette lands in exactly one pool, and a rotation still never
flips the page near-black↔near-white under you.

`modern` and `modern-light` keep parsing — from `/theme`, from `theme`,
and from `auto`'s `theme_dark` / `theme_light` pairing — resolving to the
pool that swallowed them, so a saved config opens on the appearance it
asked for instead of silently doing nothing.

## 0.17.6

`/theme` stops whispering. Two silences made a theme that works look like
a theme that does nothing, and both are gone.

A name this build doesn't know — `/theme modern-light` on any build from
before 0.17.5, say — changed nothing on screen and explained itself in a
three-second flash on the input bar's bottom border, which is exactly the
kind of message you miss while watching the page for a colour that never
arrives. That report is now an error: it raises a toast on the canvas and
stays in the LOG after the flash expires, and it still lists the modes
that DO exist.

`auto` now names the half it is serving. It used to report the bare word
"auto", so a pairing configured for the appearance you are not currently
in had no symptom at all: `theme_light = "modern-light"` under a dark
macOS renders identically to a config line that was ignored. `/theme`
with no argument now answers "theme: auto — OS is dark, serving dark; the
light half is modern-light (it shows when the OS turns light)", in the
input bar and in a chat pane's listing alike.

## 0.17.5

The modern family gets its light half. Daybreak, Blossom, Meadow and
Cirrus are the same Gemini/Codex look with the lights on — near-white
pages, deep slate ink, and each palette's two poles driving all three
of the family's signatures: the gradient ring on the focused frame,
the drifting wash under the page, and the dot lattice woven over it.
The poles are deepened rather than reused: a pastel that glows on
near-black is invisible on paper. They rotate as their own theme,
`modern-light`, which joins dark/light/crt/modern/auto in `/theme`,
the settings picker and the Ctrl+Shift+L cycle — so a rotation never
flips the page from near-black to near-white mid-session, and `auto`
can pair the light half by day with the dark half by night.

The halo had to be rebuilt to get there. The bloom chain only knew how
to add light, which on a near-white page blooms the PAGE and clips the
whole frame to flat white. On a light page the pass now keeps each
pixel's colourfulness instead of its brightness, and the composite
subtracts the blur: the ring lays a soft halo of its own hue on the
paper, coloured text glows the same way, and neutral body ink stays
crisp. Every palette was validated against the contrast suite before
it was written.

## 0.17.4

Done todos stop hiding. A todo pane whose items are all ticked used to
render the same "no todos" screen as an empty one — with nothing to
select, `H` couldn't even be reached — so finished work looked like no
work. That pane now says "all done · 7 in the history" and names the
way in, and any list holding ticked items grows a clickable
`[show 7 done]` button on its header (it flips to `[hide done]`). For
the keyboard: `/todo show` and `/todo hide` do the same from the
command bar, `h` still toggles on a focused list, and an empty
filtered history now says "nothing done in @home" instead of claiming
nothing was ever finished.

## 0.17.3

The modern themes get their aurora. Under the dot lattice, two broad
pools of each theme's own gradient poles now light the page from
opposite sides — Aurora's blue and violet, Nebula's violet and rose,
Graphene's greens, Cobalt's blues — so the backdrop reads as coloured
light instead of a flat fill. The pools turn slowly about the centre
while a pane is working and hold wherever they stopped when things go
quiet, so an idle crew still never repaints a frame, and Motion off
freezes them outright.

## 0.17.2

The modern themes' dot lattice pulls in tight. The grid was pitched
every 4th column and 2nd row, which read as sparse polka dots; it is
now a fine square weave — about six dots to a text row, with pin-fine
dots that never touch — so Aurora, Nebula, Graphene and Cobalt sit on
woven engineering paper instead of a widely spaced grid. It still
rides the cell metrics, so it scales with font size and DPI, and it is
still perfectly static.

## 0.17.1

The modern themes get their engineering paper. A faint lattice of soft
dots now sits behind everything on Aurora, Nebula, Graphene and Cobalt
— pitched to the text grid (every 4th column, every 2nd row, so it
scales with font size and DPI) and tinted with each theme's own
gradient poles, sliding from one to the other across the page. Purely
static — it never animates or costs a frame — and the other families
keep their newsprint grain untouched.

## 0.17.0

See what you finished, not just what's left. `/todo done` (or `H` on
the todo list, Esc to come back) opens the done history: a done-only
log grouped under day headers — today, yesterday, aug 10 — newest
tick first, each row wearing its tick time and `@project` chip.
Ticking an item now records *when* (items ticked before this release
group under "earlier"), `@project` filters the log — `/todo done
@crew` opens it pre-filtered — Space un-dones an entry back onto the
list, and `d` deletes it from history for good. Ticked something last
week and can't remember? Check the log before doing it twice.

## 0.16.13

Intel One Mono joins the font rotation — the allowlist gained it and
Cascadia Code, so `/font` roulette can land on both.

## 0.16.12

Markdown tables with CJK or emoji stay inside their lane. The table
renderer padded columns by display width but clamped rows by character
count, so a wide-glyph row could run up to twice the pane budget and
wrap mid-table. The clamp now counts display columns (and still bounds
characters, so zero-width runs can't slip through); a wide glyph
straddling the edge is dropped whole, never split.

## 0.16.11

Flip between projects without typing. `]` and `[` on the todo list
cycle the `@project` filter through your tags in usage order — "no
filter" is one stop on the ring — with the selection re-entering at
the top of each view. Combined with the per-project colors, flicking
through contexts is now a two-key habit.

## 0.16.10

Deadlines move without retyping them. `+` and `-` on a selected todo
postpone or advance its due date one calendar day — real calendar
math, so a 9:00 due stays 9:00 across DST and month ends. `+` on an
undated item starts it at tomorrow morning. A moved deadline arms its
due toast again.

## 0.16.9

The tag popup speaks in color. Completing an `@project` in the todo
composer now shows every candidate row in that project's own color —
the same one its chips wear in the list — so you can pick the tag by
hue before reading it. The command palette keeps its usual accent.

## 0.16.8

Done todos get a way back. Completing an item still hides it on the
spot, but `h` on the list now shows the done pile — sunk below the
open items, dimmed, `[x]`-boxed, newest completion first — so a
mistaken tick is one `Space` away from alive again. `h` again (or
just moving on) tucks them back out of sight.

## 0.16.7

Swarm runs leave a trail. When a `/smith` run spawns agents and its
tasks start, finish or fail, those beats now land in the LOG — and in
`/log`'s session file — as quiet lines that speak task titles
(`smith: task #2 'scan logs' → done`), with failures at the error
level. Quiet means quiet: unlike other LOG events they never flash the
input bar or raise a toast, and the high-volume token/output ticks
stay out entirely.

## 0.16.6

`/log` opens the whole story. The sidebar LOG shows only the last five
lines of activity; now every entry is also mirrored to `activity.log`
beside the config (fresh each session, errors marked `ERR`), and `/log`
opens the full trail in the file viewer — press `r` there to re-read.
A wedged or crashed session leaves its trail on disk for the next one.

## 0.16.5

The todo composer gains a real cursor. Until now the draft was
append-only — a typo at the start of a long multiline draft meant
backspacing everything after it. Now `←`/`→` move by character,
`Alt+←`/`Alt+→` hop words, `Ctrl+A`/`Ctrl+E` (or `Home`/`End`) jump to
the draft's ends, and typing, paste and forward-Delete all act at the
cursor — with the `▏` beam drawn exactly where you are, the live due
and `@tag` tints intact around a mid-string edit, and the capped card
following the cursor's line instead of the tail. On a wrapped draft
`↑`/`↓` travel the visual lines at the nearest column; only the edges
hand focus to the list.

## 0.16.4

The todo list pages from the keyboard. `PageUp`/`PageDown` hop the
selection a whole visible page of items — a page is the rows the window
actually shows, so wrapped multi-row titles count for their real height
— and `Home`/`End` jump to the first/last item. All four respect an
active `@project` filter and keep the selection scrolled into view; the
`Shift+` chords keep their app-wide pane-scroll meaning. The new keys
are listed in `/keys` and the manual.

## 0.16.3

Every `@project` gets its own color. A mixed todo list now reads by
project at a glance: the tag's name hashes to one of the theme's twelve
chromatic terminal-palette slots — nothing stored, so every pane,
restart and platform agrees, and the same project keeps the same slot
on every theme (switching themes recolors consistently instead of
reshuffling). Each color is lifted toward the theme's ink until it
clears a 3.0 contrast floor against the page, so light themes stay
readable. The row chip, the live tint while you type `@crew` in the
composer, the composer legend under an active filter, and the filter
header's `@tag` all agree on the color; due dates keep the accent.

## 0.16.2

Theme switches stop blanking the window. The old switch drew a solid
page-color wash at full opacity and faded it out — for its first frames
the screen was empty, and `/theme` read as a mini restart. Now the
renderer keeps a live snapshot of the last presented frame; when the
theme flips, the new look renders in full from its very first frame and
the old frame melts away over it (~450ms crossfade). Content is visible
at every instant — dark→light, CRT→modern, an OS appearance flip in
`auto`, all one continuous develop. With Motion off the switch is an
instant cut, and it is never a blank screen.

## 0.16.1

The todo composer wraps. A long title flows onto new rows as it fills
the card's width instead of scrolling out of sight in a single line —
the card grows with it, up to four rows.

## 0.16.0

Crew learns a modern look. A fifth theme mode, `modern`, joins dark /
light / crt / auto: four new palettes — `aurora` (Gemini blue→violet),
`nebula` (orchid→rose), `graphene` (Codex neutral with a mint accent)
and `cobalt` (electric blue→cyan) — with deep neutral pages, zero paper
grain, and a soft wide glow that runs the bloom chain with every retro
knob (curvature, scanlines, bezel) at zero: clean light, not a tube.
The focused pane's frame becomes a gradient light-ring, blending
corner-to-corner between each palette's two accent poles; it ignites
white when focus arrives, drifts slowly along the frame while the pane
is streaming, and holds perfectly still when idle. Modern themes lead
with modern typefaces (Google Sans Code, Commit Mono, Martian Mono,
Geist Mono) and sit in `/theme`, the settings picker and the
`Ctrl+Shift+L` cycle — `/theme modern` to move in.

## 0.15.1

The todo list stays a list of things left to do. Completing an item hides
it on the spot — done items no longer pile up under the open ones (they
remain in `todos.toml` as history), and the `@project` filter header
counts only what's open. Long todos now wrap instead of clipping: a
title flows onto as many full-width rows as it needs, the due/`@tag`
chips keep the first row, and clicking anywhere on a wrapped item selects
it — so the whole task is always readable at any pane width.

## 0.15.0

`/todo` opens a todo pane: one global list you type into. Enter captures
an item; a natural-language date fragment (`tomorrow`, `fri 5pm`,
`aug 15`) tints live as you type and becomes the due on save, and an
`@project` token becomes a free-form tag, autocompleted from tags already
in use. The list sorts overdue → due → undated, Space/Enter completes,
`d` deletes, `e` re-opens an item in the composer, and a lone `@tag`
filters the list to one project. Items live in `todos.toml`, every todo
pane shows the same list, and a due item raises a toast the minute it
lands.

## 0.14.4

Theme flips reach the programs inside the panes now. Terminals that enable
DECSET 2031 (neovim's TUI and other scheme-aware programs) get a
`CSI ? 997 ; Ps n` report the moment crew's light/dark scheme changes —
an OS appearance flip under `auto`, `/theme`, `Ctrl+Shift+L`, a Settings
apply — so they re-query OSC 10/11 and repaint for the new palette
mid-session instead of riding the contrast floor until restart. Crew also
answers the `CSI ? 996 n` "what scheme is it?" query and DECRQM support
probes for mode 2031, all from the same active-theme source of truth the
OSC 10/11 answers use. Programs that never opt in see nothing new.

## 0.14.3

Pair `auto` your way. Two new `config.toml` keys — `theme_dark` and
`theme_light` — re-wire what the OS-following `auto` theme serves per
appearance: each side names a rotation pool (`dark` | `light` | `crt`) or
a pinned palette, so night can be green phosphor while day stays light
paper (`theme_dark = "crt"`), or dark mode can live on one exact palette
(`theme_dark = "moss-blotter"`). Unset keys keep the built-in paper
pairing; `auto` can't serve as its own side. A Settings apply that
changes the pairing re-themes immediately — no waiting for the next OS
flip or rotation tick.

## 0.14.2

Crew follows the system now. `auto` — light by day, dark by night, riding
the OS appearance — graduates from unlisted back-compat alias to crew's
fourth first-class theme: it sits in the `/theme` picker, the Settings
form, the composer suggestions and the `Ctrl+Shift+L` cycle
(dark → light → crt → auto), and a fresh install with no saved theme now
defaults to it, so crew matches the system from the very first frame.
An appearance flip mid-session re-themes every pane live through the
develop-fade. Existing configs are untouched: a saved `dark`, `light`,
`crt`, or pinned palette keeps exactly its meaning — picking one is still
how you opt out of following, and `/theme auto` is how you opt back in.
Both startup and settings-apply now resolve the saved value through one
shared `theme_selection`, so the two paths can never disagree.

## 0.14.1

The glyph atlas breathes again. The vendored glyphon atlas was never
trimmed, so its in-use set only ever grew and LRU eviction was dead code —
every font size, family, weight, or smoothing change permanently pinned a
full ~570-glyph working set (the 0.13.7 prewarm made that a certainty),
ratcheting GPU memory until a long session could hit an AtlasFull panic.
`CellGrid::prepare` now opens each frame with `atlas.trim()`, so only
glyphs the frame actually draws stay protected and stale generations age
out under real LRU pressure. And the sidebar tells the whole truth again:
panes standing behind the minimized strip's `+N` overflow tile — visible
nowhere on the canvas — now carry the `[+]` restore marker on their PANES
rows, like every other off-screen pane.

## 0.14.0

One spacing rhythm across the canvas. Three layout-rhythm fixes close out
the 0.13.x look-and-feel loop. Legends now ellipsize instead of colliding
with the frame: a title too long for its card ends `… ─╮` — width-aware, so
emoji/CJK clip on a cell boundary — with the trailing rule always resuming
before the corner, and every card (panes, strip thumbnails, toasts, sidebar,
input bar) goes through the same `title_budget`/`clip_w` pair, replacing
three near-identical private clippers. The grid now sits exactly one gap
above the input bar at every font size: the content area's bottom edge is
derived from the bar's real cell-quantized top instead of a fixed reserve,
so the seam no longer wanders with the cell-height remainder (it ranged
2–22px, and could crush to nothing at large fonts). And a crowded minimized
strip caps its thumbnails at a readable width — the most-recently-active
minimized panes keep their cards, display order stays sorted, and a
trailing `+N` tile stands in for the rest, which remain one click away in
the sidebar's PANES list.

## 0.13.9

Font smoothing joins the Settings form. The flagship v0.12.5 stem-darkening
was `/smooth`-only — invisible unless you knew the command. The APPEARANCE
card now carries a **Smoothing** picker (`off · light · medium · heavy`,
Space/←/→ to cycle) sitting under the font fields. It reads and writes the
same `font_smooth` key as `/smooth` — the keyword ladder now lives in one
shared table, so the two surfaces cannot drift — a custom `/smooth 42`
strength shows as its number and is left alone until you actually cycle it,
and Save applies live through the same renderer path the command uses.

## 0.13.8

Three new themes. `crt-paperwhite` is the fifth phosphor — the P4 white
tube of the early Macintosh and the VT420: near-white ink with the faintest
blue-gray cast on a true black tube, fine scanlines, a modest cool halo and
the steadiest beam of the family. `moss-blotter` joins the dark papers: a
deep desaturated moss-green desk blotter with warm paper-white ink and
botanical accents — the study-lamp page. `glacier-bond` joins the light
papers: a cold blue-gray bond page, like overcast north light on cold-press
stock, with crisp near-black ink and slate-blue accents — where
coldpress-gray is strictly neutral, this one deliberately runs cold. All
three pass the palette arbiter's contrast floors and slot into the
dark/light/crt rotation pools.

## 0.13.7

Prewarmed glyph atlas. The first frames used to pay for every glyph in
sight — rasterize, smooth, pack, and (at Retina sizes) grow the 256² atlas
twice, re-uploading everything already packed each time. The mask atlas now
starts at its Retina steady state (1024², still just 1 MiB; the color/emoji
atlas stays small), and at startup — and after any font, size, weight, or
smoothing change — crew shapes one off-screen buffer of printable ASCII
plus the box-drawing, block, braille, and marker glyphs its chrome draws,
and runs it through the same smoothing-seeded path as real frames. Every
border, spinner, and keystroke thereafter finds its glyph already in the
atlas: no rasterization, no grow churn, mid-interaction.

## 0.13.6

Pixel-snap pane rects. The scene runs in physical pixels end-to-end, but the
grid math handed panes fractional origins — so every glyph could land in any
of four subpixel bins, quadrupling atlas entries and grow churn, and border
hairlines straddled the pixel grid. Pane rects (grid tiles, the zoom tile,
minimized strip thumbnails, zoom-transition frames) now snap edges to whole
device pixels at the single rect source: one atlas bin per glyph, crisper
stems, sharper 1px strokes. Neighbors share the same snapped boundary, so no
seam can open; the glide animation stays fractional in flight and lands on
the snapped target when it settles.

## 0.13.5

CRT goes flat. The holographic glass sheet under every tube pane — a ramped
phosphor fill with a specular hairline and inner edge-glow — read as a drop
shadow that set the panes adrift, floating farther apart than the same grid
on paper-dark. It is retired: CRT panes now sit flush on the page exactly
like the dark themes, and what says "tube" is what should have all along —
a heavier 3.5px frame (the thickest in the app), a stronger bloom on all
four phosphors, and the CRT typeface. Same grid, same gaps, more glow.

## 0.13.4

Focus spotlight. The focused pane holds full ink while every other pane's
content leans 15% toward the page — enough that the eye lands on the active
surface instantly in a full grid, mild enough that unfocused terminals stay
completely readable. The wash rides the same 260ms travel as the focus
brackets: the old pane dims exactly as the new one brightens, one motion.
Frames keep their own focus colors (this is about the ink, not the box),
backgrounds and selection bands keep their shape, and the spotlight follows
the selected pane even while the input bar owns the keyboard.

## 0.13.3

Theme switches are cinematic now. `/theme`, the cycle hotkey, auto-rotation
and an OS appearance flip used to hard-cut every pixel at once; now the new
theme's page washes the window and fades away over ~450ms — the page
*develops* its new look. One full-window quad drawn over everything (under
the CRT tube's curvature when one is active, so the dip stays inside the
glass); detection is a per-frame theme-id diff, so every switch path gets the
fade without knowing about it. Motion off keeps the instant cut, and a grace
frame past the fade guarantees the veil clears exactly to zero.

## 0.13.2

The sidebar's PANES section is alive now. A **crew pulse** line chart sits
under the header — one sample per second of how many panes are doing
background work, auto-scaled, so a working swarm reads as a mountain range
and an idle crew as a flat baseline. Busy panes' rows carry a live spinner
in the accent color where the activity dot sits (attention markers still
win the slot). Costs nothing extra: the chart rides the sidebar's existing
1 Hz refresh and the spinner rides the repaints a busy pane already makes.

## 0.13.1

Grid reflow glides. Opening, closing, minimizing or restoring a pane used to
teleport every surviving pane to its new tile in one frame; now each card
glides there (exponential smoothing toward the placed rect, ~250ms, exact
snap on arrival). Restoring a minimized pane glides it out of wherever it
last sat; leaving zoom glides the pane back into its tile. Clicks during a
glide land on the target tiles — where the panes are heading. Motion off
keeps the old instant snap, and settled panes schedule no frames.

## 0.13.0

Toast notifications. Pane events — agent finished, bell, watched-pattern
match, pane exited, waiting-for-you — and error statuses now step onto the
canvas as toast cards docked at the top-right: a small fieldset card that
slides in, rests for ~5s, then dissolves and slides back out. Waiting, exited
and error toasts border in the bell color; up to four stack, oldest dropped
first. The input-bar flash and LOG entry still happen — the toast is the loud
surface, the bar the quiet one. Motion-gated: at Motion off cards appear and
vanish with no travel, and a resting toast never repaints an idle crew (frames
run only during the slide-in and exit windows).

## 0.12.8

Paper themes are flat again; glass is now the CRT family's look alone. Since
0.12.6 reset stale glass pins, light and dark themes were drawing their
derived frosted sheet — and its drop shadow, painted on the raw pane rect
while the fieldset frame is cell-quantized, ringed every card (and most
visibly the input bar) with a phantom offset box that read as "weird shadow /
misaligned input".

- **Flat dark & light.** The derived glass style for paper themes is now
  fully transparent — no sheet, no drop shadow, no grain; the renderer builds
  no cards at all. CRT themes keep the full holographic sheet and edge-glow.
  Verified on real pixels: paper deltas vs glass-off are 0.0 even at High.
- **The sheet hugs the drawn frame.** Glass cards now span the cell-quantized
  frame (`floor(px/cell)` per axis) instead of the raw rect, so CRT's glow
  can never overhang the border by a stray sub-cell strip either.
- `/glass` strength now only has a visible effect on CRT themes.

## 0.12.7

Auto-focus of blocked panes works on real agents now. Claude Code and Codex
approval prompts were never detected (v0.11.2's heuristic was calibrated
against idealized screens); tested against a live `claude` 2.1.222 run and
Codex's own TUI snapshots, three separate bugs fell out:

- **Waiting is now rendered-tail stability, not PTY-byte silence.** Claude
  Code repaints a blinking `⏺` every ~600 ms while its approval dialog just
  sits there, so the old "quiet for 3 s" byte gate never opened. A pane now
  counts as quiescent when the *text* of its bottom rows stops changing for
  3 s — blinks and OSC title churn don't move that text, while a thinking
  spinner's ticking `(12s · esc to interrupt)` line does, so
  waiting-vs-thinking still holds.

- **The prompt matcher reaches wrapped dialogs.** In a narrow pane, Claude
  Code's wrapped option lines + hint push "Do you want to proceed?" up to 8
  non-empty rows above the bottom — past the old 5-row window. The window is
  now 12 rows, and a `❯`/`›` option selector under a question row matches at
  any depth (Codex uses `›`).

- **Auto-focus no longer forfeits when you were typing.** Focus fired only
  on the exact tick a pane *became* blocked; if that edge landed within 5 s
  of your last keystroke — almost always, in a live session — the move was
  silently skipped forever. Focus is now level-triggered: the pane is
  surfaced as soon as you go hands-off, still once per episode.

Verified end-to-end by a new live-PTY test: a real shell replays the captured
Claude Code dialog with the blink running and must be auto-focused, only
after the stability window, while bytes are still flowing.

## 0.12.6

The CRT theme actually looks like a CRT again. Three fixes to one complaint
("it's just a dark theme with some colors"):

- **Theme switches clear stale look-killing pins.** A bare `/crt` toggle
  persisted `crt = false`, Settings could persist `glass = "off"`, and
  neither was ever cleared — so a months-old pin silently gutted the whole
  post-process (scanlines, bloom, flicker) and the glass sheet on every
  later theme switch. `/theme <x>` (and a composer theme switch) now resets
  the `/crt` pin to auto and glass `off` back to `medium`; a deliberate
  `low`/`high` glass strength survives. A one-shot migration heals existing
  configs on the first launch of 0.12.6.

- **Font smoothing no longer silently reverts.** The v0.12.5 vendored
  glyphon patch covered the render path but not the atlas `grow()` path,
  which re-rasterized every cached glyph unsmoothed — 2px smaller than its
  packed rect and 1px misplaced. The 256² atlas overflows on the first
  Retina frame, so smoothing died seconds into every session. Both
  materialization sites now read the seeded stem-darkened bitmaps.

- **The "updated to crew X" note works for the first time.** `clamped()`
  dropped `last_seen_version` on every config load since the feature
  shipped, so every launch read as a first run — no update note, and no
  version-gated migration could ever fire. The field now survives loading.

## 0.12.5

Font rendering catches up with the native terminals. The same font used to
read thin and rough next to ghostty or Warp because they rasterize through
CoreText — which never hints and dilates stems ("font smoothing") — while
crew's swash path hinted by default and drew exact outlines. Glyphs now
rasterize unhinted at every DPI, and a CoreText-style stem darkening
(fractional dilation, full strength horizontally, half vertically) is
applied to every glyph mask before it reaches the atlas. `/smooth
[off|light|medium|heavy|<0-255>]` tunes the strength live (default 100,
persisted as `font_smooth`); `/smooth off` keeps the unhinted outlines but
drops the darkening. Ships a vendored one-line glyphon patch so seeded
glyph bitmaps actually reach the GPU atlas.

## 0.12.4

Light-trace frames — the holographic overhaul's finale (part 3). On the CRT
themes a focused frame is now drawn in light, not RGB steps: its four
corner cells run white-hot so the bloom turns them into glowing nodes,
gaining focus fires a ~600ms ignition sweep (the whole frame ignites at the
node colour and decays to rest), and a streaming pane's frame breathes on a
slow ~2.4s cycle that rides the redraws it already schedules. Hierarchy
lives in the glow — a grayscale render through the real bloom + composite
chain separates focused from unfocused by halo alone, pixel-asserted — and
idle still converges: sweeps settle, breathing stops at exactly
`border_focused`, and paper themes' frame pixels are untouched.

## 0.12.3

CRT glass turns luminous (holographic overhaul, part 2). The "faintest of
the family" doctrine is repealed and the sheet becomes a translucent
phosphor panel — more opaque than paper-dark's glass, tinted in the tube's
own hue, with an inner edge-glow that makes each pane body read as lit by
its own frame. Paper light/dark pixels are untouched (proven
byte-identical), and the translucent-window path still shows the desktop
through the sheet.

## 0.12.2

The CRT holographic overhaul begins (goal 2026-08-04). `Theme.crt` grows
from a bool into a per-theme `CrtStyle` — the four phosphors finally get
personalities instead of sharing four compile-time constants — and the old
two-ring neighbour sample is replaced by a real half-res gaussian bloom
chain, so a focused border radiates tens of pixels instead of dying at ~8.

## 0.12.1

`/login` and `/logout` — OAuth becomes reachable. The 0.12.0 sign-in
existed but was invisible in practice: the pane's `/model 2` opened the
catalog popup (Enter then accepted a filtered model row, or its API-key
paste card) instead of submitting the broker's numbered provider pick, and
a key already sitting in a shell rc re-labeled the provider "key present",
hiding the sign-in row entirely — so the only path anyone ever saw was
"paste a key". Now: a purely numeric `/model <n>` goes straight to the
broker; `/login` lists every provider that offers a sign-in (device-flow
rows numbered and offered even with a key present, vendor CLIs with their
exact command) and `/login <name|n>` runs the flow in-pane; a stored grant
now OUTRANKS a key — an explicit sign-in is the strongest signal a user can
send, and the grant's endpoint travels with it — with `/logout [provider]`
removing the grant so the key serves again. Proven end-to-end: a broker
with a decoy `DASHSCOPE_API_KEY` still lists the numbered sign-in, completes
the stubbed device flow, and answers chat through the grant's endpoint.

## 0.12.0

Sign in, don't paste keys — the OAuth goal's final slice. A signed-out
device-flow provider (Qwen/DashScope) is now a numbered row in `/model`'s
picker, and picking it runs the RFC 8628 sign-in right in the pane: a code
card streams into the chat, crew polls while you approve in the browser
(`/stop` cancels), and the grant is stored and selected — a clean machine
with zero API keys reaches a working smith pane that answers, fans out, and
drafts plans, proven end-to-end against a stubbed OAuth + chat server.
Grants live in the macOS keychain (`security`, probed never assumed; a 0600
file where no keychain exists — `/doctor`'s new "token store" line says
which). Access tokens refresh transparently before model calls; a hard
refresh failure discards the dead grant and prints exactly one "sign-in
expired — open /model" line. Nothing token-shaped can reach any log sink: a
sweep test greps the session logs, broker stdout/stderr, doctor output, and
the whole redirected HOME after a full stubbed round-trip and asserts zero
hits.

## 0.11.9

*Sign in, don't paste keys: subscriptions become first-class providers.*

- **The provider auth registry** (`broker/auth/`): every provider is a data
  row — name, auth modes (`cli-delegated` / `oauth-device` / `api-key`), key
  variable, and for delegated providers the vendor CLI that owns the
  sign-in. One resolution function orders discovery: explicit pin, then
  **signed-in subscriptions**, then API keys (historic order, unchanged),
  then installed CLIs. Routing and planning code no longer know provider
  names.
- **A signed-in Claude Code or Codex seat now serves smith work with no API
  key.** Crew asks the CLI itself (`claude auth status` / `codex login
  status` — consent-based, never another app's token store; 5s timeout,
  probed once per pane, `CREW_SUBSCRIPTIONS=0` opts out) and routes plain
  tasks through that CLI via the existing relay. The swarm on a
  delegated-only machine degrades to the stub planner exactly as keyless
  always has — `/doctor` says so instead of erroring.
- **`/model` becomes the whole model story**: a grouped picker — "your
  subscriptions", "your keys", "installed CLIs", each entry numbered —
  with signed-out providers grayed showing the exact sign-in command, and
  `/model <n>` switching provider through the stored pin (survives
  restarts). `CREW_PROVIDER=claude-code|codex` pins the delegated rung
  explicitly.
- **`/doctor` gains a per-provider auth line** — signed in / signed out
  (+ sign-in command) / key present / no key / not installed — states
  only; a test greps the full gather→render round-trip with a fake stored
  key to prove no key material can reach the report.

## 0.11.8

*The command diet completes: seven constructs, everything else is conversation.*

(Released as 0.11.8: the autonomy branch merged and bumped past 0.11.7 in one
motion, so that number never shipped an entry of its own.)

- **`/goal`, `/plan`, `/approve`, `/reject`, `/skill`, `/memory` and `/mcp`
  are retired** — constructs shrink from 14 to the infrastructure seven
  (`/help` `/model` `/doctor` `/restore` `/reload` `/diff` `/stop`). "Keep
  working until …" runs the judge loop, "draft a plan for …" enters plan
  mode, and typing an old slash form teaches the phrasing instantly.
- **The plan gate survives as conversation.** With a plan pending, "approve"
  / "run it" executes it and "reject" / "drop it" discards it — matched
  exactly, before any model call, so a misrouted message can never run or
  drop a plan; anything else leaves the draft pending. The pane's enter/esc
  now send those same bare words.
- **Skills apply themselves.** A task that names a loaded playbook gets it
  woven into the relay or swarm prompt automatically (at most two); loaded
  but unmatched skills ride along as a one-line roster. The drop-in
  `.crew/skills` surface is unchanged.
- **Memory answers to plain language** — "what do you remember?" surfaces
  the standing block that already rides every task; `#<note>` still saves
  one. **`/doctor` absorbs the `/mcp` listing**: each server renders with
  its tools (or its failure) under the count line.
- **Transcript history is summarized, never dropped.** When a relay outgrows
  its 8-entry window, the overflow folds into a running `[compacted N
  earlier messages: …]` block by one bounded model call, so an early
  decision stays in every later prompt; the session log folds its oldest
  half the same way. Keyless, mock, or a failed call keep the old clipping —
  degraded context, never an error — and the retained block is byte-capped.
- **The swarm graph unfreezes on failure.** The first failed task pauses
  dispatch and asks the planner — given the goal, the completed outputs
  (budget-clipped) and the error — for a replacement of the not-yet-run
  remainder; completed work is never re-run, and one re-plan per run is the
  hard cap. Keyless/mock runs and planner errors keep today's
  cascade-cancel, and re-planned tasks pass the same Api/Standard forcing
  as the original plan — a re-plan can never widen what a plan may execute.

## 0.11.6

*Smith trusts the model: say it, don't slash it.*

- **The intent router decides the shape.** A plain message gets one cheap
  classification call and routes itself — single reply, fan-out, refinement
  loop, plan-with-approval, or swarm — falling back to the swarm whenever
  the classifier can't run (no key, mock provider, `CREW_INTENT=0`).
- **`/fan`, `/loop`, `/commit`, `/review`, `/standup` and `/resume` are
  retired.** Every capability lives on behind plain language — "have every
  agent take a crack at …", "keep refining …", "commit this", "look over my
  changes", "what did I ship this week?", "pick up where we left off" — and
  typing the old slash form teaches the phrasing instantly.
- **The commit gate survives retirement.** "commit this" only ever drafts;
  the commit is created when you say "apply", matched exactly and never by
  the model, so a misrouted message can't commit.
- **crew-hive prompts are budget-aware.** Dependency outputs are clipped
  fairly (4k per dep, 12k total, visible markers) instead of concatenated
  unbounded.

## 0.11.5

*The tube gets a typeface from this decade.*

- **Modern monospace only.** Pre-Retina `Monaco` is out of the auto-select
  allowlist and out of the CRT themes' font lists — `Lilex` leads the tube
  now (the contemporary take on that IBM-terminal DNA), backed by JetBrains
  Mono, Google Sans Code, and Cascadia Mono. The allowlist gains today's
  faces — Lilex NF, Berkeley Mono, Commit Mono, Martian Mono, Cascadia Mono,
  GeistMono NF — so the rotation and theme resolution can land on them when
  installed. Menlo survives strictly as the never-fail OS tail of each list,
  and a new test pins that no theme may *lead* with a stock fallback face.

## 0.11.4

*Two commands walk in, one walks out.*

- **`/update` and `/restart` merged.** `/update` now downloads the latest
  release, installs it, and — after a brief "restarting…" beat on the UPDATE
  card — relaunches Crew into the new build by itself. An update the quiet
  background check already installed is applied instantly: `/update` sees the
  parked install and restarts without a network round-trip. The blinking nav
  reminder now reads `· /update`. Typing `/restart` gets a pointer to
  `/update` instead of a fuzzy-match guess at `/restore`. Only a run you
  typed ever restarts the app — the background check still parks quietly.

## 0.11.3

*The footer knows who's on shift.*

- **The footer mode line names the agents working right now** instead of a
  static roster, and the composer border sheds the roster strip — one line
  fewer, more signal.

## 0.11.2

*A pane waiting on you no longer waits in silence.*

- **Blocked panes surface themselves.** When a coding agent stops to ask
  something — a `(y/n)`, a "Do you want…", a permission question — or the
  smith pane has a plan pending, the pane raises a `?` attention badge and
  a "waiting for you" note; if you've been idle a few seconds and aren't
  already looking at a blocked pane, focus jumps there (once per episode,
  never while you're typing). **Cmd+.** cycles through waiting panes.
  The terminal-side detector is deliberately conservative: a foreground
  command, three quiet seconds, an unscrolled view, and question-shaped
  text in the last rows — an idle shell prompt never counts, and a
  thinking agent that keeps painting is left alone.

## 0.11.1

*The rough edges from a night of shipping, sanded.*

- **Fold toggles fire on mouse release, not press** — starting a drag
  selection over a folded card no longer expands it mid-gesture, and a
  click that merely focuses the pane can't toggle a card by accident.
- **A streaming card you clicked open stays open when it settles.**
- **Per-reply usage trailers verify the sender** before attaching, so a
  future interleaved broker message can never wear another reply's cost.
- **A contrast tripwire now guards the diff/checkmark inks** in every
  theme (measured floors: ≥4.5 vs the page, ≥3.2 vs the code card), so a
  future palette can't quietly slide red/green under readability.
- **Docs caught up with the night**: README, docs/CREW.md and the `/keys`
  overlay now cover diff fences, checklists, Ctrl+R, usage trailers,
  folding cards, drag-and-drop, quoted mentions and Cmd+F.

## 0.11.0

*Find anything you ever said — or were told.*

- **Cmd+F searches the conversation.** A find bar over the chat transcript:
  type to match case-insensitively across every message (folded system
  cards included), Enter/Ctrl+F/↓ steps to older matches, ↑ to newer, both
  wrapping, with a `find: query (k/N)` count in the legend. Jumping scrolls
  the transcript to the match — expanding a folded card when the hit lives
  in its hidden tail — and washes the matched text with the same highlight
  the terminal `/find` uses. Esc closes without touching your draft; one
  modal at a time with Ctrl+R and the popups.

## 0.10.9

*A bug-hunt pass over everything the night shipped.*

- **File drops are robust now.** A drop mid-sentence no longer glues onto
  the last word (which silently lost the attachment), paths with spaces
  round-trip as `@"quoted mentions"` end to end, a drop while the Ctrl+R
  popup is open lands in the draft instead of vanishing, and dropped-path
  relativization uses the exact cwd the send path resolves against.
- **Ctrl+R popup hygiene.** Accepting a recalled line disarms any palette
  or mention popup that was open — previously the stale popup resurfaced
  and ate the next Enter, able to run an old palette row against the
  recalled text.
- **Clicks no longer fall through popups** onto the fold toggles of system
  cards hidden beneath them.
- **Oversized mentions are refused before reading.** A dropped multi-GB
  file used to be read whole on the UI thread at send before the 64 KB cap
  was checked; the size gate now runs on metadata first.
- **Compact view stops counting metadata.** The per-reply usage trailer no
  longer produces a misleading ` … +1` on every single-line reply under
  Ctrl+O.
- **Two real test flakes fixed** — the phantom full-suite failure traced to
  torn reads of the process-global theme, and a ~25% `every_animation_
  terminates` flake from the unguarded motion level; both now serialize on
  the shared test guard (verified over eight consecutive clean runs).

## 0.10.8

*Drop a file on crew and it lands where you're working.*

- **Files dragged onto the window drop into the focused pane.** Onto a chat
  pane they become an `@mention` — relative when the file lives under the
  pane's project, absolute otherwise — ready for the mention machinery to
  inline at send. Onto a terminal they type the shell-quoted path, spaces
  and quotes escaped, through the same bracketed-paste path as Cmd+V.
  Multiple files append in order. (macOS never reports a cursor position
  during an OS drag, so the focused pane — not the pane under the mouse —
  is the deliberate target.)

## 0.10.7

*Broker noise folds itself away.*

- **Long system/telemetry cards auto-collapse.** A system-voice card (turn
  summaries, `/doctor` dumps, roster lists) longer than three lines renders
  as its header plus first line with the muted ` … +N` count. Click it to
  expand; click the expanded header to fold it back. Agent replies and your
  own messages never fold, the splash art is exempt, and the pane-global
  Ctrl+O compact view still clamps everything as before.

## 0.10.6

*Every reply owns up to what it cost.*

- **Settled agent replies carry a muted usage trailer** — `900 in / 50 out ·
  $0.012` — from the broker's per-reply stats, formatted like the footer.
  Zero-usage replies (CLI agents, dangling segments) show nothing rather
  than `0 in / 0 out`; unpriced models show tokens without a fabricated
  `$0.000`; streaming cards stay trailer-free until the reply settles.

## 0.10.5

*Find that message again: Ctrl+R searches your chat history.*

- **Ctrl+R in the chat composer opens reverse history search.** Type to
  filter your previously sent messages (substring hits first, fuzzy after),
  newest first; Ctrl+R or ↓ steps older, ↑ newer, Enter puts the match back
  in the composer without sending, Esc restores whatever you had typed.
  Chat-pane only — terminal panes still pass every key through.

## 0.10.4

*Agent plans read like plans: task lists render as real checklists.*

- **`- [ ]` / `- [x]` markdown task lists render as checklists in the chat
  pane.** An unchecked item draws `☐` in place of the bullet; a checked item
  draws a green `✓` with its text dimmed, so a Claude Code-style plan shows
  its progress at a glance. Uppercase `[X]` counts as checked, wrapped items
  keep their hanging indent (the dim survives the wrap), nested lists nest,
  and `- [normal] text` or task syntax inside a code fence stays untouched.

## 0.10.3

*Diffs in the chat transcript colour like diffs.*

- **```diff and ```patch fences colour by line.** Added lines green, removed
  lines red, `@@` hunk headers cyan, file headers (`+++`/`---`/`diff --git`/
  `index`) dimmed — the same colours the file viewer gives an opened `.patch`,
  so chat and viewer agree. An *untagged* fence whose body reads as a diff
  (a `diff --git` opener, or a hunk header alongside real `+`/`-` change
  lines) gets the treatment too, since agents rarely tag a paste.
  (This entry was backfilled — 0.10.3 shipped without one.)

## 0.10.2

*The model segment survives a Dock launch — and names the swarm actually
serving you, live.*

- **The smith pane's roster no longer vanishes when crew is launched from the
  Dock or Finder.** Launched that way the app — and so its broker child — runs
  at `/`. The broker treats its CWD as the project, so it found no
  `.crew/specialists.json`, emitted an *empty* roster, and the footer's new
  model segment had nothing to show (while its session log and `/resume`
  quietly wrote to nowhere). Plugin panes now spawn their broker in the pane's
  tracked directory; session restore steers that to the saved project before
  the spawn, so a restored `/smith` gets its project — roster, log and all.
- **A running swarm stamps the roster with its serving model, from memory.**
  The post-planning roster re-emit used to re-read specialists from disk,
  which came back empty exactly when the project dir wasn't writable. The
  invented cast now goes out directly, each specialist carrying the model slug
  serving the run — so line 1 reads `model · N agents` in real time, whatever
  directory the broker woke up in. Discovery still appends CLI and manifest
  agents the cast doesn't name.

## 0.10.1

- **Footer line 1 leads with the serving model.** `qwen-max · 7 agents`
  instead of a name list: when every API specialist agrees on a model, that
  model is the roster's identity, even with CLI agents (which report none)
  riding along. (This entry was backfilled — 0.10.1 shipped without one.)

## 0.10.0

*Read any file where you already are — and a title bar that stopped being
see-through.*

- **`/view <path>` opens any file in a pane, and `/md` is now an alias for it.**
  One read-only viewer replaces the old source|preview split, over a ladder of
  formats: code with syntax colour, markdown **rendered only** (the source half
  is gone — showing markdown source beside its render was a dev tool wearing a
  reading experience's clothes), json/yaml/toml, csv laid out as a table, and
  diffs with `+`/`−` ink.
- **PDF and Word open as honest text extracts, not fake page renders.** A
  monospace grid has no business impersonating a page, so those rungs say what
  they are in a banner and offer `o` to hand the file to the real application.
  Extractors are *probed, never required* — macOS's own `textutil` covers
  docx/rtf, and a missing `pdftotext` degrades that one rung instead of
  erroring. Binaries get a metadata card naming size, kind and mtime.
- **Nothing about it blocks the grid.** Detection, reading and extraction all
  run on a worker thread, so the pane is on screen before a byte is read: a
  stalled network mount or a 300-page `pdftotext` can no longer freeze every
  other pane, agents included. Both caps — 8 MB of bytes, 50 000 rendered lines
  — announce themselves rather than silently truncating.
- **`/far`'s F3 and F4 stopped lying.** Both used to hand the file to the OS and
  throw you out of crew; F3 now views it in place and F4 opens `$EDITOR` in a
  crew pane. Cmd+click resolves a path anywhere — including one an agent wrote
  in a reply — and `e` inside the viewer hands off to `$EDITOR` and **reloads
  when it exits**, keeping your scroll position. Open viewers survive a restart.
- **Search inside a file** with `/`, `n` and `N`; `s` shows raw source; Esc
  closes and restores the focus and zoom you came from.
- **Fixed: the title bar was see-through.** crew asks to be transparency-capable
  at window creation so Opacity % can take effect without a restart — but on
  macOS that leaves the window non-opaque for good, and the title bar is drawn
  by the OS against whatever is behind it. Panes were solid while the chrome
  showed the desktop. Being transparency-*capable* and being transparent are
  separable, so the runtime flag now follows the setting.

## 0.9.0

*Jarvis motion, iteration 6 of 6 — the goal is complete.*

- **The idle invariant is now a test, not an intention.** The condition
  deciding whether crew schedules another frame is a named predicate,
  `wants_animation_frame`, and four tests hold it to its contract: a settled app
  asks for nothing, `Motion = off` schedules nothing even in the same frame as a
  close and a focus change, full motion *does* animate a fresh event (without
  which the first test would pass on an app that never animated), and everything
  in flight is over within half a minute. A timeline that never settled would
  keep crew awake forever — the one failure mode here that costs battery rather
  than pixels.
- **Every theme still reads correctly.** All five glass renders verified
  through the pixel harness against their sheet-off baselines.

Six iterations in: focus brackets, cards that assemble and collapse, zoom that
travels, readouts that count, a live streaming caret, and a scan that sweeps a
working pane's glass — with no new colour anywhere, and an idle crew that
repaints exactly as rarely as it did before any of it.

## 0.8.5

*Jarvis motion, iteration 5 of 6.*

- **The glass sheet sweeps while a pane works.** A soft band of light travels
  down a busy card and back, drawn in `glass.wgsl` as part of the existing card
  pass — no second draw call, no new colour, and it rides the sheet's own fill
  so every theme carries it.
- **Gated on busy and nothing else.** A working pane already repaints at ~15fps,
  so the sweep costs no extra frames; an idle crew draws no scan at all. That is
  the whole trick to having an ambient layer without breaking "an idle crew
  never repaints".
- **The shader change is pixel-tested.** `glass_scan_headless` renders the same
  flat card with and without a scan and asserts the band brightens where it
  passes and nowhere else. A uniform that is plumbed but never read looks
  exactly like a working feature from the Rust side — which is precisely how
  0.7.0 shipped glass that drew nothing.

## 0.8.4

*Jarvis motion, iteration 4 of 6.*

- **Streaming cards carry a live caret.** A block on the end of the last line,
  pulsing between the muted and accent colours, marking text that is still
  arriving as distinct from a reply that happened to end mid-sentence. It
  pulses rather than blinks — a caret that vanishes half the time reads like
  the text stopped. `Motion = off` keeps it (it carries information) but holds
  it still.
- **Where the stream begins is now part of the view.** `View.streaming_from`
  rides the same value already threaded through every card render path, so the
  scroll clamp, the scrollbar, the link hit-test and the transcript cannot
  disagree about which cards are live.

## 0.8.3

*Jarvis motion, iteration 3 of 6.*

- **The footer counts.** Cost and token totals sweep to their new values rather
  than snapping, and the 5h-budget and context meters fill rather than jumping a
  cell. A number that jumps reads as a repaint; the same number sweeping reads
  as an instrument.
- **Counters live on the pane that draws them.** The first version kept them in
  a process-wide registry keyed by name, which read fine until you noticed two
  chat panes would share one `footer.cost` and overwrite each other — and which
  the footer's own tests caught within minutes by leaking values between cases.
  They are `Cell`-based fields on `ChatPane` now, since the footer renders from
  an immutable pane.
- **First sight is settled.** Sweeping every number up from zero the first time
  it was drawn made the footer's output depend on how many times it had been
  drawn before, which is not a property a footer should have. Only a value that
  changes sweeps.

## 0.8.2

*Jarvis motion, iteration 2b of 6.*

- **Dismissed panes collapse.** A closed card's frame retracts into its corners
  — the assemble run backwards — and a minimized one retracts *while travelling
  toward the nav*, which is where the pane has actually gone. The two gestures
  now look different, because they are different.
- **Ghosts, not lingering panes.** The pane leaves `panes` the instant it is
  dismissed; it has to, since focus clamping, the grid LRU and the nav rows all
  read that vector and would otherwise operate on something the user just
  dismissed. What outlives it is an inert record — a rect, a title, a timestamp
  and a direction. It holds no process and no channel, because a ghost that
  could still *do* something would be a pane that refused to die. Each is
  bounded by its own timeline and pruned every frame.
- **Restoring is an arrival.** A pane coming back out of the nav re-stamps its
  birth clock and assembles exactly as a new one does.
- **Zoom travels out of its tile.** Cmd+Z expands from the rect the pane
  occupied instead of cutting to full size, and collapses back into it.

## 0.8.1

*Jarvis motion, iteration 2 of 6.*

- **Panes assemble instead of appearing.** A new card's frame draws itself
  outward from its four corners over its first moments. Only the frame *stroke*
  animates — an early version clipped the top border's legend too, spelling the
  pane's name out one letter at a time, and a card you cannot identify is worse
  than one that simply appeared. The name is there from the first frame and the
  frame draws itself around it.
- **Every pane carries its own birth stamp.** `Pane.born_ms` rather than a
  timeline held by the app: panes are pushed, closed and reordered from a dozen
  places, and anything keyed on position or count would animate the wrong card
  the first time two of those happened in one frame.
- **Motion is read process-wide.** `motion::level()` mirrors `palette::accent` and
  `crew_theme::theme`, so a new animation respects the setting without any
  plumbing — and `apply_config` publishes it, which means Save, session restore
  and an external config edit all land on the same path.

## 0.8.0

*Jarvis motion, iteration 1 of 6 — see `.superpowers/sdd/2026-07-28-jarvis-motion/`.*

- **Motion is a setting now.** Settings → APPEARANCE → **Motion**:
  `off · subtle · full`. `off` is a real off rather than a fast one — every
  animation's duration collapses to zero, so it draws its final state once and
  schedules no further frames, and an idle crew repaints exactly as rarely as
  it did before any of this existed.
- **The focused pane wears HUD brackets.** Short accent runs down the card's
  edges from each of the four corners, growing out as focus arrives and
  travelling with it as you move between panes. They stay off the top border,
  where the legend and the `[-][x]` buttons live — decoration must not overwrite
  information.
- **A motion vocabulary to build the rest on.** `ease.rs` adds an ease-out curve
  and `Timeline`, a *bounded* window that reports when it still has frames to
  draw. `poll` schedules redraws from exactly that, which is what keeps "an idle
  crew never repaints" true while crew animates. No timers, no threads, no
  sleeps: every animation is a pure function of `anim::now_ms()`, so none of it
  can stall the winit thread.

## 0.7.1

- **Glass actually renders now.** 0.7.0 shipped "every pane sits on glass" and
  drew no sheet at all. The card was emitted only for scenes with
  `PaneScene.bordered`, a flag that went dead when panes started drawing their
  frames as cells — every scene the app builds sets it `false`, so `GlassLayer`
  received zero cards and the only visible effect of the feature was the window
  translucency, which rides a different path. The sheet now has its own `glass`
  flag, set on the scene that spans the whole pane rect, and panes, panels, the
  minimized strip and the input bar all carry it. Overlay popups stay opaque.
- **The pixel test can fail now.** The headless harness hand-built its panes
  with `bordered: true` — an arrangement production never produces — which is
  exactly why a feature that drew nothing kept a passing GPU test. It renders
  scenes from `paneview::build_scenes` instead, and compares each theme against
  the same frame with the sheet off rather than against the gap between panes
  (a pane's own cell backgrounds satisfied the old assertion whether or not any
  glass was drawn).
- **Glass moved into Settings; `/glass` is gone.** Strength lives in APPEARANCE
  as **Glass** (←/→/Space through `off · low · medium · high`) and window
  translucency in WINDOW as **Opacity %**, typed as a percentage and floored at
  35%. Saving pushes both to the renderer, so they apply live.

## 0.7.0

- **Crew never disappears in silence again.** A panic on the winit thread used
  to take the whole app down leaving *nothing* behind: a detached crew runs with
  stderr on `/dev/null`, and a panic exits through the normal path so macOS
  files no crash report — the window simply vanished. There is now a panic hook
  that writes the message, location and backtrace to `crash.log` beside the
  config, the detached child's stderr goes to `stderr.log` instead of being
  thrown away, and the next launch says what happened and where to look.
- **The window close button now confirms too.** Cmd+Q has always asked twice
  when panes are open, so a stray keystroke couldn't kill running shells and
  agents — but the traffic-light button called exit outright, and it is the
  easier of the two to hit by accident (a misplaced keystroke usually just
  lands in a pane; a misplaced click doesn't). Both paths now share one guard,
  and the prompt names how many panes are at stake.
- **Frosted glass on every theme.** Each pane card now sits on a translucent
  sheet — a fill that fades from the top down, a specular hairline along the
  upper edge, a soft drop shadow, and a whisper of frost grain. The look is
  *derived* from the active theme rather than configured per palette, so all
  thirteen get their own treatment and no future theme can ship without one:
  dark themes lift a lighter sheet off the page, light themes lean on a whiter
  sheet plus a real shadow, and CRT stays faint and grain-free because the tube
  already supplies bloom and noise. `/glass [off|low|medium|high]`.
- **`/glass window <pct>`** makes the window itself translucent, so the desktop
  shows through the page while text and pane fills stay solid. Works under the
  CRT post-process too. Opacity floors at 35% — a window any sheerer is one you
  cannot find again.
- Three keyboard shortcuts the app has always answered are now in the `/keys`
  overlay: **Ctrl+Shift+L** (cycle themes) and **Ctrl+Shift+M** (chat markdown
  preview ↔ source), which the manual documented and the overlay did not, and
  **Ctrl+O** (compact transcript view), which was implemented, tested, and
  listed nowhere a user could find it.
- The overlay and the manual are now checked against each other, so a binding
  cannot be added to one and forgotten in the other. The manual said `/keys`
  shows "this list in-app"; that is now true.

## 0.6.99

- `/crt` and `/weight` are in the manual. Both have been working, palette-listed
  commands with no mention in either page — `/crt [on|off|auto]` runs the CRT
  tube post-process independently of the theme, so a paper theme can go through
  the tube or a CRT palette can render flat.
- A link in the manual pointing at `#multi-agent-relay-crew` has gone nowhere
  since the `/smith` rename. Internal doc links are now checked against the
  headings they claim to target.

## 0.6.98

- Six environment knobs the code has always read are now in the manual:
  `CREW_HTTP_TIMEOUT_MS` and `CREW_STREAM_TEXT`, the three plugin-path
  overrides (`CREW_BROKER_PLUGIN`, `CREW_CHAT_PLUGIN`,
  `CREW_ORCHESTRATOR_PLUGIN`) that let one pane run a debug build while the
  rest of the app stays on the installed release, and `CREW_PANE`.
- `CREW_SYS_TIMEOUT_MS` was documented twice with two different defaults. The
  stale copy said 30000; it is 120000.
- Every `CREW_*` name in the shipped source is now either documented or
  declared internal in one list, so a new knob cannot arrive unmentioned.

## 0.6.97

- `/export` on a pane with no messages no longer writes a file. It reported
  success and left a 68-byte "0 message(s)" transcript wherever crew happened
  to be running; 64 of them had accumulated in this repo and 54 were committed.
  An export of nothing now says so and touches nothing.
- Stray `crew-transcript-*.md` files are gitignored, so a real export made
  inside a repo cannot be committed by accident.

## 0.6.96

- The input bar says which pane it is talking to. The focused pane's name now
  rides the bottom border as a right-aligned legend, mirroring the working
  directory on the top one — typing there has always acted on the selected
  pane, and the bar never said which one that was. A transient status message
  borrows the slot while it flashes and gives it back.

## 0.6.95

- The slash palette stopped disagreeing with the commands it labels. `/goal`
  was described as "set the crew's shared goal" — a feature that exists
  nowhere; it runs relay rounds until a judge agent rules the goal met. Every
  hint the pane does not deliberately own is now the broker's own `/help`
  sentence, so the two cannot drift apart again.
- `/goal` means two different things and now says so. In the agent smith
  composer it is the judged relay; in the command bar it plans a task graph and
  runs it as a swarm. Same word, two engines — the docs and the command-bar
  palette both name the split.
- The shell's deadline fits a real build. `sys:run` had 30 seconds, which
  `cargo test` cannot finish inside on any real project, so the shell's most
  valuable use was its broken one. It is 120 seconds now, and the timeout
  message names `CREW_SYS_TIMEOUT_MS` instead of leaving you to guess a knob
  exists.

## 0.6.94

- Three providers that shipped fully wired and entirely unmentioned are
  documented: **OpenAI**, **Gemini** and **DeepSeek** each have a key variable,
  an endpoint and a native model chain, and neither the README nor the manual
  ever said so. `docs/CREW.md` now carries the table.
- They probe **last** in discovery on purpose, so adding one never changes
  which provider an existing install resolves to — which means
  `CREW_PROVIDER=gemini` is the only way to reach one while another key is set.
  That pin worked all along and was documented nowhere, including in the
  function's own comment.

## 0.6.93

- An agent that runs out of tool calls in a turn says so. There is a budget of
  four per turn; spending it just ended the turn, leaving the agent's unrun
  `@tool` line standing as its answer with nothing anywhere explaining why it
  stopped halfway through what it was doing.
- The agent is also told how many it has left, and told plainly when the next
  one is its last. A budget it cannot see is one it plans straight past.

## 0.6.92

- `@src/main.rs:120-180` attaches just those lines. `@file` attached the whole
  file and anything past 64 KB was skipped outright — so the file you most
  want to point an agent at one function of was the file you could not point
  it at at all. `:120` is a single line; the size limit now applies to what is
  attached rather than to the file on disk, and the "too large" note names the
  idiom instead of only saying no.
- A file genuinely named `odd:10` still wins over reading that as a range, and
  a colon anywhere else in a name is left alone.

## 0.6.91

- A tool call reads as what it is. Every call an agent makes is logged in the
  pane so you can see what is happening to your machine, but it was shown in
  wire form — `[tool] sys:write_file {"path": "src/lib.rs", "content": "use
  std::fmt;\n…` — where the one thing worth reading is buried in punctuation
  and, for a write, the line is mostly the first 150 bytes of the file. It now
  reads `sys:write_file  src/lib.rs`. A tool whose arguments say nothing
  identifiable still shows them, so nothing goes missing.

## 0.6.90

- The built-in `sys` tool surface is decided once, when a session's broker is
  built, instead of being re-read from the environment on every hint and every
  tool call. It is meant to be stable for the life of a session, and one of
  the variables it read is set and cleared by the test harness while other
  tests are running — which made two tests fail about one run in six.

## 0.6.89

- A test no longer switches text streaming off for the whole process. Two
  tests set `CREW_STREAM_TEXT` and were serialised against each other but not
  against anything else, so any test building a text streamer at that instant
  saw streaming disabled. The switch is handed in now; nothing mutates the
  environment.
- Typing `/stop` drops the queue too. Esc has since the last release; the
  typed form is the same intention and sends the same string, and it was
  still stopping one run and starting everything behind it. `/stop #2` is
  left alone — calling off one of several parallel tasks says nothing about
  the rest of the session.

## 0.6.88

- Interrupting also drops what you queued behind the run. Esc stopped the
  agent, which made the pane idle, which is exactly what flushes the queue —
  so a stop immediately started every follow-up waiting behind it, each one
  written on the premise that the interrupted work was going fine. It says how
  many it dropped.
- Three tests that assert something is absent from a rendered pane now check
  the pane rendered at all first. None of them were passing vacuously; the
  guard is so that they cannot start to.

## 0.6.87

- A pane opened in a project you have worked in before says so, and names
  `/resume`. The previous conversation has been rotated into a resumable file
  since long before this and announced nowhere, so the only way to find out it
  was there was to guess the construct and try it. It offers rather than
  resumes: folding yesterday's conversation into today's first task spends
  context nobody asked for, and yesterday's task is as often unrelated as not.
- A first run in a fresh project still says nothing.

## 0.6.86

- The agent composer suggests the rest of a prompt you have sent before, dim
  after the caret, taken with Tab or Right. The docked input bar has done this
  since long before; the composer — where the long prompts are actually typed
  — did not. Tab keeps its existing meaning and gains this only where it used
  to do nothing.
- A leading `/` is left to the palette, which is already answering the same
  question as a popup.

## 0.6.85

- `/restore` removes the files the task created, instead of putting the edits
  back and leaving the new ones for you to find and delete by hand. An undo
  that covers half a change is not an undo, and the half it skipped was the
  one most likely to be unwanted — the half-written module a run was abandoned
  over. It names every file it deletes.
- Nothing that predates the snapshot is touched, and ignored files — build
  output, an untracked `.env` — are never candidates.

## 0.6.84

- `/review` reviews the files the agent created, and `/commit` describes them.
  Both read one diff, and it came from `git diff` — which cannot see a path git
  has never been told about. A review of a change whose main file is a new
  module was a review of everything except the code, reported as a clean pass.
- `/commit apply` commits what it described. It ran `git commit -am`, which
  stages tracked modifications and nothing else, so the new file stayed behind
  while the pane reported a successful commit. Staging a subset by hand still
  commits exactly that subset — that is a statement about which change you
  mean, and it is still respected.

## 0.6.83

- `/diff` shows files the agent created. It ran `git diff --stat`, which
  compares the working tree to the index — so a new file was untracked and
  invisible, and anything staged was excluded too. Those are the two states an
  agent most often leaves a repository in, and both printed "working tree
  clean — no changes". It now compares everything that exists against the last
  commit, and works in a repository with no commits at all.
- It excludes `.crew/` for the same reason the end-of-task note does, so the
  note and the command it names cannot disagree.

## 0.6.82

- A finished task says which of your files it changed. A clean run printed its
  reply and nothing else, so an agent that edited four files and answered
  "done" left no record anywhere of which four — you had to know about `/diff`
  and remember to run it. No new bookkeeping: the checkpoint taken before
  every task already pins the tree to compare against.
- It stays quiet when nothing of yours changed, which needed care — the broker
  rewrites `.crew/session-live.md` as every reply streams, so the answer to
  "what changed?" would otherwise have been "the transcript" after every
  question ever asked, in every repo but crew's own.

## 0.6.81

- Up and Down in an agent pane recall what you already sent, filtered by what
  you have typed — the docked input bar's rule, which the composer did not
  share. The arrows were classified for the popups and then dropped when no
  popup was open, so changing one word of a long prompt meant retyping it.
- `/keys` shows each description in full. The overlay was a fixed 58 columns
  wide, which left 30 for a description, and eight rows had outgrown it: `Esc`
  read "Discard a pending plan · inter", losing both the interrupt and the
  close, and `@a+b` lost "in parallel".

## 0.6.80

- The provider-key tests assert the same thing on every machine. They read the
  process environment and asserted only when the key was absent, so on any
  machine that had one they passed without testing anything — and they never
  covered an exported but blank key, which is the case that matters.

## 0.6.79

- The `@` picker says that `@a+b` fans a task out to both agents in parallel,
  and `/keys` lists it. The idiom worked and was documented in exactly one
  place the app never shows.

## 0.6.78

- Cmd+click a code block in an agent pane to copy it. Reading an answer and
  using it are different acts, and the second had no support beyond selecting
  text with the mouse. The fence chrome and language tag do not come with it,
  so there is nothing to strip after pasting.

## 0.6.77

- The first automatic snapshot of a session says so, once, and names
  `/restore`. Checkpoints have been automatic and entirely silent since
  0.6.44 — an undo nobody knows about is an undo nobody uses.

## 0.6.76

- The welcome screen names the agent pane. It said "new shell" and "commands"
  and nothing else, so the reason crew is not just a terminal went unmentioned
  on the one screen a first run is guaranteed to see. It also picks a form
  that fits the window instead of vanishing on a narrow one.
- The installer says that agents work with no API key at all when you are
  already signed in to claude, codex or opencode.

## 0.6.75

- Grouped the entries below by what the work was for. Thirteen consecutive
  per-release sections trace history well and read badly; the run was one body
  of work and now says so.

## 0.6.62 – 0.6.74

The second half of the same run. Mostly the consequences of the first half —
six providers where there had been three, ten fewer constructs — arriving as
messages that named the wrong thing, state that outlived what owned it, and
documents describing commands that no longer existed.

### Failures that now say something

- A provider's API error is a sentence, not a JSON envelope pasted into the
  chat and truncated mid-structure. A rejected key names the fix; it is the
  most common provider error and the only one a user can always fix. (0.6.68)
- A missing key names the variable that is actually missing. It said
  `ANTHROPIC_API_KEY` whatever had failed, and the OpenAI-wire client backs
  six providers through six variables. (0.6.69)
- An unknown `/command` says so and guesses, rather than doing nothing.
  (0.6.53)
- A CLI that is installed but not signed in says so. Empty output from it was
  being treated as a successful empty reply. (0.6.60)

### Things that stopped outliving what owned them

- Closing an agent pane stops the agents it was running. The broker died; the
  `claude` or `codex` it had spawned survived, reparented and still working.
  (0.6.72)
- A lost broker no longer leaves phantom running tasks in the footer, or a
  pending plan that still answers to enter. (0.6.67)
- The transcript fold marker counts everything it ever folded, and folds in
  batches rather than copying the whole transcript on every message past the
  cap. (0.6.66)

### The pane fits

- All three footer lines budget themselves and drop the least important
  segment rather than being clipped from the right. A 40-column pane was
  showing "enter runs it" and cutting "esc discards it" — half an
  instruction. (0.6.57, 0.6.58)
- `/keys` documents the agent pane, and fits a default window; it was 58 rows
  and silently truncated. (0.6.65)

### Knowing what you are running

- `crew --version` prints a version instead of launching the window;
  `crew --help` lists the CLI modes. (0.6.71)
- `/about` opens the changelog that shipped with the binary, and a build that
  is not the one that ran last says so on startup. (0.6.63, 0.6.64)
- This changelog exists, bound by a test to the version it ships with.
  (0.6.62)

### Guards, so this does not happen again

- The docs stop describing constructs that no longer exist, and a test fails
  the build if they ever do. (0.6.73)
- An end-to-end test sends every construct the router advertises to a real
  broker. (0.6.74)
- The test harness clears every provider key rather than the three it was
  written against — with an `OPENAI_API_KEY` exported, nobody could run the
  suite green. (0.6.70)

## 0.6.38 – 0.6.61

One night's work, grouped by what it was for rather than by release. Every
version in the range is a real release with its own tag.

### Fewer commands, because the pane says it instead

Thirty-three constructs became twenty-one. Each deletion moved information
somewhere it could be seen rather than asked for.

- `/agents` — the roster is in the footer, and bare `/model` already printed
  the same report. (0.6.40)
- `/tasks` — running tasks are in the footer, with their ids, because
  `/stop #n` needs one to name. The broker had no way to report a task
  ending until this release: `Tasks::reap` runs lazily on the next command,
  so a task finishing was not an observable moment anywhere. (0.6.41)
- `/status` — its turn count, token total and budget moved to `/doctor`,
  which is already the "state of this stack" report. Nothing was dropped
  without a home. (0.6.42)
- `/cwd` — the working directory leads the footer; the sandbox mode it also
  reported had moved to `/doctor` the release before. (0.6.43)
- `/checkpoint` — checkpoints are automatic now (below). (0.6.44)
- `/approve`, `/reject` — a drafted plan is answered with enter or esc. Both
  still route, for hosts driving the broker without a keyboard. (0.6.46)
- `/checkpoints`, `/skills` — folded into `/restore` and `/skill`, which list
  when given no argument. (0.6.54)
- `/compact` — the transcript folds itself and says so. (0.6.55)

### Things that now happen without being asked

- **A checkpoint before every task that can touch files.** Predicting which
  task is worth protecting is asking the user to be right in advance. Silent
  when the tree has not changed, capped at 25 snapshots, and ordered by a
  sequence in the ref name because git timestamps are second-resolution and
  automatic snapshots land inside one second. (0.6.44)
- **A signed-in CLI is a working crew.** `claude`, `codex` and `opencode`
  join the roster on PATH, with no key at all — the adapters had existed
  since the beginning with nothing calling them. A plain message relays to
  one instead of running the offline stub swarm. (0.6.39)
- **Choosing a model chooses the provider.** Holding two vendors' keys used
  to mean picking a model from the wrong one silently answered from the
  other. (0.6.51)
- **The transcript folds itself**, announcing what it folded, where it used
  to drop messages in silence at 500. (0.6.55)

### Providers

- Providers are a table: an endpoint, a key variable, a model chain and the
  vendor it serves. OpenAI, Gemini and DeepSeek joined DashScope, OpenRouter
  and Anthropic. `OPENROUTER_API_KEY` had been the answer for every vendor
  because OpenRouter was the only multi-vendor route; a row now asks for its
  own vendor's key. (0.6.47, 0.6.48)
- Every provider key is probed from the login shell, not just the original
  three — a key crew never imports is indistinguishable from no key at all.
  (0.6.49)
- One copy of the "no provider" advice, shared by the broker's roster line,
  `/doctor`, the Far pane's ask and the pane's empty state. Four wordings
  existed and two had gone stale. (0.6.50)
- A CLI that is installed but not signed in says so, instead of returning an
  empty reply. Empty stdout was treated as success. (0.6.60)

### Reading code in the pane

- **Syntax highlighting** in fenced blocks — comment, string, keyword — from
  a small hand-rolled lexer. Tokenized before wrapping, so a string crossing
  a wrap boundary keeps its colour. (0.6.45)
- Semantic colours clear a floor against body text, not just against the
  page. On the CRT themes code had measured 1.04:1 against prose, which is
  the same colour. (0.6.38)
- The syntax classes sit on a lightness ladder and keywords are marked by
  weight, so highlighting survives a single-phosphor tube where hue cannot
  vary. (0.6.45)

### The pane

- Grouped command palette while browsing, flat while filtering. (0.6.52)
- The footer fits: segments carry a priority and the least important go
  first, rather than the line being clipped from the right. (0.6.57, 0.6.58)
- An unknown `/command` says so and guesses, instead of doing nothing.
  (0.6.53)
