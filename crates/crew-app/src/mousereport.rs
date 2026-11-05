//! Mouse clicks for programs that asked for them (xterm DECSET 1000/1002/1003,
//! SGR-encoded under 1006). Before this only the wheel was ever reported
//! (`altscroll`), so a click in htop, mc, lazygit, tmux or vim `mouse=a` started
//! crew's own text selection instead of reaching the program. Shift+click keeps
//! crew's selection, the xterm convention for "the terminal, not the app".
use crew_term::InputModes;
use winit::event::MouseButton;

/// The xterm button number for a press: 0 left, 1 middle, 2 right. Back and
/// forward have no encoding programs agree on, so they are not reported.
pub(crate) fn button_code(b: MouseButton) -> Option<u8> {
    match b {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    }
}

/// The modifier bits xterm adds to the button number: Alt (Meta) 8, Ctrl 16.
/// Shift (4) is never sent: a shifted click is crew's, not the program's.
pub(crate) fn mod_bits(alt: bool, ctrl: bool) -> u8 {
    (if alt { 8 } else { 0 }) | (if ctrl { 16 } else { 0 })
}

/// Whether a click goes to the program: it enabled any mouse mode, and the
/// user is not holding Shift to select on crew's side.
pub(crate) fn reports_click(m: &InputModes, shift: bool) -> bool {
    m.mouse && !shift
}

/// One press or release report for button `code` (with modifier bits) at the
/// 0-based viewport `cell`. SGR keeps the button on release and marks it with
/// `m`; legacy X10 can only say "released" (button 3) and offsets each field
/// by 32 into one byte, so columns and rows past 223 clamp to 223.
pub(crate) fn report(code: u8, cell: (u16, u16), sgr: bool, press: bool) -> Vec<u8> {
    let x = u32::from(cell.0) + 1;
    let y = u32::from(cell.1) + 1;
    if sgr {
        let end = if press { 'M' } else { 'm' };
        return format!("\x1b[<{code};{x};{y}{end}").into_bytes();
    }
    let btn = if press { code } else { (code & !3) | 3 };
    let enc = |v: u32| (v + 32).min(255) as u8;
    vec![0x1b, b'[', b'M', enc(btn.into()), enc(x), enc(y)]
}

#[cfg(test)]
#[path = "mousereport_tests.rs"]
mod tests;
