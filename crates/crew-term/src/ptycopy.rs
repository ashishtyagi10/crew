//! The pane's copy-output hook, split from `ptyview` for the 200-line cap.
use super::*;

impl PtyTerm {
    /// What the last finished command (or, scrolled back, the one on view)
    /// printed, by the shell's own marks (see [`crate::CommandOutput`]).
    pub fn command_output(&self) -> Result<crate::CommandOutput, crate::OutputErr> {
        self.core.command_output()
    }
}
