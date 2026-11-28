//! Resolves a pane's foreground PID to a command name (e.g. `claude`, `codex`)
//! for its title. Owns a `sysinfo::System` and refreshes only the handful of PIDs
//! we ask about, throttled to ~1×/s so naming never costs a full process scan per
//! frame.
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

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
            // The argv too, once per process: `cargo` is a name, `cargo test
            // -p crew-app` is what a failure report has to say.
            ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet),
        );
    }

    /// The command name of `pid` from the last refresh, if known.
    pub(crate) fn name(&self, pid: u32) -> Option<String> {
        self.sys
            .process(Pid::from_u32(pid))
            .map(|p| p.name().to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
    }

    /// [`Self::line`] of the foreground `pid` while a command `running` has it.
    pub(crate) fn line_if(&self, pid: Option<u32>, running: &Option<String>) -> Option<String> {
        pid.filter(|_| running.is_some()).and_then(|p| self.line(p))
    }

    /// The whole command line of `pid` (`cargo test -p crew-app`) from the
    /// last refresh, if the process table had it.
    pub(crate) fn line(&self, pid: u32) -> Option<String> {
        let p = self.sys.process(Pid::from_u32(pid))?;
        let argv: Vec<String> = p
            .cmd()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        command_line(&argv)
    }
}

/// `argv` as the line a user would have typed: the program's own name, not
/// its path, then its arguments. `None` for an empty argv.
pub(crate) fn command_line(argv: &[String]) -> Option<String> {
    let (first, rest) = argv.split_first()?;
    let prog = first.rsplit(['/', '\\']).next().unwrap_or(first);
    let line = std::iter::once(prog).chain(rest.iter().map(String::as_str));
    Some(line.collect::<Vec<_>>().join(" ")).filter(|l| !l.trim().is_empty())
}

#[cfg(test)]
#[path = "procname_tests.rs"]
mod tests;
