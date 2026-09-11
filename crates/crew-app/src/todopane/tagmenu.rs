//! `@project` / `#assignee` completion in the todo composer: detect the
//! trailing tag token being typed (leading position included — a todo has no
//! `@agent` routing, unlike the chat composer), rank the tags already in the
//! store against it, and splice the accepted one back. Pure
//! string-in/string-out, modelled on `chatmention`.
//!
//! One path for both sigils: the popup completes whichever one you typed,
//! from that axis's own names.
use super::item::TodoItem;
use super::parse;

/// The trailing token's char offset — where a completion splices in.
fn token_start(input: &str) -> usize {
    input
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0)
}

/// The tag being typed at the end of `input`, as `(sigil, query)`
/// (`pay rent @ho` → `('@', "ho")`, `#` → `('#', "")`).
pub(crate) fn pending_tag(input: &str) -> Option<(char, &str)> {
    let tok = &input[token_start(input)..];
    let sigil = tok.chars().next().filter(|&c| parse::is_sigil(c))?;
    Some((sigil, &tok[1..]))
}

/// Distinct names in use on one axis, most-used first (ties alphabetical) —
/// case-insensitively deduped, keeping the first-seen spelling.
pub(crate) fn known_tags(items: &[TodoItem], sigil: char) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for it in items {
        let field = match sigil {
            parse::WHO => &it.assignee,
            _ => &it.project,
        };
        let Some(p) = field else { continue };
        match counts.iter_mut().find(|(t, _)| t.eq_ignore_ascii_case(p)) {
            Some((_, c)) => *c += 1,
            None => counts.push((p.clone(), 1)),
        }
    }
    counts.sort_by(|(ta, ca), (tb, cb)| cb.cmp(ca).then(ta.cmp(tb)));
    counts.into_iter().map(|(t, _)| t).collect()
}

/// Tags matching `q`: prefix beats substring beats subsequence; an empty
/// query keeps the usage ordering of [`known_tags`].
pub(crate) fn filter_tags(tags: &[String], q: &str) -> Vec<String> {
    if q.is_empty() {
        return tags.to_vec();
    }
    let q = q.to_lowercase();
    let mut scored: Vec<(u8, &String)> = tags
        .iter()
        .filter_map(|t| {
            let low = t.to_lowercase();
            let rank = if low.starts_with(&q) {
                0
            } else if low.contains(&q) {
                1
            } else if crate::suggest::is_subsequence(&q, &low) {
                2
            } else {
                return None;
            };
            Some((rank, t))
        })
        .collect();
    scored.sort_by_key(|&(r, _)| r);
    scored.into_iter().map(|(_, t)| t.clone()).collect()
}

/// Replace the trailing tag token with `<sigil>tag ` (the trailing space
/// ends the mention, so the popup closes).
pub(crate) fn accept(input: &str, sigil: char, tag: &str) -> String {
    format!("{}{sigil}{tag} ", &input[..token_start(input)])
}

/// The open tag popup: which axis it is completing, its current matches and
/// the selected row.
pub(crate) struct TagMenu {
    pub sigil: char,
    pub matches: Vec<String>,
    pub sel: usize,
}

/// Sync the popup to the input after an edit: open while a tag is being
/// typed and something matches, refilter as it narrows, close otherwise.
/// (A brand-new tag with no matches simply has no popup — it's accepted
/// free-form at submit.)
pub(crate) fn after_edit(
    menu: &mut Option<TagMenu>,
    input: &str,
    tags: impl FnOnce(char) -> Vec<String>,
) {
    let Some((sigil, q)) = pending_tag(input) else {
        *menu = None;
        return;
    };
    let matches = filter_tags(&tags(sigil), q);
    if matches.is_empty() {
        *menu = None;
        return;
    }
    let sel = menu
        .as_ref()
        .map_or(0, |m| m.sel.min(matches.len().saturating_sub(1)));
    *menu = Some(TagMenu {
        sigil,
        matches,
        sel,
    });
}

#[cfg(test)]
#[path = "tagmenu_tests.rs"]
mod tests;
