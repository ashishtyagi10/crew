//! What a person types to keep some hunks, and how a pane reads a question's hunk count back.
//!
//! WHY in the plugin crate: the broker words the question and the pane parses the answer, so the
//! two ends share one definition of "how many hunks" and "what a pick means" and cannot drift.

/// What typed text turned out to be.
#[derive(Debug, PartialEq, Eq)]
pub enum Picks {
    /// Not a pick at all (`ship it`): the text is an ordinary message.
    NotPicks,
    /// These hunks, 1-based, ascending.
    Keep(Vec<u32>),
    /// A pick that cannot be honoured; the words are for the person.
    Bad(String),
}

/// The keys line a question ends with when it offers picks. The broker writes it only when
/// EVERY hunk is on screen (see `hunkshow::show`), and [`declared`] reads picks only from a
/// question that ends with it: a hunk past the clip can't be chosen unseen, and a model-written
/// command whose first line happens to end in `— 3 changes` offers nothing.
pub const PICK_KEYS: &str = "Enter all \u{b7} Esc none \u{b7} type 1 3 to keep those";

/// How many hunks a question offers, from its first line (`edit a.rs — 3 changes · …`); 0 for a
/// question that is not a file edit, a new file (all-or-nothing), or one clipped short.
pub fn declared(question: &str) -> usize {
    if question.lines().last() != Some(PICK_KEYS) {
        return 0;
    }
    let first = question.lines().next().unwrap_or_default();
    let Some((_, tail)) = first.rsplit_once(" \u{2014} ") else {
        return 0;
    };
    let n: String = tail.chars().take_while(char::is_ascii_digit).collect();
    match tail[n.len()..].starts_with(" change") {
        true => n.parse().unwrap_or(0),
        false => 0,
    }
}

/// Read `input` against a question of `n` hunks: `1 3`, `1,3`, or `-2` for all but 2.
pub fn parse_picks(input: &str, n: usize) -> Picks {
    let toks: Vec<&str> = input
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .collect();
    let all_but = toks.first().is_some_and(|t| t.starts_with('-'));
    let nums: Option<Vec<usize>> = toks
        .iter()
        .map(|t| match t.strip_prefix('-') {
            Some(rest) if all_but => rest,
            Some(_) => "x",
            None => t,
        })
        .map(|t| {
            t.parse()
                .ok()
                .filter(|_| t.chars().all(|c| c.is_ascii_digit()))
        })
        .collect();
    let Some(nums) = nums.filter(|v| !v.is_empty() && n > 0) else {
        return Picks::NotPicks;
    };
    if let Some(bad) = nums.iter().find(|&&k| k == 0 || k > n) {
        return Picks::Bad(format!(
            "there is no hunk {bad} \u{2014} this change has {n}"
        ));
    }
    let keep: Vec<u32> = (1..=n)
        .filter(|k| nums.contains(k) != all_but)
        .map(|k| k as u32)
        .collect();
    match keep.is_empty() {
        true => Picks::Bad("that leaves no hunk \u{2014} Esc refuses them all".into()),
        false => Picks::Keep(keep),
    }
}

#[cfg(test)]
#[path = "hunkpicks_tests.rs"]
mod tests;
