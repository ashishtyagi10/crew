//! Ctrl+R reverse history search in the crew composer — the shell reflex,
//! shaped like the other composer popups (`chatpalette`/`chatmention`): a
//! `menu_card` above the composer, key-first routing while open. Typed
//! characters build a QUERY (the composer text is left alone); Ctrl+R again
//! steps to the next older match; Enter puts the selection in the composer
//! WITHOUT sending; Esc restores what was typed before opening.
//!
//! Matching reuses `suggest::is_subsequence` — substring hits rank before
//! subsequence hits, newest first within a rank — rather than inventing a
//! third fuzzy matcher.
use crate::chatkeys::ChatInput;
use crate::suggest::MenuItem;
use crew_render::CellView;

/// The open search popup: the query being typed, the composer text saved for
/// Esc, the filtered matches (newest first) and the selected row.
pub(crate) struct HistSearch {
    pub query: String,
    pub saved: String,
    pub matches: Vec<String>,
    pub sel: usize,
}

impl HistSearch {
    /// Re-run the filter after a query edit; the selection returns to the
    /// newest match, like the palette after a narrowing keystroke.
    fn refilter(&mut self, lines: &[String]) {
        self.matches = filter(lines, &self.query);
        self.sel = 0;
    }
}

/// Whether the popup consumed a key, accepted an entry into the composer
/// (the caller should reset its history-walk state), or wasn't open at all.
pub(crate) enum HistKey {
    Consumed,
    Accepted,
    Forward,
}

/// History entries matching `query`, newest first: substring hits (rank 0)
/// before subsequence hits (rank 1), recency preserved within a rank. An
/// empty query lists everything. Case-insensitive, like every other popup.
pub(crate) fn filter(lines: &[String], query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    let mut scored: Vec<(u8, &String)> = lines
        .iter()
        .rev()
        .filter_map(|l| rank(l, &q).map(|r| (r, l)))
        .collect();
    scored.sort_by_key(|(r, _)| *r); // stable: recency preserved within a rank
    scored.into_iter().map(|(_, l)| l.clone()).collect()
}

/// Match quality of one history line against lowercased `q`.
fn rank(line: &str, q: &str) -> Option<u8> {
    let low = line.to_lowercase();
    if low.contains(q) {
        Some(0)
    } else if crate::suggest::is_subsequence(q, &low) {
        Some(1)
    } else {
        None
    }
}

/// Popup-first key routing: Ctrl+R opens (or steps older while open), typed
/// chars edit the query, Up/Down move, Enter/Tab accept, Esc restores +
/// closes. Modal while open — every key is consumed, so nothing leaks to the
/// palette or the composer underneath.
pub(crate) fn popup_key(
    state: &mut Option<HistSearch>,
    input: &mut String,
    lines: &[String],
    key: &ChatInput,
) -> HistKey {
    let Some(s) = state else {
        if matches!(key, ChatInput::HistSearch) {
            // Nothing recorded yet → nothing to open (a zero-row card would
            // render as nothing while still eating keys).
            if !lines.is_empty() {
                *state = Some(HistSearch {
                    query: String::new(),
                    saved: input.clone(),
                    matches: filter(lines, ""),
                    sel: 0,
                });
            }
            return HistKey::Consumed;
        }
        return HistKey::Forward;
    };
    match key {
        // Shell muscle memory: Ctrl+R again steps to the next older match.
        ChatInput::HistSearch | ChatInput::Down => {
            s.sel = (s.sel + 1).min(s.matches.len().saturating_sub(1));
        }
        ChatInput::Up => s.sel = s.sel.saturating_sub(1),
        ChatInput::Char(c) => {
            s.query.push(*c);
            s.refilter(lines);
        }
        ChatInput::Backspace => {
            s.query.pop();
            s.refilter(lines);
        }
        ChatInput::Enter | ChatInput::Complete => {
            let accepted = s.matches.get(s.sel).cloned();
            *state = None;
            if let Some(line) = accepted {
                *input = line; // into the composer — never auto-sent
                return HistKey::Accepted;
            }
        }
        ChatInput::Close => {
            *input = std::mem::take(&mut s.saved);
            *state = None;
        }
        // Newline/Accept/Ignore: modal — consumed, no effect.
        _ => {}
    }
    HistKey::Consumed
}

/// The card legend, carrying the live query so the user sees what they typed.
pub(crate) fn title(h: &HistSearch) -> String {
    if h.query.is_empty() {
        "history search".to_string()
    } else {
        format!("history search: {}", h.query)
    }
}

/// The matches as menu rows; a dim placeholder when nothing matches, so the
/// popup never renders as nothing while it is still eating keys.
pub(crate) fn items(h: &HistSearch) -> Vec<MenuItem> {
    let row = |label: String, header: bool| MenuItem {
        label,
        desc: String::new(),
        fill: String::new(),
        submit: false,
        header,
        dim: false,
        needs: None,
    };
    if h.matches.is_empty() {
        return vec![row("no matches".to_string(), true)];
    }
    h.matches
        .iter()
        .map(|l| row(l.replace('\n', " \u{23ce} "), false))
        .collect()
}

/// The popup as a rendered `menu_card` plus its row count, so `render.rs`
/// only places the scene (mirrors how `keyentry` exposes `card`/`rows`).
pub(crate) fn card(h: &HistSearch, cols: u16) -> (Vec<CellView>, u16) {
    let rows = items(h);
    let n = crate::cmdmenu::menu_rows(rows.len());
    (
        crate::cmdmenu::menu_card(&title(h), &rows, h.sel, cols, n),
        n,
    )
}

#[cfg(test)]
#[path = "chathistsearch_tests.rs"]
mod tests;
