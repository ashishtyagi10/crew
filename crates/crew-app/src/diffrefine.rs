//! Word-level marks for a ```diff fence, wherever one is rendered.
//!
//! The viewer's diff rung pairs each removed line with the added line that
//! replaced it and draws only the run that differs at full strength
//! ([`crate::viewpane::diffpaint`]). A diff an agent pastes into chat, or one
//! inside a markdown file, went through a different path entirely — the
//! markdown renderer, which colours a line at a time and knows nothing about
//! the line after it.
//!
//! Rather than teach the renderer about pairs, this reads the *rendered*
//! lines back: a line whose ink is the added or removed slot is an added or
//! removed line, whatever produced it. One refinement, two surfaces.
use crate::chatbody::CardLine;
use crate::md::syntax::Token;
use crate::viewpane::diffpaint::refine;

/// How far the shared text of a refined pair recedes toward the page.
const DIM: f32 = 0.45;

/// Which side of a diff a rendered line is on, read off its ink.
fn side(line: &CardLine) -> Option<Token> {
    let ink = line.iter().find(|c| !c.c.is_whitespace())?.fg;
    [Token::Added, Token::Removed]
        .into_iter()
        .find(|&tok| ink == crate::chatink::token_fg(tok))
}

/// The line's text from its first non-space cell on — a rendered diff line
/// carries the fence card's indent, and the marker column is what `refine`
/// counts from.
fn body(line: &CardLine) -> (usize, String) {
    let start = line
        .iter()
        .position(|c| !c.c.is_whitespace())
        .unwrap_or(line.len());
    (start, line[start..].iter().map(|c| c.c).collect())
}

/// Dim everything on `line` except the marked run, and bold the run.
fn apply(line: &mut CardLine, start: usize, range: (usize, usize)) {
    let page = crew_theme::theme().page_bg;
    for (i, cell) in line.iter_mut().enumerate().skip(start) {
        // `+1`: the range is into the text after the `+`/`-` marker, and the
        // marker itself always stays at full strength.
        let at = i - start;
        if at == 0 || (range.0 + 1..range.1 + 1).contains(&at) {
            cell.bold = at != 0;
            continue;
        }
        cell.fg = crate::anim::lerp_rgb(cell.fg, page, DIM);
    }
}

/// Refine every removed/added pair in `lines`, in place.
///
/// Conservative in the same two ways the viewer's rung is: runs of unequal
/// length are not paired at all, and a pair that differs almost everywhere is
/// left as it is.
pub(crate) fn refine_lines(lines: &mut [CardLine]) {
    let sides: Vec<Option<Token>> = lines.iter().map(side).collect();
    let mut i = 0;
    while i < lines.len() {
        if sides[i] != Some(Token::Removed) {
            i += 1;
            continue;
        }
        let del_end = (i..lines.len())
            .find(|&k| sides[k] != Some(Token::Removed))
            .unwrap_or(lines.len());
        let add_end = (del_end..lines.len())
            .find(|&k| sides[k] != Some(Token::Added))
            .unwrap_or(lines.len());
        let (del, add) = (i..del_end, del_end..add_end);
        if !del.is_empty() && del.len() == add.len() {
            for (d, a) in del.clone().zip(add.clone()) {
                let ((ds, dtext), (as_, atext)) = (body(&lines[d]), body(&lines[a]));
                let (Some(dcut), Some(acut)) = (dtext.get(1..), atext.get(1..)) else {
                    continue;
                };
                if let Some((r, w)) = refine(dcut, acut) {
                    apply(&mut lines[d], ds, r);
                    apply(&mut lines[a], as_, w);
                }
            }
        }
        i = add_end.max(del_end);
    }
}

#[cfg(test)]
#[path = "diffrefine_tests.rs"]
mod tests;
