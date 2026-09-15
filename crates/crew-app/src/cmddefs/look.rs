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
        desc: "Reopen the pane you just closed (Cmd+Shift+T)",
    },
    Cmd {
        name: "/restore",
        desc: "Reopen last session's shells in their directories",
    },
    Cmd {
        name: "/nav",
        desc: "What the nav's slot shows: glance cards or the log (/nav [glance|log])",
    },
    Cmd {
        name: "/weather",
        desc: "The nav's WEATHER card for a place (/weather <place> | auto | off)",
    },
    Cmd {
        name: "/focus",
        desc: "Focus mode \u{2014} hold every notification, never steal focus, deepen the spotlight (/focus)",
    },
    Cmd {
        name: "/notify",
        desc: "Notification settings (/notify [on|off|add <text>|clear])",
    },
    Cmd {
        name: "/update",
        desc: "Update Crew to the latest release and restart into it (left-nav progress)",
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
        name: "/todo",
        desc: "Todo list — due dates (tomorrow 5pm) & @project while typing; /todo show|hide = ticked items, /todo done = the log",
    },
    Cmd {
        name: "/usage",
        desc: "What crew has spent, drawn — a week of tokens by hour, the in/out split, and cost per day",
    },
    Cmd {
        name: "/dash",
        desc: "One screen of the machine and the week — ring gauges, the CPU curve, both directions of the network, and a heatmap of token usage",
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
        desc: "Plan a goal into a task graph and run it as a swarm (in agent smith, /goal judges rounds instead)",
    },
    Cmd {
        name: "/batch",
        desc: "Run a file of jobs (one per line) as a parallel swarm (/batch <file>)",
    },
    Cmd {
        name: "/blame",
        desc: "Who last touched each line of the file in the viewer (again turns it off)",
    },
    Cmd {
        name: "/view",
        desc: "view any file — code, markdown, data, csv, diffs (/view <path>)",
    },
    Cmd {
        name: "/doc",
        desc: "open a file in a window of its own \u{2014} a document, not a tile (/doc <path>)",
    },
    Cmd {
        name: "/md",
        desc: "view a file \u{2014} alias for /view (/md <path>)",
    },
    Cmd {
        name: "/exit",
        desc: "Quit Crew",
    },
];
