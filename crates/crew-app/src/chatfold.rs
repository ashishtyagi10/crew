//! Auto-folding for system/telemetry cards in the chat transcript: a long
//! system-voice card (turn summaries, /doctor output, roster dumps) renders
//! collapsed — header + first body line + ` … +N` — so broker noise stops
//! drowning the actual replies (the Claude-Code folded-noise look). A plain
//! click on the collapsed card expands it; a click on the expanded card's
//! header folds it back. State lives on the `Message` itself (`expanded`),
//! so it survives `chatcompact` folds and streaming cards settling — both of
//! which shift transcript indices out from under any index-keyed set.
//!
//! Agent replies and user messages are never auto-folded, and the pane-global
//! compact view (Ctrl+O) wins outright: while it is on, everything is clamped
//! and nothing here toggles. The rendering itself stays in
//! `chatmsgs::card_lines` (one clamp code path for compact and fold alike);
//! this module owns the fold decision and the click plumbing.
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crate::chatmsgs::View;

/// Body lines a system-voice card may show before it auto-folds.
pub(crate) const FOLD_LINES: usize = 3;

/// Whether a card with `body_len` full body lines renders folded right now:
/// system voice, not the splash nameplate (folding box art to one `╔` line
/// would destroy it), longer than [`FOLD_LINES`], and not clicked open.
pub(crate) fn folded(m: &Message, body_len: usize) -> bool {
    crate::chatmsgs::is_system_voice(&m.sender)
        && !crate::chatmsgs::is_splash(m)
        && !m.expanded
        && body_len > FOLD_LINES
}

/// Whether the card is fold-toggleable at all — [`folded`] minus the
/// `expanded` override, measured against the message's FULL body (the
/// rendered, possibly clamped card no longer knows its real length).
pub(crate) fn foldable(m: &Message, cols: usize, view: View) -> bool {
    crate::chatmsgs::is_system_voice(&m.sender)
        && !crate::chatmsgs::is_splash(m)
        && crate::chatmsgs::full_body(m, cols, view).len() > FOLD_LINES
}

/// The card-line index sitting at absolute pane `row`, under the exact
/// bottom-anchored windowing `chatplace::window` draws with (`total` lines
/// into `rows` rows starting at `top`, scrolled `scroll` up from the live
/// bottom). `None` outside the message area.
pub(crate) fn line_index_at(
    total: usize,
    rows: u16,
    top: u16,
    scroll: usize,
    row: u16,
) -> Option<usize> {
    let start = total.saturating_sub(rows as usize).saturating_sub(scroll);
    let shown = total.min(start + rows as usize) - start;
    let offset = (row >= top).then(|| (row - top) as usize)?;
    (offset < shown).then(|| start + offset)
}

/// The visible-message index of the fold toggle a click at absolute `row`
/// hits, if any. A folded card's whole (two-line) rendering is the expand
/// target; an expanded card folds back only from its header line, so body
/// clicks stay free for text selection. Re-derives the same geometry
/// `chatplace::placed_lines` draws with, so a click can never resolve
/// against stale layout.
fn toggle_target(pane: &ChatPane, cols: u16, rows: u16, row: u16) -> Option<usize> {
    let visible = pane.visible_messages();
    let top = pane.status_rows(cols, rows);
    if cols == 0 || rows == 0 || visible.is_empty() || top == 0 {
        return None; // tiny panes use the plain fallback layout — no cards
    }
    let view = View {
        source: pane.show_source,
        compact: pane.compact_view,
        streaming_from: pane.messages.len(),
    };
    if view.compact {
        return None; // Ctrl+O wins outright — nothing to toggle under it
    }
    let budget = crate::chatplace::msg_rows_budget(pane, cols, rows);
    let (lines, spans) = crate::chatmsgs::card_lines_spanned(&visible, cols as usize, 0, view);
    let idx = line_index_at(lines.len(), budget, top, pane.scroll, row)?;
    let mi = spans.iter().position(|s| s.contains(&idx))?;
    let m = visible[mi];
    if !foldable(m, cols as usize, view) {
        return None;
    }
    (!m.expanded || idx == spans[mi].start).then_some(mi)
}

impl ChatPane {
    /// Toggle the fold of the card a click at absolute `row` hit, on a
    /// `cols` × `rows` pane. `true` when a card actually toggled.
    pub(crate) fn toggle_fold_at(&mut self, cols: u16, rows: u16, row: u16) -> bool {
        let Some(mi) = toggle_target(self, cols, rows, row) else {
            return false;
        };
        // `visible_messages` chains settled then streaming — map back.
        let settled = self.messages.len();
        let m = if mi < settled {
            &mut self.messages[mi]
        } else {
            &mut self.streaming[mi - settled]
        };
        m.expanded = !m.expanded;
        true
    }
}

impl crate::app::CrewApp {
    /// Plain-left-click arm of the fold toggle: resolve the cursor to a chat
    /// pane's cell and toggle the card there. `true` when a card toggled —
    /// the caller (`events`) still focuses the pane and arms selection (the
    /// toggle is additive), but keeps a toggle click out of the double-click
    /// zoom count.
    pub(crate) fn fold_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        let Some((row, _col)) = self.cursor_rowcol(i) else {
            return false;
        };
        if row < 0 {
            return false;
        }
        let grid = self.panes[i].grid;
        let crate::pane::PaneContent::Chat(chat) = &mut self.panes[i].content else {
            return false;
        };
        chat.toggle_fold_at(grid.cols, grid.rows, row as u16)
    }
}

#[cfg(test)]
#[path = "chatfold_tests.rs"]
mod tests;
