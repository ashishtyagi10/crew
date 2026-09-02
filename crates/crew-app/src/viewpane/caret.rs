//! A cursor that lives in the rendered document.
//!
//! Not in the source. The whole point of the editor is that you never see a
//! `**` or a `#`, so the thing you move with the arrow keys has to be a place
//! in the *render* — and every place in the render that came from the file
//! carries the byte it came from ([`crate::md::source`]). The caret is
//! therefore a rendered position, and its offset is read off the cell under
//! it whenever the file has to be touched.
//!
//! **A cell with no offset is not a place the caret can be.** A list bullet, a
//! table's rules, the border of a code field, the space a soft break became:
//! the renderer put those there and the file does not contain them, so there
//! is nothing to type into and the caret steps over them. That is not a
//! limitation to be worked around later — it is the correct behaviour, and it
//! falls out of the provenance rather than being a rule anybody has to write.
use crate::chatbody::CardLine;

/// Where the caret is, in rendered coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Caret {
    pub row: usize,
    /// Display column — what the pane draws at, counting a wide glyph as two.
    pub col: u16,
    /// The column vertical movement aims for, kept across short rows so a
    /// run of Down keys through a ragged document comes back to where it
    /// started instead of walking left.
    pub want: u16,
}

/// The row's caret positions, in order, as `(display column, source byte)`.
///
/// A caret sits **before** the character it is drawn on, which is what makes
/// typing insert where you are looking. That leaves one position no character
/// can provide: the one *after* the last character, where a line is extended
/// and a document is appended to. So each row ends with a stop the renderer
/// drew nothing at — one column past its last sourced character, holding the
/// byte just after it. Without it there is nowhere to stand at the end of a
/// document, and nothing can ever be added to one.
pub(super) fn stops(line: &CardLine) -> Vec<(u16, u32)> {
    let mut out = Vec::new();
    let mut col: u16 = 0;
    let mut end: Option<(u16, u32)> = None;
    for cell in line {
        let w = crate::chatwidth::char_w(cell.c) as u16;
        if w == 0 {
            continue;
        }
        if let Some(src) = cell.src {
            out.push((col, src));
            end = Some((col + w, src + cell.c.len_utf8() as u32));
        }
        col += w;
    }
    out.extend(end);
    out
}

/// The offset the caret is on, if it is on anything.
pub(crate) fn offset_at(lines: &[CardLine], c: Caret) -> Option<u32> {
    let row = lines.get(c.row)?;
    stops(row)
        .into_iter()
        .find(|&(col, _)| col == c.col)
        .map(|(_, s)| s)
}

/// The first place in the document a caret can be.
pub(crate) fn first(lines: &[CardLine]) -> Option<Caret> {
    lines.iter().enumerate().find_map(|(row, l)| {
        let (col, _) = *stops(l).first()?;
        Some(Caret {
            row,
            col,
            want: col,
        })
    })
}

/// The last place in the document a caret can be.
pub(crate) fn last(lines: &[CardLine]) -> Option<Caret> {
    lines
        .iter()
        .rev()
        .find_map(|l| {
            let (col, _) = *stops(l).last()?;
            Some(Caret {
                row: 0,
                col,
                want: col,
            })
        })
        .and_then(|c| {
            let row = lines.iter().rposition(|l| !stops(l).is_empty())?;
            Some(Caret { row, ..c })
        })
}

/// Which way a step goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    /// A page of rows, either way. The page is the window's height, which
    /// only the caller knows — a classifier without it would be guessing.
    Page {
        down: bool,
        rows: usize,
    },
    /// The document's first / last place.
    Top,
    Bottom,
}

/// The caret after one step, or the caret unchanged at either end of the
/// document — a cursor that wraps from the last character to the first is a
/// cursor nobody can hold on to.
pub(crate) fn step(lines: &[CardLine], c: Caret, dir: Step) -> Caret {
    match dir {
        Step::Left | Step::Right => horizontal(lines, c, dir == Step::Right),
        Step::Up | Step::Down => vertical(lines, c, dir == Step::Down),
        // Rows the caret cannot stand on are skipped, not counted: a page is
        // a page of places, and the fold stops moving at either end anyway.
        Step::Page { down, rows } => (0..rows.max(1)).fold(c, |c, _| vertical(lines, c, down)),
        Step::Top => first(lines).unwrap_or(c),
        Step::Bottom => last(lines).unwrap_or(c),
        Step::Home | Step::End => {
            let row = lines.get(c.row).map(stops).unwrap_or_default();
            let at = match dir {
                Step::Home => row.first(),
                _ => row.last(),
            };
            match at {
                Some(&(col, _)) => Caret {
                    row: c.row,
                    col,
                    want: col,
                },
                None => c,
            }
        }
    }
}

fn horizontal(lines: &[CardLine], c: Caret, right: bool) -> Caret {
    let here = lines.get(c.row).map(stops).unwrap_or_default();
    let next = match right {
        true => here.iter().find(|&&(col, _)| col > c.col),
        false => here.iter().rev().find(|&&(col, _)| col < c.col),
    };
    if let Some(&(col, _)) = next {
        return Caret {
            row: c.row,
            col,
            want: col,
        };
    }
    // Off the end of this row: the next row with anything on it. Rows the
    // renderer filled entirely with its own furniture are stepped over, not
    // stopped on.
    let rows: Vec<usize> = match right {
        true => (c.row + 1..lines.len()).collect(),
        false => (0..c.row).rev().collect(),
    };
    for row in rows {
        let s = stops(&lines[row]);
        let at = match right {
            true => s.first(),
            false => s.last(),
        };
        if let Some(&(col, _)) = at {
            return Caret {
                row,
                col,
                want: col,
            };
        }
    }
    c
}

fn vertical(lines: &[CardLine], c: Caret, down: bool) -> Caret {
    let rows: Vec<usize> = match down {
        true => (c.row + 1..lines.len()).collect(),
        false => (0..c.row).rev().collect(),
    };
    for row in rows {
        let s = stops(&lines[row]);
        if s.is_empty() {
            continue;
        }
        // The column nearest the one vertical movement is aiming for, never
        // past it — the same rule every editor uses, and the reason `want` is
        // carried rather than recomputed from where we landed.
        let col = s
            .iter()
            .rev()
            .find(|&&(col, _)| col <= c.want)
            .map(|&(col, _)| col)
            .unwrap_or(s[0].0);
        return Caret {
            row,
            col,
            want: c.want,
        };
    }
    c
}

#[cfg(test)]
#[path = "caret_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "caretjump_tests.rs"]
mod jumps;
