//! Session activity-log file: every LOG line (each one flows through
//! `set_status_level`, the single choke point) is also appended to
//! `activity.log` next to the config — so `/log` can open the FULL session
//! history in the file viewer while the sidebar LOG keeps only its 5-line
//! tail and the in-memory buffer its 64-entry cap, and a wedged or crashed
//! session leaves its trail on disk (the `crash.log` lesson). The file is
//! truncated on the first write of each process — a session log, like
//! `stderr.log` — and never written under test.
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::applog::LogLevel;

/// Full path of the session activity log.
pub(crate) fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("activity.log"))
}

/// The one formatting rule: errors carry a greppable `ERR ` lead.
fn render(level: LogLevel, line: &str) -> String {
    match level {
        LogLevel::Error => format!("ERR {line}"),
        LogLevel::Info => line.to_string(),
    }
}

static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Append one already-stamped LOG line. Lazily creates (and truncates) the
/// file on the process's first entry; every failure is swallowed — the log
/// file must never be able to take the app down or spam the LOG itself.
pub(crate) fn append(level: LogLevel, line: &str) {
    if cfg!(test) {
        return; // the CrewConfig::save rule: tests never touch the real HOME
    }
    let Ok(mut guard) = FILE.lock() else { return };
    if guard.is_none() {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(mut f) = std::fs::File::create(&p) else {
            return;
        };
        let _ = writeln!(
            f,
            "crew v{} — session activity log (newest last; ERR = error)",
            env!("CARGO_PKG_VERSION")
        );
        *guard = Some(f);
    }
    if let Some(f) = guard.as_mut() {
        let _ = writeln!(f, "{}", render(level, line));
    }
}

#[cfg(test)]
#[path = "activitylog_tests.rs"]
mod tests;
