//! The binding tables the `/keys` overlay lists: the app-wide ones, then one
//! per pane kind that answers to keys of its own. Data only: the rendering,
//! the scrolling and the docs-parity contract live in [`crate::help`].
//!
//! They stopped having a height budget when the overlay learned to scroll, so
//! a new binding is a new row here and nothing else has to give way for it.

/// `(keys, description)` rows shown in the overlay.
pub(crate) const BINDINGS: &[(&str, &str)] = &[
    ("Ctrl+Tab / Ctrl+Shift+Tab", "Next / previous pane"),
    ("Cmd+1 … 9", "Jump to pane N"),
    ("Cmd+A / Cmd+.", "Jump to next active / waiting pane"),
    ("Cmd+{ / Cmd+}", "Move pane left / right"),
    (
        "Cmd+\u{2190}\u{2191}\u{2192}\u{2193}",
        "Focus the pane that way on the grid",
    ),
    (
        "Cmd+Shift+\u{2190}\u{2191}\u{2192}\u{2193}",
        "Swap the focused pane with that neighbour",
    ),
    ("Cmd+I / Cmd+T", "Focus the input bar / new shell pane"),
    ("Cmd+F", "Find: in a chat transcript, or /find in the bar"),
    ("Cmd+, / Cmd+J", "Settings / chat pane"),
    ("Cmd+G / Cmd+Z", "Toggle sidebar / zoom focused pane"),
    ("Cmd+S", "Broadcast to all panes \u{b7} save a focused settings form"),
    (
        "Cmd+= / Cmd+- / Cmd+0 / Cmd+wheel",
        "Font size + / - / reset",
    ),
    ("Cmd+C / Cmd+V", "Copy screen / paste"),
    (
        "Cmd+E",
        "Label every URL, file and hash on the pane \u{00b7} a letter copies it, its capital opens it",
    ),
    (
        "Cmd+Click",
        "open URL/file/dir · copy a code block in an agent pane",
    ),
    ("Cmd+W / Cmd+M", "Close pane / maximize"),
    ("Cmd+Shift+T", "Reopen the pane you just closed"),
    ("Cmd+K", "Clear focused pane scrollback"),
    (
        "Ctrl+Shift+L",
        "Cycle themes (dark \u{2192} light \u{2192} crt)",
    ),
    (
        "Ctrl+Shift+G",
        "Step the canvas gradient (aurora \u{2192} tide \u{2192} \u{2026} \u{2192} the theme's own)",
    ),
    ("Ctrl+Shift+M", "Chat: markdown preview \u{2194} raw source"),
    ("Ctrl+O", "Chat: compact transcript view"),
    ("Shift+PageUp / Shift+PageDown", "Scroll the focused pane"),
    ("Shift+Home / Shift+End", "Scroll to top / bottom"),
    (
        "Drag a card's right border",
        "Scroll it \u{b7} the sidebar's LOG scrolls with the wheel",
    ),
    ("Double-click / Triple-click", "Select the word / the line"),
    (
        "On a card's top border",
        "Double-click zooms \u{b7} drag it onto another card to swap them",
    ),
    ("Cmd+/", "These keys"),
    ("/ (in input)", "Command palette"),
    (
        "! · * · ? · ?? (in input)",
        "New pane / broadcast / ask ai a command / explain this pane",
    ),
    ("Cmd+Q", "Quit"),
];

/// Keys that mean something specific inside an agent pane. The overlay
/// claimed to list "the bindings" and had none of these — including the two
/// that answer a drafted plan, added in v0.6.46.
pub(crate) const CHAT_BINDINGS: &[(&str, &str)] = &[
    (
        "Enter",
        "Send · answers a pending plan when the composer is empty",
    ),
    (
        "Esc",
        "Discard a pending plan · interrupt a running turn · close",
    ),
    ("Shift+Enter", "Newline instead of sending"),
    ("Tab", "Complete the leading @agent or /construct"),
    ("Ctrl+R", "Reverse-search prompts you've sent"),
    ("Cmd+F / Ctrl+F", "Find in the transcript, jump per match"),
    (
        "Up / Down",
        "Recall a prompt you already sent · navigate an open popup",
    ),
    (
        "Tab / Right",
        "Take the suggested rest of a prompt you sent before",
    ),
    (
        "@ · # (in composer)",
        "Attach an agent, skill or file · remember a note",
    ),
    (
        "@file:120-180 (in composer)",
        "Attach just those lines, instead of the whole file",
    ),
    (
        "@a+b (in composer)",
        "Fan one task out to both agents, in parallel",
    ),
];

/// Keys that mean something specific inside the **file viewer** — `/view`,
/// `/md`, `/diff`, `/out`, `/about`, and every path crew opens a file on.
///
/// The overlay claimed to list "the bindings" and had none of these. Every
/// one of them was documented in the manual and reachable only by reading it,
/// which is the same shape as the chat pane's keys before v0.6.46 and the
/// same shape as `Ctrl+O`: implemented, tested, and in neither list.
pub(crate) const VIEW_BINDINGS: &[(&str, &str)] = &[
    // Spelled out rather than drawn as arrows, like the chat table's own
    // "Up / Down": the overlay's "more below" hint IS a `\u{2193}`, and a
    // binding row carrying one makes the test for that hint pass forever.
    (
        "Up / Down · PageUp/Down · Home/End",
        "Scroll a line · a page · to either end",
    ),
    ("/ · n / N", "Search the file · next / previous hit"),
    (
        "] / [",
        "Step the document's structure: file to file and hunk to hunk, heading to heading",
    ),
    ("v", "Diff: side by side, or unified again"),
    ("s", "Markdown and CSV: show the raw text instead"),
    ("r", "Re-read the file from disk"),
    ("e / o", "Open in $EDITOR · hand to the OS default app"),
    (
        "w",
        "Take the document out of the grid, into a window of its own",
    ),
    ("Cmd+Click", "Follow a rendered markdown link"),
    ("Esc", "Close the viewer (a live search first)"),
];

/// Keys that mean something specific inside a **`/far` file panel** — the
/// two-panel commander, whose whole interface is its function-key row.
///
/// Those keys were drawn along the bottom of the pane and written down in the
/// manual, and were in no in-app list at all: `/keys` is where a user looks
/// for "what can I press here", and every one of these was somewhere else.
pub(crate) const FAR_BINDINGS: &[(&str, &str)] = &[
    ("Tab", "Switch panels (or complete a typed command)"),
    (
        "Enter · Backspace",
        "Descend into the selection or run the typed command \u{b7} go up",
    ),
    (
        "F1 · Alt+F1 / Alt+F2",
        "This help \u{b7} re-root the left / right panel on a drive",
    ),
    (
        "F3 / F4",
        "View the selection in the file viewer / open it in $EDITOR",
    ),
    ("F5 / F6", "Copy / move to the other panel"),
    ("F7 / F8", "Make a folder \u{b7} move to the trash"),
    (
        "F10 · Esc",
        "Close the panel \u{b7} clear the typed command, then close",
    ),
    (
        "Up / Down · PageUp/Down · Home/End",
        "Move the selection \u{b7} by a page \u{b7} to either end",
    ),
    (
        "! (in the command line)",
        "Ask the AI for the command, then Enter to run what it suggests",
    ),
];

/// Keys the **`/todo`** pane answers to. The overlay carried two rows of
/// these and the list's other six actions — delete, edit, the filter cycle
/// and the due-date bump — were in the manual only.
pub(crate) const TODO_BINDINGS: &[(&str, &str)] = &[
    ("Up / Down · PageUp/Down · Home/End", "Move the selection"),
    ("Enter · Space", "Tick the selected item off, or back on"),
    ("e · d · Backspace/Delete", "Edit it \u{b7} delete it"),
    ("] / [", "Cycle the filter forward / back"),
    ("+ / -", "Push the due date later / earlier"),
    (
        "h · H",
        "Show done items (or the [show N done] button) \u{b7} the history log",
    ),
    ("Tab", "Complete an @project tag in the composer"),
    (
        "Ctrl+A / Ctrl+E · Alt+Left / Alt+Right",
        "Composer: jump to either end \u{b7} by a word",
    ),
    (
        "Esc",
        "Back one layer: popup, then the draft, then the pane",
    ),
];

/// Keys the **`/settings`** form answers to. Every field is reached and
/// changed without the mouse; the overlay never said how.
pub(crate) const SETTINGS_BINDINGS: &[(&str, &str)] = &[
    ("Tab / Shift+Tab", "Next / previous field"),
    (
        "Left / Right · Space",
        "Step a picker's value \u{b7} toggle a checkbox",
    ),
    ("Enter", "Take the value and move on"),
    ("Cmd+S / Alt+S", "Save and apply"),
    ("Esc", "Close without saving"),
];
