//! One row of the chat find pop-up: the message cut to show WHERE it
//! matched, with the matched text marked.
//!
//! The row used to be `sender: text` from the start, ellipsized at the card's
//! edge, with nothing marked. A long tool line matched on a word past the
//! edge therefore listed a row that did not visibly contain what you typed —
//! the same "why is this a hit?" the palette answered by marking its fuzzy
//! letters (`cmdrow`). Search results everywhere now lead with the match in
//! view: the sender stays (it is how you tell rows apart), the text before
//! the match gives way to a `…`, and the match sits a third of the way into
//! what is left, so there is context on both sides of it.
use crate::chatfind::ChatFind;
use crate::chatlayout::Message;
use crate::suggest::MenuItem;

/// `sender: text` for a pane whose list rows are `room` columns wide,
/// windowed on the first match of `query`, plus the character indices of
/// every match in the result (for `MenuItem::hit`). Case-insensitive, as
/// `chatfind::filter` is. With no match the row is the plain `sender: text`.
pub(crate) fn snippet(sender: &str, text: &str, query: &str, room: usize) -> (String, Vec<usize>) {
    let head = format!("{sender}: ");
    let body: Vec<char> = text.replace('\n', " \u{23ce} ").chars().collect();
    let needle: Vec<char> = query.chars().map(fold).collect();
    let found = find_all(&body, &needle);
    let Some(&first) = found.first() else {
        return (head + &body.iter().collect::<String>(), Vec::new());
    };
    let hn = head.chars().count();
    let qn = needle.len();
    let start = if hn + first + qn <= room {
        0
    } else {
        // What is left beside the sender and the `…`, a third of it before
        // the match: enough to read the words that lead into it.
        let lead = room.saturating_sub(hn + 1 + qn) / 3;
        first - lead.min(first)
    };
    let cut = usize::from(start > 0);
    let mut label = head;
    if cut == 1 {
        label.push('\u{2026}');
    }
    label.extend(&body[start..]);
    let hits = found
        .into_iter()
        .filter(|&m| m >= start)
        .flat_map(|m| (0..qn).map(move |k| hn + cut + (m - start) + k))
        .collect();
    (label, hits)
}

/// The find pop-up's rows for a pane `cols` wide: every match of `f`, newest
/// first, as a [`snippet`]; one dim "no matches" title when there are none.
/// Match indices are re-checked against `msgs` — the transcript may have
/// shifted since the last key.
pub(crate) fn items(f: &ChatFind, msgs: &[&Message], cols: u16) -> Vec<MenuItem> {
    // The columns a list row gets in a card as wide as the pane: the card's
    // frame, the selection marker, and one of slack (see `cmdmenu`).
    let room = usize::from(cols).saturating_sub(5);
    let mut rows: Vec<MenuItem> = f
        .matches
        .iter()
        .filter_map(|&i| msgs.get(i))
        .map(|m| {
            let (label, hit) = crate::findsnip::snippet(&m.sender, &m.text, &f.query, room);
            MenuItem {
                label,
                hit,
                ..MenuItem::default()
            }
        })
        .collect();
    if rows.is_empty() {
        rows.push(MenuItem {
            label: "no matches".to_string(),
            header: true,
            ..MenuItem::default()
        });
    }
    rows
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Start indices of every non-overlapping `needle` in `hay`, case-folded.
fn find_all(hay: &[char], needle: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()]
            .iter()
            .zip(needle)
            .all(|(&h, &n)| fold(h) == n)
        {
            out.push(i);
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
#[path = "findsnip_tests.rs"]
mod tests;
