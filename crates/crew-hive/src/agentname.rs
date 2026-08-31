//! The one answer to "what is a legal agent name". Agent names became
//! LLM-authored with dynamic specialists, and every consumer already assumes
//! a strict charset without enforcing it: `relay.rs` terminates a name at
//! whitespace and reserves `+` as its multi-target separator, `chatcomplete`
//! bails on whitespace, and `stdio` routes on a leading `/`. Slugging at the
//! parse boundary makes those assumptions true.

/// Longest name kept. Empirical: a prompt spike on qwen-max produced
/// `user-experience-specialist` (26), and ~1 name in 6 exceeds 20, so a
/// tighter ceiling would mangle ordinary output. See the design doc.
const MAX: usize = 28;
/// Shortest name worth addressing.
const MIN: usize = 2;
/// Longest role hint kept, in chars.
const ROLE_MAX: usize = 60;

/// Normalize `raw` to `^[a-z0-9-]{2,28}$`, or `None` if nothing survives.
pub fn slug(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    for c in raw.trim().chars() {
        let mapped = if c.is_ascii_alphanumeric() {
            c.to_ascii_lowercase()
        } else if c.is_whitespace() || c == '-' || c == '_' || c == '+' || c == '/' {
            '-'
        } else {
            continue; // drop everything else, including non-ASCII
        };
        // Collapse runs of '-' as we go.
        if mapped == '-' && out.ends_with('-') {
            continue;
        }
        out.push(mapped);
    }
    // Hard cut, deliberately not at a '-' boundary (see the module docs).
    out.truncate(MAX);
    let trimmed = out.trim_matches('-');
    (trimmed.chars().count() >= MIN).then(|| trimmed.to_string())
}

/// [`slug`], falling back to a name derived from the task `id`.
pub fn slug_or(raw: &str, id: u64) -> String {
    slug(raw).unwrap_or_else(|| format!("specialist-{id}"))
}

/// Normalize a prose role hint: collapse whitespace, drop control chars,
/// clamp to 60 chars. `""` is a valid result — it's what `role_for` already
/// returns for unknown agents, so every consumer handles it.
pub fn role_clamp(raw: &str) -> String {
    // Whitespace wins where the two rules overlap: control-whitespace (tab,
    // \r, \v, \f) becomes a separator instead of being silently deleted, so
    // "foo\tbar" collapses to "foo bar" rather than gluing into "foobar".
    // Non-whitespace controls (e.g. bell) are still dropped outright.
    let mapped: String = raw
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                Some(' ')
            } else if c.is_control() {
                None
            } else {
                Some(c)
            }
        })
        .collect();
    let cleaned = mapped.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned.chars().take(ROLE_MAX).collect()
}

#[cfg(test)]
#[path = "agentname_tests.rs"]
mod tests;
