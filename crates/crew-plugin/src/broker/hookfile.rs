//! Where a session's hooks come from: one user-level file, `<config>/crew/hooks.json`, its
//! shape, and the switch that turns them all off. A child of `hooks`, so the `Hooks` it loads
//! is built here without a constructor that only this file would call.
use std::path::PathBuf;
use std::time::Duration;

use super::{Hooks, DEADLINE};

/// `CREW_HOOKS=0` turns every hook off: the way out when a hook has gone wrong and the agent
/// that could fix it is the thing it is blocking. The env read lives in [`Source::live`]; this
/// is the verdict, so a test can ask it without touching the process environment.
pub(crate) fn switch(value: Option<&str>) -> bool {
    value != Some("0")
}

/// The hooks file under `config`, while hooks are on.
pub(crate) fn location(on: bool, config: Option<PathBuf>) -> Option<PathBuf> {
    config
        .filter(|_| on)
        .map(|d| d.join("crew").join("hooks.json"))
}

/// Where a session's hooks are read from, how long each may run, and where.
#[derive(Debug, Clone)]
pub(crate) struct Source {
    pub path: PathBuf,
    pub deadline: Duration,
    /// The directory hooks run in; `None` is the project, the broker's own directory.
    pub cwd: Option<PathBuf>,
}

impl Source {
    /// The user's file, or `None` when `CREW_HOOKS=0` or there is no config directory.
    pub(crate) fn live() -> Option<Self> {
        let on = switch(std::env::var("CREW_HOOKS").ok().as_deref());
        location(on, dirs::config_dir()).map(|path| Self {
            path,
            deadline: DEADLINE,
            cwd: None,
        })
    }

    /// The file as it stands now, read once for both halves of one call.
    pub(crate) fn load(&self) -> Hooks {
        let shown = self.path.display();
        let file = match std::fs::read_to_string(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(File::default()),
            Err(e) => Err(format!("{shown} could not be read ({e})")),
            Ok(text) if text.trim().is_empty() => Ok(File::default()),
            Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{shown}: {e}")),
        };
        Hooks {
            file,
            deadline: self.deadline,
            cwd: self.cwd.clone(),
        }
    }
}

/// `hooks.json`. Unknown keys are refused rather than skipped: a misspelt `pre_tools` would
/// otherwise be a guard that looks configured and never runs.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct File {
    #[serde(default)]
    pub(super) pre_tool: Vec<Rule>,
    #[serde(default)]
    pub(super) post_tool: Vec<Rule>,
}

/// One hook: the calls it is for, the command, and what a hook that cannot run means.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rule {
    #[serde(rename = "match")]
    pub(super) pattern: String,
    pub(super) cmd: String,
    #[serde(default)]
    pub(super) on_failure: OnFailure,
}

/// `allow` by default, so a typo in a script path does not stop an agent dead. A hook that IS
/// the guard says `block`, and then a guard that cannot run is a guard that refuses.
#[derive(Debug, Default, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum OnFailure {
    #[default]
    Allow,
    Block,
}
