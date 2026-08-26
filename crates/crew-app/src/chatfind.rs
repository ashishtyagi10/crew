//! Cmd+F find-in-conversation for the crew pane transcript, shaped like the
//! other composer popups (`chathistsearch` is the template): typed chars
//! build a QUERY (the composer is never touched); Enter/Ctrl+F/Down step to
//! the next older matching message, Up to the next newer (both wrap); each
//! step scrolls to the match ([`jump`]); `chatview::find_wash` paints it.
use crate::chat::ChatPane;
use crate::chatkeys::ChatInput;
use crate::chatlayout::Message;
use crate::chatmsgs::View;
use crate::suggest::MenuItem;
use crew_render::CellView;

/// The open find popup: the query, the matched visible-message indices
/// (newest first) and the selected match.
#[derive(Default)]
pub(crate) struct ChatFind {
    pub query: String,
    pub matches: Vec<usize>,
    pub sel: usize,
}

impl ChatFind {
    /// Re-derive matches from the LIVE transcript, clamping the selection —
    /// the shared defence against indices outliving their messages.
    fn rescan(&mut self, msgs: &[&Message]) {
        self.matches = filter(msgs, &self.query);
        self.sel = self.sel.min(self.matches.len().saturating_sub(1));
    }

    /// A query edit (`Some` push / `None` pop): refilter, restart at newest.
    fn edit(&mut self, c: Option<char>, msgs: &[&Message]) {
        match c {
            Some(c) => self.query.push(c),
            None => drop(self.query.pop()),
        }
        self.rescan(msgs);
        self.sel = 0;
    }

    /// Step the selection one match `older` (or newer), wrapping.
    fn step(&mut self, older: bool, msgs: &[&Message]) {
        self.rescan(msgs);
        let n = self.matches.len();
        if n > 0 {
            self.sel = (self.sel + if older { 1 } else { n - 1 }) % n;
        }
    }
}

/// Consumed a key, consumed it AND moved the match target (the app should
/// re-scroll), or wasn't open at all.
pub(crate) enum FindKey {
    Consumed,
    Jump,
    Forward,
}

/// Visible-message indices whose text contains `query`, newest first.
/// Case-insensitive; an empty query matches nothing (nothing to jump to).
pub(crate) fn filter(msgs: &[&Message], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let q = query.to_lowercase();
    (0..msgs.len())
        .rev()
        .filter(|&i| msgs[i].text.to_lowercase().contains(&q))
        .collect()
}

/// Popup-first key routing: Ctrl+F opens (Cmd+F arrives via the app chord),
/// typed chars edit the query, Enter/Ctrl+F/Down step older, Up newer, Esc
/// closes. Modal while open — except Ctrl+R, forwarded so the history
/// search can open (the caller then closes find: one modal at a time).
pub(crate) fn popup_key(
    state: &mut Option<ChatFind>,
    msgs: &[&Message],
    key: &ChatInput,
) -> FindKey {
    let Some(f) = state else {
        if matches!(key, ChatInput::FindNext) {
            *state = Some(ChatFind::default());
            return FindKey::Consumed;
        }
        return FindKey::Forward;
    };
    match key {
        ChatInput::HistSearch => return FindKey::Forward,
        ChatInput::Char(c) => f.edit(Some(*c), msgs),
        ChatInput::Backspace => f.edit(None, msgs),
        ChatInput::Enter | ChatInput::FindNext | ChatInput::Down => f.step(true, msgs),
        ChatInput::Up => f.step(false, msgs),
        ChatInput::Close => {
            *state = None; // the composer draft was never touched
            return FindKey::Consumed;
        }
        _ => return FindKey::Consumed, // modal — consumed, no effect
    }
    FindKey::Jump
}

/// Scroll `pane` so the current match's line sits inside the drawn window,
/// expanding a folded system card first so the line exists. Rescans against
/// the live transcript on entry — a stale index is never dereferenced.
pub(crate) fn jump(pane: &mut ChatPane, cols: u16, rows: u16) {
    let Some(mut f) = pane.find.take() else {
        return;
    };
    f.rescan(&pane.visible_messages());
    if let Some(&mi) = f.matches.get(f.sel) {
        let view = View {
            gap_rows: crate::density::level().card_gap_rows(),
            source: pane.show_source,
            compact: pane.compact_view,
            streaming_from: pane.messages.len(),
        };
        let settled = pane.messages.len();
        let m = match mi < settled {
            true => pane.messages.get_mut(mi),
            false => pane.streaming.get_mut(mi - settled),
        };
        if let Some(m) = m {
            if crate::chatfold::foldable(m, cols as usize, view) {
                m.expanded = true; // the match may sit in the folded tail
            }
        }
        // Tiny panes use the plain fallback layout — different line math.
        if pane.status_rows(cols, rows) > 0 && cols > 0 {
            let budget = crate::chatplace::msg_rows_budget(pane, cols, rows) as usize;
            let visible = pane.visible_messages();
            let (lines, spans) =
                crate::chatmsgs::card_lines_spanned(&visible, cols as usize, 0, view);
            if let Some(span) = spans.get(mi) {
                pane.scroll = scroll_for(lines.len(), budget, target_line(&lines, span, &f.query));
            }
        }
    }
    pane.find = Some(f);
}

/// The first line of `span` containing `query` (case-insensitive), falling
/// back to the span's header line when wrapping split the phrase.
fn target_line(
    lines: &[crate::chatbody::CardLine],
    span: &std::ops::Range<usize>,
    query: &str,
) -> usize {
    let q = query.to_lowercase();
    let hit = |l: &crate::chatbody::CardLine| {
        let s: String = l.iter().map(|c| c.c).collect();
        s.to_lowercase().contains(&q)
    };
    span.clone()
        .find(|&i| lines.get(i).is_some_and(hit))
        .unwrap_or(span.start)
}

/// The `pane.scroll` that puts absolute line `target` inside a `budget`-row
/// window over `total` lines — roughly centered, clamped to the scrollback
/// (scroll counts lines up from the live bottom, see `chatscroll`).
pub(crate) fn scroll_for(total: usize, budget: usize, target: usize) -> usize {
    if budget == 0 {
        return 0;
    }
    let max_start = total.saturating_sub(budget);
    max_start - target.saturating_sub(budget / 2).min(max_start)
}

/// The card legend: `find: <query> (k/N)`, k counted from the newest match.
pub(crate) fn title(f: &ChatFind) -> String {
    let k = if f.matches.is_empty() { 0 } else { f.sel + 1 };
    format!("find: {} ({k}/{})", f.query, f.matches.len())
}

/// The popup as a rendered `menu_card` plus its row count (matched messages
/// as `sender: text` rows, newest first; a dim placeholder when nothing
/// matches). Match indices are re-checked against `msgs` — the transcript
/// may have shifted since the last key.
pub(crate) fn card(f: &ChatFind, msgs: &[&Message], cols: u16) -> (Vec<CellView>, u16) {
    let row = |label: String, header: bool| MenuItem {
        label,
        header,
        ..MenuItem::default()
    };
    let flat = |m: &&Message| format!("{}: {}", m.sender, m.text.replace('\n', " \u{23ce} "));
    let mut rows: Vec<MenuItem> = f
        .matches
        .iter()
        .filter_map(|&i| msgs.get(i))
        .map(|m| row(flat(m), false))
        .collect();
    if rows.is_empty() {
        rows.push(row("no matches".to_string(), true));
    }
    let n = crate::cmdmenu::menu_rows(rows.len());
    (
        crate::cmdmenu::menu_card(&title(f), &rows, f.sel, cols, n),
        n,
    )
}

#[cfg(test)]
#[path = "chatfind_tests.rs"]
mod tests;
