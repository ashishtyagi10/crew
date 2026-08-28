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

/// Where a pane's read mark sits this frame.
///
/// Watching is reading. The pane you are focused on, sitting at its live
/// bottom, has its output land in front of your eyes as it arrives — so its
/// mark follows the tail, and a rule is never left stranded in the middle of
/// output you are looking at. Everything else keeps the mark it has: an
/// unfocused pane is the case this whole module exists for, and a focused
/// pane you have scrolled back in is one you are still catching up on.
///
/// This used to ask for `count(total, read_at) == 0` before advancing, which
/// is only ever true when the mark is *already* at the tail — a guard that
/// could not fire once a single line had arrived. The rule it was meant to
/// state is the one `scroll` and `termwrite` already state: at the live
/// bottom you have seen everything above you.
pub(crate) fn follow_tail(read_at: usize, focused: bool, at_bottom: bool, total: usize) -> usize {
    match focused && at_bottom {
        true => total,
        false => read_at,
    }
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

/// The tag the rule carries, and the column it starts at, for a row whose own
/// text ends at `last_ink`.
///
/// A full-width line with nothing saying what it is reads as damage — a
/// rendering fault, a stray box-drawing character, the artefact of a crash —
/// which is exactly what it was reported as. So the rule names itself, in the
/// same words the card's border badge uses (`12`, `99+`), right-aligned where
/// the row's own output has already stopped.
///
/// `None` when the row's text reaches that far: covering a line of output to
/// explain a rule is a worse trade than leaving the rule bare.
fn tag(new: usize, cols: u16, last_ink: Option<u16>) -> Option<(u16, String)> {
    let s = format!("{} new", badge(new)?);
    let w = s.chars().count() as u16;
    let start = cols.checked_sub(w)?;
    // Two blank columns between the output and the tag, so the count reads as
    // a mark on the rule rather than as the last word of that line.
    match last_ink {
        Some(ink) if ink + 2 >= start => None,
        _ => Some((start, s)),
    }
}

/// The tag's character for column `col`, if it covers that column.
fn tag_char(tag: &Option<(u16, String)>, col: u16) -> Option<char> {
    let (start, s) = tag.as_ref()?;
    s.chars().nth(usize::from(col.checked_sub(*start)?))
}

/// Rule every cell on `row`, so the boundary reads as a line drawn between
/// two lines of output rather than as a decoration on one of them, and hang
/// the count of what is below it off the right end.
pub(crate) fn mark(cells: &mut Vec<CellView>, row: u16, cols: u16, new: usize) {
    let fg = crew_theme::theme().activity;
    // Where this row's own output stops. Trailing blanks are the tag's to
    // use; a glyph is not.
    let last_ink = cells
        .iter()
        .filter(|c| c.row == row && c.c != ' ')
        .map(|c| c.col)
        .max();
    let tag = tag(new, cols, last_ink);
    let mut seen: Vec<u16> = Vec::new();
    for c in cells.iter_mut().filter(|c| c.row == row) {
        c.deco.line = DecoLine::Single;
        c.deco.color = Some(fg);
        if let Some(ch) = tag_char(&tag, c.col) {
            c.c = ch;
            c.fg = fg;
        }
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
            c: tag_char(&tag, col).unwrap_or(' '),
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
