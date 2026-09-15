//! Readings taken off the docked sidebar's sampler by the other surfaces that
//! show them — today the collapsed nav's foot, which draws the same three
//! meters at five columns (`navrailfoot`).
//!
//! They sit outside `statspane.rs` because that file is at its line cap, and
//! what they are is two accessors: one sampler, so the rail and the open nav
//! can never disagree about what the machine is doing.
use crate::git::GitInfo;
use crate::stats::Stats;
use crate::statspane::StatsPane;

impl StatsPane {
    /// This tick's system reading — the sample the SYSTEM gauges draw.
    pub(crate) fn stats(&self) -> Stats {
        self.sampler.stats()
    }

    /// The watched repo's status, when the working directory is one.
    pub(crate) fn git_info(&self) -> Option<&GitInfo> {
        self.git.info()
    }
}
