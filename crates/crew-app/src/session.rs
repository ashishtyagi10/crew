use crew_term::{GridSize, RenderCell};
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::layout::Rect;

/// Return the index of the first rect that contains physical pixel `(x, y)`.
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
        // Shift+Tab is backtab (CSI Z) — used by the Claude CLI and others.
        if *n == NamedKey::Tab && shift {
            return Some(b"\x1b[Z".to_vec());
        }
        return named_bytes(*n);
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
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::pane_rects_at;

    #[test]
    fn pane_at_two_panes() {
        // 2 panes side-by-side in 800x600 with no gap → left pane [0,400) right [400,800)
        let rects = pane_rects_at(2, 0.0, 0.0, 800.0, 600.0, 0.0);
        assert_eq!(pane_at(&rects, 10.0, 10.0), Some(0));
        assert_eq!(pane_at(&rects, 410.0, 10.0), Some(1));
        assert_eq!(pane_at(&rects, 800.0, 10.0), None);
    }

    #[test]
    fn grid_for_basic() {
        let g = grid_for(800, 600, 10.0, 20.0);
        assert_eq!(g.cols, 80);
        assert_eq!(g.rows, 30);
    }

    #[test]
    fn grid_for_clamps_to_one() {
        let g = grid_for(0, 0, 10.0, 20.0);
        assert_eq!(g.cols, 1);
        assert_eq!(g.rows, 1);
    }

    #[test]
    fn grid_for_floors_partial_cells() {
        // 805 / 10 = 80.5 → floor → 80
        let g = grid_for(805, 601, 10.0, 20.0);
        assert_eq!(g.cols, 80);
        assert_eq!(g.rows, 30);
    }

    #[test]
    fn arrow_keys_map_to_escape_sequences() {
        assert_eq!(named_bytes(NamedKey::ArrowUp).unwrap(), b"\x1b[A");
        assert_eq!(named_bytes(NamedKey::ArrowDown).unwrap(), b"\x1b[B");
        assert_eq!(named_bytes(NamedKey::ArrowRight).unwrap(), b"\x1b[C");
        assert_eq!(named_bytes(NamedKey::ArrowLeft).unwrap(), b"\x1b[D");
    }

    #[test]
    fn nav_and_edit_keys_mapped() {
        assert_eq!(named_bytes(NamedKey::PageUp).unwrap(), b"\x1b[5~");
        assert_eq!(named_bytes(NamedKey::Delete).unwrap(), b"\x1b[3~");
        assert_eq!(named_bytes(NamedKey::Home).unwrap(), b"\x1b[H");
    }

    #[test]
    fn wrap_paste_normalizes_and_brackets() {
        assert_eq!(wrap_paste("ab", false), b"ab");
        assert_eq!(wrap_paste("a\r\nb\nc", false), b"a\rb\rc");
        let w = wrap_paste("x", true);
        assert!(w.starts_with(b"\x1b[200~") && w.ends_with(b"\x1b[201~"));
    }

    #[test]
    fn ctrl_letters_become_control_codes() {
        assert_eq!(ctrl_byte('c'), Some(0x03));
        assert_eq!(ctrl_byte('C'), Some(0x03));
        assert_eq!(ctrl_byte('a'), Some(0x01));
        assert_eq!(ctrl_byte('1'), None);
    }
}
