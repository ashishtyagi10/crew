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
    ("Esc", "Back to the list; clear a search; close the pane"),
    ("e", "Archive"),
    ("d", "Delete: to Gmail's Trash, never for good"),
    ("z", "Undo the last archive, delete, read or star"),
    ("m", "Mark read or unread"),
    ("s", "Star or unstar"),
    ("r", "Agent smith writes a reply, saved as a Gmail draft"),
    ("a", "Ask agent smith about the thread"),
    ("t", "A /todo row, due on the deadline the mail states"),
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
    ("?", "These keys"),
];
