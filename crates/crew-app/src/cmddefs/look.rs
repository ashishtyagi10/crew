//! The commands that change how crew looks and behaves, plus the file
//! viewer's — the second half of the palette's priority order, continued
//! from [`super::work`].
use super::Cmd;

/// Palette rows, continuing `work::WORK`.
pub(crate) const LOOK: &[Cmd] = &[
    Cmd {
        name: "/font",
        desc: "Set the font size (/font <n>) or toggle rotation (/font random)",
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
        name: "/theme",
        desc: "Switch theme — pick from the list",
    },
    Cmd {
        name: "/crt",
        desc: "CRT tube look on/off (/crt [on|off|auto])",
    },
    Cmd {
        name: "/weight",
        desc: "Text weight — thicker/lighter font (/weight [medium|semibold|bold|…])",
    },
    Cmd {
        name: "/smooth",
        desc: "Font smoothing — CoreText-style stem darkening (/smooth [off|light|medium|heavy|<0-255>])",
    },
    Cmd {
        name: "/motion",
        desc: "How much crew moves — auto follows the OS Reduce Motion switch (/motion [auto|off|subtle|full])",
    },
    Cmd {
        name: "/shapes",
        desc: "Say it with a shape as well as a colour \u{2014} auto follows the OS Differentiate Without Color switch (/shapes [auto|off|on])",
    },
    Cmd {
        name: "/contrast",
        desc: "WCAG floor every derived colour is held to \u{2014} auto follows the OS Increase Contrast switch (/contrast [auto|normal|high])",
    },
    Cmd {
        name: "/focus",
        desc: "Focus mode \u{2014} hold every notification, never steal focus, deepen the spotlight (/focus)",
    },
    Cmd {
        name: "/leading",
        desc: "Line spacing: air between rows of text (tight|normal|relaxed|loose)",
    },
    Cmd {
        name: "/density",
        desc: "How tightly the canvas packs \u{2014} pane gutter and chat-card spacing (/density [compact|cozy|roomy])",
    },
    Cmd {
        name: "/gradient",
        desc: "Canvas gradient — how far its colour breathes, or poles of your own (/gradient [off|subtle|lively|<#a> <#b>|reset])",
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
        name: "/invisibles",
        desc: "Reveal tabs, trailing spaces and CRs in the file viewer (on|off)",
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
        name: "/md",
        desc: "view a file \u{2014} alias for /view (/md <path>)",
    },
    Cmd {
        name: "/exit",
        desc: "Quit Crew",
    },
];
