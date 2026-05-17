/// AI coding tools that can be launched from Farx.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTool {
    ClaudeCode,
    Codex,
    GithubCopilot,
    Gemini,
    OpenCode,
}

impl AiTool {
    /// Human-readable label for the tool selector UI.
    pub fn label(self) -> &'static str {
        match self {
            AiTool::ClaudeCode => "Claude Code",
            AiTool::Codex => "Codex (OpenAI)",
            AiTool::GithubCopilot => "GitHub Copilot",
            AiTool::Gemini => "Gemini (Google)",
            AiTool::OpenCode => "OpenCode",
        }
    }

    /// Shell command to launch the tool.
    pub fn command(self) -> (&'static str, &'static [&'static str]) {
        match self {
            AiTool::ClaudeCode => ("claude", &[]),
            AiTool::Codex => ("codex", &[]),
            AiTool::GithubCopilot => ("gh", &["copilot"]),
            AiTool::Gemini => ("gemini", &[]),
            AiTool::OpenCode => ("opencode", &[]),
        }
    }

    /// All available AI tools.
    pub fn all() -> &'static [AiTool] {
        &[
            AiTool::ClaudeCode,
            AiTool::Codex,
            AiTool::GithubCopilot,
            AiTool::Gemini,
            AiTool::OpenCode,
        ]
    }

    /// Short description for the tool.
    pub fn description(self) -> &'static str {
        match self {
            AiTool::ClaudeCode => "Anthropic's AI coding assistant",
            AiTool::Codex => "OpenAI's CLI coding agent",
            AiTool::GithubCopilot => "GitHub's AI pair programmer",
            AiTool::Gemini => "Google's AI coding assistant",
            AiTool::OpenCode => "Open-source AI coding agent",
        }
    }
}
