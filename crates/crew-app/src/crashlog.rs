//! Never vanish silently.
//!
//! A detached crew runs with stderr on `/dev/null` (see [`crate::detach`]), and
//! a Rust panic exits through the *normal* exit path — so macOS files no crash
//! report. The two together meant a panic on the winit thread took the whole
//! app down leaving nothing at all behind: no message, no report, and (because
//! `exiting()` never runs) not even a refreshed `session.toml`. From the
//! outside the window simply disappeared.
//!
//! This module makes that impossible. A panic hook writes the panic message,
//! location and backtrace to `crash.log` next to the config, and drops a
//! one-line `last-crash` marker that the NEXT launch consumes — so instead of
//! being mysteriously gone, crew comes back and says why it went.
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;

/// Keep the log bounded; a crash loop must not fill the disk. When the file
/// exceeds this, it is truncated before the new record is appended.
const MAX_LOG_BYTES: u64 = 256 * 1024;

/// Directory holding the log + marker (the same dir as `config.toml`).
fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew"))
}

/// Full path of the human-readable crash log.
pub(crate) fn log_path() -> Option<PathBuf> {
    dir().map(|d| d.join("crash.log"))
}

/// Marker written by the panic hook and consumed by the next launch.
fn marker_path() -> Option<PathBuf> {
    dir().map(|d| d.join("last-crash"))
}

/// Where the detached child's stderr is redirected. Anything the app prints
/// outside the panic hook (`eprintln!`, wgpu/winit warnings) lands here rather
/// than in `/dev/null`, which is what made the original disappearance
/// undiagnosable.
pub fn stderr_path() -> Option<PathBuf> {
    dir().map(|d| d.join("stderr.log"))
}

/// One panic record, ready to append. Pure string building so it can be tested
/// without installing a hook or actually panicking.
fn record(when: &str, version: &str, thread: &str, msg: &str, loc: &str, trace: &str) -> String {
    format!(
        "\n===== crew panic =====\n\
         when:    {when}\n\
         version: {version}\n\
         thread:  {thread}\n\
         where:   {loc}\n\
         message: {msg}\n\
         backtrace:\n{trace}\n"
    )
}

/// The one-line summary shown to the user on the next launch.
fn summary(when: &str, msg: &str) -> String {
    // Keep it to a single line: this is flashed in the status bar, and a
    // multi-line panic message (assertion failures are often several) would
    // otherwise smear across the chrome.
    // Cut at 120, and MARKED: an assertion message runs past that as a
    // rule, and the one line saying why crew died used to end mid-sentence
    // looking complete.
    let first = msg.lines().next().unwrap_or(msg).trim();
    let first = crate::chatwidth::clip_w(first, 120);
    format!("{when} — {first}")
}

/// Append `text` to `path`, truncating first if the file has grown past the cap.
fn append_capped(path: &PathBuf, text: &str) {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_LOG_BYTES {
            let _ = std::fs::remove_file(path);
        }
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(text.as_bytes());
        let _ = f.flush();
    }
}

/// Install the panic hook. Call once, as early in `main` as possible — before
/// any window or worker thread exists, so nothing can panic un-recorded.
///
/// The previous (default) hook still runs afterwards, so an attached
/// `--no-detach` run keeps printing the familiar panic message to the terminal.
pub fn install() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Everything here must be panic-free: a panic inside the panic hook
        // aborts the process instantly and we lose the very record we came to
        // write. Hence no unwrap/expect and no indexing below.
        let when = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let msg = payload_str(info);
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let thread = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        let trace = std::backtrace::Backtrace::force_capture().to_string();

        if let Some(d) = dir() {
            let _ = std::fs::create_dir_all(&d);
        }
        if let Some(p) = log_path() {
            append_capped(
                &p,
                &record(
                    &when,
                    crate::appregister::VERSION,
                    &thread,
                    &msg,
                    &loc,
                    &trace,
                ),
            );
        }
        // The marker is what turns "it vanished" into "it crashed, here's why"
        // on the next launch. Written last: if anything above failed, the log
        // may be missing, but the user still gets told something went wrong.
        if let Some(p) = marker_path() {
            let _ = std::fs::write(&p, summary(&when, &msg));
        }
        prev(info);
    }));
}

/// The panic payload as a string. `&str` and `String` cover essentially every
/// real panic (`panic!`, `unwrap`, `expect`, assertions).
fn payload_str(info: &std::panic::PanicHookInfo<'_>) -> String {
    let p = info.payload();
    if let Some(s) = p.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// If the previous run died in a panic, return its one-line summary and clear
/// the marker (so the note shows exactly once). `None` on a clean previous run.
pub(crate) fn take_report() -> Option<String> {
    take_report_at(marker_path())
}

/// [`take_report`] against an explicit path, so the consume-exactly-once
/// behaviour is testable without touching the real config dir.
fn take_report_at(path: Option<PathBuf>) -> Option<String> {
    let p = path?;
    let s = std::fs::read_to_string(&p).ok()?;
    // Removed whether or not the text is usable: a marker that survived would
    // re-announce the same crash on every launch from here on.
    let _ = std::fs::remove_file(&p);
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// The note shown on the launch after a crash. Names the log so the user has
/// somewhere to look rather than just being told bad news.
pub(crate) fn crash_note(summary: &str) -> String {
    match log_path() {
        Some(p) => format!(
            "crew closed unexpectedly last run ({summary}) — see {}",
            p.display()
        ),
        None => format!("crew closed unexpectedly last run ({summary})"),
    }
}

#[cfg(test)]
#[path = "crashlog_tests.rs"]
mod tests;
