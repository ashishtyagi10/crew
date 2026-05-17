//! Command line state struct definition and constructor.

/// State for the command line input at the bottom of the screen.
pub struct CommandLineState {
    /// The current input text.
    pub input: String,
    /// Cursor position within the input (byte offset).
    pub cursor_pos: usize,
    /// Whether the command line is currently accepting input.
    pub visible: bool,
    /// Command history (oldest first).
    pub history: Vec<String>,
    /// Current position in history when browsing (None = not browsing).
    pub(super) history_index: Option<usize>,
    /// Saved input before history browsing started.
    pub(super) saved_input: String,
    /// LLM typeahead suggestion (ghost text shown after cursor).
    pub suggestion: Option<String>,
    /// Snapshot of input when suggestion was requested (to discard stale ones).
    pub suggestion_for: String,
    /// Whether a suggestion request is in-flight.
    pub suggestion_pending: bool,
    /// Tick counter for debounce (request after N ticks of no typing).
    pub last_typed_tick: u64,
}

impl CommandLineState {
    /// Create a new empty command line state.
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            visible: true,
            history: Vec::new(),
            history_index: None,
            saved_input: String::new(),
            suggestion: None,
            suggestion_for: String::new(),
            suggestion_pending: false,
            last_typed_tick: 0,
        }
    }
}

impl Default for CommandLineState {
    fn default() -> Self {
        Self::new()
    }
}
