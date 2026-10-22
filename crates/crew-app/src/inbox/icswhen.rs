//! An invite's time in words, in your zone: `Thu Oct 30 · 10:00–10:45`, and
//! `· 7:00–7:45 in Los Angeles` beside it when it was set somewhere whose
//! clocks say otherwise. The year shows only when it is not this one.
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone};

use super::icstz::{city, place, Placed};
use super::Invite;

/// `DURATION`: `PT45M`, `PT1H30M`, `P1D`, `P1DT2H`, `P2W`.
pub(crate) fn duration(v: &str) -> Option<Duration> {
    let v = v.trim().trim_start_matches('+');
    let rest = v.strip_prefix('P')?;
    let (mut total, mut num, mut in_time) = (Duration::zero(), String::new(), false);
    for c in rest.chars() {
        let n = || num.parse::<i64>().ok();
        let unit = match c {
            '0'..='9' => {
                num.push(c);
                continue;
            }
            'T' => {
                in_time = true;
                continue;
            }
            'W' => Duration::weeks(n()?),
            'D' => Duration::days(n()?),
            'H' if in_time => Duration::hours(n()?),
            'M' if in_time => Duration::minutes(n()?),
            'S' if in_time => Duration::seconds(n()?),
            _ => return None,
        };
        total += unit;
        num.clear();
    }
    Some(total)
}

/// When the event is, as the invite card says it, read in `here`'s clocks.
/// `None` when the invite has no start (it always has, once parsed).
pub(crate) fn span<Z: TimeZone>(inv: &Invite, here: &Z, today: NaiveDate) -> Option<String> {
    let start = place(inv.start.as_ref()?, &inv.zones);
    let end = inv.end.as_ref().map(|e| place(e, &inv.zones));
    let day = |d: NaiveDate| match d.year() == today.year() {
        true => d.format("%a %b %-d").to_string(),
        false => d.format("%a %b %-d %Y").to_string(),
    };
    Some(match (start, end) {
        (Placed::Day(d), end) => {
            let last = match end {
                Some(Placed::Day(e)) if e > d + Duration::days(1) => Some(e - Duration::days(1)),
                _ => None,
            };
            match last {
                Some(l) => format!("{} \u{2013} {} \u{b7} all day", day(d), day(l)),
                None => format!("{} \u{b7} all day", day(d)),
            }
        }
        (Placed::At(s, zone), end) => {
            let e = match end {
                Some(Placed::At(e, _)) => Some(e),
                _ => None,
            };
            let mine = range(
                s.with_timezone(here).naive_local(),
                e.map(|e| e.with_timezone(here).naive_local()),
                &day,
            );
            let there = zone.filter(|z| {
                s.with_timezone(z).offset().fix() != s.with_timezone(here).offset().fix()
            });
            match there {
                Some(z) => {
                    let (zs, ze) = (
                        s.with_timezone(&z).naive_local(),
                        e.map(|e| e.with_timezone(&z).naive_local()),
                    );
                    format!("{mine} \u{b7} {} in {}", clock(zs, ze), city(&z))
                }
                None => mine,
            }
        }
        (Placed::Floating(s), end) => {
            let e = match end {
                Some(Placed::Floating(e)) => Some(e),
                _ => None,
            };
            range(s, e, &day)
        }
        (Placed::Unplaced(s, id), end) => {
            let e = match end {
                Some(Placed::Unplaced(e, _)) => Some(e),
                _ => None,
            };
            format!("{} in {id}, as written", range(s, e, &day))
        }
    })
}

/// `Thu Oct 30 · 10:00–10:45`, or across midnight
/// `Thu Oct 30, 22:00 – Fri Oct 31, 01:00`.
fn range(s: NaiveDateTime, e: Option<NaiveDateTime>, day: &dyn Fn(NaiveDate) -> String) -> String {
    // Ending at the next midnight is still the one day: `23:00–00:00`.
    let midnight =
        |e: NaiveDateTime| e.time() == NaiveTime::MIN && e.date() == s.date() + Duration::days(1);
    match e {
        Some(e) if e.date() != s.date() && !midnight(e) => {
            format!(
                "{}, {} \u{2013} {}, {}",
                day(s.date()),
                hm(s),
                day(e.date()),
                hm(e)
            )
        }
        _ => format!("{} \u{b7} {}", day(s.date()), clock(s, e)),
    }
}

/// `10:00–10:45`, or `10:00` with no end.
fn clock(s: NaiveDateTime, e: Option<NaiveDateTime>) -> String {
    match e.filter(|e| *e > s) {
        Some(e) => format!("{}\u{2013}{}", hm(s), hm(e)),
        None => hm(s),
    }
}

fn hm(t: NaiveDateTime) -> String {
    t.format("%H:%M").to_string()
}

/// An `RRULE` in words: `every week on Tuesday`, `every weekday`,
/// `every 2 weeks on Monday and Wednesday, until Dec 18`.
pub(crate) fn repeat(rule: &str) -> String {
    let get = |k: &str| {
        rule.split(';')
            .find_map(|kv| kv.strip_prefix(k)?.strip_prefix('='))
    };
    let every = get("INTERVAL")
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| *n > 1);
    let unit = |one: &str, many: &str| match every {
        Some(n) => format!("every {n} {many}"),
        None => format!("every {one}"),
    };
    let days: Vec<String> = get("BYDAY")
        .map(|d| {
            d.split(',')
                .filter_map(|c| super::icstz::weekday(c.trim()))
                .map(|w| full(w).to_string())
                .collect()
        })
        .unwrap_or_default();
    let mut said = match get("FREQ").unwrap_or_default() {
        "DAILY" if days.len() == 5 && !days.iter().any(|d| d.starts_with('S')) => {
            "every weekday".to_string()
        }
        "DAILY" => unit("day", "days"),
        "WEEKLY" if days.is_empty() => unit("week", "weeks"),
        "WEEKLY" => format!("{} on {}", unit("week", "weeks"), and(&days)),
        "MONTHLY" => unit("month", "months"),
        "YEARLY" => unit("year", "years"),
        _ => return "repeats".to_string(),
    };
    if let Some(n) = get("COUNT") {
        said.push_str(&format!(", {n} times"));
    } else if let Some(d) =
        get("UNTIL").and_then(|u| NaiveDate::parse_from_str(u.get(..8)?, "%Y%m%d").ok())
    {
        said.push_str(&format!(", until {}", d.format("%b %-d")));
    }
    said
}

fn full(w: chrono::Weekday) -> &'static str {
    [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ][w.num_days_from_monday() as usize]
}

/// `Monday`, `Monday and Wednesday`, `Monday, Wednesday and Friday`.
fn and(items: &[String]) -> String {
    match items.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "icswhen_tests.rs"]
mod tests;
