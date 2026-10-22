//! The `/inbox` pane's keys, as `/keys` lists them: the section `?` in the
//! pane opens on (goal 2026-10-20-inbox-list-look, item 10). Its own file
//! because `helppanes` had no room left under the line cap.

/// The section's heading, which `helphere` scrolls to.
pub(crate) const HEAD: &str = "in the /inbox pane";

pub(crate) const INBOX_BINDINGS: &[(&str, &str)] = &[
    (
        "Up / Down \u{b7} j / k",
        "Move through the list; in a thread, scroll it",
    ),
    ("Enter", "Read the thread (marks it read, as Gmail does)"),
    (
        "n / p \u{b7} J / K",
        "In a thread: open the next thread down the list, or the one above",
    ),
    (
        "Click",
        "Select a thread, again to read it; a \u{2605} unstars; a key at the foot, or \u{2039} \u{203a}, does it",
    ),
    ("Esc", "Back to the list; clear a search; back to the grid, the inbox still open"),
    ("e", "Archive"),
    ("d", "Delete: to Gmail's Trash, never for good"),
    ("z", "Undo the last archive, delete, read or star"),
    ("m", "Mark read or unread"),
    ("s", "Star or unstar"),
    (
        "r",
        "Reply to everyone on the mail, in a box under it: \u{2318}/Ctrl+Enter sends, Tab drafts with smith",
    ),
    (
        "f",
        "Forward the newest mail, its pictures and files with it, under a note of yours",
    ),
    (
        "Up / Down",
        "In the reply box: through the to and cc lines, the words and the files (\u{2190} \u{2192} among the files)",
    ),
    (
        "Down, then Enter",
        "In the reply box: + attach picks files (or drop them); Enter opens one, Backspace takes it off",
    ),
    ("a", "Ask agent smith about the thread"),
    ("t", "A /todo card under the mail: title, @project, #assignee, due; Tab: smith fills it"),
    ("/", "Search with Gmail's own syntax (from:ann is:unread)"),
    ("R", "Refresh the list"),
    ("L", "One line a thread, or two (kept until crew quits)"),
    ("o", "Open the thread, or the inbox, in Gmail"),
    (
        "q",
        "In a thread: show or fold the earlier replies it quotes",
    ),
    (
        "u",
        "In a thread: unsubscribe from its sender (a second u confirms)",
    ),
    ("v", "In a thread: the plain-text version"),
    (
        "i",
        "In a thread: hide its pictures from the web, or show them",
    ),
    ("w", "In a thread: open it in a window of its own"),
    (
        "x",
        "In a long thread: open every mail, or fold the older ones",
    ),
    ("?", "These keys"),
];
