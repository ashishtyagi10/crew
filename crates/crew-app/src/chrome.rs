use crate::layout::Rect;
use crew_render::CellView;

pub const TOP_PAD: f32 = 10.0;
const ACCENT: (u8, u8, u8) = (0, 255, 160);
const TEXT: (u8, u8, u8) = (200, 200, 200);
const BG: (u8, u8, u8) = (8, 8, 16);

pub fn top_bar_h(cell_h: f32) -> f32 {
    cell_h + TOP_PAD
}

pub fn topbar_rect(sw: f32, bar_h: f32) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: sw,
        h: bar_h,
    }
}

pub fn sidebar_rect(sw: f32, sh: f32, bar_h: f32, nav_px: f32) -> Rect {
    let _ = sw;
    Rect {
        x: 0.0,
        y: bar_h,
        w: nav_px,
        h: sh - bar_h,
    }
}

pub fn content_rect(sw: f32, sh: f32, bar_h: f32, show_nav: bool, nav_px: f32) -> Rect {
    let x = if show_nav { nav_px } else { 0.0 };
    Rect {
        x,
        y: bar_h,
        w: sw - x,
        h: sh - bar_h,
    }
}

pub fn toggle_rect(bar_h: f32) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: bar_h,
        h: bar_h,
    }
}

pub fn point_in(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

pub fn topbar_cells(show_nav: bool, cols: u16) -> Vec<CellView> {
    let toggle_glyph = if show_nav { '\u{2637}' } else { '\u{2630}' };
    let content: Vec<(char, (u8, u8, u8))> = std::iter::once((toggle_glyph, ACCENT))
        .chain(std::iter::once((' ', TEXT)))
        .chain("Crew".chars().map(|c| (c, TEXT)))
        .collect();

    content
        .into_iter()
        .take(cols as usize)
        .enumerate()
        .map(|(i, (c, fg))| CellView {
            col: i as u16,
            row: 0,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_rect_no_nav() {
        assert_eq!(
            content_rect(1000.0, 800.0, 30.0, false, 200.0),
            Rect {
                x: 0.0,
                y: 30.0,
                w: 1000.0,
                h: 770.0
            }
        );
    }

    #[test]
    fn content_rect_with_nav() {
        assert_eq!(
            content_rect(1000.0, 800.0, 30.0, true, 200.0),
            Rect {
                x: 200.0,
                y: 30.0,
                w: 800.0,
                h: 770.0
            }
        );
    }

    #[test]
    fn sidebar_rect_dimensions() {
        assert_eq!(
            sidebar_rect(1000.0, 800.0, 30.0, 200.0),
            Rect {
                x: 0.0,
                y: 30.0,
                w: 200.0,
                h: 770.0
            }
        );
    }

    #[test]
    fn point_in_toggle_rect() {
        assert!(point_in(toggle_rect(30.0), 5.0, 5.0));
        assert!(!point_in(toggle_rect(30.0), 100.0, 5.0));
    }

    #[test]
    fn topbar_cells_no_nav_first_cell_is_toggle_hidden() {
        let cells = topbar_cells(false, 20);
        assert!(!cells.is_empty());
        for c in &cells {
            assert_eq!(c.row, 0);
        }
        assert_eq!(cells[0].c, '\u{2630}');
    }

    #[test]
    fn topbar_cells_with_nav_first_cell_is_toggle_shown() {
        let cells = topbar_cells(true, 20);
        assert_eq!(cells[0].c, '\u{2637}');
    }
}
