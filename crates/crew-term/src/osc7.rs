//! Incremental OSC 7 (working-directory report) sniffer.
//!
//! Shells emit `ESC ] 7 ; file://<host><path> (BEL | ESC \)` on each prompt to
//! report their current directory. The parser we use (vte/alacritty) ignores
//! OSC 7 entirely, so we scan the raw byte stream for it in parallel and keep the
//! latest reported path. It's a small state machine, so a sequence split across
//! `feed()` chunks is still recognised. Cheap and allocation-free until a real
//! `cd` lands.
/// Only the `cfg(test)` peek helper and its tests deal in borrowed paths.
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

/// Cap on a single OSC 7 payload — a real `file://` path is far shorter; this
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
    /// OSC 7 payload — collecting until the terminator.
    Payload,
    /// Saw `ESC` inside the payload (maybe `ST` = `ESC \`).
    PayloadEsc,
}

#[derive(Default)]
pub(crate) struct Osc7Scanner {
    state: State,
    /// OSC number digits collected in `Osc`.
    num: Vec<u8>,
    /// OSC 7 payload collected in `Payload`.
    buf: Vec<u8>,
    /// The latest reported directory.
    cwd: Option<PathBuf>,
    /// Set when `cwd` changed since the last `take`.
    dirty: bool,
}

impl Osc7Scanner {
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
                b';' => {
                    if self.num == b"7" {
                        self.buf.clear();
                        self.state = State::Payload;
                    } else {
                        self.state = State::Skip;
                    }
                }
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
        if let Some(path) = parse_file_uri(&self.buf) {
            if self.cwd.as_deref() != Some(path.as_path()) {
                self.cwd = Some(path);
                self.dirty = true;
            }
        }
        self.buf.clear();
        self.state = State::Ground;
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
impl Osc7Scanner {
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
        let mut s = Osc7Scanner::default();
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
        let mut s = Osc7Scanner::default();
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
        let mut s = Osc7Scanner::default();
        s.feed(b"\x1b]7;file://h/");
        s.feed(&vec![b'a'; MAX_PAYLOAD + 100]);
        // Aborted past the cap; no cwd captured, buffer released.
        assert_eq!(s.take_cwd(), None);
        assert!(s.buf.is_empty());
    }
}
