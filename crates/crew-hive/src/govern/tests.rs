use super::*;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::TaskId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Yield once so the spawned governor task runs and subscribes *before* the
/// test publishes events.  Without this yield the events race the subscription
/// and may be silently missed.
macro_rules! yield_for_subscribe {
    () => {
        tokio::task::yield_now().await;
    };
}

#[tokio::test]
async fn governor_trips_cancel_over_budget() {
    let bus = EventBus::new(64);
    let cancel = Arc::new(AtomicBool::new(false));
    let c2 = cancel.clone();
    let bus2 = bus.clone();
    // budget_governor takes bus by value and drops the sender after subscribing,
    // so the channel closes when `bus` (the last remaining sender) is dropped.
    let gov = tokio::spawn(async move {
        budget_governor(
            bus2,
            Budget {
                max_micros_usd: 1000,
            },
            c2,
        )
        .await;
    });
    yield_for_subscribe!();
    bus.publish(HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(0),
    });
    bus.publish(HiveEvent::CostDelta {
        agent: AgentId(0),
        micros_usd: 1500,
    });
    drop(bus); // close channel so governor returns after processing
    gov.await.unwrap();
    assert!(cancel.load(Ordering::Relaxed));
}

#[tokio::test]
async fn governor_stays_unset_under_budget() {
    let bus = EventBus::new(64);
    let cancel = Arc::new(AtomicBool::new(false));
    let c2 = cancel.clone();
    let bus2 = bus.clone();
    let gov = tokio::spawn(async move {
        budget_governor(
            bus2,
            Budget {
                max_micros_usd: 10_000,
            },
            c2,
        )
        .await;
    });
    yield_for_subscribe!();
    bus.publish(HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(0),
    });
    bus.publish(HiveEvent::CostDelta {
        agent: AgentId(0),
        micros_usd: 500,
    });
    drop(bus); // bus2 was already dropped inside governor; this closes the channel
    gov.await.unwrap();
    assert!(!cancel.load(Ordering::Relaxed));
}
