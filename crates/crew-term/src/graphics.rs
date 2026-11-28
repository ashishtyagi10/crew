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
//! Sixel (`DCS … q … ST`) is the third spelling and is split the same way; the
//! one hazard is that DCS also carries XTGETTCAP, tmux passthrough and
//! XTVERSION replies, so a DCS is only ours once its header is digits, `;`
//! and `q`. Nothing else is cut.
//!
//! Nothing here decodes a picture. A PNG lands on the winit thread inside a
//! `try_read`, and decoding one there would freeze every pane in the grid; the
//! payload is handed up as bytes and `crew-app` decodes it on a worker (see
//! `termimg`). This module's whole job is: find the sequence, keep it whole
//! across chunk boundaries, and say where in the stream it was.
use crate::graphicscmd::ImageCmd;
pub(crate) use types::Seg;
use types::{Probe, St, BEL, DCS_PROBE, ESC};

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
        if matches!(self.st, St::Esc | St::OscProbe | St::DcsProbe) {
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
                St::Esc if b == b'P' => {
                    self.st = St::DcsProbe;
                    self.buf.clear();
                }
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
                St::DcsProbe => match b {
                    b'0'..=b'9' | b';' if self.buf.len() < DCS_PROBE => self.buf.push(b),
                    b'q' => {
                        if run < esc_at {
                            out.push(Seg::Bytes(&chunk[run..esc_at]));
                        }
                        self.buf.push(b);
                        (self.st, self.flooded, run) = (St::Sixel, false, i + 1);
                    }
                    ESC => (self.st, esc_at) = (St::Esc, i),
                    // Not ours (`+q`, `$q`, `>|`, tmux…): it stays in the run.
                    _ => self.st = St::Ground,
                },
                // BEL does not end a DCS, only ST does.
                St::Sixel if b == ESC => self.st = St::SixelEsc,
                // CAN / SUB cancel a control string (ECMA-48): drop the
                // picture and let the parser have the byte.
                St::Sixel if b == 0x18 || b == 0x1a => {
                    (self.st, run) = (St::Ground, i);
                    self.buf.clear();
                }
                St::Sixel if b != BEL => self.push(b),
                St::Sixel => {}
                St::SixelEsc if b == b'\\' => {
                    self.finish(&mut out);
                    run = i + 1;
                }
                // `cat img.six` + Ctrl-C: the shell's next escape must not be
                // swallowed. Drop the picture and decide what the held ESC
                // opens exactly as a chunk that starts after an ESC would —
                // so a picture sent straight after the cut one still shows.
                St::SixelEsc => {
                    self.buf.clear();
                    self.st = St::Esc;
                    i += self.resume(&chunk[i..], &mut out);
                    run = i;
                    continue;
                }
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
            St::Esc | St::OscProbe | St::DcsProbe => &chunk[run..esc_at.max(run)],
            _ => &[][..],
        };
        if !tail.is_empty() {
            out.push(Seg::Bytes(tail));
        }
        out
    }
}

#[path = "graphicsresume.rs"]
mod resume;
#[path = "graphicstypes.rs"]
mod types;

#[cfg(test)]
#[path = "graphics_tests.rs"]
mod graphics_tests;
#[cfg(test)]
#[path = "graphicsdcs_tests.rs"]
mod graphicsdcs_tests;
