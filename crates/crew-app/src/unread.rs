//! The line you had read up to, in a pane you were not looking at.
//!
//! A grid of panes means most of them are producing output while you are
//! reading one of the others. Coming back to a pane, the question is always
//! the same — *where does the new part start* — and the answer used to be
//! "scroll up until you recognise something".
//!
//! So each terminal pane remembers how many lines its buffer held when you
//! last read it, and the boundary between then and now is drawn as a rule
//! under the last line you had seen. Not a banner row: a banner would cover a
//! line of output, and the thing being marked is a *gap between* lines, which
//! is exactly what an underline on the row above is.
use crew_render::CellView;
use crew_theme::deco::DecoLine;

/// Where the divider goes for a pane whose buffer now holds `total` lines,
/// was read at `read_at`, and is showing `visible` rows ending `scroll` lines
/// back from the bottom.
///
/// `None` when there is nothing new, when the boundary is off screen (above
/// or below the viewport), or when it falls on the very last visible row —
/// there is nothing after it to divide.
pub(crate) fn divider_row(
    total: usize,
    read_at: usize,
    visible: usize,
    scroll: usize,
) -> Option<u16> {
    if total <= read_at || visible == 0 {
        return None;
    }
    // The buffer line index of the top visible row.
    let first = total.saturating_sub(visible).saturating_sub(scroll);
    // The rule sits under the last line that was read, which is the row of
    // buffer line `read_at - 1`.
    let boundary = read_at.checked_sub(1)?;
    let row = boundary.checked_sub(first)?;
    (row + 1 < visible).then_some(row as u16)
}

/// How many lines arrived since the pane was read. Saturating: a cleared
/// buffer (`/clear`) is shorter than it was, and that is not negative news.
pub(crate) fn count(total: usize, read_at: usize) -> usize {
    total.saturating_sub(read_at)
}

/// The count as it rides the card's top border. Capped, because a pane that
/// produced four thousand lines while you were away is saying "a lot" and
/// four digits of border says it no better than three.
pub(crate) fn badge(n: usize) -> Option<String> {
    match n {
        0 => None,
        1..=99 => Some(n.to_string()),
        _ => Some("99+".to_string()),
    }
}

/// Rule every cell on `row`, so the boundary reads as a line drawn between
/// two lines of output rather than as a decoration on one of them.
pub(crate) fn mark(cells: &mut Vec<CellView>, row: u16, cols: u16) {
    let fg = crew_theme::theme().activity;
    let mut seen: Vec<u16> = Vec::new();
    for c in cells.iter_mut().filter(|c| c.row == row) {
        c.deco.line = DecoLine::Single;
        c.deco.color = Some(fg);
        seen.push(c.col);
    }
    // The row is mostly blank in a terminal, and a rule with gaps in it is a
    // dashed line nobody drew on purpose — fill the columns no cell occupies.
    for col in 0..cols {
        if seen.contains(&col) {
            continue;
        }
        cells.push(CellView {
            col,
            row,
            c: ' ',
            fg,
            bg: crew_theme::theme().page_bg,
            deco: crew_theme::deco::Deco {
                line: DecoLine::Single,
                color: Some(fg),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}

#[cfg(test)]
#[path = "unread_tests.rs"]
mod tests;
