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
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    cleaned.chars().take(ROLE_MAX).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_hyphenates_whitespace() {
        assert_eq!(slug("Risk Assessor").as_deref(), Some("risk-assessor"));
        assert_eq!(slug("  Archivist  ").as_deref(), Some("archivist"));
    }

    #[test]
    fn strips_chars_that_break_the_at_tokenizers() {
        // `@` would double-parse, `+` is relay.rs's multi-target separator,
        // `/` collides with construct routing in stdio.rs.
        assert_eq!(slug("@archivist").as_deref(), Some("archivist"));
        assert_eq!(slug("data+ops").as_deref(), Some("data-ops"));
        assert_eq!(slug("sec/ops").as_deref(), Some("sec-ops"));
        assert_eq!(slug("The Skeptic!").as_deref(), Some("the-skeptic"));
    }

    #[test]
    fn collapses_and_trims_hyphens() {
        assert_eq!(slug("a---b").as_deref(), Some("a-b"));
        assert_eq!(slug("--edge--").as_deref(), Some("edge"));
    }

    #[test]
    fn rejects_what_cannot_be_salvaged() {
        assert_eq!(slug(""), None);
        assert_eq!(slug("@#$"), None);
        assert_eq!(slug("x"), None, "one char is below the floor");
        assert_eq!(slug("---"), None);
    }

    #[test]
    fn non_ascii_is_dropped_not_transliterated() {
        // chips_on_border measures with byte length, which is only correct
        // because the charset is ASCII.
        assert_eq!(slug("café-critic").as_deref(), Some("caf-critic"));
        assert_eq!(slug("日本語"), None);
    }

    #[test]
    fn over_length_is_hard_cut_not_boundary_cut() {
        // A boundary cut would yield "accommodation" — a bare topic noun,
        // the exact failure the planner prompt exists to prevent. A hard cut
        // is obviously mangled instead of plausibly wrong.
        let long = "accommodation-specialist-for-travel";
        let got = slug(long).unwrap();
        assert_eq!(got.len(), 28);
        assert_eq!(got, "accommodation-specialist-for");
    }

    #[test]
    fn hard_cut_still_trims_a_trailing_hyphen() {
        let got = slug("abcdefghijklmnopqrstuvwxyz-ab").unwrap();
        assert!(!got.ends_with('-'), "got {got}");
    }

    #[test]
    fn slug_or_derives_from_id_when_unsalvageable() {
        assert_eq!(slug_or("@#$", 3), "specialist-3");
        assert_eq!(slug_or("Archivist", 3), "archivist");
    }

    #[test]
    fn role_clamp_collapses_whitespace_and_drops_controls() {
        assert_eq!(role_clamp("  records,\n retrieval  "), "records, retrieval");
        assert_eq!(role_clamp("a\u{7}b"), "ab");
        assert_eq!(role_clamp(""), "");
    }

    #[test]
    fn role_clamp_truncates_at_sixty_chars() {
        let got = role_clamp(&"x".repeat(100));
        assert_eq!(got.chars().count(), 60);
    }
}
