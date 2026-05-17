use super::state::AiAgent;
use std::path::Path;

impl AiAgent {
    /// Build a context string from the current panel's files.
    pub fn build_files_context(entries: &[(String, bool, u64)]) -> String {
        let mut ctx = String::new();
        for (name, is_dir, size) in entries.iter().take(50) {
            if *is_dir {
                ctx.push_str(&format!("  [DIR] {}\n", name));
            } else {
                ctx.push_str(&format!("  {} ({} bytes)\n", name, size));
            }
        }
        if entries.len() > 50 {
            ctx.push_str(&format!("  ... and {} more entries\n", entries.len() - 50));
        }
        ctx
    }
}

pub(super) fn build_system_prompt(current_dir: &Path, files_context: &str) -> String {
    format!(
        "You are the AI assistant for Farx, a terminal file manager (FAR Manager clone). \
         Help the user manage files through natural language.\n\n\
         Current directory: {}\n\n\
         Files in current directory:\n{}\n\n\
         Provide concise, actionable responses. When suggesting file operations, \
         describe what commands or actions the user should take. \
         Format your response for a terminal display (keep lines under 80 chars).",
        current_dir.display(),
        files_context,
    )
}

pub(super) fn build_suggest_prompt(
    partial_input: &str,
    current_dir: &Path,
    files_context: &str,
) -> String {
    format!(
        "You are a command-line autocomplete engine for a terminal file manager.\n\
         Current directory: {}\n\
         Files:\n{}\n\
         The user has typed: \"{}\"\n\n\
         Respond with ONLY the completion text to append (not the full command). \
         If the input looks like a shell command, suggest the rest of the command. \
         If it looks like natural language, suggest the rest of the sentence. \
         If no good suggestion, respond with exactly: NONE\n\
         Keep it short (under 60 chars). No explanation, no quotes, just the completion text.",
        current_dir.display(),
        files_context,
        partial_input,
    )
}

pub(super) fn not_configured_message(env_name: &str) -> String {
    format!(
        "AI assistant is not configured.\n\n\
         To enable AI features, set your API key:\n\n\
         export {}=your-api-key-here\n\n\
         Default provider: OpenRouter (free models available)\n\
         Get a free key at: https://openrouter.ai/keys\n\n\
         Then restart farx.",
        env_name
    )
}
