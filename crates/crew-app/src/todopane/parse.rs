//! Pulling the `@project` and `#assignee` tokens out of a typed todo.
//!
//! Split out of [`super`] for the line cap. Two sigils, one rule: the FIRST
//! token wearing a sigil is that axis's tag and leaves the title; later ones
//! stay title text, because an item has one project and one owner, and a
//! second one in a sentence is almost always prose.
//!
//! With one exception the rule needs: `#123` is an issue, not a person.
//! Nobody is called 966, and a team that tracks tickets types their numbers
//! into titles all day — so an all-digit `#token` is never an assignee and
//! stays in the title where it was typed ([`names_a_person`]).

/// The two sigils a todo token can wear, and what each one names.
pub(crate) const PROJECT: char = '@';
pub(crate) const WHO: char = '#';

/// Whether `c` opens a tag token.
pub(crate) fn is_sigil(c: char) -> bool {
    c == PROJECT || c == WHO
}

/// Whether `w` is a tag token — it opens with a sigil.
pub(crate) fn tagged(w: &str) -> bool {
    w.starts_with(is_sigil)
}

/// Whether a `#token` names a person rather than a ticket: `#priya` does,
/// `#966` does not. Any non-digit is enough — `#v2` is a person's shorthand
/// as readily as a release, and only the all-digit case is unambiguous.
pub(crate) fn names_a_person(name: &str) -> bool {
    !name.is_empty() && !name.chars().all(|c| c.is_ascii_digit())
}

/// The tags typed into `text`: the title with the first `@token` and the
/// first `#token` removed, then those two tags.
pub(crate) fn extract_tags(text: &str) -> (String, Option<String>, Option<String>) {
    let mut title: Vec<&str> = Vec::new();
    let (mut project, mut who) = (None, None);
    for w in text.split_whitespace() {
        let slot = match w.chars().next() {
            Some(PROJECT) => &mut project,
            Some(WHO) if names_a_person(&w[1..]) => &mut who,
            _ => {
                title.push(w);
                continue;
            }
        };
        let name = &w[1..];
        if slot.is_none() && !name.is_empty() {
            *slot = Some(name.to_string());
        } else {
            title.push(w);
        }
    }
    (title.join(" "), project, who)
}

/// Char ranges of every `@tag` / `#tag` token (length ≥ 2) in `chars`, each
/// with its sigil — what the composer tints as you type.
pub(crate) fn tag_spans(chars: &[char]) -> Vec<(usize, usize, char)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        let name: String = chars[start + 1..i].iter().collect();
        let is_tag = match chars[start] {
            WHO => names_a_person(&name),
            PROJECT => !name.is_empty(),
            _ => false,
        };
        if is_tag {
            spans.push((start, i, chars[start]));
        }
    }
    spans
}

/// A lone `@tag` / `#tag` (nothing else typed): the filter the composer sets
/// on Enter, as `(sigil, name)`. A bare sigil clears that axis (empty name).
pub(crate) fn lone_tag(trimmed: &str) -> Option<(char, &str)> {
    let mut chars = trimmed.chars();
    let s = chars.next().filter(|&c| is_sigil(c))?;
    let name = chars.as_str();
    (!name.contains(char::is_whitespace)).then_some((s, name))
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
