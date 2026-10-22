//! The `@project` a thread is about (`todoguess`): only one you already
//! have, and only on a sign crew can name. Strongest first: a `[name]` in
//! the subject (GitHub's notifications, most mailing lists), then a link to
//! the project's repository, then the project's name as a word in the
//! subject, or twice in the mails (once, "the crew is ready" is a word). The
//! first two may name a folder projects live in; a bare word must be a tag or
//! a binding you made, as a folder's name ("docs", "api") is too often an
//! ordinary word. Two projects as strong is none.
use super::super::super::api::Mail;
use super::super::Guess;
use super::Known;

pub(super) fn guess(subject: &str, mails: &[Mail], k: &Known) -> Guess {
    let strong: Vec<&String> = k.projects.iter().chain(&k.folders).collect();
    let mine: Vec<&String> = k.projects.iter().collect();
    let text: String = mails
        .iter()
        .map(|m| m.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let rungs: [(Vec<String>, &[&String], &str); 3] = [
        (bracketed(subject), &strong, "the subject\u{2019}s [{}]"),
        (repos(&text), &strong, "a link to its repository"),
        (
            named(subject, &text),
            &mine,
            "\u{201c}{}\u{201d} in the mail",
        ),
    ];
    for (said, allowed, why) in rungs {
        let mut hit: Vec<&String> = allowed
            .iter()
            .copied()
            .filter(|p| p.chars().count() >= 3 && said.iter().any(|s| s.eq_ignore_ascii_case(p)))
            .collect();
        hit.sort_by_key(|p| p.to_lowercase());
        hit.dedup_by_key(|p| p.to_lowercase());
        match hit.as_slice() {
            [one] => {
                return Guess {
                    text: one.to_string(),
                    why: why.replace("{}", one),
                }
            }
            [] => continue,
            // Two projects as strong: a guess crew cannot defend.
            _ => return Guess::default(),
        }
    }
    Guess::default()
}

/// The names in `[…]` in a subject, the last part of a path (`[owner/crew]`).
fn bracketed(subject: &str) -> Vec<String> {
    subject
        .split('[')
        .skip(1)
        .filter_map(|s| s.split_once(']'))
        .map(|(name, _)| name.rsplit('/').next().unwrap_or(name).trim().to_string())
        .collect()
}

/// The repositories linked on GitHub, GitLab or Bitbucket: `owner/name`'s name.
fn repos(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    ["github.com/", "gitlab.com/", "bitbucket.org/"]
        .iter()
        .flat_map(|host| lower.match_indices(host).map(move |(i, _)| i + host.len()))
        .filter_map(|i| {
            let path: String = lower[i..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
                .collect();
            let name = path.split('/').nth(1)?.trim_end_matches(".git");
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

/// The words that name something: each in the subject, and each said twice
/// or more in the mails.
fn named(subject: &str, text: &str) -> Vec<String> {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for w in words(text) {
        *seen.entry(w.to_lowercase()).or_default() += 1;
    }
    let mut out = words(subject);
    out.extend(seen.into_iter().filter(|(_, n)| *n >= 2).map(|(w, _)| w));
    out
}

/// Every word, names with `-`, `_` and `.` kept whole (`crew-app`).
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .map(|w| w.trim_matches('.').to_string())
        .filter(|w| !w.is_empty())
        .collect()
}
