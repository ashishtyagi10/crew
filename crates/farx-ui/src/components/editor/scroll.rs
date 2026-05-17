use super::EditorState;

impl EditorState {
    /// How many visual lines a logical line occupies when wrapped.
    pub(super) fn visual_height_of_line(&self, line_idx: usize, text_width: usize) -> usize {
        if text_width == 0 {
            return 1;
        }
        let char_count = self
            .lines
            .get(line_idx)
            .map(|l| l.chars().count())
            .unwrap_or(0);
        if char_count == 0 {
            1
        } else {
            char_count.div_ceil(text_width)
        }
    }

    pub fn scroll_to_cursor(&mut self, visible_height: usize, visible_width: usize) {
        let gutter_width = 6usize;
        let text_width = visible_width.saturating_sub(gutter_width);

        if self.wrap && text_width > 0 {
            // Ensure cursor line is at or below scroll_offset
            if self.cursor_line < self.scroll_offset {
                self.scroll_offset = self.cursor_line;
            }

            // Compute visual rows from scroll_offset to cursor position
            let mut visual = 0usize;
            for i in self.scroll_offset..=self.cursor_line.min(self.lines.len().saturating_sub(1)) {
                if i == self.cursor_line {
                    // Add the cursor's row within this wrapped line
                    let char_col = self.lines[i][..self.cursor_col.min(self.lines[i].len())]
                        .chars()
                        .count();
                    visual += char_col / text_width.max(1);
                } else {
                    visual += self.visual_height_of_line(i, text_width);
                }
            }

            // Scroll down until cursor's visual row fits on screen
            while visual >= visible_height && self.scroll_offset < self.cursor_line {
                let removed = self.visual_height_of_line(self.scroll_offset, text_width);
                visual = visual.saturating_sub(removed);
                self.scroll_offset += 1;
            }
        } else {
            // Non-wrap mode: logical-line scrolling
            if self.cursor_line < self.scroll_offset {
                self.scroll_offset = self.cursor_line;
            }
            if self.cursor_line >= self.scroll_offset + visible_height {
                self.scroll_offset = self.cursor_line - visible_height + 1;
            }
            // Horizontal scroll
            if self.cursor_col < self.horizontal_scroll {
                self.horizontal_scroll = self.cursor_col;
            }
            if self.cursor_col >= self.horizontal_scroll + text_width {
                self.horizontal_scroll = self.cursor_col - text_width + 1;
            }
        }
    }

    pub(super) fn is_markdown_file(&self) -> bool {
        let ext = self.file_path.extension().and_then(|e| e.to_str());
        matches!(ext, Some("md" | "markdown" | "mdx"))
    }
}
