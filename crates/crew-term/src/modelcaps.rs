//! XTGETTCAP, `DCS + q <hex names> ST`: a program asks the terminal what it
//! can do, by terminfo name, instead of trusting `$TERM`'s entry.
//!
//! neovim asks for `Tc`, `RGB`, `setrgbf` and `setrgbb` whenever
//! `$COLORTERM` did not reach it (over ssh, under sudo, inside tmux) and turns
//! on 24-bit colour only if one comes back found; unanswered, it waits out a
//! one-second timeout and stays on 256 colours. tmux, kakoune and notcurses
//! ask the same way. vte has no arm for this DCS, so it went unanswered.
//!
//! One reply per name, `DCS 1 + r <name>=<value> ST` or `DCS 0 + r <name> ST`,
//! as kitty and foot send them. xterm instead answers every name in one reply
//! and stops at the first unknown one; neovim 0.10 reads only the first pair
//! of a reply, so the per-name form is the one every reader understands.

/// What crew answers, as terminfo SOURCE text (`\E` for ESC, `%p1%d` for a
/// parameter), hex-encoded in the reply; the reader decodes it. `None` marks a
/// boolean: found, with no value. Each is true of crew:
/// - `TN`: the `$TERM` every pane gets (`ptyspawn`).
/// - `Co`/`colors`: the 256-colour palette alacritty keeps.
/// - `RGB`/`Tc`/`setrgbf`/`setrgbb`: crew draws SGR `38;2`/`48;2` colours
///   exactly (`COLORTERM=truecolor` says the same). `8/8/8` is xterm's form of
///   `RGB`, read by ncurses as "direct colour, 8 bits a channel".
/// - `Ms`: OSC 52 writes reach the system clipboard (`modelclip`).
/// - `Ss`/`Se`: DECSCUSR shapes and blinks the cursor; `0` restores crew's own.
/// - `Smulx`: SGR `4:1`…`4:5` draw single, double, curly, dotted and dashed
///   underlines (`celldeco`).
const CAPS: &[(&str, Option<&str>)] = &[
    ("TN", Some("xterm-256color")),
    ("Co", Some("256")),
    ("colors", Some("256")),
    ("RGB", Some("8/8/8")),
    ("Tc", None),
    ("setrgbf", Some("\\E[38;2;%p1%d;%p2%d;%p3%dm")),
    ("setrgbb", Some("\\E[48;2;%p1%d;%p2%d;%p3%dm")),
    ("Ms", Some("\\E]52;%p1%s;%p2%s\\007")),
    ("Ss", Some("\\E[%p1%d q")),
    ("Se", Some("\\E[0 q")),
    ("Smulx", Some("\\E[4:%p1%dm")),
];

/// The longest name list kept. Real requests are a few dozen bytes; past this
/// it is not a query (a `cat`ed binary), and dropping it bounds the memory.
const MAX_BODY: usize = 1024;

const ESC: u8 = 0x1b;

/// Where the scanner is in `ESC P + q <names> ESC \`.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    #[default]
    Ground,
    Esc,
    /// `ESC P`: the next byte must be `+`.
    Dcs,
    /// `ESC P +`: the next byte must be `q`.
    Plus,
    /// Collecting names until ESC.
    Body,
    /// ESC inside the body: `\` ends the request, anything else abandons it.
    BodyEsc,
}

/// Watches the raw pty bytes for XTGETTCAP, one byte at a time so the caller
/// keeps its replies in the order asked and a request split across reads
/// still answers.
#[derive(Default)]
pub(crate) struct Caps {
    state: State,
    body: Vec<u8>,
}

impl Caps {
    /// Step one byte; the replies owed when it completes a request.
    pub(crate) fn step(&mut self, b: u8) -> Option<String> {
        use State::*;
        self.state = match (self.state, b) {
            (BodyEsc, b'\\') => {
                self.state = Ground;
                return Some(replies(&std::mem::take(&mut self.body)));
            }
            // CAN and SUB cancel any sequence, as vte treats them.
            (Body, 0x18 | 0x1a) => Ground,
            (Body, ESC) => BodyEsc,
            (Body, _) if self.body.len() < MAX_BODY => {
                self.body.push(b);
                Body
            }
            (Esc | BodyEsc, b'P') => Dcs,
            (Dcs, b'+') => Plus,
            (Plus, b'q') => Body,
            // An ESC always starts a new sequence, whatever came before.
            (_, ESC) => Esc,
            _ => Ground,
        };
        if !matches!(self.state, Body | BodyEsc) {
            self.body.clear();
        }
        None
    }
}

/// One reply per non-empty name, in the order asked.
fn replies(body: &[u8]) -> String {
    body.split(|&c| c == b';')
        .filter(|n| !n.is_empty())
        .map(reply)
        .collect()
}

fn reply(hex_name: &[u8]) -> String {
    let name = unhex(hex_name);
    let known = name
        .as_deref()
        .and_then(|n| CAPS.iter().find(|(k, _)| k.as_bytes() == n));
    match known {
        Some((k, Some(v))) => format!("\x1bP1+r{}={}\x1b\\", hex(k), hex(v)),
        Some((k, None)) => format!("\x1bP1+r{}\x1b\\", hex(k)),
        // Echo the name only when it is hex digits: anything else came from
        // the program's output, and writing it back would type it into the
        // shell's input.
        None if hex_name.iter().all(u8::is_ascii_hexdigit) => {
            format!("\x1bP0+r{}\x1b\\", String::from_utf8_lossy(hex_name))
        }
        None => "\x1bP0+r\x1b\\".to_string(),
    }
}

/// Two hex digits a byte, either case; `None` on an odd length or a non-digit.
fn unhex(s: &[u8]) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let digit = |c: u8| (c as char).to_digit(16);
    s.chunks(2)
        .map(|p| Some((digit(p[0])? * 16 + digit(p[1])?) as u8))
        .collect()
}

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02X}")).collect()
}

#[cfg(test)]
#[path = "modelcaps_tests.rs"]
mod tests;
