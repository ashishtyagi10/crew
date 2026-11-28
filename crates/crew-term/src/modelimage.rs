//! Placing a picture the program sent: where it is anchored, the rows it
//! reserves, and the answer to a kitty `a=q` probe. Split from `model.rs`
//! for the 200-line cap (child module, parent-private access kept).
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;

use super::TermCore;

impl TermCore {
    /// Record where a picture goes, and move the cursor past it.
    ///
    /// The anchor is an ABSOLUTE line — `screen_line` plus the cursor's row —
    /// so the picture scrolls with the text it arrived in instead of staying
    /// at a screen position the output has long since left.
    pub(super) fn place_image(&mut self, cmd: crate::graphicscmd::ImageCmd) {
        // A producer asks before it sends: `a=q` is "can you draw one of
        // these?", and a terminal that never answers is one every image tool
        // treats as unable. The answer is per-command, so a format crew
        // cannot decode is refused rather than accepted and dropped.
        if cmd.action == b'q' {
            let body = match cmd.supported() {
                true => "OK",
                false => "ENOTSUPPORTED:unsupported format",
            };
            self.answer(&cmd, body);
            return;
        }
        // A delete reaches the pictures the app holds too. By id (`d=i`/`I`)
        // takes that picture alone; every other kind (by cell, by z-index…)
        // is not told apart yet and still drops them all, as `d=a` does.
        if cmd.deletes() {
            self.wipe(match cmd.delete.to_ascii_lowercase() {
                b'i' => crate::wipescan::ImageWipe::Id(cmd.id),
                _ => crate::wipescan::ImageWipe::All,
            });
            return;
        }
        if !cmd.displays() {
            return;
        }
        // A sender that named its picture (`i=`) is told how it went — a
        // file it cannot have read, or a picture crew dropped, is one it can
        // resend as bytes. One with no id asked for no answer.
        let refusal = match () {
            _ if !cmd.supported() => Some("ENOTSUPPORTED:unsupported format"),
            _ if cmd.data.is_empty() => Some("ENODATA:no image data"),
            _ if cmd.path().is_some_and(|p| !p.is_file()) => Some("EBADF:cannot read file"),
            _ => None,
        };
        if let Some(why) = refusal {
            self.answer(&cmd, why);
            return;
        }
        let point = self.term.grid().cursor.point;
        let line = self.screen_line() + point.line.0.max(0) as u64;
        // How much of the grid the picture claims. The sender may say (`c`,
        // `r`); otherwise it is the picture's own pixel size over the cell's,
        // which is why the header is read before the bytes are decoded.
        let cells = match cmd.cells {
            (c, r) if c > 0 && r > 0 => (c, r),
            _ => {
                let Some((w, h)) = cmd.pixel_size() else {
                    self.answer(&cmd, "EINVAL:image size unknown");
                    return;
                };
                let up = |n: u32, d: u32| n.div_ceil(d.max(1)).clamp(1, u16::MAX.into()) as u16;
                (up(w, self.cell_px.0), up(h, self.cell_px.1))
            }
        };
        let rows = cells.1;
        let (stay, sixel) = (cmd.stay, cmd.sixel);
        // Where a sixel leaves the cursor follows what the data DRAWS, not
        // the raster attributes' claim (foot: "does *not* affect cursor
        // positioning", the cursor goes to the last text row the last band
        // touched). Taken now, while the command is still here.
        let drawn = crate::sixel::drawn_height(&cmd.data).div_ceil(self.cell_px.1.max(1));
        self.answer(&cmd, "OK");
        self.images.push(crate::model::PlacedImage {
            line,
            col: point.column.0 as u16,
            cells,
            alt: self.term.mode().contains(TermMode::ALT_SCREEN),
            cmd,
        });
        // The protocol leaves the cursor past the picture; a line feed at the
        // bottom of the screen scrolls, which is exactly what reserves the
        // room the picture is drawn over. `C=1` is the program saying it
        // has made the room itself.
        if stay {
            return;
        }
        // Sixel scrolling on (DECSDM reset, the default) leaves the cursor
        // ON the picture's last text row, in the column it started at — not
        // below it, as kitty does. foot's `sixel_unhook` (sixel.c) does
        // `linefeed_count = max(0, image.rows - 1)` and then
        // `term_cursor_to(row, image.pos.col)`; a program that wants to
        // continue below writes its own newline (img2sixel does). Column
        // survives because a line feed does not return the carriage.
        // Gap: a raster taller than the data (an opaque region foot keeps
        // when P2 is not 1) reserves its rows, but the cursor stops at the
        // data. Gaps: DECSDM (mode 80, scrolling off, picture at the home
        // position) and mode 8452 (cursor to the right of the picture) are
        // not implemented; the default behaviour is always used.
        // All the picture's rows are fed so the grid scrolls to make room (as
        // foot's `linefeed_count`), then the cursor climbs back to the last
        // row the data touched, which CUU does without scrolling.
        let feeds = if sixel { rows.saturating_sub(1) } else { rows };
        self.parse(&vec![b'\n'; usize::from(feeds)]);
        let up = feeds.saturating_sub((drawn.clamp(1, rows.into()) as u16).saturating_sub(1));
        if sixel && up > 0 {
            self.parse(format!("\x1b[{up}A").as_bytes());
        }
    }

    /// Reply to a graphics command, unless its `q=` asked for silence: 1
    /// drops the `OK`, 2 drops errors as well. A picture sent with no id
    /// asked for no answer; a probe (`a=q`) is always one.
    fn answer(&self, cmd: &crate::graphicscmd::ImageCmd, body: &str) {
        if cmd.id == 0 && cmd.action != b'q' {
            return;
        }
        let silenced = match body {
            "OK" => cmd.quiet >= 1,
            _ => cmd.quiet >= 2,
        };
        if !silenced {
            let reply = format!("\x1b_Gi={};{body}\x1b\\", cmd.id);
            self.events.replies.lock().unwrap().push_str(&reply);
        }
    }

    /// Take the pictures placed since the last call — `crew-app` decodes them
    /// off this thread.
    pub(crate) fn take_images(&mut self) -> Vec<crate::model::PlacedImage> {
        std::mem::take(&mut self.images)
    }

    /// Lines of scrollback above the screen. Not a picture's base: it stops
    /// growing once the scrollback is full (`screen_line` does not).
    pub(crate) fn history_lines(&self) -> usize {
        self.term.grid().history_size()
    }
}

#[cfg(test)]
#[path = "modelsixel_tests.rs"]
mod modelsixel_tests;
