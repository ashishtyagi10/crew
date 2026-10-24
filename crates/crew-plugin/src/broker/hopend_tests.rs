//! How a relay hop's turn on the harness ends: past its budget, on a failed
//! model call, at a stop, and with its routing line under the harness's notes.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crew_hive::{CompletionRequest, Provider, ProviderError};

use super::kit::*;
use crate::broker::adapter::Adapter;
use crate::broker::apiadapter::ApiAdapter;
use crate::broker::Broker;
use crate::{PluginEvent, Registry};

/// One round on the budget: the second batch still runs, and a last call
/// with no tools on it turns both into the answer.
#[test]
fn past_the_budget_the_batch_runs_and_the_last_word_answers() {
    for native in [true, false] {
        let ask = |p: &str| match native {
            true => calling("fs__read", serde_json::json!({ "path": p })),
            false => said(&format!("@tool fs:read {{\"path\": \"{p}\"}}")),
        };
        let replies = vec![
            Ok(ask("a.rs")),
            Ok(ask("b.rs")),
            Ok(said("both read\n@done")),
        ];
        let t = hop(native, replies, 1, Arc::default());
        assert_eq!(t.ran.len(), 2, "native {native}: {:?}", t.ran);
        let last = t.seen.last().unwrap();
        assert!(
            last.tools.is_empty() && last.turns.is_empty(),
            "native {native}"
        );
        assert!(
            last.prompt.contains("b.rs"),
            "native {native}: {}",
            last.prompt
        );
        let done = t.done().unwrap();
        assert!(done.starts_with("both read"), "native {native}: {done}");
        assert!(
            done.contains("tool budget spent"),
            "native {native}: {done}"
        );
    }
}

/// A model that fails before any tool ran ends the hop on an error card,
/// as a dial does; one that fails after keeps the turn, said under it.
#[test]
fn a_failure_is_an_error_card_and_after_a_tool_the_answer_says_so() {
    let down = || Err(ProviderError::Http("reset".into()));
    let t = go(true, vec![down()]);
    assert_eq!(t.errors(), vec!["http error: reset"]);
    assert_eq!(t.done(), None);
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            down(),
        ],
    );
    assert_eq!(t.errors(), vec!["http error: reset"]);
    let done = t.done().unwrap();
    assert!(
        done.contains("the model's next reply failed: http error: reset"),
        "{done}"
    );
}

/// A stop seen between rounds dials nothing more, and the answer says how far
/// the turn got.
#[test]
fn a_stop_between_rounds_dials_nothing_more() {
    let stop = Arc::new(AtomicBool::new(false));
    struct Stopping(Arc<AtomicBool>, Arc<Mutex<u32>>);
    impl Provider for Stopping {
        fn supports_tools(&self) -> bool {
            true
        }
        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Reply> + Send>> {
            *self.1.lock().unwrap() += 1;
            self.0.store(true, Ordering::Relaxed);
            let c = calling("fs__read", serde_json::json!({"path": "a.rs"}));
            Box::pin(async move { Ok(c) })
        }
    }
    let calls: Arc<Mutex<u32>> = Arc::default();
    let model = Stopping(Arc::clone(&stop), Arc::clone(&calls));
    let agent = ApiAdapter::new("coder", "m", "", None, Arc::new(model)).unwrap();
    let ran: Arc<Mutex<Vec<String>>> = Arc::default();
    let broker = Broker::new(
        Registry::new(vec![Box::new(agent) as Box<dyn Adapter>]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Files(Arc::clone(&ran))))
    .with_cancel_flag(stop);
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut hops = Vec::new();
    broker.run("user", "coder", "read a.rs", "t1", &emit, &mut |h| {
        hops.push(h)
    });
    assert_eq!(*calls.lock().unwrap(), 1, "no dial after the stop");
    assert!(
        ran.lock().unwrap().is_empty(),
        "nothing started after the stop"
    );
}

/// Through the whole hop: a budget stop whose last word ends on `@done`
/// answers without it.
#[test]
fn a_budget_stop_answers_without_its_directive_showing() {
    let ask = |p: &str| said(&format!("@tool fs:read {{\"path\": \"{p}\"}}"));
    let replies = vec![
        Ok(ask("a.rs")),
        Ok(ask("b.rs")),
        Ok(said("both read\n@done")),
    ];
    let t = hop(false, replies, 1, Arc::default());
    let done = t.done().unwrap();
    assert!(!done.contains("@done"), "{done}");
    assert!(done.starts_with("both read"), "{done}");
    assert!(
        done.ends_with("[tool budget spent \u{2014} 1 round of tool calls]"),
        "{done}"
    );
}
