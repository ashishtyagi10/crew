use super::command::SlashCommand;

/// Second half of the slash command catalog (sorted, M through Y).
pub const PART_B: &[SlashCommand] = &[
    SlashCommand {
        command: "/menu",
        description: "Open menu bar",
    },
    SlashCommand {
        command: "/open",
        description: "Open with system application",
    },
    SlashCommand {
        command: "/opencode",
        description: "Launch OpenCode AI agent",
    },
    SlashCommand {
        command: "/plugin",
        description: "List or run plugins",
    },
    SlashCommand {
        command: "/recent",
        description: "Recent directories",
    },
    SlashCommand {
        command: "/refresh",
        description: "Refresh both panels",
    },
    SlashCommand {
        command: "/rename-batch",
        description: "Batch rename files",
    },
    SlashCommand {
        command: "/search",
        description: "Search in files",
    },
    SlashCommand {
        command: "/shell",
        description: "Open embedded terminal shell",
    },
    SlashCommand {
        command: "/select",
        description: "Select files by mask",
    },
    SlashCommand {
        command: "/size",
        description: "Calculate directory size",
    },
    SlashCommand {
        command: "/sort",
        description: "Sort by name|ext|size|date",
    },
    SlashCommand {
        command: "/ssh",
        description: "Browse remote via SSH",
    },
    SlashCommand {
        command: "/stats",
        description: "Show file statistics",
    },
    SlashCommand {
        command: "/swap",
        description: "Swap left & right panels",
    },
    SlashCommand {
        command: "/symlink",
        description: "Create symbolic link",
    },
    SlashCommand {
        command: "/terminal",
        description: "Open terminal here",
    },
    SlashCommand {
        command: "/touch",
        description: "Create empty file",
    },
    SlashCommand {
        command: "/treemap",
        description: "Show disk usage treemap",
    },
    SlashCommand {
        command: "/undo",
        description: "Undo last file operation",
    },
    SlashCommand {
        command: "/update",
        description: "Check for and install a new farx release",
    },
    SlashCommand {
        command: "/yank",
        description: "Copy path to clipboard",
    },
];
