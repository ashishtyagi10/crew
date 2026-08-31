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

/// Body lines this card may show before it folds, or `None` if it never
/// folds at all.
///
/// ONE threshold, read by both [`folded`] (what the frame draws) and
/// [`foldable`] (what a click may toggle). They were two separate predicates,
/// and a card the first folded but the second did not consider foldable is a
/// card collapsed with no way to open it — invisible until the exact content
/// that splits them shows up.
///
/// TOOL CARDS fold at ONE line, not [`FOLD_LINES`]: their first line already
/// says everything a summary needs (`sys:run ✓ 1.2s`) and every line under it
/// is raw output — three lines of a JSON body is not a preview, it is three
/// lines of a JSON body. The splash nameplate never folds; clamping box art to
/// its `╔` line would destroy it.
fn fold_threshold(m: &Message) -> Option<usize> {
    if crate::chatmsgs::is_splash(m) {
        return None;
    }
    if crate::chatmsgs::is_tool_card(m) {
        return Some(1);
    }
    crate::chatmsgs::is_system_voice(&m.sender).then_some(FOLD_LINES)
}

/// Whether a card with `body_len` full body lines renders folded right now.
pub(crate) fn folded(m: &Message, body_len: usize) -> bool {
    !m.expanded && fold_threshold(m).is_some_and(|t| body_len > t)
}

/// Whether the card is fold-toggleable at all — [`folded`] minus the
/// `expanded` override, measured against the message's FULL body (the
/// rendered, possibly clamped card no longer knows its real length).
pub(crate) fn foldable(m: &Message, cols: usize, view: View) -> bool {
    fold_threshold(m).is_some_and(|t| crate::chatmsgs::full_body(m, cols, view).len() > t)
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
    // A transcript shorter than its window sits on the bottom of it.
    let first = top + crate::chatplace::top_pad(total, rows);
    let offset = (row >= first).then(|| (row - first) as usize)?;
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
    // An open popup (Ctrl+R search, Cmd+F find, palette, mention) overlays
    // the transcript: a click on one of its rows belongs to it, never to the
    // card invisibly beneath.
    if pane.histsearch.is_some()
        || pane.find.is_some()
        || pane.palette.is_some()
        || pane.mention.is_some()
    {
        return None;
    }
    let view = View {
        gap_rows: crate::density::level().card_gap_rows(),
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
    /// Whether a click at absolute `row` would toggle a fold — the press-time
    /// dry run of [`ChatPane::toggle_fold_at`], for arming a release toggle.
    pub(crate) fn fold_target_at(&self, cols: u16, rows: u16, row: u16) -> bool {
        toggle_target(self, cols, rows, row).is_some()
    }

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
    /// Mouse-press arm of the fold toggle: resolve the cursor to a chat
    /// pane's card and REMEMBER the hit (`fold_click`) instead of toggling.
    /// The toggle fires on release ([`Self::fold_release`]) so starting a
    /// drag-selection on a folded card can't expand it mid-gesture and shift
    /// the layout under the cursor. Returns whether a candidate armed — the
    /// caller (`events`) still focuses the pane and arms selection (the
    /// toggle is additive), but keeps an armed click out of the double-click
    /// zoom count.
    pub(crate) fn fold_press_at_cursor(&mut self) -> bool {
        self.fold_click = None;
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
        let crate::pane::PaneContent::Chat(chat) = &self.panes[i].content else {
            return false;
        };
        if chat.fold_target_at(grid.cols, grid.rows, row as u16) {
            self.fold_click = Some((i, row as u16));
        }
        self.fold_click.is_some()
    }

    /// Mouse-release arm: consume the press candidate, toggling only when the
    /// press-release pair stayed a plain click — `dragged` is
    /// [`Self::selection_release`]'s verdict, so a drag that moved (and
    /// copied its selection) never also toggles the card it started on.
    /// `true` when a card toggled.
    pub(crate) fn fold_release(&mut self, dragged: bool) -> bool {
        let Some((i, row)) = self.fold_click.take() else {
            return false;
        };
        if dragged {
            return false;
        }
        let Some(pane) = self.panes.get_mut(i) else {
            return false;
        };
        let grid = pane.grid;
        let crate::pane::PaneContent::Chat(chat) = &mut pane.content else {
            return false;
        };
        chat.toggle_fold_at(grid.cols, grid.rows, row)
    }
}

#[cfg(test)]
#[path = "chatfold_tests.rs"]
mod tests;
