//! The per-pane binding tables the `/keys` overlay lists after the app-wide
//! ones in [`crate::helptable`]: one per pane kind that answers to keys of
//! its own. Data only, like its sibling; `help_tests` reads each pane's key
//! map and holds these tables to it.

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

/// Keys a **document window** answers to — the markdown editor `w` (or
/// `/doc`) opens in a window of its own. The viewer's keys still apply there
/// (Esc, the search, the scroll); these are the ones the caret adds.
///
/// The whole map lived in `docwin/keys.rs` and the manual, and in no in-app
/// list: the one pane kind added after `/keys` learned to hold every pane to
/// its own key map was the one pane kind it did not.
pub(crate) const DOC_BINDINGS: &[(&str, &str)] = &[
    (
        "Arrows \u{b7} Home / End",
        "Move the caret \u{b7} to either end of the line",
    ),
    (
        "Shift+arrows \u{b7} Cmd+A",
        "Select \u{b7} select the whole document",
    ),
    ("Click", "Put the caret where you are looking"),
    (
        "Backspace / Delete",
        "Remove the character behind \u{b7} at the caret",
    ),
    (
        "Cmd+B / Cmd+I",
        "Bold / italic the selection (or unwrap it again)",
    ),
    (
        "Cmd+K",
        "Edit the link's URL under the caret, or make one out of the selection",
    ),
    ("Tab", "Next table cell \u{b7} two spaces anywhere else"),
    ("Cmd+C / Cmd+X / Cmd+V", "Copy / cut / paste, as markdown"),
    ("Cmd+Z / Cmd+Shift+Z", "Undo / redo"),
    (
        "Cmd+S",
        "Save \u{2014} only the bytes you changed are rewritten",
    ),
    (
        "Cmd+R",
        "Re-read the file from disk (asks once if you have unsaved edits)",
    ),
];
