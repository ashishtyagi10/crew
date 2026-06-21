//! Two-column form rendering for the settings pane: each value sits in a real
//! bordered input box (rounded card with the field name as a legend), plus the
//! font-family dropdown and Save/Cancel buttons.
use crew_render::CellView;

use super::{Field, SettingsPane, ACCENT, BG, DIM, TEXT};
use crate::boxdraw::{titled_box, BoxRect};

const BOX_LEFT: u16 = 2;
const BOX_MAX_W: u16 = 46;
/// Top row of each field's box; boxes are 3 rows tall with a 1-row gap.
const FAM_TOP: u16 = 1;
const SIZE_TOP: u16 = 5;
const NAV_TOP: u16 = 9;
const SHOW_TOP: u16 = 13;
const BTN_ROW: u16 = 17;

/// Render the whole form into a flat `CellView` list.
pub(crate) fn render(p: &SettingsPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 24 || rows < 6 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let f = p.focused_field();
    let nav = if p.draft.show_nav { "on" } else { "off" };

    // (top row, legend, value, field, draw block cursor)
    let boxes: [(u16, &str, &str, Field, bool); 4] = [
        (
            FAM_TOP,
            "Font family",
            &p.family_query,
            Field::FontFamily,
            true,
        ),
        (SIZE_TOP, "Font size", &p.size_buf, Field::FontSize, true),
        (NAV_TOP, "Nav width", &p.nav_buf, Field::NavWidth, true),
        (SHOW_TOP, "Show nav", nav, Field::ShowNav, false),
    ];
    for &(top, legend, value, field, cursor) in &boxes {
        input_box(&mut out, top, legend, value, f == field, cursor, cols);
    }

    button(
        &mut out,
        BOX_LEFT + 2,
        BTN_ROW,
        "Save",
        f == Field::Save,
        cols,
    );
    button(
        &mut out,
        BOX_LEFT + 14,
        BTN_ROW,
        "Cancel",
        f == Field::Cancel,
        cols,
    );

    if p.family_open {
        dropdown(&mut out, p, cols, rows);
    }
    out
}

/// Right column of every input box, clamped to the pane width.
fn box_right(cols: u16) -> u16 {
    (BOX_LEFT + BOX_MAX_W).min(cols.saturating_sub(2))
}

/// A bordered input box: rounded card, field name in the legend, value inside.
fn input_box(
    out: &mut Vec<CellView>,
    top: u16,
    legend: &str,
    value: &str,
    focused: bool,
    cursor: bool,
    cols: u16,
) {
    let right = box_right(cols);
    if right <= BOX_LEFT + 2 {
        return;
    }
    let border = if focused { ACCENT } else { DIM };
    let leg_fg = if focused { ACCENT } else { TEXT };
    out.extend(titled_box(
        BoxRect {
            left: BOX_LEFT,
            top,
            right,
            bottom: top + 2,
        },
        legend,
        border,
        leg_fg,
        BG,
    ));
    let avail = (right - BOX_LEFT - 2) as usize;
    let mut val: String = value.chars().take(avail.saturating_sub(1)).collect();
    if cursor && focused {
        val.push('█');
    }
    let vfg = if focused { ACCENT } else { TEXT };
    put(out, BOX_LEFT + 2, top + 1, &val, vfg, cols, false);
}

/// A `[ Label ]` button; accent + bold when focused.
fn button(out: &mut Vec<CellView>, col: u16, row: u16, label: &str, focused: bool, cols: u16) {
    let fg = if focused { ACCENT } else { DIM };
    put(out, col, row, &format!("[ {label} ]"), fg, cols, focused);
}

/// The type-to-search font list, drawn over the rows below the family box.
fn dropdown(out: &mut Vec<CellView>, p: &SettingsPane, cols: u16, rows: u16) {
    let list = p.filtered();
    let start = FAM_TOP + 3;
    let avail = rows.saturating_sub(start + 1) as usize;
    let width = (box_right(cols) - BOX_LEFT) as usize;
    for (i, name) in list.iter().take(avail.min(8)).enumerate() {
        let row = start + i as u16;
        let selected = i == p.family_sel;
        let fg = if selected { ACCENT } else { TEXT };
        let marker = if selected { "> " } else { "  " };
        let mut line = format!("{marker}{name}");
        while line.chars().count() < width {
            line.push(' ');
        }
        put(out, BOX_LEFT, row, &line, fg, cols, selected);
    }
}

fn put(
    out: &mut Vec<CellView>,
    col: u16,
    row: u16,
    s: &str,
    fg: (u8, u8, u8),
    cols: u16,
    bold: bool,
) {
    for (i, c) in s.chars().enumerate() {
        let cc = col + i as u16;
        if cc >= cols {
            break;
        }
        out.push(CellView {
            col: cc,
            row,
            c,
            fg,
            bg: BG,
            bold,
            italic: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CrewConfig;

    #[test]
    fn renders_bordered_input_boxes() {
        let p = SettingsPane::new(CrewConfig::default(), Vec::new());
        let cells = p.cells(60, 20);
        // rounded box corners are present (real borders, not just brackets)
        assert!(cells.iter().any(|c| c.c == '╭'));
        assert!(cells.iter().any(|c| c.c == '╰'));
        // the field legend renders in the top border
        assert!(cells.iter().any(|c| c.c == 'F'));
    }

    #[test]
    fn tiny_pane_renders_nothing() {
        let p = SettingsPane::new(CrewConfig::default(), Vec::new());
        assert!(p.cells(10, 4).is_empty());
    }
}
