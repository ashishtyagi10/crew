use super::*;
use crate::graph::{TaskId, TaskState};

#[tokio::test]
async fn subscriber_receives_published_events() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();
    bus.publish(HiveEvent::TaskStateChanged {
        task: TaskId(1),
        state: TaskState::Running,
    });
    let ev = rx.recv().await.unwrap();
    assert_eq!(
        ev,
        HiveEvent::TaskStateChanged {
            task: TaskId(1),
            state: TaskState::Running
        }
    );
}

#[test]
fn publish_without_subscribers_does_not_panic() {
    let bus = EventBus::new(8);
    // No subscriber; must be a no-op, not an error/panic.
    bus.publish(HiveEvent::Failed {
        agent: AgentId(0),
        error: "x".into(),
    });
}

#[tokio::test]
async fn two_subscribers_both_receive() {
    let bus = EventBus::new(16);
    let mut a = bus.subscribe();
    let mut b = bus.subscribe();
    bus.publish(HiveEvent::TokenDelta {
        agent: AgentId(3),
        input: 10,
        output: 20,
    });
    let ea = a.recv().await.unwrap();
    let eb = b.recv().await.unwrap();
    assert_eq!(ea, eb);
}

#[test]
fn agentid_event_serde_roundtrip() {
    let ev = HiveEvent::CostDelta {
        agent: AgentId(2),
        micros_usd: 1500,
    };
    let json = serde_json::to_string(&ev).unwrap();
    let back: HiveEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(ev, back);
}

#[tokio::test]
async fn default_capacity_is_applied_and_lag_reports_the_miss_count() {
    // Fill past the ring: the slow subscriber's first recv reports exactly
    // how many events it lost — the number the broker's telemetry-gap note
    // surfaces. Also pins that DEFAULT_CAPACITY is the real ring size
    // (tokio broadcast keeps power-of-two capacities exact).
    let bus = EventBus::new(EventBus::DEFAULT_CAPACITY);
    let mut sub = bus.subscribe();
    for i in 0..(EventBus::DEFAULT_CAPACITY as u64 + 10) {
        bus.publish(HiveEvent::TokenDelta {
            agent: AgentId(i),
            input: 1,
            output: 0,
        });
    }
    match sub.recv().await {
        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => assert_eq!(n, 10),
        other => panic!("expected Lagged(10), got {other:?}"),
    }
}
