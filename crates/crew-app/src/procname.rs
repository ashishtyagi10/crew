//! Resolves a pane's foreground PID to a command name (e.g. `claude`, `codex`)
//! for its title. Owns a `sysinfo::System` and refreshes only the handful of PIDs
//! we ask about, throttled to ~1×/s so naming never costs a full process scan per
//! frame.
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// How often the process table may be refreshed for pane titles.
const SCAN_EVERY: Duration = Duration::from_millis(1000);

pub(crate) struct ProcNames {
    sys: System,
    last: Option<Instant>,
}

impl Default for ProcNames {
    fn default() -> Self {
        Self {
            sys: System::new(),
            last: None,
        }
    }
}

impl ProcNames {
    /// Whether a refresh is due (≥ `SCAN_EVERY` since the last one).
    pub(crate) fn due(&self) -> bool {
        self.last.is_none_or(|t| t.elapsed() >= SCAN_EVERY)
    }

    /// Refresh just `pids` (one batched query), marking the scan time. Empty
    /// `pids` still advances the throttle so an idle tick stays cheap.
    pub(crate) fn refresh(&mut self, pids: &[u32]) {
        self.last = Some(Instant::now());
        if pids.is_empty() {
            return;
        }
        let pids: Vec<Pid> = pids.iter().map(|&p| Pid::from_u32(p)).collect();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&pids),
            false,
            ProcessRefreshKind::nothing(),
        );
    }

    /// The command name of `pid` from the last refresh, if known.
    pub(crate) fn name(&self, pid: u32) -> Option<String> {
        self.sys
            .process(Pid::from_u32(pid))
            .map(|p| p.name().to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_this_test_process() {
        // Resolve our own PID — proves the refresh + lookup path works end to end
        // without depending on any particular external program being alive.
        let me = std::process::id();
        let mut pn = ProcNames::default();
        assert!(pn.due(), "a fresh ProcNames is due immediately");
        pn.refresh(&[me]);
        assert!(!pn.due(), "after a refresh the throttle holds");
        assert!(pn.name(me).is_some(), "our own process should resolve");
    }

    #[test]
    fn unknown_pid_is_none() {
        let mut pn = ProcNames::default();
        // PID 0 is never a real userland process to name.
        pn.refresh(&[]);
        assert_eq!(pn.name(0), None);
    }
}
