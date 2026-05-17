/// A single slash command entry for the suggestion list.
#[derive(Debug, Clone, Copy)]
pub struct SlashCommand {
    /// Primary command (e.g. "/cd").
    pub command: &'static str,
    /// Short description shown next to the command.
    pub description: &'static str,
}
