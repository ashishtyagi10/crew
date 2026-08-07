//! DECSET 2031 color-scheme notifications (the contour convention): a TUI
//! that enables private mode 2031 asks to be TOLD when the terminal's
//! light/dark scheme changes, instead of sampling OSC 10/11 once at startup
//! and riding the contrast floor forever. Crew's parser (alacritty_terminal)
//! ignores private modes it doesn't know, so — like the OSC 7 cwd sniffer —
//! this is a small incremental scanner over the raw PTY bytes:
//!
//! - `CSI ? 2031 h` / `l` — enable/disable change notifications
//! - `CSI ? 2031 $ p` (DECRQM) — "do you support this?" → DECRPM reply
//! - `CSI ? 996 n` — "what's the scheme right now?" → an immediate report
//!
//! The report itself (`CSI ? 997 ; 1 n` dark / `; 2 n` light) is produced by
//! [`scheme_report`]; the app pushes it to every enabled pane when the active
//! theme's darkness flips (any switch path — OS flip, `/theme`, rotation).
/// Longest CSI parameter run worth buffering; anything longer is not one of
/// ours, so the scanner bails to ground rather than hoarding bytes.
const MAX_PARAMS: usize = 16;

/// The `CSI ? 997 ; Ps n` color-scheme report: `Ps` 1 = dark, 2 = light.
pub fn scheme_report(dark: bool) -> &'static str {
    if dark {
        "\x1b[?997;1n"
    } else {
        "\x1b[?997;2n"
    }
}

/// Scanner state: which byte of a `CSI ?` sequence we're inside. Persists
/// across [`SchemeNotify::feed`] calls, so sequences split across PTY reads
/// reassemble for free.
#[derive(Default)]
enum State {
    #[default]
    Ground,
    /// Saw ESC.
    Esc,
    /// Saw `ESC [`; waiting to learn whether it's private (`?`).
    Csi,
    /// Inside `ESC [ ?` collecting parameter/intermediate bytes.
    Private {
        params: String,
        intermediate: Option<u8>,
    },
}

/// Per-terminal DECSET 2031 state + the scanner that maintains it.
#[derive(Default)]
pub(crate) struct SchemeNotify {
    state: State,
    enabled: bool,
}

impl SchemeNotify {
    /// Whether the program asked for scheme-change notifications.
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    /// Scan a chunk of PTY output. `dark` is the active scheme (for query
    /// replies). Returns the reply bytes owed to the child, empty when none.
    pub(crate) fn feed(&mut self, bytes: &[u8], dark: bool) -> String {
        let mut replies = String::new();
        for &b in bytes {
            self.state = match std::mem::take(&mut self.state) {
                State::Ground => match b {
                    0x1b => State::Esc,
                    _ => State::Ground,
                },
                State::Esc => match b {
                    b'[' => State::Csi,
                    0x1b => State::Esc,
                    _ => State::Ground,
                },
                State::Csi => match b {
                    b'?' => State::Private {
                        params: String::new(),
                        intermediate: None,
                    },
                    // Not a private sequence — this scanner doesn't care.
                    // (A final byte right here also lands in Ground.)
                    0x1b => State::Esc,
                    _ => State::Ground,
                },
                State::Private {
                    mut params,
                    intermediate,
                } => match b {
                    b'0'..=b'9' | b';' if params.len() < MAX_PARAMS => {
                        params.push(b as char);
                        State::Private {
                            params,
                            intermediate,
                        }
                    }
                    // Intermediate byte (DECRQM's `$`).
                    0x20..=0x2f => State::Private {
                        params,
                        intermediate: Some(b),
                    },
                    // Final byte: act, back to ground.
                    0x40..=0x7e => {
                        self.dispatch(b, &params, intermediate, dark, &mut replies);
                        State::Ground
                    }
                    0x1b => State::Esc,
                    _ => State::Ground,
                },
            };
        }
        replies
    }

    fn dispatch(
        &mut self,
        final_byte: u8,
        params: &str,
        intermediate: Option<u8>,
        dark: bool,
        replies: &mut String,
    ) {
        let has_2031 = params.split(';').any(|p| p == "2031");
        match (final_byte, intermediate) {
            (b'h', None) if has_2031 => self.enabled = true,
            (b'l', None) if has_2031 => self.enabled = false,
            // DECRQM → DECRPM: 1 = set, 2 = reset (we always support the
            // mode, so never the "unknown" 0).
            (b'p', Some(b'$')) if params == "2031" => {
                let ps = if self.enabled { 1 } else { 2 };
                replies.push_str(&format!("\x1b[?2031;{ps}$y"));
            }
            // "Report the current color scheme."
            (b'n', None) if params == "996" => replies.push_str(scheme_report(dark)),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "schemenotify_tests.rs"]
mod tests;
