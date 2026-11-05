//! The strokes of a tab's card in the nav ([`crate::tabnav`]): its titled
//! top edge, its sides beside each pane row, its bottom edge, and the rail's
//! one-row divider. The card spans the same columns as the section rules
//! above it — 1 to `cols - 2` — so it sits flush inside the nav's margins.
use crew_render::CellView;

use crate::tabnav::TabBlock;

type Rgb = (u8, u8, u8);

fn cell(col: u16, row: u16, c: char, fg: Rgb, bold: bool) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: crew_theme::theme().page_bg,
        bold,
        ..Default::default()
    }
}

/// Write `s` from `col`, stopping before `max`; returns the next free column.
fn put(out: &mut Vec<CellView>, s: &str, col: u16, row: u16, max: u16, fg: Rgb, bold: bool) -> u16 {
    crate::chatwidth::place_row(col, max, s.chars().map(|c| (c, ())), |x, c, marks, _| {
        let mut v = cell(x, row, c, fg, bold);
        v.marks = marks;
        out.push(v);
    })
}

/// `╭─ 2 · web ────── 3 ─╮`: the number in the frame's colour, the name lit
/// on the tab on screen (and under the pointer), the pane count quiet.
pub(crate) fn top(b: &TabBlock, cols: u16, row: u16, frame: Rgb) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if cols < 8 {
        return out;
    }
    let right = cols - 2;
    let count = b.rows.len().to_string();
    // ` 3 ─╮` from the right edge in; the count goes first on a narrow nav.
    let count_at = right.saturating_sub(3 + count.len() as u16);
    let legend_end = if count_at > 10 {
        count_at - 1
    } else {
        right - 1
    };
    for x in 1..=right {
        let c = match x {
            1 => '\u{256d}',
            x if x == right => '\u{256e}',
            _ => '\u{2500}',
        };
        out.push(cell(x, row, c, frame, false));
    }
    let mut legend = Vec::new();
    let num = format!(" {} ", b.num);
    let mut x = put(&mut legend, &num, 3, row, legend_end, frame, true);
    let name_fg = if b.active || b.hovered {
        t.ink
    } else {
        t.text_muted
    };
    let room = legend_end.saturating_sub(x + 1) as usize;
    let name = crate::chatwidth::clip_words(&b.title, room);
    if !name.is_empty() {
        x = put(
            &mut legend,
            &format!("\u{b7} {name} "),
            x,
            row,
            legend_end,
            name_fg,
            b.active,
        );
    }
    if count_at > 10 && x < count_at {
        put(
            &mut legend,
            &format!(" {count} "),
            count_at,
            row,
            right,
            t.text_muted,
            false,
        );
    }
    // The legend overwrites the rule beneath it: the last writer is drawn.
    let covered: Vec<u16> = legend.iter().map(|c| c.col).collect();
    out.retain(|c| !covered.contains(&c.col));
    out.extend(legend);
    out
}

/// The card's two sides on a pane row.
pub(crate) fn sides(cols: u16, row: u16, frame: Rgb) -> Vec<CellView> {
    if cols < 8 {
        return Vec::new();
    }
    vec![
        cell(1, row, '\u{2502}', frame, false),
        cell(cols - 2, row, '\u{2502}', frame, false),
    ]
}

/// `╰──────╯`.
pub(crate) fn bottom(cols: u16, row: u16, frame: Rgb) -> Vec<CellView> {
    if cols < 8 {
        return Vec::new();
    }
    let right = cols - 2;
    (1..=right)
        .map(|x| {
            let c = match x {
                1 => '\u{2570}',
                x if x == right => '\u{256f}',
                _ => '\u{2500}',
            };
            cell(x, row, c, frame, false)
        })
        .collect()
}

/// The rail's tab marker: `─2───`, lit for the tab on screen.
pub(crate) fn divider(b: &TabBlock, cols: u16, row: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let (rule, num) = match b.active {
        true => (
            crate::accentink::accent_ink(),
            crate::accentink::accent_ink(),
        ),
        false => (t.border_normal, t.text_muted),
    };
    let label: Vec<char> = b.num.to_string().chars().collect();
    (0..cols)
        .map(|x| match label.get(usize::from(x).wrapping_sub(1)) {
            Some(&c) if x >= 1 => cell(x, row, c, num, b.active),
            _ => cell(x, row, '\u{2500}', rule, false),
        })
        .collect()
}
