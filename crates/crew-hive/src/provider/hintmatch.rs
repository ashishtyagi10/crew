//! How [`super::hint`] reads an error: the words it matches on, and the model a sentence
//! names. Split from `hint.rs` for the line cap.

/// The sentence and the envelope's `type`/`code` words, lowercased. A code
/// is often the plainest statement of the problem: OpenRouter's wrapped 429
/// says only "Provider returned error", and its `code` says 429.
pub(super) fn hay(msg: &str, body: &str) -> String {
    let mut hay = msg.to_lowercase();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        for obj in [&v, &v["error"]] {
            for key in ["type", "code"] {
                let word = match &obj[key] {
                    serde_json::Value::String(s) => s.to_lowercase(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => continue,
                };
                hay.push(' ');
                hay.push_str(&word);
            }
        }
    }
    hay
}

/// Whether `frag` is in `hay`. A number must stand alone: "401" matched
/// inside "you requested 40100 tokens" and told a person with an overlong
/// thread that their key was wrong.
pub(super) fn has(hay: &str, frag: &str) -> bool {
    if !frag.bytes().all(|b| b.is_ascii_digit()) {
        return hay.contains(frag);
    }
    let digit = |i: Option<&u8>| i.is_some_and(u8::is_ascii_digit);
    let b = hay.as_bytes();
    hay.match_indices(frag)
        .any(|(i, _)| !digit(b.get(i.wrapping_sub(1))) && !digit(b.get(i + frag.len())))
}

/// The model the sentence names: OpenAI's and DashScope's put it in
/// backticks, Anthropic's after `model: `. "that model" when it names none.
pub(super) fn named(msg: &str) -> &str {
    let ticked = msg
        .split('`')
        .nth(1)
        .filter(|_| msg.matches('`').count() >= 2);
    let after = || msg.split_once("model: ")?.1.split_whitespace().next();
    ticked
        .filter(|s| !s.trim().is_empty())
        .or_else(after)
        .unwrap_or("that model")
}
