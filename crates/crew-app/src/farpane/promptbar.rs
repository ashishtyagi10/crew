//! The F7 make-folder prompt, which takes the function-key row over while it
//! is open. Split from `bars.rs` (child module of `render.rs`).
use super::*;

/// The bottom-row text prompt (F7 make-folder), replacing the function bar.
///
/// Its caret shows only on a focused pane, like the command line's: a pane
/// you are not typing into does not claim to be listening.
pub(super) fn prompt_bar(
    buf: &mut Buffer,
    area: Rect,
    prompt: &super::super::Prompt,
    focused: bool,
) {
    let t = crew_theme::theme();
    let bar_bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let bar_fg = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let label = match prompt.kind {
        super::super::PromptKind::MkDir => "Create folder: ",
    };
    let mut line = prompt_text(label, &prompt.input, area.width as usize);
    if !focused {
        line.pop(); // the caret `prompt_text` ends every row with
    }
    Paragraph::new(Line::from(Span::styled(
        line,
        Style::new().fg(bar_fg).bg(bar_bg),
    )))
    .style(Style::new().bg(bar_bg))
    .render(area, buf);
}

/// The prompt's row: the label, then the input with its caret. A name longer
/// than the row keeps its END — the part being typed — behind a `…`, so the
/// caret never leaves the screen. It did: on a 2×2 tile a folder name past
/// forty characters was typed blind.
pub(super) fn prompt_text(label: &str, input: &str, width: usize) -> String {
    use crate::chatwidth::{char_w, str_w};
    const CARET: char = '\u{258f}';
    let room = width.saturating_sub(str_w(label) + 1);
    if str_w(input) <= room {
        return format!("{label}{input}{CARET}");
    }
    // Everything after `…` that fits, taken from the end.
    let mut w = 0;
    let mut tail: Vec<char> = Vec::new();
    for c in input.chars().rev() {
        if w + char_w(c) > room.saturating_sub(1) {
            break;
        }
        w += char_w(c);
        tail.push(c);
    }
    let tail: String = tail.into_iter().rev().collect();
    format!("{label}\u{2026}{tail}{CARET}")
}
