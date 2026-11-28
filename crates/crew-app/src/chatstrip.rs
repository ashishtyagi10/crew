//! Where a turn's time went: think, read, edit, run.
//!
//! The receipt said what a turn changed but not why it took 40 s: thinking,
//! reading, or a slow test run? The pane already hears every tool call and
//! result for the code line's word (`chatphase`), so the same events, read
//! with a clock, say how long each phase held. No tool durations are needed:
//! the time between two events belongs to the phase the earlier one started,
//! which also covers `[tool]` cards that carry no `ms` and tools that overlap.
//! Model time is then whatever no tool held, with no subtraction to get wrong.
use crew_plugin::PluginEvent;

use crate::chatphase::{of_event, Phase};

/// The four things a turn's time is split into, in strip order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Think,
    Read,
    Edit,
    Run,
}

impl Kind {
    /// The strip's order; also the index into a turn's `[u64; 4]`.
    pub(crate) const ALL: [Kind; 4] = [Kind::Think, Kind::Read, Kind::Edit, Kind::Run];

    /// The bucket a phase's time goes in. Web, git and sub-agent waits are
    /// not the model's own thinking, but only git/web are a *look* at
    /// something, so those read; what no bucket names stays with the model.
    pub(crate) fn of(p: Phase) -> Kind {
        match p {
            Phase::Tracing | Phase::Decrypting | Phase::Reaching => Kind::Read,
            Phase::Rewriting => Kind::Edit,
            Phase::Compiling => Kind::Run,
            _ => Kind::Think,
        }
    }
}

/// The turn's clock: when it began, the phase now holding, and the time
/// each phase has held so far.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Clock {
    began: Option<u64>,
    at: u64,
    cur: usize,
    ms: [u64; 4],
}

impl Clock {
    pub(crate) fn start(now: u64) -> Self {
        Self {
            began: Some(now),
            at: now,
            ..Self::default()
        }
    }

    /// Fold in an event heard at `now`: the time since the last event goes
    /// to the phase that was holding, and the event may change the phase.
    pub(crate) fn hear(&mut self, ev: &PluginEvent, now: u64) {
        let Some(p) = of_event(ev) else { return };
        if self.began.is_some() {
            self.ms[self.cur] += now.saturating_sub(self.at);
            (self.at, self.cur) = (now, Kind::of(p) as usize);
        }
    }

    /// The durations at `now` (the open phase counted up to it), `None`
    /// for a turn that was never started or has run under a second.
    pub(crate) fn snapshot(&self, now: u64) -> Option<[u64; 4]> {
        self.began?;
        let mut ms = self.ms;
        ms[self.cur] += now.saturating_sub(self.at);
        (ms.iter().sum::<u64>() >= 1_000).then_some(ms)
    }
}

/// How the durations ride in a message's `meta`: ` · strip:t,r,e,u`, last,
/// so the task tag stays in front where `task_tag` reads it.
const TAG: &str = "strip:";

pub(crate) fn stamp(meta: &str, ms: [u64; 4]) -> String {
    let v: Vec<String> = ms.iter().map(u64::to_string).collect();
    let tag = format!("{TAG}{}", v.join(","));
    match meta.is_empty() {
        true => tag,
        false => format!("{meta}{}{tag}", crew_plugin::metatag::SEP),
    }
}

/// The durations a `meta` carries, if it was stamped.
pub(crate) fn read(meta: &str) -> Option<[u64; 4]> {
    let tag = meta.rsplit(crew_plugin::metatag::SEP).next()?;
    let v: Vec<u64> = tag
        .strip_prefix(TAG)?
        .split(',')
        .map(|n| n.parse().ok())
        .collect::<Option<_>>()?;
    v.try_into().ok()
}

/// `meta` without a stamp, for readers that want the latency.
pub(crate) fn unstamped(meta: &str) -> &str {
    match read(meta) {
        Some(_) => meta
            .rsplit_once(crew_plugin::metatag::SEP)
            .map_or("", |(rest, _)| rest),
        None => meta,
    }
}

#[cfg(test)]
#[path = "chatstrip_tests.rs"]
mod tests;
