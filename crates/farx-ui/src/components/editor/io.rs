use super::{EditorMode, EditorState, MAX_FILE_SIZE};
use std::path::Path;

impl EditorState {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let meta = std::fs::metadata(path)?;
            if meta.len() > MAX_FILE_SIZE {
                anyhow::bail!(
                    "File too large ({:.1} MB). Max: 100 MB",
                    meta.len() as f64 / 1_048_576.0
                );
            }
        }
        let contents = if path.exists() {
            std::fs::read_to_string(path)?
        } else {
            String::new()
        };
        let lines: Vec<String> = if contents.is_empty() {
            vec![String::new()]
        } else {
            contents.lines().map(String::from).collect()
        };

        Ok(Self {
            file_path: path.to_path_buf(),
            lines,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            horizontal_scroll: 0,
            modified: false,
            active: true,
            wrap: true,
            preview_mode: false,
            preview_lines: Vec::new(),
            preview_scroll: 0,
            mode: EditorMode::Normal,
            search_query: String::new(),
            search_cursor: 0,
            goto_line_input: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_save_undo_len: 0,
        })
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let content = self.lines.join("\n");
        // Add trailing newline if file had content
        let content = if content.is_empty() {
            content
        } else {
            content + "\n"
        };
        std::fs::write(&self.file_path, &content)?;
        self.modified = false;
        self.last_save_undo_len = self.undo_stack.len();
        Ok(())
    }
}
