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
        // Any delete drops them all, the ones the app holds too: the
        // narrower kinds (by id, by cell) are not told apart yet.
        if cmd.deletes() {
            self.wipe(crate::wipescan::ImageWipe::All);
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
        let stay = cmd.stay;
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
        self.parse(&vec![b'\n'; usize::from(rows)]);
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
