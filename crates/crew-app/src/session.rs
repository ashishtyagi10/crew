use crew_term::{GridSize, RenderCell};
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

#[cfg(test)]
use crate::layout::Rect;

/// Return the index of the first rect that contains physical pixel `(x, y)`.
#[cfg(test)]
pub fn pane_at(rects: &[Rect], x: f32, y: f32) -> Option<usize> {
    rects
        .iter()
        .position(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
}

/// Compute the terminal grid size that fits in `width x height` pixels given
/// the font cell dimensions.  Each dimension is clamped to a minimum of 1.
pub fn grid_for(width: u32, height: u32, cell_w: f32, cell_h: f32) -> GridSize {
    let cols = ((width as f32 / cell_w).floor() as u16).max(1);
    let rows = ((height as f32 / cell_h).floor() as u16).max(1);
    GridSize { cols, rows }
}

/// Map a winit key press event to the bytes that should be sent to the PTY.
/// `ctrl`/`shift` are the live modifier states (Ctrl+letter control codes and
/// Shift+Tab "backtab").
pub fn key_to_bytes(event: &KeyEvent, ctrl: bool, shift: bool) -> Option<Vec<u8>> {
    if !event.state.is_pressed() {
        return None;
    }
    if let Key::Named(n) = &event.logical_key {
        return named_bytes_shift(*n, shift);
    }
    if let Key::Character(s) = &event.logical_key {
        // Ctrl+<letter/@-_> → the ASCII control code (Ctrl+C = 0x03, etc.).
        if ctrl {
            if let Some(b) = s.chars().next().and_then(ctrl_byte) {
                return Some(vec![b]);
            }
        }
        return Some(s.as_bytes().to_vec());
    }
    None
}

/// Named-key bytes honouring Shift: Shift+Tab is backtab (CSI Z), and Shift+Enter
/// is a line feed (0x0a) rather than carriage-return (0x0d) — the de-facto
/// terminal convention for a soft return, so agent CLIs (Claude/codex) and
/// editors insert a newline instead of submitting. Otherwise the plain mapping.
fn named_bytes_shift(n: NamedKey, shift: bool) -> Option<Vec<u8>> {
    if shift {
        match n {
            NamedKey::Tab => return Some(b"\x1b[Z".to_vec()),
            NamedKey::Enter => return Some(b"\n".to_vec()),
            _ => {}
        }
    }
    named_bytes(n)
}

/// Bytes for a named key: control chars and xterm escape sequences for the
/// navigation/editing keys so TUI programs (editors, the Claude CLI, …) work.
fn named_bytes(n: NamedKey) -> Option<Vec<u8>> {
    let bytes: &[u8] = match n {
        NamedKey::Enter => b"\r",
        NamedKey::Backspace => &[0x7f],
        NamedKey::Tab => b"\t",
        NamedKey::Escape => &[0x1b],
        NamedKey::Space => b" ",
        NamedKey::ArrowUp => b"\x1b[A",
        NamedKey::ArrowDown => b"\x1b[B",
        NamedKey::ArrowRight => b"\x1b[C",
        NamedKey::ArrowLeft => b"\x1b[D",
        NamedKey::Home => b"\x1b[H",
        NamedKey::End => b"\x1b[F",
        NamedKey::PageUp => b"\x1b[5~",
        NamedKey::PageDown => b"\x1b[6~",
        NamedKey::Delete => b"\x1b[3~",
        NamedKey::Insert => b"\x1b[2~",
        _ => return None,
    };
    Some(bytes.to_vec())
}

/// The ASCII control byte for a Ctrl+`c` chord (`Ctrl+C` → 0x03), or `None` if
/// `c` has no control mapping.
fn ctrl_byte(c: char) -> Option<u8> {
    let up = c.to_ascii_uppercase();
    (up.is_ascii() && ('@'..='_').contains(&up)).then_some((up as u8) & 0x1f)
}

/// Prepare clipboard `text` for writing to a PTY: normalize newlines to `\r`,
/// and wrap in bracketed-paste markers when the program enabled that mode (so a
/// multi-line paste isn't executed line-by-line).
pub fn wrap_paste(text: &str, bracketed: bool) -> Vec<u8> {
    let body = text.replace("\r\n", "\r").replace('\n', "\r");
    if bracketed {
        let mut out = b"\x1b[200~".to_vec();
        out.extend_from_slice(body.as_bytes());
        out.extend_from_slice(b"\x1b[201~");
        out
    } else {
        body.into_bytes()
    }
}

/// Map `crew_term::RenderCell` slices to `crew_render::CellView` — field-for-field.
pub fn to_cellviews(cells: &[RenderCell]) -> Vec<crew_render::CellView> {
    cells
        .iter()
        .map(|c| crew_render::CellView {
            col: c.col,
            row: c.row,
            c: c.c,
            fg: c.fg,
            bg: c.bg,
            bold: c.bold,
            italic: c.italic,
            deco: c.deco,
            cursor: c.cursor,
        })
        .collect()
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
