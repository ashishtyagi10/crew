//! DECRQSS, `DCS $ q <setting> ST`: a program asks what a setting is NOW.
//!
//! nvim 0.10 decides on 24-bit colour this way: it sets a background of
//! `48;2;r;g;b` and asks for `m` (SGR) in the same write, and turns
//! 'termguicolors' on only when the reply ends with those three numbers —
//! the one check that survives ssh, sudo and tmux, where `$COLORTERM` does
//! not. vim asks for ` q` (DECSCUSR) at start-up to learn the cursor's shape
//! and blink. vte has no arm for this DCS, so it went unanswered: nvim stayed
//! on 256 colours and vim on its guess.
//!
//! Answered from the terminal's state AT the request, not at the end of the
//! read: the bytes before it reach the parser first, so a set-then-ask in
//! one write is told what was just set. Replies are xterm's: `DCS 1 $ r
//! <value> ST` for a setting crew knows, `DCS 0 $ r ST` for one it does not —
//! which includes `r` (DECSTBM): alacritty keeps the scroll margins private,
//! and a wrong answer is worse than none, since the asker falls back as for
//! any terminal that does not know the setting.
use super::TermCore;

/// The longest setting kept. Real ones are one to three bytes.
const MAX_BODY: usize = 16;
const ESC: u8 = 0x1b;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    #[default]
    Ground,
    Esc,
    /// `ESC P`: the next byte must be `$`.
    Dcs,
    /// `ESC P $`: the next byte must be `q`.
    Dollar,
    Body,
    /// ESC inside the body: `\` ends the request, anything else abandons it.
    BodyEsc,
}

/// Watches the raw pty bytes for DECRQSS; a request split across reads still
/// answers.
#[derive(Default)]
pub(crate) struct Rqss {
    state: State,
    body: Vec<u8>,
}

impl Rqss {
    /// Step one byte; the setting asked for when it completes a request.
    fn step(&mut self, b: u8) -> Option<Vec<u8>> {
        use State::*;
        self.state = match (self.state, b) {
            (BodyEsc, b'\\') => {
                self.state = Ground;
                return Some(std::mem::take(&mut self.body));
            }
            // CAN and SUB cancel any sequence, as vte treats them.
            (Body, 0x18 | 0x1a) => Ground,
            (Body, ESC) => BodyEsc,
            (Body, _) if self.body.len() < MAX_BODY => {
                self.body.push(b);
                Body
            }
            (Esc | BodyEsc, b'P') => Dcs,
            (Dcs, b'$') => Dollar,
            (Dollar, b'q') => Body,
            (_, ESC) => Esc,
            _ => Ground,
        };
        if !matches!(self.state, Body | BodyEsc) {
            self.body.clear();
        }
        None
    }
}

impl TermCore {
    /// One piece of a chunk, cut at each DECRQSS so the bytes before a
    /// request reach the parser before it is answered.
    pub(super) fn feed_part(&mut self, bytes: &[u8]) {
        let mut from = 0;
        for (i, &b) in bytes.iter().enumerate() {
            if let Some(setting) = self.rqss.step(b) {
                self.feed_piece(&bytes[from..=i]);
                from = i + 1;
                let reply = reply(&setting, self);
                self.events.replies.lock().unwrap().push_str(&reply);
            }
        }
        if from < bytes.len() {
            self.feed_piece(&bytes[from..]);
        }
    }
}

/// The reply to one DECRQSS for `setting`.
fn reply(setting: &[u8], core: &TermCore) -> String {
    let value = match setting {
        b" q" => Some(format!("{} q", decscusr(core.term.cursor_style()))),
        b"m" => Some(format!(
            "{}m",
            super::modelrqsssgr::sgr(&core.term.grid().cursor.template)
        )),
        _ => None,
    };
    match value {
        Some(v) => format!("\x1bP1$r{v}\x1b\\"),
        None => "\x1bP0$r\x1b\\".to_string(),
    }
}

/// DECSCUSR's number for a style: 1/2 block, 3/4 underline, 5/6 bar, odd
/// blinking. The hollow and hidden shapes are not DECSCUSR's to set; they
/// report as the block they stand in for.
fn decscusr(s: alacritty_terminal::vte::ansi::CursorStyle) -> u8 {
    use alacritty_terminal::vte::ansi::CursorShape::*;
    let steady = match s.shape {
        Underline => 4,
        Beam => 6,
        Block | HollowBlock | Hidden => 2,
    };
    steady - u8::from(s.blinking)
}

#[cfg(test)]
#[path = "modelrqss_tests.rs"]
mod tests;
