//! The `meta` field's grammar, owned in ONE place because both sides of the
//! wire read it: the broker writes it, the host renders from it.
//!
//! A message's `meta` is a ` · `-separated list of tags followed by the
//! reply's latency — `"task:7 · sub · 3.2s"`. The tags are written
//! outside-in as the event travels (`stdio` stamps the task id on everything
//! a worker emits, over whatever the construct already put there), so a
//! reader must look for a tag anywhere in the list rather than at a fixed
//! position.
//!
//! [`SUB`] is the one this module exists for: it marks a card as ONE
//! subagent's work inside a larger turn — a fan-out reply, not the answer to
//! the user. Eleven of those arrive at once and each can be hundreds of
//! lines; the host folds them into their own collapsed section (`chatsub`)
//! instead of pouring eleven transcripts into the pane.

/// The separator between `meta` tags (and before the latency).
pub const SEP: &str = " \u{00b7} ";

/// Tag: this card is one subagent's work inside a bigger turn.
pub const SUB: &str = "sub";

/// `meta` with `tag` in front of it — the tag alone when there is nothing
/// else to say.
pub fn tagged(tag: &str, meta: &str) -> String {
    if meta.is_empty() {
        return tag.to_string();
    }
    format!("{tag}{SEP}{meta}")
}

/// Whether `meta` carries `tag`, wherever in the list it sits.
pub fn has(meta: &str, tag: &str) -> bool {
    meta.split(SEP).any(|t| t == tag)
}

/// `meta` with a LEADING [`SUB`] tag removed — what is left is the latency
/// (or the next tag). Borrowed, so this is the shape a renderer wants.
pub fn strip_sub(meta: &str) -> &str {
    match meta.strip_prefix(SUB) {
        // A whole tag, not a prefix of a longer word: `subtotal` is latency
        // the broker wrote, and eating its first three letters would be a
        // renderer editing someone else's field.
        Some("") => "",
        Some(rest) => rest.strip_prefix(SEP).unwrap_or(meta),
        None => meta,
    }
}

/// Stamp `tag` onto a message event's `meta`; any other event is left alone.
pub fn mark(ev: &mut crate::PluginEvent, tag: &str) {
    if let crate::PluginEvent::Message { meta, .. } = ev {
        *meta = tagged(tag, meta);
    }
}

#[cfg(test)]
#[path = "metatag_tests.rs"]
mod tests;
