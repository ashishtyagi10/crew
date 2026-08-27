//! The commands that act on panes, their contents and the session — the
//! first half of the palette's priority order. See the parent module for why
//! the table is one ordered sequence split across files rather than one file.
use super::Cmd;

/// Palette rows, highest priority first.
pub(crate) const WORK: &[Cmd] = &[
    Cmd {
        name: "/settings",
        desc: "Open settings",
    },
    Cmd {
        name: "/smith",
        desc: "Open agent smith — the multi-agent relay pane (alias: /crew)",
    },
    Cmd {
        name: "/diff",
        desc: "Review the working tree's changes in the file viewer",
    },
    Cmd {
        name: "/find",
        desc: "Search scrollback, highlighting matches (/find <text>)",
    },
    Cmd {
        name: "/marks",
        desc: "Card border marks — command ticks and error bars (on|off)",
    },
    Cmd {
        name: "/pin",
        desc: "Keep the focused pane on the grid (never demoted to the strip)",
    },
    Cmd {
        name: "/blocks",
        desc: "What you ran in this pane, how long each took and which failed",
    },
    Cmd {
        name: "/out",
        desc: "Open the last command's output on its own, in the file viewer",
    },
    Cmd {
        name: "/errorsall",
        desc: "Which panes have errors, and how many (then land on the first)",
    },
    Cmd {
        name: "/errors",
        desc: "Scroll back to the most recent error in this pane (repeat for the one before)",
    },
    Cmd {
        name: "/findall",
        desc: "Search every pane's scrollback (/findall <text>)",
    },
    Cmd {
        name: "/name",
        desc: "Rename the focused pane (/name <text>)",
    },
    Cmd {
        name: "/clear",
        desc: "Clear the focused pane's scrollback",
    },
    Cmd {
        name: "/clearall",
        desc: "Clear every pane's scrollback",
    },
    Cmd {
        name: "/clearlog",
        desc: "Clear the live activity log in the sidebar",
    },
    Cmd {
        name: "/only",
        desc: "Close all panes except the focused one",
    },
    Cmd {
        name: "/closeall",
        desc: "Close every pane",
    },
    Cmd {
        name: "/pwd",
        desc: "Copy the working directory to the clipboard",
    },
    Cmd {
        name: "/about",
        desc: "Show the Crew version",
    },
    Cmd {
        name: "/log",
        desc: "Open this session's full activity log in the viewer",
    },
    Cmd {
        name: "/copy",
        desc: "Copy the focused pane's full scrollback to the clipboard",
    },
    Cmd {
        name: "/dump",
        desc: "Save scrollback to a file (/dump <file>)",
    },
];
