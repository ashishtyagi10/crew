//! Box-drawing helpers for grouping sidebar sections into bordered cards with an
//! inline legend embedded in the top border — the HTML `<fieldset>`/`<legend>`
//! pattern: `╭─ TITLE ──────╮`.
use crew_render::CellView;

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg,
        bold: false,
        italic: false,
    }
}

/// Draw a rounded box `[left..=right] x [top..=bottom]` with `title` embedded in
/// the top border (`╭─ TITLE ──╮`). Border glyphs use `border`; the title uses
/// `title_fg`.
pub fn titled_box(
    left: u16,
    top: u16,
    right: u16,
    bottom: u16,
    title: &str,
    border: (u8, u8, u8),
    title_fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> Vec<CellView> {
    let mut v = Vec::new();
    if right <= left || bottom <= top {
        return v;
    }
    // Corners + vertical sides + bottom edge.
    v.push(cell(left, top, '╭', border, bg));
    v.push(cell(right, top, '╮', border, bg));
    v.push(cell(left, bottom, '╰', border, bg));
    v.push(cell(right, bottom, '╯', border, bg));
    for row in (top + 1)..bottom {
        v.push(cell(left, row, '│', border, bg));
        v.push(cell(right, row, '│', border, bg));
    }
    for col in (left + 1)..right {
        v.push(cell(col, bottom, '─', border, bg));
    }

    // Top edge with the legend: ─ space TITLE space ─…
    let mut col = left + 1;
    if col < right {
        v.push(cell(col, top, '─', border, bg));
        col += 1;
    }
    if col < right {
        v.push(cell(col, top, ' ', border, bg));
        col += 1;
    }
    for tc in title.chars() {
        if col >= right {
            break;
        }
        v.push(cell(col, top, tc, title_fg, bg));
        col += 1;
    }
    if col < right {
        v.push(cell(col, top, ' ', border, bg));
        col += 1;
    }
    while col < right {
        v.push(cell(col, top, '─', border, bg));
        col += 1;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titled_box_has_corners_and_legend() {
        let cells = titled_box(
            0,
            0,
            14,
            4,
            "SYS",
            (70, 130, 140),
            (0, 255, 160),
            (8, 8, 16),
        );
        let has = |ch: char| cells.iter().any(|c| c.c == ch);
        assert!(has('╭') && has('╮') && has('╰') && has('╯'));
        // legend sits on the top border row
        assert!(cells.iter().any(|c| c.c == 'S' && c.row == 0));
    }

    #[test]
    fn titled_box_degenerate_is_empty() {
        assert!(titled_box(5, 5, 5, 5, "x", (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
    }
}
