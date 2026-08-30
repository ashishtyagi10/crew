//! Pictures inside a terminal pane: the app's half of the graphics protocol.
//!
//! `crew_term` lifts the escape sequence out of the byte stream and says where
//! in the buffer it landed (`PlacedImage`); everything after that is here,
//! because everything after that is presentation and threads.
//!
//! Two rules shape it. **Nothing decodes on the winit thread** — a screenshot
//! arrives inside a `try_read`, and running a PNG decoder there would freeze
//! every pane in the grid, agents included; so each picture goes to a worker
//! and arrives back over a channel. And **a picture belongs to the text it
//! arrived in**: the anchor is an absolute buffer line, so it scrolls with the
//! output, off the top of the screen and back again, instead of hanging at a
//! screen position the session left minutes ago.
use std::sync::mpsc::{self, Receiver};

use crew_render::Paint;
use crew_term::PlacedImage;

use crate::viewpane::bitmap::{self, Bitmap};

/// How many pictures one pane keeps. A pane that draws a chart every second
/// would otherwise hold every chart it ever drew; the oldest go first, which
/// are also the ones furthest up the scrollback.
const KEEP: usize = 32;

/// A picture on its way to the screen.
enum Art {
    Loading(Receiver<Option<Bitmap>>),
    Ready(Bitmap),
    /// Decoded to nothing — a truncated transmission, or a format this build
    /// cannot read. Kept as a placeholder so the row it reserved is not
    /// silently reused by the next picture's bookkeeping.
    Failed,
}

struct Shown {
    line: u64,
    col: u16,
    cells: (u16, u16),
    art: Art,
}

/// Every picture a terminal pane is holding.
#[derive(Default)]
pub(crate) struct TermImages {
    shown: Vec<Shown>,
}

impl TermImages {
    /// Take what the program asked to show since the last call and start
    /// decoding it. Called where the pane's bytes are read.
    pub(crate) fn collect(&mut self, placed: Vec<PlacedImage>) {
        for p in placed {
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(decode(&p.cmd));
            });
            self.shown.push(Shown {
                line: p.line,
                col: p.col,
                cells: p.cells,
                art: Art::Loading(rx),
            });
        }
        let over = self.shown.len().saturating_sub(KEEP);
        self.shown.drain(..over);
    }

    /// Land any picture whose worker has finished. Returns whether one did —
    /// which is a frame owed, since nothing else changed to ask for it.
    pub(crate) fn poll(&mut self) -> bool {
        let mut landed = false;
        for s in self.shown.iter_mut() {
            let Art::Loading(rx) = &s.art else { continue };
            match rx.try_recv() {
                Ok(Some(bm)) => {
                    s.art = Art::Ready(bm);
                    landed = true;
                }
                Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                    s.art = Art::Failed;
                    landed = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        landed
    }

    /// Whether a worker is still out — the term that keeps frames coming
    /// until every picture has landed, and stops as soon as they have.
    pub(crate) fn loading(&self) -> bool {
        self.shown.iter().any(|s| matches!(s.art, Art::Loading(_)))
    }

    /// The pictures visible in this pane's viewport, as paint.
    ///
    /// `history` is the scrollback above the screen and `offset` how far back
    /// the view is: together they say which absolute line the top row is
    /// showing, which is the whole mapping from "where the picture arrived" to
    /// "where it is now".
    pub(crate) fn paint(
        &self,
        history: usize,
        offset: usize,
        cols: u16,
        rows: u16,
        aspect: f32,
    ) -> Vec<Paint> {
        let top = history as i64 - offset as i64;
        let (cw, ch) = (f32::from(cols), f32::from(rows));
        let mut out = Vec::new();
        for s in &self.shown {
            let Art::Ready(bm) = &s.art else { continue };
            let row = s.line as i64 - top;
            // Entirely above or below the window: not merely invisible but
            // not worth rasterizing, which is most of a long session's worth.
            if row + i64::from(s.cells.1) <= 0 || row >= i64::from(rows) {
                continue;
            }
            out.extend(bitmap::paint_at(
                bm,
                f32::from(s.col),
                row as f32,
                f32::from(s.cells.0),
                f32::from(s.cells.1),
                aspect,
                (0.0, 0.0, cw, ch),
            ));
        }
        out
    }
}

/// Turn one command's payload into pixels. Runs on a worker.
fn decode(cmd: &crew_term::ImageCmd) -> Option<Bitmap> {
    if let Some(path) = cmd.path() {
        let bytes = std::fs::read(path).ok()?;
        return bitmap::decode(&bytes);
    }
    match cmd.format {
        // A file in a transmission: PNG is the only one the protocol names,
        // and `decode` sniffs the bytes anyway, so anything else a producer
        // sends still works.
        100 => bitmap::decode(&cmd.data),
        24 | 32 => {
            let (w, h) = cmd.pixel_size()?;
            bitmap::from_raw(&cmd.data, w, h, (cmd.format / 8) as usize)
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "termimg_tests.rs"]
mod termimg_tests;
