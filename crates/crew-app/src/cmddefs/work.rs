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
        name: "/find",
        desc: "Search scrollback, highlighting matches (/find <text>, /find all <text>)",
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
        name: "/tools",
        desc: "What agents ran through the tool gate \u{b7} /tools <term> narrows it",
    },
    Cmd {
        name: "/watching",
        desc: "What crew is waiting to do on its clock \u{b7} cancel <id> \u{b7} snooze <id> 30m",
    },
    Cmd {
        name: "/integrations",
        desc: "What crew can reach: every manifest, its credential, every tool's tier",
    },
    Cmd {
        name: "/lsp",
        desc: "Language servers: which crew knows, which are installed, which are running",
    },
    Cmd {
        name: "/out",
        desc: "The last command\'s output in a pane (/out <n> for an earlier one, /out list for all)",
    },
    Cmd {
        name: "/errors",
        desc: "Back to the most recent error here (repeat for the one before; /errors all counts every pane)",
    },
    Cmd {
        name: "/name",
        desc: "Rename the focused pane (/name <text>)",
    },
    Cmd {
        name: "/clear",
        desc: "Clear scrollback — this pane, or /clear all, or /clear log",
    },
    Cmd {
        name: "/close",
        desc: "Close panes — /close all, or /close others (keeps the focused one)",
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
        name: "/copy",
        desc: "Copy the focused pane's full scrollback to the clipboard",
    },
    Cmd {
        name: "/dump",
        desc: "Save scrollback to a file (/dump <file>)",
    },
];
