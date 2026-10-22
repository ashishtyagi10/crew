//! The todo's title (`todoguess`): the ask, when the newest mail someone
//! else sent makes one plainly ("can you send me the Q4 numbers by Friday?"
//! becomes "Send Ann the Q4 numbers"), else "Reply to Ann: Q4 numbers", as
//! `t` always wrote. The deadline leaves the title: the due line has it.
use super::super::super::api::Mail;
use super::super::super::wire::sender_name;
use super::super::super::Reading;
use super::super::Guess;
use super::{whose, words};

/// How an ask opens, the longest first so "could you please" wins.
const ASKS: [&str; 7] = [
    "could you please ",
    "can you please ",
    "could you ",
    "can you ",
    "would you ",
    "will you ",
    "please ",
];
/// What follows "please" or "can you" without asking for anything.
const NOT: [&str; 7] = [
    "find",
    "see",
    "note",
    "ignore",
    "disregard",
    "let me know if",
    "be advised",
];
/// Where the ask's own words end and its deadline or a nicety begins.
const ENDS: [&str; 8] = [
    " by ",
    " before ",
    " until ",
    " no later than ",
    " due ",
    " asap",
    " when you",
    " if you",
];
/// The longest title the card writes, in chars.
const MOST: usize = 72;

pub(super) fn title(r: &Reading, theirs: Option<&Mail>) -> Guess {
    let plain = || Guess {
        text: super::super::super::smith::todo_title(r),
        why: String::new(),
    };
    let Some(m) = theirs else {
        return plain();
    };
    let first = sender_name(&m.from);
    let first = first
        .split_whitespace()
        .next()
        .unwrap_or("them")
        .to_string();
    match ask(words(m), &first) {
        Some(text) => Guess {
            text: with_subject(&text, r.thread.subject.trim()),
            why: format!("the ask in {}", whose(m)),
        },
        None => plain(),
    }
}

/// An ask that says little on its own ("Send it") carries the subject:
/// "Send it (Q4 review)".
fn with_subject(ask: &str, subject: &str) -> String {
    let words: Vec<&str> = ask.split_whitespace().collect();
    let vague = words.len() < 3
        || words.last().is_some_and(|w| {
            ["it", "this", "that", "them", "these", "those"].contains(&w.to_lowercase().as_str())
        });
    match vague && !subject.is_empty() {
        true => clip(&format!("{ask} ({subject})")),
        false => ask.to_string(),
    }
}

/// The first plain ask in `text`, written as a todo, `me` made the asker.
fn ask(text: &str, asker: &str) -> Option<String> {
    let clean: String = text.chars().filter(|c| !matches!(c, '*' | '`')).collect();
    clean.split(['.', '?', '!', '\n']).find_map(|s| {
        let low = s.to_lowercase();
        let (at, opener) = ASKS
            .iter()
            .filter_map(|a| low.find(a).map(|i| (i, a.len())))
            .min_by_key(|&(i, len)| (i, usize::MAX - len))?;
        let low_rest = low[at + opener..].trim_start();
        if low_rest.split_whitespace().count() < 2 || NOT.iter().any(|n| low_rest.starts_with(n)) {
            return None;
        }
        let cut = ENDS
            .iter()
            .filter_map(|e| low_rest.find(e))
            .min()
            .unwrap_or(low_rest.len());
        // The words as written, where lowercasing kept every offset.
        let from = s.len() - low_rest.len();
        let rest = match s.len() == low.len() {
            true => &s[from..from + cut],
            false => &low_rest[..cut],
        };
        let words: Vec<String> = rest
            .split_whitespace()
            .map(|w| match w.to_lowercase().as_str() {
                "me" => asker.to_string(),
                "my" => format!("{asker}\u{2019}s"),
                _ => w.to_string(),
            })
            .collect();
        let title = words.join(" ");
        Some(clip(&capital(title.trim_end_matches([',', ';', ':', ' ']))))
    })
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// At most [`MOST`] chars, cut at a word with `…`.
fn clip(s: &str) -> String {
    if s.chars().count() <= MOST {
        return s.to_string();
    }
    let head: String = s.chars().take(MOST - 1).collect();
    let cut = head.rfind(' ').unwrap_or(head.len());
    format!("{}\u{2026}", &head[..cut])
}
