//! The commands that change how crew looks and behaves, plus the file
//! viewer's — the second half of the palette's priority order, continued
//! from [`super::work`].
//!
//! Fifteen of these rows were one family — `/theme`, `/gamma`, `/grain`,
//! `/leading` and the rest — and they are one row now: `/look`, whose picker
//! walks subject then value (`crate::lookcmd`). They still RUN when typed;
//! what the palette offers is what a user has to know, and a family belongs
//! there once.
use super::Cmd;

/// Palette rows, continuing `work::WORK`.
pub(crate) const LOOK: &[Cmd] = &[
    Cmd {
        name: "/look",
        desc: "How crew looks — theme, font, weight, motion, grain and the rest (/look [subject] [value])",
    },
    Cmd {
        name: "/reopen",
        desc: "Reopen the pane you just closed",
    },
    Cmd {
        name: "/restore",
        desc: "Reopen last session's panes \u{2014} shells in their directories, /far, agent smith, /todo, the drawn panes and open files",
    },
    Cmd {
        name: "/nav",
        desc: "The left column: /nav glance, /nav log, /nav weather <place>",
    },
    Cmd {
        name: "/focus",
        desc: "Focus mode \u{2014} hold every notification, never steal focus, deepen the spotlight",
    },
    Cmd {
        name: "/notify",
        desc: "Notification settings (/notify [on|off|add <text>|clear])",
    },
    Cmd {
        name: "/update",
        desc: "Update crew to the latest release and restart into it (progress shows in the open nav)",
    },
    Cmd {
        name: "/broadcast",
        desc: "Type into every terminal at once, or stop",
    },
    Cmd {
        name: "/zoom",
        desc: "Zoom the focused pane to fill the grid, or put it back",
    },
    Cmd {
        name: "/sidebar",
        desc: "Show or hide the left nav",
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
        name: "/todo",
        desc: "Todo list — due dates (tomorrow 5pm) & @project while typing; r runs a row in its project with the best agent; /todo run|project|show|hide|done",
    },
    Cmd {
        name: "/usage",
        desc: "What crew has spent, drawn — a week of tokens by hour, the in/out split, and cost per day",
    },
    Cmd {
        name: "/dash",
        desc: "One screen of the machine and the week — CPU, memory and disk dials, the CPU curve, both directions of the network, and a heatmap of token usage",
    },
    Cmd {
        name: "/disk",
        desc: "Where the space went — this directory as a treemap; arrows pick, enter opens, backspace up",
    },
    Cmd {
        name: "/model",
        desc: "Set the model for agent smith's agents — pick from the list",
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
        name: "/view",
        desc: "The file viewer: /view <path>, or /view diff | blame | log",
    },
    Cmd {
        name: "/doc",
        desc: "Open a file in a window of its own \u{2014} a document, not a tile (/doc <path>)",
    },
    Cmd {
        name: "/exit",
        desc: "Quit crew",
    },
];
