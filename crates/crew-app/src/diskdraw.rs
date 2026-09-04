//! Drawing the `/disk` pane: the treemap and the row of tiles under it.
//!
//! Split out of [`crate::diskpane`] for the line cap.
use crate::boxdraw::section_header;
use crate::diskpane::{
    bytes, label_ink, put, short_path, tile_alpha, tile_bg, tile_colors, tiles, DiskPane,
    MIN_PATH_W,
};
use crate::palette::accent;
use crate::plot::Canvas;
use crew_render::{CellView, Paint};

impl DiskPane {
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        let t = crew_theme::theme();
        let mut out = Vec::new();
        if cols < 20 || rows < 6 {
            return crate::toosmall::note(cols, rows);
        }
        out.extend(section_header(
            "DISK",
            cols,
            t.border_normal,
            accent(),
            t.page_bg,
        ));
        // The reading is placed first and the path takes what is left. Both
        // used to be one string clipped at the pane's edge, so a narrow tile
        // showed a path cut mid-component and no total at all — the two
        // numbers the header exists to say, gone, on the pane where the map is
        // hardest to read. And the path elides from the LEFT: the tail is the
        // directory you are in, the head is the road you took to it.
        let reading = match self.scanning {
            true => format!(
                "{} so far, {} files scanned\u{2026}",
                bytes(self.total),
                self.files
            ),
            false => format!("{} in {} entries", bytes(self.total), self.children.len()),
        };
        let sep = "  \u{2014}  ";
        let reading_w = crate::chatwidth::str_w(&reading) as u16;
        let path_room = (cols.saturating_sub(2))
            .checked_sub(reading_w + sep.chars().count() as u16)
            .filter(|&r| r >= MIN_PATH_W);
        match path_room {
            Some(room) => {
                let path = crate::cwd::fit_legend(&short_path(&self.root), room as usize);
                put(
                    &mut out,
                    &format!("{path}{sep}{reading}"),
                    1,
                    1,
                    t.ink,
                    cols,
                );
            }
            // Too narrow to say both: the reading wins. A path you cannot
            // read is not a path, and the map under it already says where
            // you are by what is in it.
            None => put(&mut out, &reading, 1, 1, t.ink, cols),
        }
        // A scanned directory with nothing in it: the map has no tiles to
        // draw, and a header over a blank read as a map that had not come.
        if !self.scanning && self.children.is_empty() {
            put(&mut out, "empty directory", 1, 3, t.text_muted, cols);
        }

        // A label per tile that has the room for one: name on the first row,
        // size under it. A tile too small for its own name gets none — the
        // area is still the reading.
        let map = tiles(&self.children, cols, rows);
        let colors = tile_colors(&self.children, &map);
        for (i, tile) in map.iter().enumerate() {
            let Some(child) = self.children.get(tile.index) else {
                continue;
            };
            if tile.w < 5.0 || tile.h < 1.0 {
                continue;
            }
            // Ink chosen against the TILE, not against the page. It used to
            // be the page's own ink on the selected tile and the page's own
            // background on every other one — so on a dark theme the picked
            // tile wrote near-white on a bright pastel fill and was the one
            // label on the map you could not read. The ring already says which
            // tile is picked; the label only has to be legible.
            let fg = label_ink(tile_bg(
                colors[i],
                child.is_dir,
                tile.index == self.selected,
            ));
            let room = (tile.w - 1.0) as usize;
            // `vend` is not a directory anybody has. A tile that cuts a name
            // without saying so reads as a complete, wrong name; `ven…` reads
            // as a name that did not fit — which is the truth.
            let name = crate::chatwidth::clip_w(&child.name, room);
            put(
                &mut out,
                &name,
                tile.x as u16 + 1,
                tile.y as u16,
                fg,
                cols.saturating_sub(1),
            );
            if tile.h >= 2.0 {
                put(
                    &mut out,
                    &bytes(child.bytes),
                    tile.x as u16 + 1,
                    tile.y as u16 + 1,
                    fg,
                    cols.saturating_sub(1),
                );
            }
        }
        let hint =
            "\u{2190}\u{2192} pick \u{00b7} enter opens \u{00b7} backspace up \u{00b7} r rescans";
        put(
            &mut out,
            hint,
            1,
            rows.saturating_sub(1),
            t.text_muted,
            cols,
        );
        out
    }

    pub fn paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        let t = crew_theme::theme();
        if cols < 20 || rows < 6 || self.children.is_empty() {
            return Vec::new();
        }
        let mut c = Canvas::new(cols, rows, aspect);
        let map = tiles(&self.children, cols, rows);
        let colors = tile_colors(&self.children, &map);
        for (i, tile) in map.iter().enumerate() {
            let Some(child) = self.children.get(tile.index) else {
                continue;
            };
            // The roster's tag pool, dealt so no two touching tiles match.
            let color = colors[i];
            let (x, y) = (tile.x, tile.y * aspect);
            let (w, h) = (
                (tile.w - 0.08).max(0.05),
                (tile.h * aspect - 0.08).max(0.05),
            );
            let selected = tile.index == self.selected;
            let alpha = tile_alpha(child.is_dir, selected);
            c.rect(x, y, w, h, color, alpha);
            if selected {
                // A ring around the picked tile, drawn as four thin bars: the
                // fill alone cannot say "this one" on a busy map.
                let k = 0.2;
                let ink = t.ink;
                c.rect(x, y, w, k, ink, 1.0);
                c.rect(x, y + h - k, w, k, ink, 1.0);
                c.rect(x, y, k, h, ink, 1.0);
                c.rect(x + w - k, y, k, h, ink, 1.0);
            }
        }
        c.paint()
    }
}
