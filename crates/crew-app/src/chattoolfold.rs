//! The crew pane's side of the tool blocks ([`crate::chattool`]): resolving
//! a click to a block's summary line or one of its calls, toggling what it
//! hit, and the redraw predicate that keeps the spinner and the clock
//! ticking while a call is in flight. Shares `chatfold::hit_line`'s
//! geometry with the card fold, so the two toggles read one layout.
use crate::chat::ChatPane;
use crate::chattoolview::{above_of, below_of, block_lines, hit, orphan_ids, ToolHit};

/// A click's target among the pane's tool blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// Block `b`'s summary line: expand or collapse it.
    Summary(usize),
    /// Block `b`'s call `l`: open or close its result text.
    Line(usize, usize),
}

impl ChatPane {
    /// The tool-block target a click at absolute `row` hits, if any. Walks
    /// the same placement `card_lines_spanned` drew with: a block above a
    /// settled card takes the first rows of that card's span after the
    /// agent's thought (`chatthoughtview::above`), one under a streaming
    /// card the last rows, and the orphans stand after the last span in
    /// `orphan_ids` order.
    pub(crate) fn tool_target(&self, cols: u16, rows: u16, row: u16) -> Option<Target> {
        let (spans, idx) = crate::chatfold::hit_line(self, cols, rows, row)?;
        let view = self.view();
        let visible = self.visible_messages();
        let width = cols as usize;
        let rows_of = |b: usize| block_lines(&view.tools[b], 0, width, crate::glyphs::on()).len();
        let resolve = |b: usize, off: usize| match hit(&view.tools[b], width, off)? {
            ToolHit::Summary => Some(Target::Summary(b)),
            ToolHit::Line(l) => view.tools[b].lines[l]
                .has_text()
                .then_some(Target::Line(b, l)),
        };
        for (mi, m) in visible.iter().enumerate() {
            let streaming = mi >= view.streaming_from;
            let s = &spans[mi];
            if let Some(b) = above_of(view, m, streaming) {
                let start =
                    s.start + crate::chatthoughtseat::above(view, m, streaming, 0, width).len();
                if (start..start + rows_of(b)).contains(&idx) {
                    return resolve(b, idx - start);
                }
            }
            if let Some(b) = below_of(view, m, streaming) {
                let start = s.end - rows_of(b);
                if (start..s.end).contains(&idx) {
                    return resolve(b, idx - start);
                }
            }
        }
        let mut at = spans.last().map_or(0, |s| s.end);
        for b in orphan_ids(view, &visible) {
            at += view.gap_rows + 1; // the card gap and the agent's header
            let n = rows_of(b);
            if (at..at + n).contains(&idx) {
                return resolve(b, idx - at);
            }
            at += n;
        }
        None
    }

    /// Whether a click at `row` would toggle a tool block — the press-time
    /// dry run of [`Self::toggle_tool_at`].
    pub(crate) fn tool_target_at(&self, cols: u16, rows: u16, row: u16) -> bool {
        self.tool_target(cols, rows, row).is_some()
    }

    /// Toggle whatever tool-block target a click at `row` hit: a summary
    /// line opens or collapses its block, a call's line opens or closes its
    /// result text. `true` when something toggled.
    pub(crate) fn toggle_tool_at(&mut self, cols: u16, rows: u16, row: u16) -> bool {
        match self.tool_target(cols, rows, row) {
            Some(Target::Summary(b)) => {
                let b = &mut self.tools.blocks[b];
                b.expanded = !b.expanded;
                true
            }
            Some(Target::Line(b, l)) => {
                let l = &mut self.tools.blocks[b].lines[l];
                l.show_text = !l.show_text;
                true
            }
            None => false,
        }
    }

    /// Whether any tool call is in flight — the redraw-scheduling predicate
    /// (`panebusy::pane_animating`) that keeps the spinner and the elapsed
    /// clock ticking even when nothing else on the pane animates. Bounded:
    /// every call ends in a result, or in `ToolLines::abandon` when the run
    /// does.
    pub(crate) fn tools_running(&self) -> bool {
        self.tools.pending() > 0
    }
}

#[cfg(test)]
#[path = "chattoolfold_tests.rs"]
mod tests;
