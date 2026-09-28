//! A plan that means what it says is taken on the first ask, however it
//! spelled its ids and deps; one that points at a task that is not there is
//! still re-asked.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::plan_with_repair;
use crate::graph::{ModelTier, TaskGraph, TaskId};
use crate::planner::PlanError;
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};

const PLAN: &str = r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#;

/// Answers from a queue and counts the calls.
struct Counted {
    replies: Mutex<VecDeque<&'static str>>,
    calls: AtomicUsize,
}

impl Provider for Counted {
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let text = self.replies.lock().unwrap().pop_front().expect("scripted");
        Box::pin(async move {
            Ok(Completion {
                text: text.into(),
                ..Default::default()
            })
        })
    }
}

/// Plan from `replies`, and how many planning calls it took.
async fn plan(replies: &[&'static str]) -> (Result<TaskGraph, PlanError>, usize) {
    let p = Arc::new(Counted {
        replies: Mutex::new(replies.iter().copied().collect()),
        calls: AtomicUsize::new(0),
    });
    let req = CompletionRequest {
        model: "m".into(),
        prompt: "the goal".into(),
        max_tokens: 64,
        ..Default::default()
    };
    let graph = plan_with_repair(Arc::clone(&p), req, ModelTier::Standard)
        .await
        .0;
    (graph, p.calls.load(Ordering::SeqCst))
}

#[tokio::test]
async fn string_ids_and_string_deps_plan_on_the_first_ask() {
    let reply = r#"[{"id":"1","title":"a","prompt":"p","deps":[]},
                    {"id":"2","title":"b","prompt":"q","deps":["1"]}]"#;
    let (graph, calls) = plan(&[reply, PLAN]).await;
    let graph = graph.unwrap();
    assert_eq!(calls, 1, "a plan that parses is never re-asked");
    assert_eq!(graph.len(), 2);
    assert_eq!(graph.get(TaskId(1)).unwrap().deps, vec![]);
    assert_eq!(graph.get(TaskId(2)).unwrap().deps, vec![TaskId(1)]);
}

#[tokio::test]
async fn a_root_task_without_deps_plans_on_the_first_ask() {
    let reply = r#"[{"id":0,"title":"a","prompt":"p"},{"id":1,"title":"b","prompt":"q"},
                    {"id":2,"title":"c","prompt":"r","deps":[0,1]}]"#;
    let (graph, calls) = plan(&[reply, PLAN]).await;
    let graph = graph.unwrap();
    assert_eq!(calls, 1);
    assert_eq!(
        graph.get(TaskId(2)).unwrap().deps,
        vec![TaskId(0), TaskId(1)]
    );
}

#[tokio::test]
async fn depends_on_is_read_as_deps() {
    let reply = r#"[{"id":0,"title":"a","prompt":"p","depends_on":[]},
                    {"id":1,"title":"b","prompt":"q","depends_on":[0]}]"#;
    let (graph, calls) = plan(&[reply, PLAN]).await;
    let graph = graph.unwrap();
    assert_eq!(calls, 1);
    assert_eq!(graph.get(TaskId(1)).unwrap().deps, vec![TaskId(0)]);
}

#[tokio::test]
async fn a_dep_on_a_task_that_is_not_there_is_still_re_asked() {
    let reply = r#"[{"id":"1","title":"a","prompt":"p"},
                    {"id":"2","title":"b","prompt":"q","deps":["7"]}]"#;
    let (graph, calls) = plan(&[reply, PLAN]).await;
    assert_eq!(calls, 2, "the missing dep costs the re-ask it always did");
    assert_eq!(graph.unwrap().len(), 1, "the re-ask's plan is the one kept");
}
