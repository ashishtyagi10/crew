use crew_render::CellView;

type Color = (u8, u8, u8);
type ColoredLine = Vec<(char, Color)>;

pub(crate) const READY_HINT: &str = "Type a message and press Enter to talk to the agent.";
pub(crate) const CONNECTING_HINT: &str = "Connecting to the agent…";

pub struct Message {
    pub sender: String,
    pub text: String,
    /// Unix-epoch milliseconds when the message was produced ("" = unknown).
    pub ts: String,
    /// Per-message metadata from the plugin (e.g. reply latency, `"4.2s"`).
    pub meta: String,
}

/// Word-aware wrap of `full` to width `cols`: the `[start, end)` char ranges of
/// each line. Words longer than `cols` are hard-broken; the single space at a
/// wrap point is dropped. Always returns at least one (possibly empty) line.
pub(crate) fn wrap_indices(full: &[char], cols: usize) -> Vec<(usize, usize)> {
    if cols == 0 || full.is_empty() {
        return vec![(0, full.len())];
    }
    let n = full.len();
    let mut lines = Vec::new();
    let mut start = 0;
    while start < n {
        // Width-aware: a wide glyph counts two columns (see `chatwidth`).
        let max_end = crate::chatwidth::fit_end(full, start, cols);
        if max_end == n {
            lines.push((start, n));
            break;
        }
        // Line is full; prefer breaking at the last space within it.
        match full[start..max_end].iter().rposition(|&c| c == ' ') {
            Some(p) if p > 0 => {
                lines.push((start, start + p));
                start += p + 1; // skip the break space
            }
            _ => {
                lines.push((start, max_end)); // a too-long word: hard break
                start = max_end;
            }
        }
    }
    lines
}

/// Total number of wrapped message lines for the given width.
pub fn wrapped_line_count(messages: &[Message], cols: u16) -> usize {
    if cols == 0 {
        return 0;
    }
    messages
        .iter()
        .map(|m| {
            let full: Vec<char> = format!("{}: {}", m.sender, m.text).chars().collect();
            wrap_indices(&full, cols as usize).len()
        })
        .sum()
}

/// Render messages + input prompt as CellView cells.
///
/// - Rows `0..rows-1`: most recent messages, top-down, wrapped to `cols`.
///   Sender chars in ACCENT_FG, rest in TEXT_FG.
/// - Row `rows-1`: `"> " + input` in INPUT_FG.
/// - All cells use DEFAULT_BG.
pub fn layout_cells(
    messages: &[Message],
    input: &str,
    cols: u16,
    rows: u16,
    scroll: usize,
    connected: bool,
) -> Vec<CellView> {
    if rows == 0 || cols == 0 {
        return Vec::new();
    }
    let mut cells: Vec<CellView> = Vec::new();

    // Bottom row: input bar
    let input_row = rows - 1;
    for (i, c) in format!("> {}", input)
        .chars()
        .take(cols as usize)
        .enumerate()
    {
        cells.push(CellView {
            col: i as u16,
            row: input_row,
            c,
            fg: crew_theme::theme().ink,
            bg: crew_theme::theme().page_bg,
            bold: false,
            italic: false,
        });
    }

    let msg_rows = rows - 1;
    if msg_rows == 0 {
        return cells;
    }

    // A fresh agent pane (no messages) shows a dim hint: how to start once the
    // agent is connected, or that it's still connecting.
    if messages.is_empty() {
        let hint = if connected {
            READY_HINT
        } else {
            CONNECTING_HINT
        };
        for (i, c) in hint.chars().take(cols as usize).enumerate() {
            cells.push(CellView {
                col: i as u16,
                row: 0,
                c,
                fg: crew_theme::theme().hint_fg,
                bg: crew_theme::theme().page_bg,
                bold: false,
                italic: false,
            });
        }
        return cells;
    }

    // Build word-wrapped, coloured lines from messages.
    let mut all_lines: Vec<ColoredLine> = Vec::new();
    for msg in messages {
        let prefix_len = format!("{}: ", msg.sender).chars().count();
        let full: Vec<char> = format!("{}: {}", msg.sender, msg.text).chars().collect();
        for (s, e) in wrap_indices(&full, cols as usize) {
            let line = full[s..e]
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let fg = if s + i < prefix_len {
                        crate::palette::accent()
                    } else {
                        crew_theme::theme().text_muted
                    };
                    (c, fg)
                })
                .collect();
            all_lines.push(line);
        }
    }

    // Show a msg_rows-tall window, `scroll` lines up from the bottom.
    let max_start = all_lines.len().saturating_sub(msg_rows as usize);
    let start = max_start.saturating_sub(scroll);
    let end = (start + msg_rows as usize).min(all_lines.len());
    for (row_offset, line) in all_lines[start..end].iter().enumerate() {
        let row = row_offset as u16;
        let mut col: u16 = 0;
        for &(c, fg) in line.iter() {
            let w = crate::chatwidth::char_w(c) as u16;
            if w == 0 {
                continue; // zero-width marks don't get their own cell
            }
            if col + w > cols {
                break;
            }
            cells.push(CellView {
                col,
                row,
                c,
                fg,
                bg: crew_theme::theme().page_bg,
                bold: false,
                italic: false,
            });
            col += w;
        }
    }
    cells
}

#[cfg(test)]
#[path = "chatlayout_tests.rs"]
mod tests;
