//! Incremental sniffer for the OSC sequences the parser we use ignores.
//!
//! Three of them matter, and none reaches vte/alacritty:
//!
//! * **OSC 7** — the working-directory report shells emit on each prompt
//!   (`ESC ] 7 ; file://<host><path> ST`).
//! * **OSC 9** — a program asking for a **notification** (`ESC ] 9 ; text ST`,
//!   iTerm2/ConEmu), or reporting **progress** in the ConEmu/Windows Terminal
//!   form `ESC ] 9 ; 4 ; <state> ; <percent> ST`.
//! * **OSC 777** — the other notification spelling
//!   (`ESC ] 777 ; notify ; <title> ; <body> ST`), which is what most Linux
//!   tooling emits.
//!
//! It is one small state machine, so a sequence split across `feed()` chunks
//! is still recognised, and it stays allocation-free until a real payload
//! lands.
/// Only the `cfg(test)` peek helper and its tests deal in borrowed paths.
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

/// Cap on a single payload — a real path or notification is far shorter; this
/// guards against an unterminated sequence growing the buffer without bound.
const MAX_PAYLOAD: usize = 4096;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;

#[derive(Default)]
enum State {
    /// Outside any escape sequence.
    #[default]
    Ground,
    /// Saw `ESC`.
    Esc,
    /// Saw `ESC ]` — collecting the OSC number up to `;`.
    Osc,
    /// An OSC we don't care about — draining to its terminator.
    Skip,
    /// Saw `ESC` while skipping (maybe `ST` = `ESC \`).
    SkipEsc,
    /// A payload we care about — collecting until the terminator.
    Payload,
    /// Saw `ESC` inside the payload (maybe `ST` = `ESC \`).
    PayloadEsc,
}

/// What a program said about its own progress (OSC 9;4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Progress {
    /// `0..=100`, or `None` while the work is indeterminate.
    pub percent: Option<u8>,
    /// The program called this an error or a warning rather than plain work.
    pub alarm: bool,
}

impl Progress {
    /// `state ; percent` — state 0 clears, 1 sets, 2 is an error, 3 is
    /// indeterminate, 4 is a warning (the ConEmu/Windows Terminal ladder).
    fn parse(rest: &str) -> Option<Progress> {
        let (state, pct) = rest.split_once(';').unwrap_or((rest, ""));
        let pct = pct.trim().parse::<u8>().ok().map(|p| p.min(100));
        match state.trim() {
            "0" => None,
            "3" => Some(Progress {
                percent: None,
                alarm: false,
            }),
            "2" | "4" => Some(Progress {
                percent: pct,
                alarm: true,
            }),
            _ => Some(Progress {
                percent: Some(pct?),
                alarm: false,
            }),
        }
    }
}

#[derive(Default)]
pub(crate) struct OscScanner {
    state: State,
    /// OSC number digits collected in `Osc`.
    num: Vec<u8>,
    /// Payload collected in `Payload`.
    buf: Vec<u8>,
    /// Which OSC that payload belongs to (7, 9 or 777).
    which: u16,
    /// A notification a program asked for: `(title, body)`.
    notify: Option<(String, String)>,
    /// The latest progress report, `None` once cleared by the program.
    progress: Option<Progress>,
    /// The latest reported directory.
    cwd: Option<PathBuf>,
    /// Set when `cwd` changed since the last `take`.
    dirty: bool,
}

impl OscScanner {
    /// The reported directory if it changed since the last call, else `None`.
    pub(crate) fn take_cwd(&mut self) -> Option<PathBuf> {
        if self.dirty {
            self.dirty = false;
            self.cwd.clone()
        } else {
            None
        }
    }

    /// Scan a chunk of raw PTY output, updating the cwd when a complete OSC 7
    /// report is seen.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.step(b);
        }
    }

    fn step(&mut self, b: u8) {
        match self.state {
            State::Ground => {
                if b == ESC {
                    self.state = State::Esc;
                }
            }
            State::Esc => match b {
                b']' => {
                    self.num.clear();
                    self.state = State::Osc;
                }
                ESC => {} // back-to-back ESC: stay primed
                _ => self.state = State::Ground,
            },
            State::Osc => match b {
                b';' => match std::str::from_utf8(&self.num)
                    .ok()
                    .and_then(|n| n.parse().ok())
                {
                    Some(n @ (7 | 9 | 777)) => {
                        self.which = n;
                        self.buf.clear();
                        self.state = State::Payload;
                    }
                    _ => self.state = State::Skip,
                },
                BEL => self.state = State::Ground, // OSC with no payload
                ESC => self.state = State::SkipEsc,
                // Bail on an absurdly long "number" rather than buffer forever.
                _ if self.num.len() >= 8 => self.state = State::Skip,
                _ => self.num.push(b),
            },
            State::Skip => match b {
                BEL => self.state = State::Ground,
                ESC => self.state = State::SkipEsc,
                _ => {}
            },
            State::SkipEsc => self.state = State::Ground, // ESC \ (or resync)
            State::Payload => match b {
                BEL => self.finish(),
                ESC => self.state = State::PayloadEsc,
                _ if self.buf.len() >= MAX_PAYLOAD => self.abort(),
                _ => self.buf.push(b),
            },
            State::PayloadEsc => {
                if b == b'\\' {
                    self.finish(); // ESC \ = ST terminates the payload
                } else {
                    self.abort();
                }
            }
        }
    }

    fn abort(&mut self) {
        self.buf.clear();
        self.state = State::Ground;
    }

    fn finish(&mut self) {
        match self.which {
            7 => self.finish_cwd(),
            9 => self.finish_osc9(),
            _ => self.finish_osc777(),
        }
        self.buf.clear();
        self.state = State::Ground;
    }

    fn finish_cwd(&mut self) {
        if let Some(path) = parse_file_uri(&self.buf) {
            if self.cwd.as_deref() != Some(path.as_path()) {
                self.cwd = Some(path);
                self.dirty = true;
            }
        }
    }

    /// `9 ; 4 ; state ; percent` is progress; anything else under 9 is the
    /// notification text a program wants shown.
    fn finish_osc9(&mut self) {
        let Ok(text) = std::str::from_utf8(&self.buf) else {
            return;
        };
        match text.strip_prefix("4;") {
            Some(rest) => self.progress = Progress::parse(rest),
            None if !text.trim().is_empty() => {
                self.notify = Some((String::new(), text.to_string()))
            }
            None => {}
        }
    }

    /// `777 ; notify ; title ; body` — the body is optional, and a title on
    /// its own is still a notification worth showing.
    fn finish_osc777(&mut self) {
        let Ok(text) = std::str::from_utf8(&self.buf) else {
            return;
        };
        let Some(rest) = text.strip_prefix("notify;") else {
            return;
        };
        let (title, body) = rest.split_once(';').unwrap_or((rest, ""));
        if !title.trim().is_empty() || !body.trim().is_empty() {
            self.notify = Some((title.to_string(), body.to_string()));
        }
    }

    /// A notification a program asked for, once.
    pub(crate) fn take_notify(&mut self) -> Option<(String, String)> {
        self.notify.take()
    }

    /// The program's latest progress report, or `None` when it has none.
    pub(crate) fn progress(&self) -> Option<Progress> {
        self.progress
    }
}

/// Extract the filesystem path from an OSC 7 `file://<host>/<path>` payload,
/// percent-decoding it. `None` if it isn't a usable `file://` URI.
fn parse_file_uri(payload: &[u8]) -> Option<PathBuf> {
    let s = std::str::from_utf8(payload).ok()?;
    let rest = s.strip_prefix("file://")?;
    // After the scheme comes an optional host, then the absolute path beginning at
    // the first '/'. e.g. `file://host/Users/me` → `/Users/me`.
    let path = &rest[rest.find('/')?..];
    Some(PathBuf::from(percent_decode(path)))
}

/// Minimal percent-decoding (`%20` → space, etc.). Leaves malformed escapes as-is.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
impl OscScanner {
    /// Peek the current cwd without clearing the dirty flag (test helper).
    /// `cfg(test)` rather than `allow(dead_code)`: it is not code that happens
    /// to be unused, it is code that exists only for the tests, and the
    /// stricter gate says so and keeps it out of the shipped binary.
    fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(chunks: &[&[u8]]) -> Option<PathBuf> {
        let mut s = OscScanner::default();
        for c in chunks {
            s.feed(c);
        }
        s.take_cwd()
    }

    #[test]
    fn parses_bel_terminated_report() {
        let cwd = scan(&[b"\x1b]7;file://host/Users/me/code\x07"]);
        assert_eq!(cwd, Some(PathBuf::from("/Users/me/code")));
    }

    #[test]
    fn parses_st_terminated_report() {
        let cwd = scan(&[b"\x1b]7;file://host/tmp\x1b\\"]);
        assert_eq!(cwd, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn empty_host_is_fine() {
        let cwd = scan(&[b"\x1b]7;file:///var/log\x07"]);
        assert_eq!(cwd, Some(PathBuf::from("/var/log")));
    }

    #[test]
    fn percent_decodes_spaces() {
        let cwd = scan(&[b"\x1b]7;file://h/Users/me/My%20Code\x07"]);
        assert_eq!(cwd, Some(PathBuf::from("/Users/me/My Code")));
    }

    #[test]
    fn reassembles_a_split_sequence() {
        // The report is delivered across three feed() chunks.
        let cwd = scan(&[b"\x1b]7;file://host/Use", b"rs/me/co", b"de\x07"]);
        assert_eq!(cwd, Some(PathBuf::from("/Users/me/code")));
    }

    #[test]
    fn ignores_other_osc_sequences() {
        // OSC 0 (title) must not be mistaken for a cwd report.
        assert_eq!(scan(&[b"\x1b]0;some title\x07"]), None);
        assert_eq!(scan(&[b"\x1b]2;another\x07"]), None);
    }

    #[test]
    fn take_is_one_shot_until_it_changes() {
        let mut s = OscScanner::default();
        s.feed(b"\x1b]7;file://h/a\x07");
        assert_eq!(s.take_cwd(), Some(PathBuf::from("/a")));
        // No new report → nothing to take.
        assert_eq!(s.take_cwd(), None);
        // Same dir reported again → still nothing (no change).
        s.feed(b"\x1b]7;file://h/a\x07");
        assert_eq!(s.take_cwd(), None);
        // A real change is reported.
        s.feed(b"\x1b]7;file://h/b\x07");
        assert_eq!(s.take_cwd(), Some(PathBuf::from("/b")));
        assert_eq!(s.cwd(), Some(Path::new("/b")));
    }

    #[test]
    fn unterminated_payload_does_not_grow_without_bound() {
        let mut s = OscScanner::default();
        s.feed(b"\x1b]7;file://h/");
        s.feed(&vec![b'a'; MAX_PAYLOAD + 100]);
        // Aborted past the cap; no cwd captured, buffer released.
        assert_eq!(s.take_cwd(), None);
        assert!(s.buf.is_empty());
    }
}

#[cfg(test)]
mod osc9_tests {
    use super::*;

    fn fed(bytes: &[u8]) -> OscScanner {
        let mut s = OscScanner::default();
        s.feed(bytes);
        s
    }

    /// The iTerm2/ConEmu spelling: everything after `9;` is the message.
    #[test]
    fn osc_9_carries_a_notification_a_program_asked_for() {
        let mut s = fed(b"\x1b]9;build finished\x07");
        assert_eq!(
            s.take_notify(),
            Some((String::new(), "build finished".to_string()))
        );
        assert_eq!(s.take_notify(), None, "the same notification fired twice");
    }

    /// The other spelling, which is what most Linux tooling emits.
    #[test]
    fn osc_777_carries_a_title_and_a_body() {
        let mut s = fed(b"\x1b]777;notify;tests;42 passed\x1b\\");
        assert_eq!(
            s.take_notify(),
            Some(("tests".to_string(), "42 passed".to_string()))
        );
        // A title on its own is still worth showing.
        let mut only = fed(b"\x1b]777;notify;done\x07");
        assert_eq!(
            only.take_notify(),
            Some(("done".to_string(), String::new()))
        );
        // …and something that is not a notify request is not one.
        assert_eq!(fed(b"\x1b]777;other;x\x07").take_notify(), None);
    }

    /// An empty message is not a notification — a program clearing its own
    /// state must not put an empty toast on the canvas.
    #[test]
    fn an_empty_message_is_not_a_notification() {
        assert_eq!(fed(b"\x1b]9;\x07").take_notify(), None);
        assert_eq!(fed(b"\x1b]9;   \x07").take_notify(), None);
        assert_eq!(fed(b"\x1b]777;notify;;\x07").take_notify(), None);
    }

    /// `9;4` is progress, not a message — the two share an OSC number and
    /// telling them apart is the whole of this branch.
    #[test]
    fn osc_9_4_is_progress_rather_than_a_message() {
        let mut s = fed(b"\x1b]9;4;1;40\x07");
        assert_eq!(
            s.progress(),
            Some(Progress {
                percent: Some(40),
                alarm: false
            })
        );
        assert_eq!(s.take_notify(), None, "progress was shown as a toast");
    }

    #[test]
    fn every_state_in_the_ladder_reads_as_what_it_means() {
        // 3 = indeterminate: working, with no number to show.
        assert_eq!(
            fed(b"\x1b]9;4;3;0\x07").progress(),
            Some(Progress {
                percent: None,
                alarm: false
            })
        );
        // 2 = error, 4 = warning: both keep their percentage and raise it.
        for state in [b'2', b'4'] {
            let seq = [b"\x1b]9;4;".as_slice(), &[state], b";80\x07"].concat();
            assert_eq!(
                fed(&seq).progress(),
                Some(Progress {
                    percent: Some(80),
                    alarm: true
                }),
                "state {}",
                state as char
            );
        }
        // 0 = remove: the bar goes away.
        assert_eq!(fed(b"\x1b]9;4;0;0\x07").progress(), None);
        let mut cleared = fed(b"\x1b]9;4;1;50\x07");
        cleared.feed(b"\x1b]9;4;0;0\x07");
        assert_eq!(cleared.progress(), None, "the bar outlived its program");
    }

    /// Out-of-range and malformed percentages must not panic or lie.
    #[test]
    fn a_nonsense_percentage_is_clamped_or_ignored() {
        assert_eq!(
            fed(b"\x1b]9;4;1;250\x07")
                .progress()
                .and_then(|p| p.percent),
            Some(100)
        );
        assert_eq!(fed(b"\x1b]9;4;1;abc\x07").progress(), None);
        assert_eq!(fed(b"\x1b]9;4;1\x07").progress(), None);
    }

    /// A sequence split across two reads is still one sequence — the whole
    /// reason this is a state machine and not a search.
    #[test]
    fn a_sequence_split_across_chunks_is_still_recognised() {
        let mut s = OscScanner::default();
        s.feed(b"\x1b]9;4;1;");
        s.feed(b"75\x07");
        assert_eq!(s.progress().and_then(|p| p.percent), Some(75));
    }

    /// The OSC 7 path still works — it shares every byte of this machine.
    #[test]
    fn the_working_directory_report_is_unaffected() {
        let mut s = fed(b"\x1b]7;file://host/tmp/x\x07");
        assert_eq!(
            s.take_cwd().as_deref(),
            Some(std::path::Path::new("/tmp/x"))
        );
    }
}
