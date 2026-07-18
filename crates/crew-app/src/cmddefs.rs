//! The command palette's table: every slash command with its palette
//! description, in priority order (prefix ties break by list position —
//! e.g. `/res` completes to /restart, not /restore). Logic lives in
//! `suggest`.

/// A slash command shown in the command palette.
pub(crate) struct Cmd {
    pub name: &'static str,
    pub desc: &'static str,
}

/// Known slash commands (kept in sync with run_slash_command; /shell and /run stay dispatchable but bare text replaced their palette rows).
pub(crate) const COMMANDS: &[Cmd] = &[
    Cmd {
        name: "/settings",
        desc: "Open settings",
    },
    Cmd {
        name: "/smith",
        desc: "Open the multi-agent pane — the agent relay (alias: /crew)",
    },
    Cmd {
        name: "/diff",
        desc: "Review the working tree's git diff in a new pane",
    },
    Cmd {
        name: "/find",
        desc: "Search scrollback, highlighting matches (/find <text>)",
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
        name: "/copy",
        desc: "Copy the focused pane's full scrollback to the clipboard",
    },
    Cmd {
        name: "/dump",
        desc: "Save scrollback to a file (/dump [file])",
    },
    Cmd {
        name: "/font",
        desc: "Set the font size (/font <n>) or toggle rotation (/font random)",
    },
    Cmd {
        name: "/restart",
        desc: "Restart Crew — relaunch detached, applying an installed /update",
    },
    Cmd {
        name: "/restore",
        desc: "Reopen last session's shells in their directories",
    },
    Cmd {
        name: "/theme",
        desc: "Switch theme — pick from the list",
    },
    Cmd {
        name: "/crt",
        desc: "CRT tube look on/off (/crt [on|off|auto])",
    },
    Cmd {
        name: "/notify",
        desc: "Notification settings (/notify [on|off|add <text>|clear])",
    },
    Cmd {
        name: "/update",
        desc: "Update Crew to the latest release (left-nav progress; /restart applies)",
    },
    Cmd {
        name: "/broadcast",
        desc: "Toggle synchronized input to all panes (Cmd+S)",
    },
    Cmd {
        name: "/zoom",
        desc: "Toggle zoom of the focused pane (Cmd+Z)",
    },
    Cmd {
        name: "/sidebar",
        desc: "Toggle the stats sidebar (Cmd+G)",
    },
    Cmd {
        name: "/keys",
        desc: "Show keyboard shortcuts",
    },
    Cmd {
        name: "/far",
        desc: "Open a dual-pane file manager",
    },
    Cmd {
        name: "/goal",
        desc: "Plan a goal into a task graph and run it as a swarm (/goal <text>)",
    },
    Cmd {
        name: "/batch",
        desc: "Run a file of jobs (one per line) as a parallel swarm (/batch <file>)",
    },
    Cmd {
        name: "/md",
        desc: "view a markdown file (source | preview)",
    },
    Cmd {
        name: "/exit",
        desc: "Quit Crew",
    },
];
