//! A deep review's findings get a second look before they are shown.
//!
//! `reviewfindings` drops a finding whose citation is not there, but a
//! finding citing a real line can still be wrong: the guard it says is
//! missing sits two lines up, the caller it says is stale was updated in the
//! same diff. A deep review is asked for when the person will act on it, so
//! each finding goes back to the reviewer — one batched call, read-only as the
//! review was (`reviewguard`) — to be kept or withdrawn against the code. A
//! withdrawn finding is dropped and said; a second look that does not answer
//! leaves the findings as they were, marked unverified, and one that skips
//! some says how many were not re-checked.
use std::path::Path;
use std::time::Duration;

use super::adapter::Adapter;
use super::reviewfindings::{finding, is_verdict, tidy, Cited, Tidied};

/// The second-look prompt: the findings numbered, the code they cite.
pub(crate) fn prompt(diff: &str, callers: &str, findings: &[&str]) -> String {
    let list: Vec<String> = findings
        .iter()
        .enumerate()
        .map(|(i, f)| format!("{}. {}", i + 1, f.trim()))
        .collect();
    let mut p = format!(
        "You reviewed this diff and reported the findings below. Check each \
         one against the code again, as a second reviewer would: keep it only \
         if the code shows it. Change no file. Answer one line per finding, \
         `N keep` or `N drop — why`, and nothing else.\n\nFINDINGS:\n{}\n\n\
         DIFF:\n{diff}",
        list.join("\n")
    );
    if !callers.is_empty() {
        p.push_str("\n\n");
        p.push_str(callers);
    }
    p
}

/// What the second look said of one finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Look {
    Kept,
    /// Withdrawn, and why.
    Dropped(String),
    /// Not named in the reply: kept, but not re-checked.
    Unanswered,
}

/// Per finding, what the second look said; `None` when the reply answers
/// for none of them. Reads `1 keep`, `2. drop — why`, `3 — drop — why`,
/// `**4** keep` (the prompt's own `—` is often echoed after the number).
pub(crate) fn withdrawn(reply: &str, n: usize) -> Option<Vec<Look>> {
    let mut out = vec![Look::Unanswered; n];
    let mut any = false;
    for line in reply.lines() {
        let t = line.trim().trim_start_matches(['-', '*', ' ']);
        let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
        let Ok(i) = digits.parse::<usize>() else {
            continue;
        };
        let rest = t[digits.len()..].trim_start_matches(|c: char| ".):` —–-*".contains(c));
        let (word, why) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let word = word
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_lowercase();
        if i == 0 || i > n || !matches!(word.as_str(), "keep" | "drop") {
            continue;
        }
        any = true;
        out[i - 1] = if word == "drop" {
            let why = why.trim_start_matches(|c: char| c.is_whitespace() || "—–-:".contains(c));
            Look::Dropped(why.trim().to_string())
        } else {
            Look::Kept
        };
    }
    any.then_some(out)
}

/// `t` after the second look by `a`, plus the smith rows it leaves (an
/// edit put back, or the look not answering). No finding, no call.
pub(crate) fn second_look(
    dir: &Path,
    diff: &str,
    callers: &str,
    a: &dyn Adapter,
    timeout: Duration,
    t: Tidied,
) -> (Tidied, Vec<String>) {
    let findings: Vec<&str> = t.text.lines().filter(|l| finding(l).is_some()).collect();
    if findings.is_empty() {
        return (t, Vec::new());
    }
    let body = prompt(diff, callers, &findings);
    let (reply, edited) = super::reviewguard::call(dir, a, &body, timeout);
    let mut notes: Vec<String> = edited.into_iter().collect();
    let Some(verdicts) = reply.ok().and_then(|r| withdrawn(&r, findings.len())) else {
        notes.push("deep review: the second look did not answer — findings unverified".into());
        return (t, notes);
    };
    let unanswered = verdicts.iter().filter(|v| **v == Look::Unanswered).count();
    if unanswered > 0 {
        notes.push(format!(
            "deep review: {unanswered} not re-checked — the second look did not name them"
        ));
    }
    let (mut kept, mut dropped, mut i) = (Vec::new(), t.dropped, 0);
    for line in t.text.lines() {
        let Some((_, file, at)) = finding(line) else {
            kept.push(line);
            continue;
        };
        match &verdicts[i] {
            Look::Dropped(why) => {
                let cite = at.map_or(file.clone(), |n| format!("{file}:{n}"));
                let sep = if why.is_empty() { "" } else { " — " };
                dropped.push(format!("{cite} withdrawn on a second look{sep}{why}"));
            }
            Look::Kept | Look::Unanswered => kept.push(line),
        }
        i += 1;
    }
    let text = kept.join("\n");
    let mut shown = if kept.iter().any(|l| finding(l).is_some()) {
        // Recount the verdict; every citation was checked already.
        tidy(&text, &|_| Cited::Unknown)
    } else {
        let rest: Vec<&str> = kept.into_iter().filter(|l| !is_verdict(l)).collect();
        let rest = rest.join("\n");
        let rest = rest.trim();
        let text = if rest.is_empty() {
            "no findings".to_string()
        } else {
            format!("{rest}\n\nno findings")
        };
        Tidied {
            text,
            dropped: Vec::new(),
        }
    };
    shown.dropped = dropped;
    (shown, notes)
}

#[cfg(test)]
#[path = "reviewverify_tests.rs"]
mod tests;
