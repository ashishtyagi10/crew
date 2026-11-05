//! Mouse clicks for programs that asked for them (xterm DECSET 1000/1002/1003,
//! SGR-encoded under 1006). Before this only the wheel was ever reported
//! (`altscroll`), so a click in htop, mc, lazygit, tmux or vim `mouse=a` started
//! crew's own text selection instead of reaching the program. Shift+click keeps
//! crew's selection, the xterm convention for "the terminal, not the app".
//! Moves are reported too where the program asked (1002 with a button held,
//! 1003 always), so tmux splitters drag, vim's visual mode follows the mouse
//! and btop lights what the pointer hovers.
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

/// The modifier bits a wheel report carries: Shift 4 and Alt 8. Unlike a
/// click, Shift+wheel has no crew meaning to keep, so the program gets it.
/// Ctrl (and Cmd) + wheel never gets here: it is crew's font zoom.
pub(crate) fn wheel_mods(m: winit::keyboard::ModifiersState) -> u8 {
    mod_bits(m.alt_key(), false) | if m.shift_key() { 4 } else { 0 }
}

/// The bit xterm adds to a button number when the report is a move.
const MOTION: u8 = 32;

/// The button code a pointer move reports, or `None` when the program did not
/// ask for this kind of move: with a button held (`held`, its press code) under
/// drag (1002) or any-motion (1003) mode; with none held only under 1003, as
/// button 3, "no button". `mods` are the modifier bits held now.
pub(crate) fn motion_code(m: &InputModes, held: Option<u8>, mods: u8) -> Option<u8> {
    match held {
        Some(code) if m.mouse_drag || m.mouse_motion => Some((code & 3) | mods | MOTION),
        None if m.mouse_motion => Some(3 | mods | MOTION),
        _ => None,
    }
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
