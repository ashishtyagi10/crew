use crew_render::CellView;

type Color = (u8, u8, u8);
type ColoredLine = Vec<(char, Color)>;

pub const DEFAULT_BG: (u8, u8, u8) = (8, 8, 16);
pub const ACCENT_FG: (u8, u8, u8) = (0, 255, 160);
pub const TEXT_FG: (u8, u8, u8) = (200, 200, 200);
pub const INPUT_FG: (u8, u8, u8) = (220, 220, 220);

pub struct Message {
    pub sender: String,
    pub text: String,
}

/// Total number of wrapped message lines for the given width.
pub fn wrapped_line_count(messages: &[Message], cols: u16) -> usize {
    if cols == 0 {
        return 0;
    }
    messages
        .iter()
        .map(|m| {
            let len = format!("{}: {}", m.sender, m.text).chars().count();
            len.div_ceil(cols as usize).max(1)
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
            fg: INPUT_FG,
            bg: DEFAULT_BG,
            bold: false,
            italic: false,
        });
    }

    let msg_rows = rows - 1;
    if msg_rows == 0 {
        return cells;
    }

    // Build wrapped lines from messages
    let mut all_lines: Vec<ColoredLine> = Vec::new();
    for msg in messages {
        let prefix = format!("{}: ", msg.sender);
        let prefix_len = prefix.chars().count();
        let full: Vec<char> = format!("{}{}", prefix, msg.text).chars().collect();
        let total = full.len();
        if total == 0 {
            all_lines.push(Vec::new());
            continue;
        }
        let mut pos = 0usize;
        while pos < total {
            let end = (pos + cols as usize).min(total);
            let line = full[pos..end]
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    (
                        c,
                        if pos + i < prefix_len {
                            ACCENT_FG
                        } else {
                            TEXT_FG
                        },
                    )
                })
                .collect();
            all_lines.push(line);
            pos = end;
        }
    }

    // Show a msg_rows-tall window, `scroll` lines up from the bottom.
    let max_start = all_lines.len().saturating_sub(msg_rows as usize);
    let start = max_start.saturating_sub(scroll);
    let end = (start + msg_rows as usize).min(all_lines.len());
    for (row_offset, line) in all_lines[start..end].iter().enumerate() {
        let row = row_offset as u16;
        for (col, &(c, fg)) in line.iter().enumerate() {
            cells.push(CellView {
                col: col as u16,
                row,
                c,
                fg,
                bg: DEFAULT_BG,
                bold: false,
                italic: false,
            });
        }
    }
    cells
}

/// Pure input reducer.
///
/// - `enter`: return `Some(old_input)`, clear `input`.
/// - `backspace`: pop last char, return `None`.
/// - `ch=Some(c)` (non-control): push `c`, return `None`.
pub fn input_reduce(
    input: &mut String,
    ch: Option<char>,
    enter: bool,
    backspace: bool,
) -> Option<String> {
    if enter {
        Some(std::mem::take(input))
    } else if backspace {
        input.pop();
        None
    } else if let Some(c) = ch {
        if !c.is_control() {
            input.push(c);
        }
        None
    } else {
        None
    }
}

#[cfg(test)]
#[path = "chatlayout_tests.rs"]
mod tests;
