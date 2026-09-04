//! The one line a drawn pane shows when it is too small to draw.
//!
//! `/usage` below 24×6 and `/disk` below 20×6 returned no cells at all. The
//! card kept its border and legend, so a short tile looked like a rendering
//! fault rather than "too small" — and the way out (zoom it) was the one
//! thing the blank did not say.
use crew_render::CellView;

/// What the note says. One row, muted, clipped to the card.
pub(crate) const NOTE: &str = "too small \u{b7} Cmd+Z zooms";

/// The note as cells for a `cols × rows` interior, or nothing when there is
/// not even a row to say it in.
pub(crate) fn note(cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 4 || rows < 2 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    crate::chatwidth::clip_w(NOTE, usize::from(cols - 2))
        .chars()
        .enumerate()
        .map(|(i, c)| CellView {
            col: 1 + i as u16,
            row: 1,
            c,
            fg: t.text_muted,
            bg: t.page_bg,
            ..Default::default()
        })
        .collect()
}

/// One muted row of `text` at the top-left of a `cols`-wide card — the
/// viewer's word for a file with nothing in it.
pub(crate) fn row(text: &str, cols: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    crate::chatwidth::clip_w(text, usize::from(cols))
        .chars()
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row: 0,
            c,
            fg: t.text_muted,
            bg: t.page_bg,
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_fits_the_card_and_marks_its_cut() {
        let _g = crate::app::theme_test_guard();
        let text = |cells: &[CellView]| cells.iter().map(|c| c.c).collect::<String>();
        assert_eq!(text(&note(40, 4)), NOTE);
        let cut = text(&note(12, 4));
        assert!(cut.ends_with('\u{2026}'), "{cut:?}");
        assert!(note(12, 4).iter().all(|c| c.col < 11));
        assert!(note(3, 4).is_empty());
        assert!(note(40, 1).is_empty());
    }

    fn text(cells: Vec<CellView>) -> String {
        cells.iter().map(|c| c.c).collect()
    }

    /// `/usage` below its minimum says so instead of returning no cells.
    #[test]
    fn usage_says_too_small() {
        let _g = crate::app::theme_test_guard();
        use crate::usageledger::{DAYS, HOURS};
        let b = crate::usageledger::Buckets {
            hourly: vec![0u64; DAYS * HOURS],
            daily_cost: vec![0; DAYS],
            tok_in: 0,
            tok_out: 0,
            cost_microusd: 0,
        };
        assert!(text(crate::usagepane::cells(&b, 20, 40)).contains("too small"));
        assert!(crate::usagepane::paint(&b, 20, 40, 2.0).is_empty());
        assert!(text(crate::usagepane::cells(&b, 60, 4)).contains("too small"));
    }

    /// `/disk` below its minimum says so too, and paints nothing.
    #[test]
    fn disk_says_too_small() {
        let _g = crate::app::theme_test_guard();
        let p = crate::diskpane::DiskPane::new(std::env::temp_dir());
        assert!(text(p.cells(16, 20)).contains("too small"));
        assert!(p.paint(16, 20, 2.0).is_empty());
    }

    /// A scanned directory with nothing in it says that — a header over a
    /// blank read as a map that had not come yet.
    #[test]
    fn an_empty_directory_is_said_once_the_scan_is_done() {
        let _g = crate::app::theme_test_guard();
        let mut p = crate::diskpane::DiskPane::new(std::env::temp_dir());
        p.scanning = false;
        p.children.clear();
        assert!(text(p.cells(60, 12)).contains("empty directory"));
        p.scanning = true;
        assert!(!text(p.cells(60, 12)).contains("empty directory"));
    }

    /// A zero-byte file says so; it used to be a titled empty box.
    #[test]
    fn an_empty_file_says_it_is_empty() {
        use crate::viewpane::{detect::Format, load::Loaded, LoadState, ViewPane};
        let _g = crate::app::theme_test_guard();
        let pane = |s: &str| {
            let mut p = ViewPane::open(std::env::temp_dir().join("e.txt"));
            p.state = LoadState::Ready {
                format: Format::Code { lang: "" },
                loaded: Loaded {
                    text: s.into(),
                    truncated: None,
                    meta: None,
                    image: None,
                },
            };
            p
        };
        assert_eq!(text(pane("").cells(40, 10)), "(empty file)");
        assert!(!text(pane("x").cells(40, 10)).contains("(empty file)"));
    }
}
