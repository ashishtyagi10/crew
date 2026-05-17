#[derive(Debug, Clone, PartialEq)]
pub enum AiBarAction {
    None,
    Close,
    Submit(String),
}

pub struct AiBarState {
    pub active: bool,
    pub input: String,
    pub cursor_pos: usize,
    pub response: Vec<String>,
    pub thinking: bool,
    pub scroll_offset: usize,
    pub copied: bool,
}

impl Default for AiBarState {
    fn default() -> Self {
        Self {
            active: true,
            input: String::new(),
            cursor_pos: 0,
            response: Vec::new(),
            thinking: false,
            scroll_offset: 0,
            copied: false,
        }
    }
}

impl AiBarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_response(&mut self, text: String) {
        self.response = text.lines().map(String::from).collect();
        self.thinking = false;
        self.scroll_offset = 0;
    }

    pub fn append_response(&mut self, text: &str) {
        if self.response.is_empty() {
            self.response.push(String::new());
        }
        // Append text, handling newlines
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.response.push(String::new());
            }
            if let Some(last) = self.response.last_mut() {
                last.push_str(part);
            }
        }
    }
}
