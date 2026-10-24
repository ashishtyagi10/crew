//! A CLI agent's hop with tools: a model with no tool-use, on the harness's
//! text loop, its failures in its own words.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::kit::Files;
use crate::broker::adapter::Adapter;
use crate::broker::hop::HopKind;
use crate::broker::Broker;
use crate::{PluginEvent, Registry};

/// A CLI agent is a model with no tool-use: the `@tool` line it prints is
/// the harness's text loop to run, and the follow-up is the harness's.
#[test]
fn a_cli_agent_runs_on_the_text_loop() {
    struct Cli;
    impl Adapter for Cli {
        fn name(&self) -> &str {
            "coder"
        }
        fn probe(&self) -> bool {
            true
        }
        fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
            Ok(match body.contains("TOOL EXCHANGES SO FAR") {
                true => "read it\n@done".into(),
                false => "@tool fs:read {\"path\": \"a.rs\"}".into(),
            })
        }
    }
    let ran: Arc<Mutex<Vec<String>>> = Arc::default();
    let broker = Broker::new(
        Registry::new(vec![Box::new(Cli)]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Files(Arc::clone(&ran))));
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut hops = Vec::new();
    broker.run("user", "coder", "read a.rs", "t1", &emit, &mut |h| {
        hops.push(h)
    });
    assert_eq!(ran.lock().unwrap().len(), 1);
    let done = hops.iter().find(|h| h.kind == HopKind::Done).unwrap();
    assert_eq!(done.text, "read it");
}

/// A CLI agent's failure is its own words on the card, as its dial showed
/// them, not a provider's.
#[test]
fn a_cli_agents_error_is_its_own_words() {
    struct Down;
    impl Adapter for Down {
        fn name(&self) -> &str {
            "coder"
        }
        fn probe(&self) -> bool {
            true
        }
        fn call(&self, _: &str, _: Duration) -> Result<String, String> {
            Err("claude: not signed in".into())
        }
    }
    let broker = Broker::new(
        Registry::new(vec![Box::new(Down)]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Files(Arc::default())));
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut hops = Vec::new();
    broker.run("user", "coder", "read a.rs", "t1", &emit, &mut |h| {
        hops.push(h)
    });
    let errors: Vec<&str> = hops
        .iter()
        .filter(|h| h.kind == HopKind::Error)
        .map(|h| h.text.as_str())
        .collect();
    assert_eq!(errors, ["claude: not signed in"]);
}
