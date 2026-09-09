//! The card types the [`crate::ipc_types`] replies are built from: one row of a roster, a
//! session listing, a broadcast answer, a standing intent — and the reason an ask came back empty.
//!
//! Split from the envelope so the protocol file stays the protocol — every new listing adds a
//! struct here and one variant there.
use serde::{Deserialize, Serialize};

/// Why an ask returned without an answer.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum NoAnswer {
    /// Target went idle having produced nothing (no agent, or it ignored us).
    IdleNoEngage,
    /// Target produced output but never closed the sentinel.
    Stalled,
    /// Target was busy on its own work; we didn't disturb it.
    BusyElsewhere,
    /// No pane matched the address.
    Unreachable,
}

/// One pane's outcome within a broadcast reply. `text` is `Some` when it
/// answered; otherwise `no_answer` says why (both never set at once).
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct CastAnswer {
    pub pane: String,
    pub label: Option<String>,
    pub text: Option<String>,
    pub no_answer: Option<NoAnswer>,
}

/// One addressable pane in the `crew panes` roster.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PaneCard {
    pub id: String,
    pub label: Option<String>,
    pub kind: String,
    pub running: Option<String>,
    pub dir: Option<String>,
    pub busy: bool,
}

/// One session in a [`Reply::Sessions`] listing.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SessionCard {
    pub id: String,
    pub label: String,
    pub cwd: Option<String>,
    pub alive: bool,
}

/// One standing intent in a [`Reply::Watchlist`] listing. `repeat` is the
/// cadence spelled the way it was typed (`daily`, `every 30m`, `once`) rather
/// than a number, so an older client stays readable if the cadences grow.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct IntentCard {
    pub id: String,
    pub text: String,
    pub to: String,
    pub fire_ms: u64,
    pub repeat: String,
    pub created_ms: u64,
}
