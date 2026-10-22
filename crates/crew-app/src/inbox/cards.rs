//! The inbox's cards (the user, 2026-10-21: "add border to each section",
//! choosing the list, the mail and the reply box): each a crew card with
//! its edge and its name on the top edge. The list fills the pane, or the
//! left of a wide one beside an open thread (`split`); the open mail and the
//! reply box under it share the rest, and the mail's card is always the
//! taller ("mail panel should have tallest height", `composedraw`).
//!
//! Everything that draws or finds a thing in a card (the list's rows, the
//! mail's links and pictures, the box's lines) asks [`InboxPane::cards`]
//! where the card is, so a click and what was drawn cannot disagree.
use crew_render::CellView;

use super::draw::Canvas;
use super::{InboxPane, Stage};
use crate::chatwidth::str_w;

/// The mail's words start on the row under its card's top edge.
pub(crate) const READ_TOP: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct Rect {
    pub col: u16,
    pub row: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    /// What the card's edge leaves inside it.
    pub(crate) fn inner(self) -> Rect {
        Rect {
            col: self.col + 1,
            row: self.row + 1,
            w: self.w.saturating_sub(2),
            h: self.h.saturating_sub(2),
        }
    }

    /// The pane's `(row, col)` as this rect's own, when it is inside.
    pub(crate) fn local(self, row: u16, col: u16) -> Option<(u16, u16)> {
        let inside = (self.row..self.row + self.h).contains(&row)
            && (self.col..self.col + self.w).contains(&col);
        inside.then(|| (row - self.row, col - self.col))
    }
}

/// Where each card is, its edge included; none when it is not shown.
#[derive(Debug, Default)]
pub(crate) struct Cards {
    pub list: Option<Rect>,
    pub mail: Option<Rect>,
    pub reply: Option<Rect>,
}

impl InboxPane {
    pub(crate) fn cards(&self, cols: u16, rows: u16) -> Cards {
        let body = self.body_rows(cols, rows);
        // A column of air each side, between the cards and the pane's edge.
        let whole = Rect {
            col: 1,
            row: 0,
            w: cols.saturating_sub(2),
            h: body,
        };
        match (&self.stage, &self.reading) {
            (Stage::Ready(_), None) => Cards {
                list: Some(whole),
                ..Cards::default()
            },
            (Stage::Ready(_), Some(_)) => {
                let (col, w) = self.mail_cols(cols);
                let h = self.compose_h(cols, rows);
                Cards {
                    list: self.split(cols).map(|s| Rect {
                        w: s.list - 1,
                        ..whole
                    }),
                    mail: Some(Rect {
                        col,
                        row: 0,
                        w,
                        h: body - h,
                    }),
                    reply: (h > 0).then_some(Rect {
                        col,
                        row: body - h,
                        w,
                        h,
                    }),
                }
            }
            _ => Cards::default(),
        }
    }
}

impl InboxPane {
    /// The pane's `(row, col)` inside the open mail's card, as the reader's
    /// own rows and columns (its words' canvas).
    pub(crate) fn mail_local(
        &self,
        cols: u16,
        rows: u16,
        row: u16,
        col: u16,
    ) -> Option<(u16, u16)> {
        self.cards(cols, rows).mail?.inner().local(row, col)
    }

    /// What a drag that began at the pane's `(col, row)` may copy: the
    /// inside of the card it began in, so a copy holds no card's edge and
    /// none of the card beside it. Anywhere else, every cell.
    pub(crate) fn copy_cells(&self, cols: u16, rows: u16, (col, row): (u16, u16)) -> Vec<CellView> {
        let k = self.cards(cols, rows);
        let card = [k.list, k.mail, k.reply]
            .into_iter()
            .flatten()
            .find(|b| b.local(row, col).is_some());
        let cells = self.cells(cols, rows);
        match card.map(Rect::inner) {
            Some(b) => cells
                .into_iter()
                .filter(|c| b.local(c.row, c.col).is_some())
                .collect(),
            None => cells,
        }
    }
}

/// The edge of the card the pane's keys go to: half way from a resting
/// edge to the ink. Not the pane's own focused stroke, which the ring and
/// the glow repaint wherever they find it (`modernring`, `panecardglow`).
pub(crate) fn keyed_edge() -> (u8, u8, u8) {
    let t = crew_theme::theme();
    crate::anim::lerp_rgb(t.border_normal, t.ink, 0.5)
}

/// A card's edge on `cv` around `b`, `name` on its top edge at the left
/// and `right` (when there is one) at its right, each cell drawn once; the
/// edge stronger on the card the keys go to (`keys`).
pub(crate) fn frame(cv: &mut Canvas, b: Rect, name: &str, right: &str, keys: bool) {
    if b.w < 6 || b.h < 2 {
        return;
    }
    let t = crew_theme::theme();
    let edge = if keys { keyed_edge() } else { t.border_normal };
    let line = |n: u16| "\u{2500}".repeat(usize::from(n));
    let (end, bottom) = (b.col + b.w, b.row + b.h - 1);
    // The top edge: corner, a dash, the name, dashes, the right, a dash, corner.
    let right = match right.is_empty() {
        true => String::new(),
        false => format!(" {right} "),
    };
    let right_at = end.saturating_sub(2 + str_w(&right) as u16);
    let name = match name.is_empty() {
        true => String::new(),
        false => format!(" {name} "),
    };
    let name_end = (b.col + 2 + str_w(&name) as u16).min(right_at);
    cv.text(
        b.row,
        b.col,
        b.col + 2,
        &format!("\u{256d}{}", line(1)),
        edge,
        false,
    );
    cv.text(b.row, b.col + 2, name_end, &name, t.ink, true);
    cv.text(
        b.row,
        name_end,
        right_at,
        &line(right_at - name_end),
        edge,
        false,
    );
    cv.text(b.row, right_at, end - 2, &right, t.text_muted, false);
    cv.text(
        b.row,
        end - 2,
        end + 1,
        &format!("{}\u{256e}", line(1)),
        edge,
        false,
    );
    let rule = format!("\u{2570}{}\u{256f}", line(b.w - 2));
    cv.text(bottom, b.col, end + 1, &rule, edge, false);
    for r in b.row + 1..bottom {
        cv.text(r, b.col, b.col + 1, "\u{2502}", edge, false);
        cv.text(r, end - 1, end, "\u{2502}", edge, false);
    }
}

#[cfg(test)]
#[path = "cards_tests.rs"]
mod tests;
