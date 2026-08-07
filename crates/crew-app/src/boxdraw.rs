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

/// Display columns available to a title on a `cols`-wide rule before it must
/// ellipsize: the leading `─ ` and the trailing ` ─` (breathing room before
/// the right corner) each cost two, and the two corner/edge columns bound the
/// rule — six columns of frame around the text. Shared with the callers that
/// pre-fit their legends (`cwd::fit_legend` budgets) so no one disagrees.
pub fn title_budget(cols: u16) -> usize {
    cols.saturating_sub(6) as usize
}

/// Draw a horizontal rule across `[1..=cols-2]` on row 0 with `title` embedded
/// near the left (`─ TITLE ──────`). The rule uses `border`; the title uses
/// `title_fg`. Callers shift the returned cells to the section's top row.
/// When the title is empty, the entire rule is filled with `─` for a solid border.
///
/// A title wider than [`title_budget`] is ellipsized (`…`), width-aware, so a
/// long legend on a narrow card ends `… ─╮` — never flush against the corner.
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
    let title = crate::chatwidth::clip_w(title, title_budget(cols));
    if title.is_empty() {
        // No title (or no room for one): fill the row with solid border.
        while col <= right {
            v.push(cell(col, 0, '─', border, bg));
            col += 1;
        }
        return v;
    }
    v.push(cell(col, 0, ' ', border, bg));
    col += 1;
    // Width-aware placement: a wide (emoji/CJK) glyph advances two columns,
    // so the trailing rule resumes on the right cell instead of overlapping.
    col = crate::chatwidth::place_row(
        col,
        right,
        title.chars().map(|c| (c, title_fg)),
        |x, c, fg| v.push(cell(x, 0, c, fg, bg)),
    );
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
#[path = "boxdraw_tests.rs"]
mod tests;
