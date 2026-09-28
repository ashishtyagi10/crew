//! A worker's system prompt ends with the host's standing context.
use super::*;
use crate::agent::AgentContext;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError};
use std::sync::Mutex;

/// Records the system prompt of every request.
struct Seen(Arc<Mutex<Vec<Option<String>>>>);

impl Provider for Seen {
    fn complete(
        &self,
        req: crate::provider::CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.0.lock().unwrap().push(req.system.clone());
        Box::pin(async {
            Ok(Completion {
                text: "ok".into(),
                ..Default::default()
            })
        })
    }
}

async fn system_for(persona: Option<&str>, preamble: Option<&str>) -> Option<String> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut f = ApiFactory::new(Arc::new(Seen(Arc::clone(&seen))), 64);
    if let Some(p) = preamble {
        f = f.with_preamble(p);
    }
    let kind = AgentKind::Api {
        system: persona.map(str::to_string),
    };
    let agent = f.make(&kind);
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(1),
        task: TaskSpec {
            id: TaskId(1),
            title: "Define crew-term".into(),
            agent: kind,
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "Define crew-term".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: EventBus::new(8),
    };
    assert!(agent.run(ctx).await.success);
    let got = seen.lock().unwrap().first().cloned();
    got.flatten()
}

#[tokio::test]
async fn the_preamble_follows_the_persona() {
    let s = system_for(Some("You are the researcher."), Some("PROJECT: crew")).await;
    assert_eq!(
        s.as_deref(),
        Some("You are the researcher.\n\nPROJECT: crew")
    );
}

#[tokio::test]
async fn a_preamble_alone_is_the_whole_system_prompt() {
    let s = system_for(None, Some("PROJECT: crew")).await;
    assert_eq!(s.as_deref(), Some("PROJECT: crew"));
}

#[tokio::test]
async fn no_preamble_leaves_the_persona_byte_identical() {
    let s = system_for(Some("You are the researcher."), None).await;
    assert_eq!(s.as_deref(), Some("You are the researcher."));
}
