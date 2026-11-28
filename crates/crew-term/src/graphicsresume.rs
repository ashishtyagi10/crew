//! The scanner's bookkeeping: continuing a hold across a chunk boundary, the
//! OSC prefix probe, and ending a sequence. Split from `graphics.rs` for the
//! 200-line cap (child module: the scanner's fields stay private).
use super::types::{Probe, Seg, St, DCS_PROBE, ESC, MAX_PAYLOAD, OSC_PROBE};
use super::GraphicsScanner;
use crate::graphicscmd::ImageCmd;

impl GraphicsScanner {
    /// Continue a hold from the previous chunk: the bytes held were `ESC`
    /// plus whatever the probe state had read. Returns where in `chunk` the
    /// ordinary scan should start. Whatever turns out not to be ours is handed
    /// on as [`Seg::Held`], byte for byte.
    pub(super) fn resume<'a>(&mut self, chunk: &'a [u8], out: &mut Vec<Seg<'a>>) -> usize {
        let mut st = self.st;
        let mut held = vec![ESC];
        match st {
            St::OscProbe => held.extend_from_slice(b"]"),
            St::DcsProbe => held.extend_from_slice(b"P"),
            _ => {}
        }
        if st != St::Esc {
            held.extend_from_slice(&self.buf);
        }
        self.st = St::Ground;
        for (n, &b) in chunk.iter().enumerate() {
            let i = n + 1;
            match st {
                St::Esc => match b {
                    b'_' => {
                        self.open(St::Apc);
                        return i;
                    }
                    b']' | b'P' => {
                        st = if b == b']' {
                            St::OscProbe
                        } else {
                            St::DcsProbe
                        };
                        held.push(b);
                        self.buf.clear();
                    }
                    // `ESC ESC`: the first is plain, the second decides.
                    ESC => out.push(Seg::Held(vec![ESC])),
                    _ => {
                        held.push(b);
                        out.push(Seg::Held(held));
                        return i;
                    }
                },
                St::OscProbe => match self.probe(b) {
                    Probe::More => held.push(b),
                    Probe::Picture => {
                        self.open(St::Osc);
                        return i;
                    }
                    Probe::No => {
                        held.push(b);
                        out.push(Seg::Held(held));
                        return i;
                    }
                },
                _ => match b {
                    b'0'..=b'9' | b';' if self.buf.len() < DCS_PROBE => {
                        self.buf.push(b);
                        held.push(b);
                    }
                    b'q' => {
                        self.buf.push(b);
                        (self.st, self.flooded) = (St::Sixel, false);
                        return i;
                    }
                    ESC => {
                        out.push(Seg::Held(std::mem::replace(&mut held, vec![ESC])));
                        st = St::Esc;
                    }
                    _ => {
                        held.push(b);
                        out.push(Seg::Held(held));
                        return i;
                    }
                },
            }
        }
        // Still undecided at the end of this chunk too: keep holding.
        self.st = st;
        chunk.len()
    }

    /// Feed one byte to the OSC prefix probe.
    pub(super) fn probe(&mut self, b: u8) -> Probe {
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
    pub(super) fn open(&mut self, st: St) {
        self.st = st;
        self.buf.clear();
        self.flooded = false;
    }

    pub(super) fn push(&mut self, b: u8) {
        if self.buf.len() >= MAX_PAYLOAD {
            self.flooded = true;
            return;
        }
        self.buf.push(b);
    }

    /// End the sequence in `buf`: parse it, join it to any chunked
    /// predecessor, and emit it unless more chunks are promised.
    pub(super) fn finish(&mut self, out: &mut Vec<Seg<'_>>) {
        let was = self.st;
        self.st = St::Ground;
        let buf = std::mem::take(&mut self.buf);
        if matches!(was, St::SixelEsc) {
            // A sixel is one piece: no `m=1` chaining, and a flooded one is
            // dropped whole rather than half-drawn.
            if !self.flooded {
                out.push(Seg::Image(ImageCmd::sixel(buf)));
            }
            return;
        }
        if self.flooded {
            self.partial = None;
            return;
        }
        let cmd = match matches!(was, St::Osc | St::OscEsc) {
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
