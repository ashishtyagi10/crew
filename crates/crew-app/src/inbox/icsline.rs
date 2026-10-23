//! An iCalendar file's content lines (RFC 5545 section 3.1): unfolded, each
//! split into its name, its parameters (a quoted value may hold `;` and
//! `:`) and its value, and a TEXT value's escapes undone; then the values
//! an event's lines carry: a time, a person, a meeting's link. [`super`]
//! reads the event out of them.
use chrono::{NaiveDate, NaiveDateTime};

use super::{Person, When};

/// One content line, unfolded: `NAME;PARAM=V;…:value`.
#[derive(Debug)]
pub(crate) struct Prop {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: String,
}

impl Prop {
    pub(crate) fn param(&self, key: &str) -> Option<&str> {
        let hit = self
            .params
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key));
        hit.map(|(_, v)| v.as_str())
    }

    /// The line after its name, as written: `;TZID=America/New_York:2026…`.
    pub(crate) fn tail(&self) -> String {
        let quoted = |v: &str| match v.contains([':', ';', ',']) {
            true => format!("\"{v}\""),
            false => v.to_string(),
        };
        let params: String = (self.params.iter())
            .map(|(k, v)| format!(";{k}={}", quoted(v)))
            .collect();
        format!("{params}:{}", self.value.trim())
    }
}

/// The file's content lines: a line that starts with a space or a tab
/// continues the one before it.
pub(crate) fn props(ics: &str) -> Vec<Prop> {
    let mut lines: Vec<String> = Vec::new();
    for raw in ics.lines() {
        match (raw.strip_prefix([' ', '\t']), lines.last_mut()) {
            (Some(more), Some(last)) => last.push_str(more),
            _ => lines.push(raw.to_string()),
        }
    }
    lines.iter().filter_map(|l| prop(l)).collect()
}

/// One unfolded line; `None` for one with no `:` outside quotes.
fn prop(line: &str) -> Option<Prop> {
    let mut quoted = false;
    let colon = line.char_indices().find_map(|(i, c)| match c {
        '"' => {
            quoted = !quoted;
            None
        }
        ':' if !quoted => Some(i),
        _ => None,
    })?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let mut parts = split_unquoted(head, ';').into_iter();
    let name = parts.next()?.trim().to_ascii_uppercase();
    let params = parts
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().trim_matches('"').to_string()))
        .collect();
    Some(Prop {
        name,
        params,
        value: value.to_string(),
    })
}

fn split_unquoted(s: &str, sep: char) -> Vec<&str> {
    let (mut out, mut from, mut quoted) = (Vec::new(), 0, false);
    for (i, c) in s.char_indices() {
        match c {
            '"' => quoted = !quoted,
            c if c == sep && !quoted => {
                out.push(&s[from..i]);
                from = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[from..]);
    out
}

/// A TEXT value with its escapes undone: `\n`, `\,`, `\;`, `\\`.
pub(crate) fn text(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut it = v.chars();
    while let Some(c) = it.next() {
        match (c, c == '\\') {
            (_, true) => match it.next() {
                Some('n' | 'N') => out.push('\n'),
                Some(other) => out.push(other),
                None => {}
            },
            _ => out.push(c),
        }
    }
    out
}

/// `DTSTART`/`DTEND`: a date, a UTC time, or a time in its `TZID`.
pub(super) fn when(p: &Prop) -> Option<When> {
    let v = p.value.trim();
    if v.len() == 8
        || p.param("VALUE")
            .is_some_and(|x| x.eq_ignore_ascii_case("DATE"))
    {
        return NaiveDate::parse_from_str(&v[..v.len().min(8)], "%Y%m%d")
            .ok()
            .map(When::Day);
    }
    let t = NaiveDateTime::parse_from_str(&v[..v.len().min(15)], "%Y%m%dT%H%M%S").ok()?;
    match v.ends_with(['Z', 'z']) {
        true => Some(When::Utc(t)),
        false => Some(When::Local(t, p.param("TZID").map(str::to_string))),
    }
}

/// The first meeting link in `text`: Zoom, Teams, Meet, Webex, Whereby,
/// GoTo or Chime.
pub(super) fn join(text: &str) -> Option<String> {
    const HOSTS: [&str; 8] = [
        "zoom.us/",
        "teams.microsoft.com/",
        "teams.live.com/",
        "meet.google.com/",
        "webex.com/",
        "whereby.com/",
        "gotomeeting.com/",
        "chime.aws/",
    ];
    let urls = text.split(|c: char| c.is_whitespace() || "<>\"()".contains(c));
    let mut hits =
        urls.filter(|u| u.starts_with("https://") && HOSTS.iter().any(|h| u.contains(h)));
    hits.next()
        .map(|u| u.trim_end_matches(['.', ',', ';']).to_string())
}

pub(super) fn person(p: &Prop) -> Person {
    let v = p.value.trim();
    let email = match v.get(..7) {
        Some(m) if m.eq_ignore_ascii_case("mailto:") => &v[7..],
        _ => v,
    };
    Person {
        name: p.param("CN").map(text).unwrap_or_default(),
        email: email.to_ascii_lowercase(),
        answer: p
            .param("PARTSTAT")
            .unwrap_or("NEEDS-ACTION")
            .to_ascii_uppercase(),
    }
}

#[cfg(test)]
#[path = "icsline_tests.rs"]
mod tests;
