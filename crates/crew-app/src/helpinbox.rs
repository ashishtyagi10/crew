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
    ("u", "Undo the last archive, delete, read or star"),
    ("m", "Mark read or unread"),
    ("s", "Star or unstar"),
    ("r", "Agent smith writes a reply, saved as a Gmail draft"),
    ("a", "Ask agent smith about the thread"),
    ("t", "A /todo row, due on the deadline the mail states"),
    ("/", "Search with Gmail's own syntax (from:ann is:unread)"),
    ("R", "Refresh the list"),
    ("L", "One line a thread, or two (kept until crew quits)"),
    ("o", "Open the thread, or the inbox, in Gmail"),
    ("z", "In a thread: show or fold the quoted history"),
    ("v", "In a thread: the plain-text version"),
    ("i", "In a thread: fetch its pictures from the web"),
    ("w", "In a thread: open it in a window of its own"),
    ("?", "These keys"),
];
