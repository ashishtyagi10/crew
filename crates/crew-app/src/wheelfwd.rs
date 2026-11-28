//! Forward a wheel scroll to a full-screen program (split from `scroll` for
//! the 200-line cap).
use crate::pane::{Pane, PaneContent};

/// Forward a wheel scroll to a full-screen program (alt-screen/mouse app) as
/// mouse or arrow-key bytes. `at` is the hovered 0-based cell and the 1-based
/// pixel position in the pane's text area: a program that set SGR-Pixels
/// (1016) gets the pixels. Returns true when forwarded — false means the pane
/// owns no full-screen program, so the caller should scroll local scrollback.
pub(crate) fn forward_wheel(
    pane: &mut Pane,
    lines: i32,
    at: ((u16, u16), Option<(u32, u32)>),
    mods: u8,
) -> bool {
    let PaneContent::Terminal(t) = &pane.content else {
        return false;
    };
    let m = t.pty.input_modes();
    let bytes = match at.1 {
        Some(px) if m.mouse && m.pixel_mouse => crate::altscroll::wheel_bytes_px(lines, px, mods),
        _ => crate::altscroll::wheel_bytes(&m, lines, at.0, mods),
    };
    let Some(bytes) = bytes else {
        return false;
    };
    crate::scroll::forward_to_program(pane, &bytes);
    true
}
