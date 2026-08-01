//! Box-drawing helpers in the fieldset/legend style: a horizontal `section_header`
//! divider (`─ TITLE ─────`) for stacking sidebar sections, and a full
//! `titled_card` rounded box with the legend embedded in its top border — used by
//! the input bar so its working-directory legend sits on the frame.
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

/// Draw a horizontal rule across `[1..=cols-2]` on row 0 with `title` embedded
/// near the left (`─ TITLE ──────`). The rule uses `border`; the title uses
/// `title_fg`. Callers shift the returned cells to the section's top row.
/// When the title is empty, the entire rule is filled with `─` for a solid border.
pub fn section_header(
    title: &str,
    cols: u16,
    border: (u8, u8, u8),
    title_fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> Vec<CellView> {
    let mut v = Vec::new();
    if cols < 4 {
        return v;
    }
    let right = cols - 2; // inclusive last column of the rule
    let mut col = 1u16;
    v.push(cell(col, 0, '─', border, bg));
    col += 1;
    if title.is_empty() {
        // No title: fill the entire row with solid border
        while col <= right {
            v.push(cell(col, 0, '─', border, bg));
            col += 1;
        }
        return v;
    }
    if col <= right {
        v.push(cell(col, 0, ' ', border, bg));
        col += 1;
    }
    for tc in title.chars() {
        if col > right {
            break;
        }
        v.push(cell(col, 0, tc, title_fg, bg));
        col += 1;
    }
    if col <= right {
        v.push(cell(col, 0, ' ', border, bg));
        col += 1;
    }
    while col <= right {
        v.push(cell(col, 0, '─', border, bg));
        col += 1;
    }
    v
}

/// Draw a full rounded card filling `cols × rows` with `title` embedded in the
/// top border (`╭─ TITLE ─────╮`) and the interior left blank for the caller to
/// fill. Border glyphs use `border`; the legend uses `title_fg`.
pub fn titled_card(
    cols: u16,
    rows: u16,
    title: &str,
    border: (u8, u8, u8),
    title_fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> Vec<CellView> {
    let mut v = Vec::new();
    if cols < 4 || rows < 2 {
        return v;
    }
    let (right, bottom) = (cols - 1, rows - 1);
    // Top edge: the section-header rule (cols 1..=cols-2) plus the two corners.
    v.extend(section_header(title, cols, border, title_fg, bg));
    v.push(cell(0, 0, '╭', border, bg));
    v.push(cell(right, 0, '╮', border, bg));
    v.push(cell(0, bottom, '╰', border, bg));
    v.push(cell(right, bottom, '╯', border, bg));
    for r in 1..bottom {
        v.push(cell(0, r, '│', border, bg));
        v.push(cell(right, r, '│', border, bg));
    }
    for c in 1..right {
        v.push(cell(c, bottom, '─', border, bg));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_header_has_rule_and_legend() {
        let cells = section_header("SYS", 16, (70, 130, 140), (0, 255, 160), (8, 8, 16));
        // a horizontal rule is drawn on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        // the legend sits on the same row in the title colour
        assert!(cells.iter().any(|c| c.c == 'S' && c.row == 0));
        // no box glyphs — this is a flat divider, not a card
        assert!(!cells
            .iter()
            .any(|c| matches!(c.c, '╭' | '╮' | '╰' | '╯' | '│')));
    }

    #[test]
    fn section_header_too_narrow_is_empty() {
        assert!(section_header("x", 3, (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
    }

    #[test]
    fn titled_card_has_corners_and_legend() {
        let cells = titled_card(20, 3, "~/code", (110, 110, 120), (0, 255, 160), (0, 0, 0));
        let has = |ch: char| cells.iter().any(|c| c.c == ch);
        assert!(has('╭') && has('╮') && has('╰') && has('╯'));
        // legend on the top border, in the title colour
        assert!(cells
            .iter()
            .any(|c| c.c == '~' && c.row == 0 && c.fg == (0, 255, 160)));
        // side borders on the interior row
        assert!(cells.iter().any(|c| c.c == '│' && c.row == 1 && c.col == 0));
    }

    #[test]
    fn titled_card_too_small_is_empty() {
        assert!(titled_card(3, 3, "x", (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
        assert!(titled_card(20, 1, "x", (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
    }
}
