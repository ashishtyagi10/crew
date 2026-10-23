//! What the sync remembers between passes, in `reminders.toml` beside
//! `todos.toml`: which `crew` list it keeps in step, and each todo's pair
//! with what both sides last agreed ([`Pair`]). Losing the file costs
//! nothing but a pass that pairs by title again.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::plan::Pair;

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub(crate) struct State {
    /// The Reminders list's identifier; empty before the first pass.
    #[serde(default)]
    pub list: String,
    #[serde(default)]
    pub pairs: Vec<Pair>,
    /// `/todo reminders` was typed since the last pass: a deleted list may be
    /// made again, once.
    #[serde(default)]
    pub remake: bool,
}

pub(crate) fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("reminders.toml"))
}

pub(crate) fn load_at(p: Option<&Path>) -> State {
    let text = p.and_then(|p| std::fs::read_to_string(p).ok());
    text.and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

/// Which `crew` list a pass keeps in step: the one remembered, else one so
/// named, else a new one, but a new one only before the first or when asked
/// (`remake`). A remembered list that is gone was deleted in Reminders;
/// making it again put every open todo back on the phone, unasked and unsaid.
#[derive(Debug, PartialEq)]
pub(crate) enum Pick {
    Remembered,
    Named,
    Make,
    Deleted,
}

pub(crate) fn pick(remembered: &str, found: bool, named: bool, remake: bool) -> Pick {
    match (found, named) {
        (true, _) => Pick::Remembered,
        (_, true) => Pick::Named,
        _ if remembered.is_empty() || remake => Pick::Make,
        _ => Pick::Deleted,
    }
}

pub(crate) const DELETED: &str =
    "Reminders: the crew list was deleted there \u{b7} /todo reminders makes it again";

/// `/todo reminders` turned on: a deleted list may be made again, and that is
/// all. It used to forget the pairs too, so every pass after a spell with the
/// sync off (or access refused, or the command typed again to check) paired
/// by title alone: what you had ticked or deleted in crew came back as new
/// open todos from its still-open reminder, and a rename doubled on the phone.
pub(crate) fn turned_on(p: Option<&Path>) {
    let s = load_at(p);
    if !s.list.is_empty() && !s.remake {
        save_at(p, &asked(s));
    }
}

/// The state once `/todo reminders` is typed: the list and pairs kept.
pub(crate) fn asked(s: State) -> State {
    State { remake: true, ..s }
}

/// Never from the test harness, as `todos.toml` is never written there.
pub(crate) fn save_at(p: Option<&Path>, s: &State) {
    let Some(p) = p.filter(|_| !cfg!(test)) else {
        return;
    };
    if let (Some(dir), Ok(text)) = (p.parent(), toml::to_string(s)) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(p, text);
    }
}

#[cfg(test)]
mod tests {
    use super::{asked, load_at, pick, Pick, State};
    use crate::reminders::{fields::Fields, plan::Pair};

    #[test]
    fn the_state_reads_back_and_a_missing_or_broken_file_is_a_fresh_start() {
        let s = State {
            list: "L1".into(),
            pairs: vec![Pair {
                todo: 7,
                rem: "r7".into(),
                last: Fields {
                    title: "milk".into(),
                    done: false,
                    due: Some("2026-10-24".into()),
                },
            }],
            remake: false,
        };
        let back: State = toml::from_str(&toml::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        assert_eq!(load_at(None), State::default());
        let dir = std::env::temp_dir().join(format!("crew-rem-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("reminders.toml");
        std::fs::write(&p, "pairs = 3").unwrap();
        assert_eq!(load_at(Some(&p)), State::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The list deleted in Reminders stays deleted until you ask again.
    #[test]
    fn a_remembered_list_that_is_gone_is_never_made_again_unasked() {
        assert_eq!(pick("L1", true, true, false), Pick::Remembered);
        assert_eq!(
            pick("L1", false, true, false),
            Pick::Named,
            "made again by you"
        );
        assert_eq!(pick("", false, false, false), Pick::Make, "the first pass");
        assert_eq!(pick("L1", false, false, false), Pick::Deleted);
        assert_eq!(pick("L1", false, false, true), Pick::Make, "asked for");
        assert_eq!(
            pick("L1", true, true, true),
            Pick::Remembered,
            "still there"
        );
    }

    /// Typing `/todo reminders` keeps what both sides agreed: the next pass
    /// finds the list by its id and goes on from the pairs.
    #[test]
    fn turning_it_on_again_keeps_the_list_and_its_pairs() {
        let pair = Pair {
            todo: 7,
            rem: "r7".into(),
            last: Fields {
                title: "milk".into(),
                done: false,
                due: None,
            },
        };
        let s = asked(State {
            list: "L1".into(),
            pairs: vec![pair.clone()],
            remake: false,
        });
        assert_eq!(
            (s.list.as_str(), s.pairs, s.remake),
            ("L1", vec![pair], true)
        );
    }
}
