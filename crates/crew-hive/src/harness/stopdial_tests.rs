//! A dial that blocks — a relay CLI on its own thread — cannot be dropped
//! when the stop is pressed; it ends itself and comes back an error. The
//! turn reads that as the stop it was, not as a failure the planner would
//! re-plan around and the pane would print as "failed".
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::*;
use crate::bus::{AgentId, EventBus};
use crate::provider::{Completion, CompletionRequest, ProviderError};
use crate::tools::budget::ToolBudget;

/// The user presses stop while the call runs; the call ends on it.
struct Stopped(Arc<AtomicBool>);

impl Provider for Stopped {
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let stop = Arc::clone(&self.0);
        Box::pin(async move {
            stop.store(true, Ordering::Relaxed);
            Err(ProviderError::Api("stopped by you".into()))
        })
    }
}

#[tokio::test]
async fn a_dial_that_ends_on_the_stop_ends_the_turn_stopped() {
    let cancel = Arc::new(AtomicBool::new(false));
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::new(EventBus::new(16)),
        budget: ToolBudget::turn(1),
        cancel: Arc::clone(&cancel),
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
    let ended = run(ctx, spec, Arc::new(Stopped(cancel)), None, None).await;
    assert_eq!(ended, Ended::Stopped);
}
