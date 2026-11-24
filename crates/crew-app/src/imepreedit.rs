//! Text an input method is still composing, drawn where it will land.
//!
//! With a Japanese, Chinese or Korean input source — or an Option+e accent
//! waiting for its letter — the Mac holds the keys as *preedit*: text that is
//! not typed yet and reaches crew only as `Ime::Preedit`. Crew used to drop
//! it, so whatever you were composing was invisible until it committed, and
//! the candidate window opened wherever the window's origin happened to be.
//!
//! This keeps the composition ([`ImeState::note`]) and lays it over the cells
//! at the caret ([`overlay`]): underlined in the accent ink, a wide glyph one
//! cell two columns wide, as the grid draws every CJK character. The pty never
//! sees it — the grid under it comes back the moment it commits or is
//! dropped. The window learns the caret's rect from [`cell_rect`], so the
//! candidate list opens under the text it is choosing for.
use crew_render::CellView;
use crew_theme::deco::{CursorMark, CursorShape, Deco, DecoLine};
use winit::event::Ime;

use crate::chatwidth::char_w;

/// Who the composition is for: the input bar, or the terminal pane at this
/// index. A preedit belongs to the surface that had focus when it began.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Owner {
    Bar,
    Pane(usize),
}

/// The text being composed, its caret (a byte offset into `text`, as winit
/// hands it; `None` when the input method hides it) and its owner.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Preedit {
    pub text: String,
    pub caret: Option<usize>,
    pub owner: Owner,
}

/// The window's IME, as crew last left it.
#[derive(Default)]
pub(crate) struct ImeState {
    /// What the window was last told about IME input (`composeinput`).
    pub on: Option<bool>,
    /// The composition in flight, if any.
    pub preedit: Option<Preedit>,
    /// The caret rect last sent to the window, `[x, y, w, h]` in physical px:
    /// sent again only when it changes, as the caret moves at most once a frame.
    pub area: Option<[i32; 4]>,
}

impl ImeState {
    /// Take one IME event for `owner`. A non-empty preedit replaces the one
    /// held; an empty one, a commit or the input method going away clears it.
    /// True when what is drawn changed.
    pub(crate) fn note(&mut self, ime: &Ime, owner: Owner) -> bool {
        let next = match ime {
            Ime::Enabled => return false,
            Ime::Preedit(text, caret) if !text.is_empty() => Some(Preedit {
                text: text.clone(),
                caret: caret.map(|(start, _)| start),
                owner,
            }),
            _ => None,
        };
        let changed = self.preedit != next;
        self.preedit = next;
        changed
    }

    /// Move a composition to the surface that takes text now (`owner`), or drop it when none
    /// does. Moving to another pane keeps the window's IME on and the OS still holds the
    /// marked text — its next commit lands in the NEW surface — so the composition is shown
    /// there, not hidden. True when it moved or was dropped.
    pub(crate) fn follow(&mut self, owner: Option<Owner>) -> bool {
        let Some(p) = self.preedit.as_mut().filter(|p| Some(p.owner) != owner) else {
            return false;
        };
        match owner {
            Some(o) => p.owner = o,
            None => self.preedit = None,
        }
        true
    }
}

/// Lay `pre` over `cells` from the caret cell `at`, never reaching column
/// `end`: a composition too long for the room left is drawn further left
/// rather than cut. The grid cells it covers are taken out first — the
/// renderer draws every cell it is given — and with `tail` everything else on
/// the row up to `end` goes too (the input bar's caret and history ghost).
pub(crate) fn overlay(
    cells: &mut Vec<CellView>,
    at: (u16, u16),
    end: u16,
    tail: bool,
    pre: &Preedit,
    (ink, page): ((u8, u8, u8), (u8, u8, u8)),
) {
    let widths: Vec<(usize, char, u16)> = pre
        .text
        .char_indices()
        .map(|(i, c)| (i, c, char_w(c) as u16))
        // A combining mark (゙ U+3099) rides on the glyph before it, as in the grid.
        .filter(|&(_, _, w)| w > 0)
        .collect();
    let caret_at_end = pre.caret.is_some_and(|b| b >= pre.text.len());
    let w: u16 = widths.iter().map(|&(_, _, w)| w).sum::<u16>() + u16::from(caret_at_end);
    let (start, row) = (at.0.min(end.saturating_sub(w)), at.1);
    let stop = if tail { end } else { start + w };
    cells.retain(|c| c.row != row || c.col < start || c.col >= stop);
    let rule = Deco {
        line: DecoLine::Single,
        color: Some(ink),
        ..Deco::NONE
    };
    let beam = CursorMark {
        shape: CursorShape::Beam,
        color: ink,
    };
    let mut col = start;
    for (i, c, cw) in widths {
        if col + cw > end {
            break;
        }
        cells.push(CellView {
            col,
            row,
            c,
            fg: ink,
            bg: page,
            deco: rule,
            cursor: if pre.caret == Some(i) {
                beam
            } else {
                Default::default()
            },
            ..Default::default()
        });
        col += cw;
    }
    if caret_at_end && col < end {
        cells.push(CellView {
            col,
            row,
            c: ' ',
            fg: ink,
            bg: page,
            cursor: beam,
            ..Default::default()
        });
    }
}

/// The caret cell `at` of a grid whose top-left cell starts at `origin`, in
/// whole physical px `[x, y, w, h]` — what the window hands the input method
/// as the text's place, so the candidate list opens just under it.
pub(crate) fn cell_rect(origin: (f32, f32), at: (u16, u16), cw: f32, ch: f32) -> [i32; 4] {
    let x = origin.0 + f32::from(at.0) * cw;
    let y = origin.1 + f32::from(at.1) * ch;
    [
        x.round() as i32,
        y.round() as i32,
        cw.round() as i32,
        ch.round() as i32,
    ]
}

#[cfg(test)]
#[path = "imepreedit_tests.rs"]
mod tests;
