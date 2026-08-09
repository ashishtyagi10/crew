//! `@project` completion in the todo composer: detect the trailing `@token`
//! being typed (leading position included — a todo has no `@agent` routing,
//! unlike the chat composer), rank the tags already in the store against it,
//! and splice the accepted one back. Pure string-in/string-out, modelled on
//! `chatmention`.
use super::item::TodoItem;

/// The query of a tag being typed: the trailing token starts with `@`
/// (`pay rent @ho` → `Some("ho")`, `@` → `Some("")`).
pub(crate) fn pending_tag(input: &str) -> Option<&str> {
    let cut = input
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    input[cut..].strip_prefix('@')
}

/// Distinct project tags in use, most-used first (ties alphabetical) —
/// case-insensitively deduped, keeping the first-seen spelling.
pub(crate) fn known_tags(items: &[TodoItem]) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for it in items {
        let Some(p) = &it.project else { continue };
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

/// Replace the trailing `@query` token with `@tag ` (the trailing space
/// ends the mention, so the popup closes).
pub(crate) fn accept(input: &str, tag: &str) -> String {
    let cut = input
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    format!("{}@{tag} ", &input[..cut])
}

/// The open tag popup: current matches and the selected row.
pub(crate) struct TagMenu {
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
    tags: impl FnOnce() -> Vec<String>,
) {
    let Some(q) = pending_tag(input) else {
        *menu = None;
        return;
    };
    let matches = filter_tags(&tags(), q);
    if matches.is_empty() {
        *menu = None;
        return;
    }
    let sel = menu
        .as_ref()
        .map_or(0, |m| m.sel.min(matches.len().saturating_sub(1)));
    *menu = Some(TagMenu { matches, sel });
}

#[cfg(test)]
#[path = "tagmenu_tests.rs"]
mod tests;
