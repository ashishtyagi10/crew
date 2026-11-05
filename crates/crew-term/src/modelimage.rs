//! Placing a picture the program sent: where it is anchored, the rows it
//! reserves, and the answer to a kitty `a=q` probe. Split from `model.rs`
//! for the 200-line cap (child module, parent-private access kept).
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;

use super::TermCore;

impl TermCore {
    /// Record where a picture goes, and move the cursor past it.
    ///
    /// The anchor is an ABSOLUTE line — history plus the cursor's screen row —
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
            let reply = format!("\x1b_Gi={};{body}\x1b\\", cmd.id);
            self.events.replies.lock().unwrap().push_str(&reply);
            return;
        }
        // Any delete drops them all, the ones the app holds too: the
        // narrower kinds (by id, by cell) are not told apart yet.
        if cmd.deletes() {
            self.wipe(crate::wipescan::ImageWipe::All);
            return;
        }
        if !cmd.displays() || cmd.data.is_empty() {
            return;
        }
        let point = self.term.grid().cursor.point;
        let line = self.term.grid().history_size() as u64 + point.line.0.max(0) as u64;
        // How much of the grid the picture claims. The sender may say (`c`,
        // `r`); otherwise it is the picture's own pixel size over the cell's,
        // which is why the header is read before the bytes are decoded.
        let cells = match cmd.cells {
            (c, r) if c > 0 && r > 0 => (c, r),
            _ => {
                let Some((w, h)) = cmd.pixel_size() else {
                    return;
                };
                let up = |n: u32, d: u32| n.div_ceil(d.max(1)).clamp(1, u16::MAX.into()) as u16;
                (up(w, self.cell_px.0), up(h, self.cell_px.1))
            }
        };
        let rows = cells.1;
        self.images.push(crate::model::PlacedImage {
            line,
            col: point.column.0 as u16,
            cells,
            alt: self.term.mode().contains(TermMode::ALT_SCREEN),
            cmd,
        });
        // The protocol leaves the cursor past the picture; a line feed at the
        // bottom of the screen scrolls, which is exactly what reserves the
        // room the picture is drawn over.
        for _ in 0..rows {
            self.parser.advance(&mut self.term, b"\n");
        }
    }

    /// Take the pictures placed since the last call — `crew-app` decodes them
    /// off this thread.
    pub(crate) fn take_images(&mut self) -> Vec<crate::model::PlacedImage> {
        std::mem::take(&mut self.images)
    }

    /// Lines of scrollback above the screen, for mapping an image's absolute
    /// anchor back to the row it is on now.
    pub(crate) fn history_lines(&self) -> usize {
        self.term.grid().history_size()
    }
}
