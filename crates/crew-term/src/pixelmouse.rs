//! DECSET 1016 (SGR-Pixels mouse, the xterm extension kitty, WezTerm, foot
//! and Ghostty speak): the program wants mouse reports in the 1006 form but
//! with PIXEL coordinates, for sub-cell UIs (image viewers, drag handles,
//! smooth sliders). alacritty ignores the mode, so, like 2031/2048 in
//! `schemenotify`, a small scanner over the raw PTY bytes tracks it:
//!
//! - `CSI ? 1016 h` / `l` set and reset it (also inside a run of modes)
//! - `CSI ? 1016 $ p` (DECRQM) answers `;1$y` set / `;2$y` reset
//! - RIS (`ESC c`) and DECSTR (`CSI ! p`) clear it
//!
//! The app reads the flag as `InputModes::pixel_mouse` and encodes reports
//! in pixels only while a mouse mode (1000/1002/1003) is also on.

/// Longest CSI body worth buffering; a run of modes in one DECSET fits.
const MAX_BODY: usize = 64;

#[derive(Default)]
enum State {
    #[default]
    Ground,
    Esc,
    /// Inside `ESC [`, holding the parameter and intermediate bytes so far.
    Csi,
}

/// Per-terminal 1016 state plus the scanner that maintains it. State persists
/// across [`PixelMouse::feed`] calls, so a sequence split across reads works.
#[derive(Default)]
pub(crate) struct PixelMouse {
    state: State,
    body: Vec<u8>,
    on: bool,
}

impl PixelMouse {
    /// Whether the program set DECSET 1016.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Scan a chunk of PTY output; returns the DECRPM reply owed, if any.
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> String {
        let mut reply = String::new();
        for &b in bytes {
            self.state = match (std::mem::take(&mut self.state), b) {
                (_, 0x1b) => State::Esc,
                (State::Esc, b'[') => {
                    self.body.clear();
                    State::Csi
                }
                (State::Esc, b'c') => {
                    self.on = false;
                    State::Ground
                }
                (State::Csi, 0x20..=0x3f) if self.body.len() < MAX_BODY => {
                    self.body.push(b);
                    State::Csi
                }
                (State::Csi, 0x40..=0x7e) => {
                    self.dispatch(b, &mut reply);
                    State::Ground
                }
                _ => State::Ground,
            };
        }
        reply
    }

    fn dispatch(&mut self, final_byte: u8, reply: &mut String) {
        match (final_byte, self.body.as_slice()) {
            (b'p', b"!") => self.on = false,
            (b'p', b"?1016$") => {
                let ps = if self.on { 1 } else { 2 };
                reply.push_str(&format!("\x1b[?1016;{ps}$y"));
            }
            (b'h' | b'l', [b'?', rest @ ..])
                if rest.split(|&c| c == b';').any(|p| p == b"1016") =>
            {
                self.on = final_byte == b'h';
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "pixelmouse_tests.rs"]
mod tests;
