//! Budget governor: watches the event bus and trips the scheduler cancel flag
//! when fleet cost exceeds the configured cap.
//!
//! Spawn `budget_governor` as a background task alongside a `Scheduler::run`.
//! Pass the same `Arc<AtomicBool>` to both via `Scheduler::with_cancel`.
//!
//! # Channel lifetime
//! `budget_governor` takes `bus: EventBus` **by value** and drops the sender
//! immediately after subscribing.  This means the channel closes as soon as
//! all *other* senders (the scheduler, the test harness, etc.) are dropped —
//! the governor itself does not extend the channel's lifetime.
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast::error::RecvError;

use crate::bus::EventBus;
use crate::telemetry::Fleet;

/// Cost cap configuration for the budget governor.
#[derive(Copy, Clone, Debug)]
pub struct Budget {
    /// Maximum fleet spend in micro-USD (1 USD = 1 000 000 micros_usd).
    pub max_micros_usd: u64,
}

/// Subscribe to `bus`, apply every event to a `Fleet`, and set `cancel` to
/// `true` when `fleet.totals().micros_usd > budget.max_micros_usd`.
///
/// `bus` is taken **by value** and dropped immediately after subscribing so
/// the governor does not keep the broadcast channel alive.  The function
/// returns when the channel closes (all remaining senders are dropped) or
/// when the budget cap is exceeded.  `RecvError::Lagged` is handled by
/// continuing — the governor may under-count during an overflow burst but
/// will not crash.
pub async fn budget_governor(bus: EventBus, budget: Budget, cancel: Arc<AtomicBool>) {
    let mut rx = bus.subscribe();
    // Release the sender immediately so we don't extend the channel lifetime.
    drop(bus);

    let mut fleet = Fleet::new();

    loop {
        match rx.recv().await {
            Ok(ev) => {
                fleet.apply(&ev);
                if fleet.totals().micros_usd > budget.max_micros_usd {
                    cancel.store(true, Ordering::Relaxed);
                    return;
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => return,
        }
    }
}
