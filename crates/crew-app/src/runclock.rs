//! How long the command in a pane has been running.
//!
//! The card's legend already names the foreground command; what it never said
//! is how long it has been at it. With agents in half the panes that is the
//! question you actually have — a build at nine seconds and a build at nine
//! minutes look identical otherwise, and the second one is news.
//!
//! Drawn only past [`MIN_SECS`]: every command is briefly a running command,
//! and a clock that appears on every `ls` is chrome.
use std::time::Duration;

/// How long a command must have been running before its clock is worth
/// drawing.
pub(crate) const MIN_SECS: u64 = 5;

/// The elapsed time as it rides the border: seconds under a minute, then
/// minutes and seconds, then hours and minutes. Never more than five columns,
/// because the border is shared with the legend, the git badge and the status
/// glyphs.
pub(crate) fn label(d: Duration) -> Option<String> {
    let secs = d.as_secs();
    if secs < MIN_SECS {
        return None;
    }
    Some(match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m{:02}", secs / 60, secs % 60),
        // Past four days the number stops being a duration anyone reads and
        // starts being a width problem on a shared border.
        3600..=345_599 => format!("{}h{:02}", secs / 3600, (secs % 3600) / 60),
        _ => format!("{}d", (secs / 86_400).min(99)),
    })
}

#[cfg(test)]
#[path = "runclock_tests.rs"]
mod tests;
