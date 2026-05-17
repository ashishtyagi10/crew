use crossterm::event::KeyEvent;

/// The type of dialog currently shown
#[derive(Debug, Clone)]
pub enum DialogKind {
    /// Input dialog for MkDir, Rename, etc.
    Input {
        title: String,
        prompt: String,
        input: String,
        cursor_pos: usize,
    },
    /// Confirmation dialog for Copy, Move, Delete
    Confirm {
        title: String,
        message: String,
        details: Vec<String>,
    },
    /// Message/alert dialog
    Message { title: String, message: String },
    /// Error dialog
    Error { title: String, message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogResult {
    /// User confirmed (Enter) - for Input dialogs, contains the input string
    Confirm(Option<String>),
    /// User cancelled (Escape)
    Cancel,
    /// Dialog is still open
    Pending,
}

pub struct DialogState {
    pub kind: DialogKind,
    pub result: DialogResult,
}

impl DialogState {
    pub fn new_input(
        title: impl Into<String>,
        prompt: impl Into<String>,
        default: impl Into<String>,
    ) -> Self {
        let input = default.into();
        let cursor_pos = input.len();
        Self {
            kind: DialogKind::Input {
                title: title.into(),
                prompt: prompt.into(),
                input,
                cursor_pos,
            },
            result: DialogResult::Pending,
        }
    }

    pub fn new_confirm(
        title: impl Into<String>,
        message: impl Into<String>,
        details: Vec<String>,
    ) -> Self {
        Self {
            kind: DialogKind::Confirm {
                title: title.into(),
                message: message.into(),
                details,
            },
            result: DialogResult::Pending,
        }
    }

    pub fn new_message(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: DialogKind::Message {
                title: title.into(),
                message: message.into(),
            },
            result: DialogResult::Pending,
        }
    }

    pub fn new_error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: DialogKind::Error {
                title: title.into(),
                message: message.into(),
            },
            result: DialogResult::Pending,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        super::keys::handle_key_event(self, key);
    }

    pub fn is_resolved(&self) -> bool {
        self.result != DialogResult::Pending
    }
}
