//! The invite reader held to its corpus (`invite_corpus.toml`): every
//! invite's time, read in two zones that differ from each other and from
//! most invites' own, with the details the card shows.
use serde::Deserialize;

use super::icswhen::{repeat, span};
use super::parse;

#[derive(Deserialize)]
struct Corpus {
    invite: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    ics: String,
    summary: String,
    new_york: String,
    kolkata: String,
    #[serde(default)]
    organizer: Option<String>,
    #[serde(default)]
    link: Option<String>,
    #[serde(default)]
    repeat: Option<String>,
    #[serde(default)]
    cancelled: bool,
    #[serde(default)]
    sequence: u32,
}

#[test]
fn every_invite_reads_right_in_new_york_and_in_kolkata() {
    let corpus: Corpus = toml::from_str(include_str!("invite_corpus.toml")).unwrap();
    assert!(corpus.invite.len() >= 20, "the goal asks for about twenty");
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
    for c in &corpus.invite {
        let inv = parse(&c.ics).unwrap_or_else(|| panic!("{}: not read", c.name));
        let ny = span(&inv, &chrono_tz::America::New_York, today);
        let kol = span(&inv, &chrono_tz::Asia::Kolkata, today);
        assert_eq!(
            ny.as_deref(),
            Some(c.new_york.as_str()),
            "{}: New York",
            c.name
        );
        assert_eq!(
            kol.as_deref(),
            Some(c.kolkata.as_str()),
            "{}: Kolkata",
            c.name
        );
        assert_eq!(inv.summary, c.summary, "{}", c.name);
        let org = inv.organizer.as_ref().map(|o| o.name.clone());
        if c.organizer.is_some() {
            assert_eq!(org, c.organizer, "{}", c.name);
        }
        assert_eq!(inv.link, c.link, "{}", c.name);
        assert_eq!(inv.rrule.as_deref().map(repeat), c.repeat, "{}", c.name);
        assert_eq!(
            (inv.cancelled, inv.sequence),
            (c.cancelled, c.sequence),
            "{}",
            c.name
        );
    }
}
