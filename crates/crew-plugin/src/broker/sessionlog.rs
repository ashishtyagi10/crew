//! Session persistence (à la Claude Code's `--continue` / OpenCode sessions):
//! the broker auto-saves the conversation — the user's tasks and every agent
//! reply — to `./.crew/session-live.md` as it streams. On the next broker
//! start the live log rotates to `./.crew/last-session.md`, and `/resume`
//! folds its tail into the next task as restored context, so a fresh `/crew`
//! pane can pick up where the last one left off.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// On-disk budget for the live log; the oldest half is dropped past this.
const LOG_CAP: usize = 32 * 1024;
/// How much of the previous session `/resume` folds into the next task.
const RESUME_CAP: usize = 2048;

/// The restored-context slot: set by `/resume`, consumed by the next task.
pub(crate) type SharedResume = Arc<Mutex<Option<String>>>;

fn live(base: &Path) -> PathBuf {
    base.join(".crew").join("session-live.md")
}

fn last(base: &Path) -> PathBuf {
    base.join(".crew").join("last-session.md")
}

/// The project dir the log lives under. Mirrors `specialists::base_dir`
/// exactly, and for the same reason: `CREW_PROJECT_DIR` overrides the process
/// CWD — the seam tests use, since lib tests share one CWD and cannot each
/// chdir. Production never sets it: the broker's CWD *is* the project. Before
/// this seam existed, `rotate`/`append`/`tail` hardcoded `Path::new(".")`, so
/// any in-process test that reached them (not just the ones written against
/// `specialists`) wrote a real `./.crew/session-live.md` into the crate's own
/// working tree.
fn base_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Broker startup: the previous run's live log becomes the resumable
/// last-session file (a crash still leaves it resumable), and a fresh live
/// log begins.
pub(crate) fn rotate() {
    rotate_at(&base_dir());
}

pub(crate) fn rotate_at(base: &Path) {
    let (lv, la) = (live(base), last(base));
    if lv.exists() {
        let _ = std::fs::rename(&lv, &la);
    }
    let _ = std::fs::remove_file(&lv);
}

/// Append one line of conversation to the live log (best-effort — logging
/// must never break the relay). Empty text and the `agent smith` system voice
/// are skipped; the file is capped at [`LOG_CAP`] by dropping the oldest half.
pub(crate) fn append(sender: &str, text: &str) {
    append_at(&base_dir(), sender, text);
}

pub(crate) fn append_at(base: &Path, sender: &str, text: &str) {
    let text = text.trim();
    if text.is_empty() || sender == "agent smith" || sender == "crew" {
        return;
    }
    let path = live(base);
    if std::fs::create_dir_all(path.parent().unwrap()).is_err() {
        return;
    }
    let mut log = std::fs::read_to_string(&path).unwrap_or_default();
    log.push_str(&format!("{sender}: {text}\n"));
    if log.len() > LOG_CAP {
        // Drop the oldest half at a line boundary.
        let mut cut = log.len() / 2;
        while cut < log.len() && log.as_bytes()[cut] != b'\n' {
            cut += 1;
        }
        log = log.split_off(cut.min(log.len().saturating_sub(1)) + 1);
    }
    let _ = std::fs::write(&path, log);
}

/// The tail of the previous session, at most [`RESUME_CAP`] chars on a line
/// boundary. `None` when there is nothing to resume.
pub(crate) fn tail() -> Option<String> {
    tail_at(&base_dir())
}

pub(crate) fn tail_at(base: &Path) -> Option<String> {
    let text = std::fs::read_to_string(last(base)).ok()?;
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if text.len() <= RESUME_CAP {
        return Some(text.to_string());
    }
    let mut start = text.len() - RESUME_CAP;
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    let cut = text[start..].find('\n').map_or(start, |i| start + i + 1);
    Some(text[cut..].trim().to_string())
}

/// Wrap the next task with restored context (the `/resume` payload).
pub(crate) fn with_resume(prev: &str, task: &str) -> String {
    format!(
        "PREVIOUS SESSION (restored context — the conversation so far):\n\
         {prev}\n\nCURRENT TASK:\n{task}"
    )
}

#[cfg(test)]
#[path = "sessionlog_tests.rs"]
mod tests;
