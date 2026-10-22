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
}

pub(crate) fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("reminders.toml"))
}

pub(crate) fn load_at(p: Option<&Path>) -> State {
    let text = p.and_then(|p| std::fs::read_to_string(p).ok());
    text.and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
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
    use super::{load_at, State};
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
}
