//! One answer to "what is this pane doing", for its entry in the nav's PANES
//! list: the state word, the spine's colour and whether the entry animates
//! all read [`state_of`], so the three never disagree.
//!
//! Every pane kind says it from what it already knows. A terminal is the
//! case that matters most and knows least — no protocol says "an agent is
//! thinking" — so it is working while a command runs and its screen's tail
//! is still moving ([`crate::panegist::Gist::moving`]: a timer ticking, output
//! arriving) or while the program reports progress (OSC 9;4); waiting while
//! [`crate::blocked`] reads a question on a still tail.
use crate::pane::{Pane, PaneContent};

/// What a pane's entry says about it, loudest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum EntryState {
    /// Holding a question, or rang: it needs you.
    Waiting,
    Working,
    /// Its command or agent turn failed.
    Failed,
    /// Its agent finished a turn you have not looked at.
    Done,
    /// Its program exited.
    Exited,
    #[default]
    Idle,
}

impl EntryState {
    /// The word on the entry's first line; `None` for an idle pane, whose
    /// entry says nothing it does not need to.
    pub(crate) fn word(self) -> Option<&'static str> {
        Some(match self {
            EntryState::Waiting => "you",
            EntryState::Working => "working",
            EntryState::Failed => "failed",
            EntryState::Done => "done",
            EntryState::Exited => "exited",
            EntryState::Idle => return None,
        })
    }
}

/// The state of `p` at `now`.
pub(crate) fn state_of(p: &Pane, now: u64) -> EntryState {
    let raised = p.attention.map(|a| a.glyph());
    if crate::blocked::pane_blocked(p, now) || matches!(raised, Some('!' | '⚑' | '?' | '▸')) {
        return EntryState::Waiting;
    }
    if working(p, now) {
        return EntryState::Working;
    }
    match raised {
        Some('✗') => EntryState::Failed,
        Some('✓') => EntryState::Done,
        Some('⊗') => EntryState::Exited,
        _ => EntryState::Idle,
    }
}

fn working(p: &Pane, now: u64) -> bool {
    crate::paneview::pane_working(p)
        || matches!(&p.content, PaneContent::Terminal(t)
            if t.cmd.is_some() && (t.tail.gist.moving(now) || t.pty.progress().is_some()))
}

/// The glyph that leads an entry's second line: what kind of thing runs
/// there. Agent CLIs by the mark they draw for themselves; everything crew
/// draws by its kind.
pub(crate) fn glyph_of(p: &Pane) -> char {
    match &p.content {
        PaneContent::Terminal(t) => match t.cmd.as_deref() {
            Some("claude") => '✳',
            Some("codex") => '◎',
            Some("gemini") => '✦',
            Some(_) => '›',
            None => '❯',
        },
        PaneContent::Chat(_) | PaneContent::Swarm(_) => '◆',
        PaneContent::Far(_) | PaneContent::Disk(_) => '▤',
        PaneContent::View(_) => '¶',
        PaneContent::Todo(_) => '☐',
        PaneContent::Inbox(_) => '✉',
        PaneContent::Settings(_) => '⚙',
        PaneContent::Usage(_) | PaneContent::Dash(_) => '▦',
    }
}

/// The entry's second line: what the pane is doing ([`crate::panegist`] for
/// a terminal running something, its folder for a shell at its prompt, its
/// own one-line glance for everything crew draws).
pub(crate) fn detail_of(p: &Pane) -> Option<String> {
    match &p.content {
        PaneContent::Terminal(t) => match (&t.cmd, &t.tail.gist.summary) {
            (Some(_), Some(s)) => Some(s.clone()),
            (Some(cmd), None) => Some(cmd.clone()),
            (None, _) => p.dir.as_deref().map(crate::panestate::home_path),
        },
        _ => crate::minpreview::of(p),
    }
}

/// The entry's first line: the pane's name without what runs in it, which
/// its second line says — [`Pane::title_text`]'s folder, for a terminal.
pub(crate) fn name_of(p: &Pane) -> String {
    let title = p.title_text();
    match (&p.name, &p.content) {
        (None, PaneContent::Terminal(t)) => match &t.cmd {
            Some(cmd) => title
                .strip_suffix(&format!(" \u{b7} {cmd}"))
                .map(str::to_string)
                .unwrap_or(title),
            None => title,
        },
        _ => title,
    }
}

/// `path` with the home folder as `~`.
pub(crate) fn home_path(path: &std::path::Path) -> String {
    match std::env::var_os("HOME").map(std::path::PathBuf::from) {
        Some(home) if path.starts_with(&home) => {
            let rest = path.strip_prefix(&home).unwrap_or(path);
            if rest.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", rest.display())
            }
        }
        _ => path.display().to_string(),
    }
}

#[cfg(test)]
#[path = "panestate_tests.rs"]
mod tests;
