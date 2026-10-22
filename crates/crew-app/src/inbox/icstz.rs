//! Where an invite's time falls. A named zone is looked up in the IANA
//! database (`chrono-tz`), by Windows' own name through CLDR's table (Outlook
//! writes "Pacific Standard Time"), and, failing both, by the `VTIMEZONE`
//! rules the invite carries. A zone none of those can place is shown as
//! written and never added to a calendar: a wrong time is worse than none.
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;

use super::{Prop, When};

/// A [`When`] placed in time.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Placed {
    /// A whole day, wherever you are.
    Day(NaiveDate),
    /// An instant, with the IANA zone it was set in when the file named one.
    At(DateTime<Utc>, Option<Tz>),
    /// A time meant in your own zone: the file names none.
    Floating(NaiveDateTime),
    /// A time in a zone crew cannot place, and that zone's name.
    Unplaced(NaiveDateTime, String),
}

pub(crate) fn place(w: &When, zones: &[Vec<Prop>]) -> Placed {
    match w {
        When::Day(d) => Placed::Day(*d),
        When::Utc(t) => Placed::At(t.and_utc(), None),
        When::Local(t, None) => Placed::Floating(*t),
        When::Local(t, Some(id)) => match (tz(id), vzone(id, zones, *t)) {
            (Some(z), _) => Placed::At(local(&z, *t), Some(z)),
            (None, Some(off)) => Placed::At((*t - off).and_utc(), None),
            (None, None) => Placed::Unplaced(*t, id.clone()),
        },
    }
}

/// `t` on the clocks of `z`; a time the clocks skip (the spring-forward
/// hour) is read an hour on, as calendars read it.
fn local(z: &Tz, t: NaiveDateTime) -> DateTime<Utc> {
    let at = z.from_local_datetime(&t).earliest();
    let at = at.or_else(|| z.from_local_datetime(&(t + Duration::hours(1))).earliest());
    at.map_or_else(|| t.and_utc(), |d| d.with_timezone(&Utc))
}

/// The IANA zone a `TZID` names: as written, as the tail of a longer name
/// (Thunderbird's `/mozilla.org/20050126_1/America/New_York`), or by its
/// Windows name.
pub(crate) fn tz(id: &str) -> Option<Tz> {
    let id = id.trim().trim_matches('"');
    let segs: Vec<&str> = id.split('/').collect();
    let tails = (0..segs.len().saturating_sub(1)).map(|n| segs[n..].join("/"));
    let found = std::iter::once(id.to_string())
        .chain(tails)
        .find_map(|s| s.parse().ok());
    found.or_else(|| windows(id)?.parse().ok())
}

fn windows(id: &str) -> Option<&'static str> {
    let rows = include_str!("windowszones.txt")
        .lines()
        .filter(|l| !l.starts_with('#'));
    let hit = rows
        .filter_map(|l| l.split_once('\t'))
        .find(|(w, _)| w.eq_ignore_ascii_case(id));
    hit.map(|(_, iana)| iana)
}

/// The place a zone is named for: `America/Los_Angeles` is Los Angeles.
pub(crate) fn city(z: &Tz) -> String {
    z.name()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .replace('_', " ")
}

/// One STANDARD or DAYLIGHT block: when it first began, the offset it
/// moves the clocks to, and the yearly rule it begins again by.
#[derive(Default)]
struct Block {
    start: Option<NaiveDateTime>,
    to: Option<Duration>,
    rule: Option<(u32, i8, Weekday)>,
}

/// The offset (local minus UTC) the invite's own `VTIMEZONE` named `id`
/// gives at local time `t`: that of the block that began most recently.
fn vzone(id: &str, zones: &[Vec<Prop>], t: NaiveDateTime) -> Option<Duration> {
    let named = |z: &&Vec<Prop>| {
        z.iter()
            .any(|p| p.name == "TZID" && p.value.trim() == id.trim())
    };
    let zone = zones.iter().find(named)?;
    let mut best: Option<(NaiveDateTime, Duration)> = None;
    for b in blocks(zone) {
        let (Some(start), Some(to)) = (b.start, b.to) else {
            continue;
        };
        let onset = match b.rule {
            Some(r) => [t.year(), t.year() - 1].into_iter().find_map(|y| {
                let on = nth(y, r.0, r.1, r.2)?.and_time(start.time());
                (on <= t && on >= start).then_some(on)
            }),
            None => (start <= t).then_some(start),
        };
        if let Some(on) = onset.filter(|on| best.map_or(true, |(b, _)| *on > b)) {
            best = Some((on, to));
        }
    }
    best.map(|(_, to)| to)
}

fn blocks(zone: &[Prop]) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for p in zone {
        let v = p.value.trim();
        match (p.name.as_str(), out.last_mut()) {
            ("BEGIN", _) => out.push(Block::default()),
            ("DTSTART", Some(b)) => {
                b.start = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()
            }
            ("TZOFFSETTO", Some(b)) => b.to = offset(v),
            ("RRULE", Some(b)) => b.rule = yearly(v),
            _ => {}
        }
    }
    out
}

/// `-0800`, `+0530`, `+053000`.
fn offset(v: &str) -> Option<Duration> {
    let (sign, digits) = match v.split_at_checked(1)? {
        ("-", d) => (-1, d),
        ("+", d) => (1, d),
        _ => return None,
    };
    let num = |r: std::ops::Range<usize>| digits.get(r).and_then(|s| s.parse::<i64>().ok());
    let secs = num(0..2)? * 3600 + num(2..4)? * 60 + num(4..6).unwrap_or(0);
    Some(Duration::seconds(sign * secs))
}

/// `FREQ=YEARLY;BYMONTH=3;BYDAY=2SU`: the month, which one, and the day.
fn yearly(rule: &str) -> Option<(u32, i8, Weekday)> {
    let get = |k: &str| {
        rule.split(';')
            .find_map(|kv| kv.strip_prefix(k)?.strip_prefix('='))
    };
    let month = get("BYMONTH")?.parse().ok()?;
    let day = get("BYDAY")?;
    let (n, wd) = day.split_at(day.len().checked_sub(2)?);
    Some((
        month,
        if n.is_empty() { 1 } else { n.parse().ok()? },
        weekday(wd)?,
    ))
}

pub(crate) fn weekday(code: &str) -> Option<Weekday> {
    let days = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
    let i = days.iter().position(|d| d.eq_ignore_ascii_case(code))?;
    Weekday::try_from(i as u8).ok()
}

/// The `n`th `wd` of month `m` in year `y`; a negative `n` counts from the end.
fn nth(y: i32, m: u32, n: i8, wd: Weekday) -> Option<NaiveDate> {
    if n > 0 {
        return NaiveDate::from_weekday_of_month_opt(y, m, wd, n as u8);
    }
    let first_next = NaiveDate::from_ymd_opt(y + (m / 12) as i32, m % 12 + 1, 1)?;
    let last = first_next.pred_opt()?;
    let back = (7 + last.weekday().num_days_from_monday() - wd.num_days_from_monday()) % 7;
    Some(last - Duration::days(i64::from(back)) - Duration::weeks(i64::from(-n - 1)))
}

#[cfg(test)]
#[path = "icstz_tests.rs"]
mod tests;
