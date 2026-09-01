//! Setting an alarm from your phone: the clock's channel face.
//!
//! `crew daemon at …` needs a terminal on the machine, which is exactly the thing you do not
//! have when you think of the errand. So the three watch commands are readable from a channel
//! too — and the parse is deliberately EXPLICIT rather than clever, because everything it does
//! not claim becomes a task for an agent instead. "book me a flight tomorrow" must stay a task;
//! only "remind me tomorrow …" is an alarm.
use super::intent::Repeat;

/// A watch command read out of a message.
#[derive(Debug, PartialEq)]
pub(crate) enum Ask {
    /// Stand something up: what to run, when, how often.
    Register {
        text: String,
        fire_ms: u64,
        repeat: Repeat,
    },
    /// What are you watching for?
    List,
    /// Call one off.
    Cancel(String),
}

/// Read a watch command, if the message is one. The outer `None` means "not for me — give it to
/// an agent"; an inner `Err` means it WAS one and could not be honoured, and carries the
/// sentence to send back.
pub(crate) fn read(said: &str, now_ms: u64) -> Option<Result<Ask, String>> {
    let trimmed = said.trim();
    let lower = trimmed.to_lowercase();
    let first = lower.split_whitespace().next().unwrap_or("");
    match first {
        "watching" | "/watching" | "reminders" => return Some(Ok(Ask::List)),
        "cancel" | "/cancel" => {
            // Only with a real id. A bare "cancel" is somebody refusing an approval, and
            // stealing that word here would answer the wrong question.
            let id = lower.split_whitespace().nth(1)?;
            if !id.starts_with('w') || id[1..].parse::<u64>().is_err() {
                return None;
            }
            return Some(Ok(Ask::Cancel(id.to_string())));
        }
        "remind" | "/remind" => {}
        _ => return None,
    }
    // "remind me …" and "remind …" both reach here; the rest is the errand.
    let rest = strip_words(trimmed, &["remind", "me"]);
    Some(register(&rest, now_ms))
}

/// The body of a `remind` once the verb is off: a time, a cadence, and the errand.
fn register(rest: &str, now_ms: u64) -> Result<Ask, String> {
    let (rest, repeat) = take_cadence(rest);
    let now = crate::todopane::duedate::now_local();
    let Some(hit) = crate::todopane::duedate::find(&rest, now) else {
        return Err("when? say something like \"remind me tomorrow 9am to call the bank\"".into());
    };
    let stripped = crate::todopane::duedate::strip(&rest, hit.start, hit.end);
    let text = strip_words(&stripped, &["to", "that", "about"]);
    if text.trim().is_empty() {
        return Err("that is a time with nothing to do at it".into());
    }
    let Some(fire_ms) = crate::todopane::duedate::to_epoch_ms(hit.due) else {
        return Err("that time is outside the range I can hold".into());
    };
    if fire_ms <= now_ms {
        return Err(format!(
            "that time has already passed \u{2014} {} ago. When should it NEXT happen?",
            super::intent::spell((now_ms - fire_ms) / 1000)
        ));
    }
    Ok(Ask::Register {
        text,
        fire_ms,
        repeat,
    })
}

/// Cadence phrases somebody says out loud, longest first so `every day` beats a bare `every`.
const PHRASES: &[(&str, u64)] = &[
    ("every day", super::intent::DAY_SECS),
    ("everyday", super::intent::DAY_SECS),
    ("daily", super::intent::DAY_SECS),
    ("every week", super::intent::WEEK_SECS),
    ("weekly", super::intent::WEEK_SECS),
    ("every hour", 3_600),
    ("hourly", 3_600),
];

/// Take the cadence out of the sentence, returning what is left and how often it repeats. The
/// phrase is REMOVED because it is not part of the errand — "every day brief me" is a daily
/// intent to "brief me", and leaving the words in would send them to the agent.
fn take_cadence(said: &str) -> (String, Repeat) {
    let lower = said.to_lowercase();
    for (phrase, secs) in PHRASES {
        if let Some(at) = lower.find(phrase) {
            let mut rest = said.to_string();
            rest.replace_range(at..at + phrase.len(), "");
            return (squeeze(&rest), Repeat::Every { secs: *secs });
        }
    }
    // `every 30m`, `every 2h`: the word plus one token the cadence grammar accepts.
    let words: Vec<&str> = said.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        if w.to_lowercase() != "every" {
            continue;
        }
        let Some(next) = words.get(i + 1) else {
            continue;
        };
        if let Some(r @ Repeat::Every { .. }) = Repeat::parse(next) {
            let kept: Vec<&str> = words
                .iter()
                .enumerate()
                .filter(|(n, _)| *n != i && *n != i + 1)
                .map(|(_, w)| *w)
                .collect();
            return (kept.join(" "), r);
        }
    }
    (said.to_string(), Repeat::Once)
}

/// Drop `words` from the front of `s`, in any order, case-insensitively.
fn strip_words(s: &str, words: &[&str]) -> String {
    let mut out = s.trim();
    loop {
        let Some(first) = out.split_whitespace().next() else {
            return String::new();
        };
        let bare = first.trim_end_matches([',', ':']).to_lowercase();
        if !words.contains(&bare.as_str()) {
            return out.to_string();
        }
        out = out[first.len()..].trim_start();
    }
}

/// Collapse the whitespace a removed phrase leaves behind.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "watchchat_tests.rs"]
mod tests;
