//! OSC 22 pointer shape (xterm; ghostty, kitty, foot and wezterm speak it): a
//! program with clickable UI (lazygit, yazi, helix, btop, textual apps) asks
//! for the mouse pointer to be a hand over a button with `ESC ] 22 ; <css
//! name> ST`. The ANSI parser drops OSC 22, so, like the OSC 7 sniffer, this
//! is a small incremental scanner over the raw PTY bytes; it only remembers
//! the NAME. Which pointer that is, and whether crew's own targets outrank
//! it, is the app's call (`crew-app::pointer`).
//!
//! An empty name, `default`, or a RIS (`ESC c`) clears it. kitty's `=name`
//! (set) is honoured; its query/push/pop forms (`?` `>` `<`) are ignored
//! rather than half-implemented.

/// Longest name worth keeping: CSS names are short, and kitty's comma-list
/// of fallbacks fits; anything longer is not a pointer name.
const MAX_NAME: usize = 64;

#[derive(Default)]
enum State {
    #[default]
    Ground,
    Esc,
    /// Inside `ESC ]`, collecting the payload (None once it overflowed).
    Osc(Option<Vec<u8>>),
    /// An ESC inside an OSC: `\` ends it (ST), anything else aborts it.
    OscEsc(Option<Vec<u8>>),
}

/// Per-terminal OSC 22 state plus the scanner that maintains it.
#[derive(Default)]
pub(crate) struct PointerShape {
    state: State,
    name: Option<String>,
}

impl PointerShape {
    /// The shape the program asked for, `None` for the default pointer.
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Scan a chunk of PTY output. State persists across calls, so a
    /// sequence split across reads reassembles.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state = match std::mem::take(&mut self.state) {
                State::Ground | State::Esc if b == 0x1b => State::Esc,
                State::Ground => State::Ground,
                State::Esc => match b {
                    b']' => State::Osc(Some(Vec::new())),
                    // RIS puts the pointer back like every other mode.
                    b'c' => {
                        self.name = None;
                        State::Ground
                    }
                    _ => State::Ground,
                },
                State::Osc(buf) => match b {
                    0x07 => {
                        self.finish(buf);
                        State::Ground
                    }
                    0x1b => State::OscEsc(buf),
                    // CAN / SUB cancel a sequence.
                    0x18 | 0x1a => State::Ground,
                    _ => State::Osc(buf.filter(|v| v.len() < MAX_NAME + 4).map(|mut v| {
                        v.push(b);
                        v
                    })),
                },
                State::OscEsc(buf) => match b {
                    b'\\' => {
                        self.finish(buf);
                        State::Ground
                    }
                    b']' => State::Osc(Some(Vec::new())),
                    _ => State::Ground,
                },
            };
        }
    }

    fn finish(&mut self, buf: Option<Vec<u8>>) {
        let Some(buf) = buf else { return };
        let Some(rest) = buf.strip_prefix(b"22;") else {
            return;
        };
        let Ok(rest) = std::str::from_utf8(rest) else {
            return;
        };
        let rest = rest.trim();
        // kitty's query / push / pop: not supported, leave the shape alone.
        if rest.starts_with(['?', '>', '<']) {
            return;
        }
        let rest = rest.strip_prefix('=').unwrap_or(rest).trim();
        self.name = (!rest.is_empty() && rest != "default").then(|| rest.to_string());
    }
}

#[cfg(test)]
#[path = "pointershape_tests.rs"]
mod tests;
