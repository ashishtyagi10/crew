//! Messages typed while the crew is busy: queued instead of sent immediately
//! (see the queued-messages design doc), then flushed one at a time as each
//! turn settles (the flush itself lives in `chat::ChatPane::poll`, since it
//! needs private field access). This module holds the pure bits: the `/stop`
//! bypass check and (a follow-up commit adds) the one-line "N queued"
//! indicator that claims a row above the composer, mirroring how
//! `chatswarmview::swarm_rows` claims rows for the live swarm block.

/// Whether `text` is (or starts) a `/stop` command — the one send that must
/// bypass the queue and reach the broker immediately even while busy, since
/// it's the cancel path for the in-flight run.
pub(crate) fn is_stop(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "/stop" || trimmed.starts_with("/stop ")
}

#[cfg(test)]
#[path = "chatqueue_tests.rs"]
mod tests;
