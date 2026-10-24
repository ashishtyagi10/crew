//! Every reply a turn gets is whole (`provider::cutoff::Whole`): one the
//! token ceiling cut is continued once, on either engine.
use std::sync::{Arc, Mutex};

use super::*;
use crate::provider::{Completion, CompletionRequest, ProviderError};

/// Cut at the ceiling on the first call, finished on the second; keeps the
/// requests.
struct Ceiling(Mutex<Vec<CompletionRequest>>);

impl Provider for Ceiling {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let mut seen = self.0.lock().unwrap();
        seen.push(req);
        let c = match seen.len() {
            1 => Completion {
                text: "The answer is forty".into(),
                truncated: true,
                ..Default::default()
            },
            _ => Completion {
                text: "-two.".into(),
                ..Default::default()
            },
        };
        Box::pin(async move { Ok(c) })
    }
}

#[tokio::test]
async fn a_reply_the_ceiling_cut_is_continued_once() {
    let model = Arc::new(Ceiling(Mutex::default()));
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::new(EventBus::new(16)),
        budget: ToolBudget::turn(1),
        cancel: Arc::default(),
        advice: None,
    };
    let spec = TurnSpec {
        ask: "q".into(),
        prompt: "q".into(),
        hinted: None,
        shed: None,
        ending: None,
        call_timeout: None,
        system: None,
        model: "m".into(),
        tier: ModelTier::Standard,
        max_tokens: 16,
    };
    let ended = run(ctx, spec, model.clone(), None, None).await;
    assert_eq!(ended, Ended::Answered("The answer is forty-two.".into()));
    assert_eq!(
        model.0.lock().unwrap().len(),
        2,
        "one reply, one continuation"
    );
}
