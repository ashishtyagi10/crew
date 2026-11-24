//! A model chain on the Anthropic wire.
//!
//! WHY: the OpenAI-wire providers walk a chain (`fellback`) and say what
//! they skipped, but an Anthropic request retried its ONE model and then
//! failed: at peak hours a 529 `overloaded_error` on Opus failed the turn
//! while Sonnet stood idle on the same key, and a retired model id 404ed on
//! every call with nothing to step to. [`AnthropicChain`] walks down the
//! family instead, with the same [`Walk`] that puts "fell back to …" under
//! the reply, and the same `account_wide` stop.
//!
//! Which rungs follow an ask, and which failures walk: `anthropicrungs`.
//!
//! Each rung is asked through the plain provider with the request's `model`
//! swapped, so its body is BUILT for that model (`AnthropicProvider::body`:
//! effort only where the model takes it) rather than copied from the first.
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::anthropicrungs::{tail, walks_on};
use super::fellback::{attempt_chain, parse_model_chain, Walk};
use super::{AnthropicProvider, ChunkFn, Completion, CompletionRequest, Provider, ProviderError};

/// [`AnthropicProvider`] that steps down a model chain when a model, not
/// the request or the account, is what failed.
#[derive(Clone)]
pub struct AnthropicChain {
    inner: AnthropicProvider,
    /// The configured chain (`CREW_ANTHROPIC_MODEL="a,b"`); empty = the
    /// built-in ladder below a request that asks for one of its ids ([`tail`]).
    chain: Vec<String>,
}

impl AnthropicProvider {
    /// This provider behind a model chain: `chain` in order, or with an
    /// empty one, each request's model and then the ladder's rungs below it.
    pub fn with_fallbacks(self, chain: Vec<String>) -> AnthropicChain {
        AnthropicChain { inner: self, chain }
    }
}

impl AnthropicChain {
    /// [`AnthropicProvider::from_env`], walking `CREW_ANTHROPIC_MODEL` as the
    /// broker does, so the swarm and a relay turn fall back the same way.
    pub fn from_env() -> Result<Self, ProviderError> {
        let knob = std::env::var("CREW_ANTHROPIC_MODEL").ok();
        AnthropicProvider::from_env().map(|p| configured(p, knob))
    }
}

/// `p` behind the chain the `CREW_ANTHROPIC_MODEL` value `knob` names.
pub(super) fn configured(p: AnthropicProvider, knob: Option<String>) -> AnthropicChain {
    p.with_fallbacks(parse_model_chain(knob, Vec::new()))
}

/// `req`, asking `model`.
fn asking(req: &CompletionRequest, model: &str) -> CompletionRequest {
    CompletionRequest {
        model: model.to_string(),
        ..req.clone()
    }
}

impl Provider for AnthropicChain {
    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }

    fn warm(&self) {
        self.inner.warm();
    }

    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let inner = self.inner.clone();
        let chain = attempt_chain(&req.model, &tail(&req.model, &self.chain));
        Box::pin(async move {
            let mut walk = Walk::default();
            for model in &chain {
                match inner.complete(asking(&req, model)).await {
                    Ok(c) => return walk.answered(model, c),
                    Err(e) if walks_on(&e) => walk.failed(model, e),
                    Err(e) => return Err(e),
                }
            }
            walk.exhausted()
        })
    }

    /// As [`Self::complete`], but only while nothing has reached the pane:
    /// once a rung has typed a word, its failure stands, or a second model
    /// would type its own reply out under the first one's half.
    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let inner = self.inner.clone();
        let chain = attempt_chain(&req.model, &tail(&req.model, &self.chain));
        Box::pin(async move {
            let started = Arc::new(AtomicBool::new(false));
            let seen = Arc::clone(&started);
            let forward: ChunkFn = Arc::new(move |c| {
                seen.store(true, Ordering::SeqCst);
                on_chunk(c)
            });
            let mut walk = Walk::default();
            for model in &chain {
                let asked = asking(&req, model);
                match inner.complete_streaming(asked, Arc::clone(&forward)).await {
                    Ok(c) => return walk.answered(model, c),
                    Err(e) if walks_on(&e) && !started.load(Ordering::SeqCst) => {
                        walk.failed(model, e)
                    }
                    Err(e) => return Err(e),
                }
            }
            walk.exhausted()
        })
    }
}

#[cfg(test)]
#[path = "anthropicchain_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "anthropicchainrule_tests.rs"]
mod rule_tests;
