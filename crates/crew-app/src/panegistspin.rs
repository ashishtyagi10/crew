//! An agent's working line, read: the words before its parenthesis and the
//! time inside it (`✳ Pondering… (1m 23s · esc to interrupt)` → `Pondering ·
//! 1m23s`), and that time split back off for the card to draw at the edge.

/// An agent's working line, as `Pondering · 12s`: the words before its
/// parenthesis, the spinner glyph dropped, and the first time in it.
pub(super) fn spinner(row: &str) -> Option<String> {
    let lower = row.to_lowercase();
    if !(lower.contains("to interrupt") || lower.contains("esc to cancel")) {
        return None;
    }
    let (head, rest) = row.split_once('(')?;
    // The spinner glyph before it and the agent's own `…` after it go: the
    // time that follows says it is still going, and a clip adds its own.
    let head = head
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim();
    let head = head.trim_end_matches(['…', '.']).trim_end();
    if head.is_empty() {
        return None;
    }
    // The first run of time words, so `1m 23s` reads whole.
    let words: Vec<&str> = rest
        .split(|c: char| c == '·' || c == '•' || c == ',' || c == ')' || c.is_whitespace())
        .filter(|w| !w.is_empty())
        .collect();
    let time: String = (words.iter())
        .skip_while(|w| !is_elapsed(w))
        .take_while(|w| is_elapsed(w))
        .copied()
        .collect();
    Some(match time.is_empty() {
        false => format!("{head} \u{b7} {time}"),
        true => head.to_string(),
    })
}

/// A summary's words and its elapsed time, when it ends in one
/// (`Pondering · 12s` → `("Pondering", Some("12s"))`).
pub(crate) fn split_time(s: &str) -> (&str, Option<&str>) {
    match s.rsplit_once(" \u{b7} ") {
        Some((head, t)) if is_elapsed(t) => (head, Some(t)),
        _ => (s, None),
    }
}

/// `12s`, `1m`, `3m20s`, `2h`: digits and unit letters, a digit first.
fn is_elapsed(w: &str) -> bool {
    w.starts_with(|c: char| c.is_ascii_digit())
        && w.ends_with(['s', 'm', 'h'])
        && w.chars().all(|c| c.is_ascii_digit() || "smh".contains(c))
}
