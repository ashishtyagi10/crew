//! Provider auth: the registry (which providers exist and how each one
//! authenticates — data, not forks) and the one resolution function that
//! orders them: pin, then signed-in subscriptions, then API keys, then
//! installed CLIs. Consent-based only: the CLI-delegated rung drives a
//! vendor CLI's own commands and never reads another app's token store.

// The resolution layer's live callers (the probe + delegated starter) land
// with the CLI-delegated rung, the next commit on this branch; until then
// the table-driven tests are its only users.
#![allow(dead_code, unused_imports)]

pub(crate) mod registry;
pub(crate) mod resolve;

pub(crate) use resolve::{resolve, Resolved, Signals};
