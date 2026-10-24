//! A steer meeting an API agent's turn on the harness (`apiturn`): taken
//! between rounds, announced as it is taken, and said with the relay's own
//! closing line, not the swarm's.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::tests::{offer, taking, turn, Log};
use super::*;
use crate::broker::adapter::Adapter;
use crate::broker::apiadapter::ApiAdapter;
use crate::broker::Broker;
use crate::Registry;
use crew_hive::{Completion, CompletionRequest, Provider, ProviderError};

type Reply = Result<Completion, ProviderError>;

/// A model with no tool-use that answers from a script and keeps every prompt.
struct Model(Mutex<Vec<&'static str>>, Arc<Mutex<Vec<String>>>);

impl Provider for Model {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Reply> + Send>> {
        self.1.lock().unwrap().push(req.prompt);
        let text = self.0.lock().unwrap().remove(0).to_string();
        Box::pin(async move {
            Ok(Completion {
                text,
                ..Default::default()
            })
        })
    }
}

/// A read during which the user types.
struct Typing;

impl crate::broker::toolcall::ToolRunner for Typing {
    fn hint(&self) -> String {
        "TOOLS: fs:read".into()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        offer("also check the tests");
        Ok("FILE".into())
    }
}

#[test]
fn a_steer_joins_the_follow_up_with_the_relays_closing_line() {
    let _t = turn();
    let log = Log::default();
    let _task = taking(&log);
    let prompts: Arc<Mutex<Vec<String>>> = Arc::default();
    let replies = vec!["@tool fs:read {\"path\": \"a\"}", "checked both\n@done"];
    let model = Model(Mutex::new(replies), Arc::clone(&prompts));
    let agent = ApiAdapter::new("planner", "m", "", None, Arc::new(model)).unwrap();
    let b = Broker::new(
        Registry::new(vec![Box::new(agent) as Box<dyn Adapter>]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Typing));
    let emit: Arc<dyn Fn(crate::PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut answer = String::new();
    b.run("user", "planner", "task", "t1", &emit, &mut |h| {
        if h.kind == crate::broker::hop::HopKind::Done {
            answer = h.text;
        }
    });
    assert_eq!(answer, "checked both");
    let prompts = prompts.lock().unwrap().clone();
    assert_eq!(prompts.len(), 2);
    let follow = &prompts[1];
    let (exchanges, added) = follow
        .split_once(HEAD)
        .expect("the section rides the follow-up");
    assert!(exchanges.contains("TOOL EXCHANGES SO FAR"), "{follow}");
    assert!(added.starts_with("\n- also check the tests\n"), "{follow}");
    assert!(added.contains(TAIL), "the relay's line: {follow}");
    assert!(!follow.contains(crew_hive::steers::WORKER_TAIL), "{follow}");
    let order = log.lock().unwrap().clone();
    assert_eq!(order, ["steered crew: also check the tests"]);
}
