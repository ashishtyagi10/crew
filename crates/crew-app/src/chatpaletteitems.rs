//! Row builders for the leading-token palettes (`/` and `@`) — split out of
//! `chatpalette.rs` (child module) to keep that file under the house line
//! cap, same pattern as `modelpick.rs`'s `modelbadge`/`modelrecents`.
use crate::chatcomplete::{describe, CONSTRUCTS};
use crate::suggest::MenuItem;

pub(super) fn slash_items(query: &str) -> Vec<MenuItem> {
    CONSTRUCTS
        .iter()
        .filter(|c| c[1..].starts_with(query))
        .map(|c| MenuItem {
            label: c.to_string(),
            desc: describe(c).to_string(),
            fill: c.to_string(),
            submit: false,
            header: false,
            dim: false,
        })
        .collect()
}

/// Rows for the leading `@`: the full attach picker (agents, skills, files
/// — `chatmention::filter`'s section order), agents only once the token has
/// a `+` (multi-target selectors route, they don't attach).
pub(super) fn attach_items(
    query: &str,
    entries: &[crate::chatmention::MentionEntry],
    multi: bool,
) -> Vec<MenuItem> {
    use crate::chatmention::MentionEntry;
    crate::chatmention::filter(entries, query)
        .into_iter()
        .filter(|e| !multi || matches!(e, MentionEntry::Agent { .. }))
        .map(|e| MenuItem {
            label: format!("@{}", e.token()),
            desc: e.desc(),
            fill: e.token(),
            submit: false,
            header: false,
            dim: false,
        })
        .collect()
}
