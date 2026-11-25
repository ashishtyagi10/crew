# Windows keys: Alt is crew's key, every shortcut is two keys

Status: planned 2026-11-25, approved by the user ("sounds good"). Step 1 done
(`winkeys.rs`, not wired yet); steps 2–8 to do.

## Goal

On Windows, every crew shortcut is **one modifier plus one key**. Ctrl+Shift+key
and Ctrl+Alt+key go away. The user, 2026-11-25: "I don't like two key
combination [three keys], re-assign keys so that everything can be driven by
two key combination in windows."

The rule that makes this possible:

- **Alt is crew's key.** It sits where Cmd sits on a Mac keyboard, so Cmd+D on a
  Mac is Alt+D on Windows, Cmd+T is Alt+T, and so on.
- **Ctrl belongs to whatever you type into**: the shell in a terminal, and
  editing keys (undo, save, select all) in the editor and the input bar.

macOS does not change. Linux follows Windows, because today's code treats
"off a Mac" as one case (`cfg!(target_os = "macos")`). Splitting Linux out is
an open question below.

## Why today's scheme feels broken

Off a Mac, a Cmd chord today has two stand-ins (`keychord::stand_in`):

- **Plain Ctrl+key** works only where focus isn't on a terminal or a typed
  line. Inside those, the keys in `keychord::TYPED` (`cdzrlaekumjihpn[`) go to
  the shell. So Ctrl+D opens a pane over the Explorer, but sends EOF in a
  terminal.
- **Ctrl+Shift+key** works everywhere, but it is three keys.

So the same key does different things depending on focus. The 2026-10-21 work
(`project-windows-plain-ctrl`) was only ever checked with unit tests and
`cargo xwin check`, never on a Windows machine.

## The keymap

### Same as the Mac, with Alt for Cmd

| Windows | Mac | Action |
|---|---|---|
| Alt+D | Cmd+D | New shell pane in this tab |
| Alt+T | Cmd+T | New tab with a shell |
| Alt+W | Cmd+W | Close pane |
| Alt+1 … 9 | Cmd+1 … 9 | Go to pane N |
| Alt+] / Alt+[ | Cmd+] / Cmd+[ | Next / previous pane |
| Alt+arrow | Cmd+arrow | Move focus that way on the grid (already Alt today) |
| Alt+A / Alt+. | Cmd+A / Cmd+. | Next active / next waiting pane |
| Alt+I | Cmd+I | Focus the input bar |
| Alt+P | Cmd+P | Go to file |
| Alt+O | Cmd+O | Open agent smith |
| Alt+J | Cmd+J | New chat pane |
| Alt+, | Cmd+, | Settings |
| Alt+G | Cmd+G | Show / hide the nav |
| Alt+Z | Cmd+Z | Zoom the focused pane |
| Alt+F | Cmd+F | Find |
| Alt+E | Cmd+E | Label every URL, file and hash |
| Alt+K | Cmd+K | Clear scrollback |
| Alt+C / Alt+V | Cmd+C / Cmd+V | Copy / paste (crew-level, works in every pane) |
| Alt+S | Cmd+S | Save settings form / inbox draft |
| Alt+N | Cmd+N | New crew window |
| Alt+= / Alt+- / Alt+0 | Cmd+= / - / 0 | Font size up / down / reset |
| Alt+/ | Cmd+/ | The keys overlay |
| Alt+Q | Cmd+Q | Quit |
| Alt+Enter | Cmd+Enter | Send the inbox reply being written |
| Alt+Click | Cmd+Click | Open URL / file / dir |

### Three-key shortcuts, made two

| Windows | Mac (unchanged) | Action |
|---|---|---|
| Alt+U | Cmd+Shift+T | Reopen the pane you just closed |
| Alt+X | Cmd+Shift+E | Explorer takes the keys |
| Alt+M | Cmd+Shift+M | Send the pane to another tab (`/tab move`) |
| F11 | Cmd+M | Maximize the window (Alt+M is taken above) |
| Ctrl+PgUp / Ctrl+PgDn | Cmd+{ / Cmd+} | Previous / next tab |
| Ctrl+1 … 9 | Cmd+Opt+1 … 9 | Go to tab N |
| Alt+PgUp / Alt+PgDn | Cmd+Opt+↑ / ↓ | Jump to previous / next command in a terminal |
| Alt+Home / Alt+End | Cmd+Opt+← / → | Previous / next screen |
| Alt+L | Ctrl+Shift+L | Cycle themes |
| Alt+B | Ctrl+Shift+G | Step the background gradient |
| Alt+H | Ctrl+Shift+F | Focus mode |
| Alt+Y | Ctrl+Shift+M | Markdown source in a chat pane |
| Alt+Shift+arrow | Cmd+Shift+arrow | Swap pane with its neighbour (the one 3-key exception; drag-to-swap also works) |

Alt+Y replaces the Alt+J proposed in chat: Cmd+J already opens a chat pane
(`chords.rs`), so Alt+J keeps that meaning. Alt+R is left free for panes that
read Cmd+R themselves (the viewer's re-read).

### Ctrl, by where the focus is

| Focus | Ctrl keys |
|---|---|
| Terminal | Every Ctrl key goes to the program, with two exceptions taken from Windows Terminal: **Ctrl+C copies when text is selected** (and sends ^C otherwise), **Ctrl+V pastes**. |
| Editor (viewer pane editing), input bar, settings fields, inbox compose | Ctrl+Z undo, Ctrl+Y redo, Ctrl+S save, Ctrl+A select all, Ctrl+C / X / V copy / cut / paste, Ctrl+F find, Ctrl+B / I bold / italic in markdown. |
| Anywhere | Ctrl+Tab / Ctrl+Shift+Tab next / previous pane (kept as is), Ctrl+PgUp / PgDn tabs, Ctrl+1 … 9 tab N. |

Ctrl+O (chat compact view, `is_compact_chord`) stays as the chat pane's own
Ctrl key.

### What Alt gives up

- **Alt as Meta in terminals.** Today Alt+letter sends ESC+letter (`keyenc.rs`).
  After this, only Alt+letters crew does not use still reach the shell as Meta
  (R and the unused punctuation). bash/zsh word jumps Alt+B / Alt+F become
  crew's. PowerShell's own keys use Ctrl+arrow, so it barely notices. A
  settings switch `alt_is_meta` could give terminals Alt back while a terminal
  has focus (crew chords then need focus outside it). This is out of scope
  unless the user asks.
- **OS keys stay the OS's**: Alt+F4 (close window), Alt+Tab, Alt+Space (window
  menu), Win+anything. crew must never consume them.

## Design

### 1. One translation point: `winkeys.rs` (new)

`winkeys::chord(event, mods) -> Option<String>` returns the **same chord string
the Mac's Cmd produces**, so `CrewApp::handle_super_chord` (chords.rs) stays the
single place actions live:

- Alt (not Ctrl, not AltGr, which arrives as Ctrl+Alt) + a key in the table
  gives the plain letter: Alt+D gives `"d"`.
- The remapped ones give the Mac's shifted string: Alt+U gives `"T"`, Alt+X
  gives `"E"`, Alt+M gives `"M"`.
- Theme / gradient / focus / md-source get chord strings of their own (for
  example `"theme"`, `"gradient"`, `"focus"`, `"mdsource"`). `keys.rs` routes
  those to the functions the Ctrl+Shift blocks call today, so the four
  hand-written Ctrl+Shift blocks become one table lookup.
- Read the key with `key_without_modifiers()` / `physical_key` so a non-US
  layout's Alt+key still names the letter printed on the key.
- Quit is its own variant (`WinChord::Quit`): Cmd+Q is handled in `keys.rs`
  before the Cmd chords, not by `handle_super_chord`.
- Digits match the physical key (AZERTY's `&` is still 1); `0` and the
  punctuation (`, . / [ ] = -`) match the layout's character, so on a layout
  where one needs Shift or AltGr that chord is not two keys. Fine on US
  layouts; revisit if a user hits it.
- Returns `None` for Alt+F4, Alt+Tab, Alt+Space and any key not in the table,
  so those reach the OS or the terminal (as Meta).

`keychord::cmd_chord` calls `winkeys::chord` off a Mac **instead of**
`stand_in`. Super (the Windows key) no longer counts as Cmd off a Mac. It never
reached crew reliably anyway, because the OS keeps Win+key.

### 2. Delete the old stand-ins

- `keychord::stand_in`, `plain`, `stands_in`, `shifted`, `TYPED` and
  `ctrlchord::typing_here`'s role in chords. `typing_here` stays only if
  something else uses it.
- The four `Ctrl+Shift+L/G/F/M` blocks in `keys.rs` become Mac-only (on a Mac
  they are still Ctrl+Shift, which is two modifiers plus a key, but the user
  asked about Windows only).
- `ctrl_letter` (stray letters in Far / settings): re-check it. With plain Ctrl
  no longer a chord, those panes must not type a letter for Ctrl+key.

### 3. Arrows, Home/End, PgUp/PgDn, digits

- `ctrlchord::grid_held`: off a Mac it is already Alt. Keep it, and keep
  Alt+Shift+arrow as swap.
- `ctrlchord::screen_held` (Ctrl+Alt off a Mac) goes away off a Mac. Screens
  move to **Alt+Home / Alt+End**, command jump to **Alt+PgUp / Alt+PgDn**
  (`cmdjump`), and tabs to **Ctrl+PgUp / Ctrl+PgDn** (`step_tab`).
- `tablines::tab_digit` off a Mac becomes **Ctrl+1 … 9**. Alt+1 … 9 goes
  through `winkeys::chord` as `"1"` … `"9"`, and the existing fall-through in
  `handle_super_chord` focuses pane N.
- F11 off a Mac toggles maximize (`toggle_maximize`).

### 4. Ctrl in panes

- **Terminal:** in the terminal key route (`keyroute.rs` / `keyenc.rs`), off a
  Mac only: Ctrl+C with a selection copies and clears the selection, with no
  byte sent. Ctrl+V pastes through the bracketed-paste path `paste()` uses.
  Everything else is encoded as today.
- **Editor** (`viewedit.rs` / `viewpane/editkeys.rs`): it already treats Ctrl
  like Cmd (`super_key() || control_key()`). Make sure Ctrl+Z / Y / S / A / C /
  X / V / F / B / I all reach it before any app chord. Add Ctrl+Y = redo off a
  Mac (Cmd+Shift+Z keeps redo on the Mac).
- **Input bar** (`input.on_key(event, ctrl)`) and docwin already take Ctrl as
  Cmd (`chordglyph::shown_ctrl`). Check that Ctrl+Z / Y / A / C / V / X work
  there.

### 5. Alt alone must not open the window menu

On Windows, pressing and releasing Alt alone can put the window into menu mode
(WM_SYSKEYUP leads to SC_KEYMENU), which swallows the next key or beeps. Check
whether winit 0.30 already suppresses this for a window with no menu. If it
doesn't, handle SC_KEYMENU in a `#[cfg(windows)]` subclass or message hook so a
lone Alt does nothing, while Alt+F4 and Alt+Space still reach DefWindowProc.
This can only be confirmed on a real Windows machine (see Verification).

### 6. Labels everywhere

- `chordglyph::offmac` / `native` / `shown` / `prose`: off a Mac, write
  `Cmd+X` as `Alt+X`, and write the remapped chords with their Windows keys
  through one override table shared with `winkeys` (the same data, so the label
  and the key cannot drift). For example `Cmd+Shift+T` becomes `Alt+U`, and
  `Cmd+{ / Cmd+}` becomes `Ctrl+PgUp / Ctrl+PgDn`.
- `helptable.rs` BINDINGS keep the portable Mac spelling (the README parity
  tests read them). The overlay rewrites them through `shown`.
- `welcometext.rs` ("Ctrl+Shift+T shell · Ctrl+Shift+O agents"), `cmdkeys.rs`
  (`/reopen`, `/focus`), `tabcmd.rs` usage text, toasts that name chords, and
  the settings form: all go through `shown` / `prose`.
- Docs: `docs/CREW.md` (about 125 chord mentions) and `README.md` (about 52)
  get a **Windows keys** section with the tables above. They don't need a
  Windows spelling beside every Mac chord.

## Steps (one change per step, each merged green)

1. **`winkeys.rs` + tests**: the table and pure `chord(key, mods)`, with no
   wiring yet. The tests cover every row of both tables, AltGr (Ctrl+Alt) not
   being a chord, Alt+F4 / Tab / Space being `None`, and a non-US layout.
2. **Wire it in**: `cmd_chord` uses it off a Mac. Delete the stand-ins and
   `TYPED`. Route theme / gradient / focus / md-source. Update the
   `keychord` / `ctrlchord` tests that assert the old Ctrl stand-ins.
3. **Arrows, PgUp/PgDn, Home/End, digits, F11** (`ctrlchord`, `tablines`,
   `cmdjump`, `step_tab`).
4. **Ctrl in panes**: terminal Ctrl+C copy-on-selection, Ctrl+V paste, editor
   Ctrl+Y redo, plus input bar checks.
5. **Lone-Alt menu guard** (`#[cfg(windows)]`), after checking winit's
   behaviour.
6. **Labels**: `chordglyph` override table, welcome text, cmdkeys, tabcmd,
   toasts. Extend `chordglyph_tests` so every BINDINGS row renders off a Mac
   with no `Shift+` and no `Ctrl+Alt+`, except the swap row.
7. **Docs**: the Windows keys section in `docs/CREW.md` and `README.md`, plus a
   CHANGELOG entry.
8. **Release**, one per day, through `scripts/release.sh`.

Every new file stays under the 200-line cap (`line-cap-debt.txt`).

## Tests

- Unit: `winkeys` table (every row), `chordglyph` off-Mac rendering (no
  three-key label left), terminal Ctrl+C with and without selection, editor
  Ctrl+Y.
- A guard test: no BINDINGS row, once rendered off a Mac, has two modifiers,
  except `Alt+Shift+arrow`.
- Cross-compile: `cargo xwin check` for `x86_64-pc-windows-msvc`. The Windows
  CI workflow must stay green.

## Verification on Windows (needs the user)

Nobody has run crew's Windows keys on a real machine. After the release, the
user checks on Windows:

1. Alt+D, Alt+T, Alt+W, Alt+U open, tab and close panes from inside a terminal
   pane.
2. In PowerShell: Ctrl+C interrupts a running `ping -t`, and Ctrl+C with text
   selected copies instead. Ctrl+V pastes and Ctrl+R searches history (PSReadLine).
3. Pressing and releasing Alt alone, then typing, loses no key and makes no
   beep.
4. Alt+F4 closes the window and Alt+Tab switches apps.
5. The `/keys` overlay shows Alt / Ctrl spellings with no Ctrl+Shift left.

## Open questions

- **Linux**: follow Windows (the default in this plan), or keep today's scheme
  there? Many Linux desktops use Alt+drag and some Alt chords for the window
  manager.
- **Mac Ctrl+Shift+L / G / F / M**: the Mac keeps these (the user asked about
  Windows). Moving them to Cmd+L / Cmd+B / Cmd+H / Cmd+Y would make the
  platforms match. Cmd+H is the Mac's Hide, so focus mode would need another
  key there.
- **`alt_is_meta` setting** for people who want Alt as Meta in terminals:
  later, only if asked.
