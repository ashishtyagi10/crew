//! The terminal graphics protocol: pictures arriving as escape sequences.
//!
//! A program that wants to show you an image writes it to its own stdout, in
//! kitty's `APC G` form — `ESC _ G <key=value,…> ; <base64 payload> ESC \` —
//! which every terminal that can draw one now speaks (kitty, Ghostty, WezTerm,
//! Konsole). vte hands APC strings to nobody, so like OSC 7 this is sniffed
//! off the raw byte stream rather than parsed.
//!
//! Unlike OSC 7 the *position* matters: an image lands where the cursor is
//! when the sequence arrives, so the scanner cannot simply watch the stream go
//! past. It **splits** it — the bytes before a sequence are fed to the parser
//! first, so the cursor is where the program left it before the image is
//! placed, and the bytes after it are fed once the placement is recorded.
//!
//! Nothing here decodes a picture. A PNG lands on the winit thread inside a
//! `try_read`, and decoding one there would freeze every pane in the grid; the
//! payload is handed up as bytes and `crew-app` decodes it on a worker (see
//! `termimg`). This module's whole job is: find the sequence, keep it whole
//! across chunk boundaries, and say where in the stream it was.
use crate::graphicscmd::ImageCmd;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;

/// Cap on one accumulated payload. Kitty sends at most 4096 base64 bytes per
/// chunk and a real screenshot is a few hundred of them; this is the guard
/// against a sequence that never terminates growing the buffer forever.
const MAX_PAYLOAD: usize = 16 * 1024 * 1024;

/// What the OSC prefix probe has decided so far.
enum Probe {
    More,
    Picture,
    No,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum St {
    #[default]
    Ground,
    /// Saw `ESC`, waiting to learn whether this is an APC.
    Esc,
    /// Inside `ESC _ …`.
    Apc,
    /// Inside an APC and saw `ESC` — one byte from the string terminator.
    ApcEsc,
    /// Saw `ESC ]` and is reading just enough of the OSC to know whether it
    /// is iTerm2's picture (`1337;File=`) or somebody else's sequence.
    OscProbe,
    /// Inside an `ESC ] 1337 ; File=…` picture.
    Osc,
    /// Inside one, and saw `ESC` — one byte from the string terminator.
    OscEsc,
}

/// How much of an OSC is read before deciding it is not a picture. Exactly
/// the length of the prefix that says it is one.
const OSC_PROBE: &[u8] = b"1337;File=";

/// One piece of the split stream.
pub(crate) enum Seg<'a> {
    /// Bytes for the ANSI parser, exactly as they arrived.
    Bytes(&'a [u8]),
    /// A complete graphics command, at this point in the stream.
    Image(ImageCmd),
    /// Bytes held back across a chunk boundary that turned out not to start a
    /// picture after all — a lone `ESC`, or the first bytes of an OSC that is
    /// somebody else's. Its own variant because they are not a slice of the
    /// chunk being split.
    Held(Vec<u8>),
}

/// The incremental splitter. One per terminal; a sequence divided across
/// `feed` calls is still recognised.
#[derive(Default)]
pub(crate) struct GraphicsScanner {
    st: St,
    /// The current sequence's bytes (params and payload, no delimiters).
    buf: Vec<u8>,
    /// A chunked transmission in progress (`m=1`): the first chunk's keys and
    /// the payload accumulated so far.
    partial: Option<ImageCmd>,
    /// True once a payload overflowed [`MAX_PAYLOAD`] — the rest of that
    /// sequence is dropped rather than half-decoded.
    flooded: bool,
}

impl GraphicsScanner {
    /// Split `chunk` into parser bytes and graphics commands, in order.
    ///
    /// Returns owned segments rather than calling back, because the caller
    /// holds `&mut` on both the parser and the terminal and cannot lend
    /// either to a closure this borrows from.
    pub(crate) fn feed<'a>(&mut self, chunk: &'a [u8]) -> Vec<Seg<'a>> {
        let mut out = Vec::new();
        let mut i = 0;
        // Bytes held back at the end of the last chunk, waiting to learn what
        // they were the start of. They cannot be handed over as a slice of
        // THIS chunk, which is why held bytes are a segment of their own.
        if matches!(self.st, St::Esc | St::OscProbe) {
            i = self.resume(chunk, &mut out);
        }
        // Start of the run of plain bytes we are currently inside, and where
        // the `ESC` we may yet have to cut it at sits.
        let mut run = i;
        let mut esc_at = 0usize;
        while i < chunk.len() {
            let b = chunk[i];
            match self.st {
                St::Ground if b == ESC => {
                    self.st = St::Esc;
                    esc_at = i;
                }
                St::Ground => {}
                St::Esc if b == b'_' => {
                    // Only now is it certain the escape was not the parser's:
                    // cut the run before it and drop both bytes.
                    if run < esc_at {
                        out.push(Seg::Bytes(&chunk[run..esc_at]));
                    }
                    self.open(St::Apc);
                    run = i + 1;
                }
                // An OSC might be a picture (iTerm2's `1337;File=`) or might
                // be any of the four crew already listens for. Nothing is cut
                // until the prefix says which.
                St::Esc if b == b']' => {
                    self.st = St::OscProbe;
                    self.buf.clear();
                }
                St::Esc if b == ESC => esc_at = i,
                // Some other escape sequence: it stays inside the plain run.
                St::Esc => self.st = St::Ground,
                St::OscProbe => match self.probe(b) {
                    Probe::More => {}
                    Probe::Picture => {
                        if run < esc_at {
                            out.push(Seg::Bytes(&chunk[run..esc_at]));
                        }
                        self.open(St::Osc);
                        run = i + 1;
                    }
                    // Somebody else's OSC: everything since the ESC is still
                    // inside the run, untouched, and goes on as text.
                    Probe::No => self.st = St::Ground,
                },
                St::Apc | St::Osc if b == ESC => {
                    self.st = match self.st {
                        St::Apc => St::ApcEsc,
                        _ => St::OscEsc,
                    }
                }
                St::Apc | St::Osc if b == BEL => {
                    self.finish(&mut out);
                    run = i + 1;
                }
                St::Apc | St::Osc => self.push(b),
                St::ApcEsc | St::OscEsc if b == b'\\' => {
                    self.finish(&mut out);
                    run = i + 1;
                }
                St::ApcEsc | St::OscEsc => {
                    // A stray ESC inside the string: keep both bytes and
                    // stay in the sequence.
                    self.push(ESC);
                    self.push(b);
                    self.st = match self.st {
                        St::ApcEsc => St::Apc,
                        _ => St::Osc,
                    };
                }
            }
            i += 1;
        }
        // Whatever is still plain text goes on now. A half-seen sequence — or
        // bytes that may yet open one — is held for the next chunk.
        let tail = match self.st {
            St::Ground => &chunk[run..],
            St::Esc | St::OscProbe => &chunk[run..esc_at.max(run)],
            _ => &[][..],
        };
        if !tail.is_empty() {
            out.push(Seg::Bytes(tail));
        }
        out
    }

    /// Continue a hold from the previous chunk. Returns where in `chunk` the
    /// ordinary scan should start.
    fn resume<'a>(&mut self, chunk: &'a [u8], out: &mut Vec<Seg<'a>>) -> usize {
        let mut held = vec![ESC];
        if self.st == St::OscProbe {
            held.push(b']');
            held.extend_from_slice(&self.buf);
        }
        let was_probe = self.st == St::OscProbe;
        self.st = St::Ground;
        let mut i = 0;
        while i < chunk.len() {
            let b = chunk[i];
            i += 1;
            if !was_probe && held.len() == 1 {
                // Deciding what the held ESC opened.
                match b {
                    b'_' => {
                        self.open(St::Apc);
                        return i;
                    }
                    b']' => {
                        held.push(b);
                        self.st = St::OscProbe;
                        self.buf.clear();
                        continue;
                    }
                    _ => {
                        held.push(b);
                        break;
                    }
                }
            }
            match self.probe(b) {
                Probe::More => held.push(b),
                Probe::Picture => {
                    self.open(St::Osc);
                    return i;
                }
                Probe::No => {
                    held.push(b);
                    break;
                }
            }
        }
        if self.st == St::OscProbe && i == chunk.len() {
            // Still undecided at the end of this chunk too: keep holding.
            return i;
        }
        self.st = St::Ground;
        out.push(Seg::Held(held));
        i
    }

    /// Feed one byte to the OSC prefix probe.
    fn probe(&mut self, b: u8) -> Probe {
        self.buf.push(b);
        let n = self.buf.len();
        if self.buf[..] != OSC_PROBE[..n.min(OSC_PROBE.len())] {
            return Probe::No;
        }
        match n >= OSC_PROBE.len() {
            true => Probe::Picture,
            false => Probe::More,
        }
    }

    /// Begin collecting a sequence.
    fn open(&mut self, st: St) {
        self.st = st;
        self.buf.clear();
        self.flooded = false;
    }

    fn push(&mut self, b: u8) {
        if self.buf.len() >= MAX_PAYLOAD {
            self.flooded = true;
            return;
        }
        self.buf.push(b);
    }

    /// End the sequence in `buf`: parse it, join it to any chunked
    /// predecessor, and emit it unless more chunks are promised.
    fn finish(&mut self, out: &mut Vec<Seg<'_>>) {
        let was_osc = matches!(self.st, St::Osc | St::OscEsc);
        self.st = St::Ground;
        let buf = std::mem::take(&mut self.buf);
        if self.flooded {
            self.partial = None;
            return;
        }
        let cmd = match was_osc {
            true => ImageCmd::parse_iterm(&buf),
            false => ImageCmd::parse(&buf),
        };
        let Some(cmd) = cmd else {
            return;
        };
        let joined = match self.partial.take() {
            Some(mut first) => {
                first.data.extend_from_slice(&cmd.data);
                first
            }
            None => cmd.clone(),
        };
        match cmd.more {
            true => self.partial = Some(joined),
            false => out.push(Seg::Image(joined)),
        }
    }
}

#[cfg(test)]
#[path = "graphics_tests.rs"]
mod graphics_tests;
