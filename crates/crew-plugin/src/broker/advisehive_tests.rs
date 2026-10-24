//! The advisor meeting an API agent's turn on the harness (`hopturn`): a
//! failed build is read before the next call, and what it said rides the
//! rest of the turn — in each follow-up on the text loop, as a turn of its
//! own on the native one.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::tests::{advising, Runs};
use super::text::HEAD;
use crate::broker::adapter::Adapter;
use crate::broker::apiadapter::ApiAdapter;
use crate::broker::Broker;
use crate::Registry;
use crew_hive::provider::ToolInvocation;
use crew_hive::{Completion, CompletionRequest, Provider, ProviderError, ToolSpec, Turn};

type Reply = Result<Completion, ProviderError>;

/// A model that answers from a script and keeps every request.
struct Model {
    native: bool,
    replies: Mutex<Vec<Completion>>,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl Provider for Model {
    fn supports_tools(&self) -> bool {
        self.native
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Reply> + Send>> {
        self.seen.lock().unwrap().push(req);
        let c = self.replies.lock().unwrap().remove(0);
        Box::pin(async move { Ok(c) })
    }
}

/// [`Runs`] with a schema, for the native loop.
struct Schemed;

impl crate::broker::toolcall::ToolRunner for Schemed {
    fn hint(&self) -> String {
        Runs.hint()
    }
    fn call(&self, s: &str, t: &str, args: &str) -> Result<String, String> {
        Runs.call(s, t, args)
    }
    fn failed(&self, s: &str, t: &str, out: &str) -> bool {
        Runs.failed(s, t, out)
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "sys".into(),
            tool: "run".into(),
            description: "runs".into(),
            input_schema: serde_json::json!({ "type": "object" }),
        }]
    }
}

/// The turn that builds (and fails), tests (and passes), then answers: the
/// requests the model saw.
fn build_then_test(native: bool) -> Vec<CompletionRequest> {
    let run = |cmd: &str| match native {
        true => Completion {
            calls: vec![ToolInvocation {
                id: cmd.into(),
                name: "sys__run".into(),
                input: serde_json::json!({ "cmd": cmd }),
                bad_args: None,
            }],
            ..Default::default()
        },
        false => Completion {
            text: format!("@tool sys:run {{\"cmd\": \"{cmd}\"}}"),
            ..Default::default()
        },
    };
    let answer = Completion {
        text: "it builds\n@done".into(),
        ..Default::default()
    };
    let seen: Arc<Mutex<Vec<CompletionRequest>>> = Arc::default();
    let model = Model {
        native,
        replies: Mutex::new(vec![run("cargo build"), run("cargo test"), answer]),
        seen: Arc::clone(&seen),
    };
    let agent = ApiAdapter::new("engineer", "m", "", None, Arc::new(model)).unwrap();
    let b = Broker::new(
        Registry::new(vec![Box::new(agent) as Box<dyn Adapter>]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Schemed));
    let emit: Arc<dyn Fn(crate::PluginEvent) + Send + Sync> = Arc::new(|_| {});
    b.run(
        "user",
        "engineer",
        "make it build",
        "t1",
        &emit,
        &mut |_| {},
    );
    let seen = seen.lock().unwrap().clone();
    seen
}

#[test]
fn a_failed_build_is_advised_on_both_loops_for_the_rest_of_the_turn() {
    for native in [false, true] {
        let (briefs, _on) = advising(Ok("E0308 at src/a.rs:3 \u{2014} fix the call site"));
        let seen = build_then_test(native);
        assert_eq!(seen.len(), 3, "native {native}");
        let briefs = briefs.lock().unwrap().clone();
        assert_eq!(
            briefs.len(),
            1,
            "native {native}: the passing test asked nothing"
        );
        assert!(
            briefs[0].contains("THE REQUEST:\nmake it build"),
            "{}",
            briefs[0]
        );
        let advice = "- E0308 at src/a.rs:3 \u{2014} fix the call site";
        for req in &seen[1..] {
            let said = match native {
                false => req.prompt.clone(),
                true => req
                    .turns
                    .iter()
                    .filter_map(|t| match t {
                        Turn::User(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            };
            assert!(said.contains(HEAD), "native {native}: {said}");
            assert_eq!(said.matches(advice).count(), 1, "native {native}: {said}");
        }
    }
}

#[test]
fn a_turn_with_no_advisor_carries_no_section() {
    for native in [false, true] {
        let seen = build_then_test(native);
        let shown = |r: &CompletionRequest| format!("{} {:?}", r.prompt, r.turns);
        assert!(
            seen.iter().all(|r| !shown(r).contains(HEAD)),
            "native {native}"
        );
    }
}
