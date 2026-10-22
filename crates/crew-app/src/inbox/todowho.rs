//! The `#assignee` a thread asks (`todoguess`): only someone on it (a
//! sender, or on a To or Cc line), and only when the newest mail asks them
//! by name: "Bo, can you…", "@bo", "assigned to Bo", a greeting "Hi Bo,",
//! or an action item that starts a line, "Bo to send…", "Bo — the deck".
//! Written with the spelling of a `#assignee` tag you already use for them,
//! else their first name. Asked of you, or of two people, the line stays
//! blank: a row with no tag is yours.
use super::super::super::addr;
use super::super::super::api::Mail;
use super::super::super::wire::sender_name;
use super::super::Guess;
use super::{whose, Known};

/// Someone on the thread: their name as written, and their address.
struct Person {
    name: String,
    address: String,
}

impl Person {
    /// The ways the mail can name them: the whole name, the first name.
    fn names(&self) -> Vec<String> {
        let full = self.name.to_lowercase();
        let first = full.split_whitespace().next().unwrap_or("").to_string();
        [full, first]
            .into_iter()
            .filter(|n| n.chars().count() >= 2)
            .collect()
    }
}

pub(super) fn guess(mails: &[Mail], theirs: Option<&Mail>, k: &Known) -> Guess {
    let Some(m) = theirs else {
        return Guess::default();
    };
    let text = super::words(m).to_lowercase();
    // The one asking is not the one asked.
    let sender = addr::address(&m.from).to_lowercase();
    let mut asked: Vec<Person> = people(mails, &k.me)
        .into_iter()
        .filter(|p| p.address != sender && p.names().iter().any(|n| asks(&text, n)))
        .collect();
    asked.dedup_by(|a, b| a.address == b.address);
    let [p] = asked.as_slice() else {
        return Guess::default();
    };
    let first = p.names().pop().unwrap_or_default();
    let local = p.address.split('@').next().unwrap_or("").to_lowercase();
    let full = p
        .name
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    let tag = k
        .people
        .iter()
        .find(|t| {
            [&first, &full, &local]
                .iter()
                .any(|n| t.eq_ignore_ascii_case(n))
        })
        .cloned()
        .unwrap_or(first);
    Guess {
        text: tag,
        why: format!("asked by name in {}", whose(m)),
    }
}

/// Everyone on the thread but you, once each.
fn people(mails: &[Mail], me: &str) -> Vec<Person> {
    let mut out: Vec<Person> = Vec::new();
    let lines = mails
        .iter()
        .flat_map(|m| [m.from.as_str(), m.reply.to.as_str(), m.reply.cc.as_str()]);
    for one in lines.flat_map(addr::split) {
        let address = addr::address(&one).to_lowercase();
        let name = sender_name(&one);
        if address.is_empty() || address.eq_ignore_ascii_case(me) || name.contains('@') {
            continue;
        }
        if !out.iter().any(|p| p.address == address) {
            out.push(Person { name, address });
        }
    }
    out
}

/// Whether `text` (lowercased) asks `name` by name: a direct ask anywhere
/// ("bo, can you", "@bo", "assigned to bo"), a greeting line ("hi bo,"), or
/// an action item that starts a line ("bo to send…", "bo — the deck").
fn asks(text: &str, name: &str) -> bool {
    const ASK: [&str; 4] = [", can you", ", could you", ", would you", ", please"];
    const TAG: [&str; 5] = ["@", "assigned to ", "owner: ", "owner - ", "action: "];
    const ITEM: [&str; 5] = [" will ", " to ", " \u{2014} ", " - ", ": "];
    const HELLO: [&str; 4] = ["hi ", "hey ", "hello ", "dear "];
    text.lines().any(|line| {
        let line = line
            .trim()
            .trim_start_matches(['-', '*', '\u{2022}'])
            .trim_start();
        let greeting = line == format!("{name},")
            || HELLO
                .iter()
                .any(|g| line == format!("{g}{name},") || line == format!("{g}{name}"));
        let item = line
            .strip_prefix(name)
            .is_some_and(|rest| ITEM.iter().any(|i| rest.starts_with(i)));
        greeting
            || item
            || ASK.iter().any(|a| word_at(line, &format!("{name}{a}")))
            || TAG.iter().any(|t| word_at(line, &format!("{t}{name}")))
    })
}

/// Whether `needle` is in `line` as whole words (`@bo` is not `@bob`).
fn word_at(line: &str, needle: &str) -> bool {
    line.match_indices(needle).any(|(i, _)| {
        let start = line[..i]
            .chars()
            .last()
            .is_none_or(|c| !c.is_alphanumeric());
        let end = line[i + needle.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        start && end
    })
}
