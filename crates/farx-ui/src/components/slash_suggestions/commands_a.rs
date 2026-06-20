use super::command::SlashCommand;

/// First half of the slash command catalog (sorted, A through I).
pub const PART_A: &[SlashCommand] = &[
    SlashCommand {
        command: "/actions",
        description: "Quick actions for selected file",
    },
    SlashCommand {
        command: "/ai",
        description: "Open AI assistant bar",
    },
    SlashCommand {
        command: "/ai-tools",
        description: "AI coding tools panel",
    },
    SlashCommand {
        command: "/agents",
        description: "List running agent tiles",
    },
    SlashCommand {
        command: "/focus",
        description: "Focus an agent tile by number",
    },
    SlashCommand {
        command: "/back",
        description: "Navigate to previous directory",
    },
    SlashCommand {
        command: "/bookmark",
        description: "Show / add bookmarks",
    },
    SlashCommand {
        command: "/cd",
        description: "Change directory",
    },
    SlashCommand {
        command: "/checksum",
        description: "Show SHA-256 checksums",
    },
    SlashCommand {
        command: "/chmod",
        description: "Change file permissions",
    },
    SlashCommand {
        command: "/claude",
        description: "Launch Claude Code",
    },
    SlashCommand {
        command: "/codex",
        description: "Launch OpenAI Codex",
    },
    SlashCommand {
        command: "/compare",
        description: "Compare left & right directories",
    },
    SlashCommand {
        command: "/compress",
        description: "Compress selected files",
    },
    SlashCommand {
        command: "/copilot",
        description: "Launch GitHub Copilot",
    },
    SlashCommand {
        command: "/deselect",
        description: "Deselect files by mask",
    },
    SlashCommand {
        command: "/diff",
        description: "Compare files from both panels",
    },
    SlashCommand {
        command: "/duplicates",
        description: "Find duplicate files",
    },
    SlashCommand {
        command: "/exit",
        description: "Exit Farx",
    },
    SlashCommand {
        command: "/extract",
        description: "Extract archive",
    },
    SlashCommand {
        command: "/filter",
        description: "Filter file listing by pattern",
    },
    SlashCommand {
        command: "/find-file",
        description: "Fuzzy file finder",
    },
    SlashCommand {
        command: "/forward",
        description: "Navigate forward in history",
    },
    SlashCommand {
        command: "/grep",
        description: "Search inside files by content",
    },
    SlashCommand {
        command: "/gemini",
        description: "Launch Google Gemini",
    },
    SlashCommand {
        command: "/goto",
        description: "Navigate to a directory",
    },
    SlashCommand {
        command: "/help",
        description: "Show help screen",
    },
    SlashCommand {
        command: "/hidden",
        description: "Toggle hidden files",
    },
    SlashCommand {
        command: "/info",
        description: "Toggle file info panel",
    },
    SlashCommand {
        command: "/invert",
        description: "Invert file selection",
    },
];
