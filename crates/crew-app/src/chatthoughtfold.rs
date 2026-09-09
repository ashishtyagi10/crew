//! The crew pane's side of the thought blocks ([`crate::chatthought`]):
//! resolving a click to a settled block's folded row and toggling it, and
//! the one entry the card fold (`chatfold`) calls for EVERY block kind — tool
//! lines and thoughts — so it need not know how many kinds there are. Shares
//! `chatfold::hit_line`'s geometry, so every toggle reads one layout.
use crate::chat::ChatPane;
use crate::chatthoughtseat::{above, above_of, orphan_seats, Seat};

impl ChatPane {
    /// The settled thought whose folded row a click at absolute `row` hits.
    /// Walks the placement `card_lines_spanned` drew with: a block above a
    /// card takes the FIRST rows of that card's span (ahead of the tool
    /// block's), and the orphans stand after the tool block's orphans.
    pub(crate) fn thought_target(&self, cols: u16, rows: u16, row: u16) -> Option<usize> {
        let (spans, idx) = crate::chatfold::hit_line(self, cols, rows, row)?;
        let view = self.view();
        let visible = self.visible_messages();
        let width = cols as usize;
        for (mi, m) in visible.iter().enumerate() {
            let streaming = mi >= view.streaming_from;
            let n = above(view, m, streaming, 0, width).len();
            let s = &spans[mi];
            if n > 0 && (s.start..s.start + n).contains(&idx) {
                return match above_of(view, m, streaming) {
                    Some(Seat::Settled(b)) if idx == s.start => Some(b),
                    _ => None,
                };
            }
        }
        let tools = crate::chattoolview::orphans(view, &visible, 0, width).len();
        let mut at = spans.last().map_or(0, |s| s.end) + tools;
        let preceded = !visible.is_empty() || tools > 0;
        for (k, seat) in orphan_seats(view, &visible).into_iter().enumerate() {
            if preceded || k > 0 {
                at += view.gap_rows;
            }
            at += 1; // the agent's header
            let n = match seat {
                Seat::Live(i) => {
                    crate::chatthoughtview::live_lines(&view.thoughts.live[i], 0, width).len()
                }
                Seat::Settled(i) => {
                    crate::chatthoughtview::block_lines(&view.thoughts.settled[i], width).len()
                }
            };
            if (at..at + n).contains(&idx) {
                return match seat {
                    Seat::Settled(b) if idx == at => Some(b),
                    _ => None,
                };
            }
            at += n;
        }
        None
    }

    /// Toggle the thought whose row a click at `row` hit. `true` when one did.
    pub(crate) fn toggle_thought_at(&mut self, cols: u16, rows: u16, row: u16) -> bool {
        match self.thought_target(cols, rows, row) {
            Some(b) => {
                let b = &mut self.thoughts.settled[b];
                b.expanded = !b.expanded;
                true
            }
            None => false,
        }
    }

    /// Whether a click at `row` would toggle ANY block — tool or thought —
    /// the press-time dry run of [`Self::toggle_block_at`].
    pub(crate) fn block_target_at(&self, cols: u16, rows: u16, row: u16) -> bool {
        self.tool_target_at(cols, rows, row) || self.thought_target(cols, rows, row).is_some()
    }

    /// Toggle whichever block a click at `row` hit. `true` when one did.
    pub(crate) fn toggle_block_at(&mut self, cols: u16, rows: u16, row: u16) -> bool {
        self.toggle_tool_at(cols, rows, row) || self.toggle_thought_at(cols, rows, row)
    }
}

#[cfg(test)]
#[path = "chatthoughtfold_tests.rs"]
mod tests;
