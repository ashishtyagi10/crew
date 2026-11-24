//! A model chain on the Anthropic wire.
//!
//! WHY: the OpenAI-wire providers walk a chain (`fellback`) and say what
//! they skipped, but an Anthropic request retried its ONE model and then
//! failed: at peak hours a 529 `overloaded_error` on Opus failed the turn
//! while Sonnet stood idle on the same key, and a retired model id 404ed on
//! every call with nothing to step to. [`AnthropicChain`] walks down the
//! family instead, with the same [`Walk`] that puts "fell back to …" under
//! the reply, and the same [`account_wide`] stop.
//!
//! Each rung is asked through the plain provider with the request's `model`
//! swapped, so its body is BUILT for that model (`AnthropicProvider::body`:
//! effort only where the model takes it) rather than copied from the first.
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::fellback::{account_wide, attempt_chain, Walk};
use super::{AnthropicProvider, ChunkFn, Completion, CompletionRequest, Provider, ProviderError};
use crate::ModelTier;

/// [`AnthropicProvider`] that steps down a model chain when a model, not
/// the request or the account, is what failed.
#[derive(Clone)]
pub struct AnthropicChain {
    inner: AnthropicProvider,
    /// The configured chain (`CREW_ANTHROPIC_MODEL="a,b"`); empty = the
    /// family ladder below whatever each request asks for ([`tail`]).
    chain: Vec<String>,
}

impl AnthropicProvider {
    /// This provider behind a model chain: `chain` in order, or with an
    /// empty one, each request's model and then the cheaper families below it.
    pub fn with_fallbacks(self, chain: Vec<String>) -> AnthropicChain {
        AnthropicChain { inner: self, chain }
    }
}

impl AnthropicChain {
    /// [`AnthropicProvider::from_env`], walking each ask's family ladder.
    pub fn from_env() -> Result<Self, ProviderError> {
        AnthropicProvider::from_env().map(|p| p.with_fallbacks(Vec::new()))
    }
}

/// The rungs after `model`. In a configured chain that holds it, the ones
/// after it; otherwise the families below it — opus → sonnet → haiku — and
/// never ABOVE: one provider serves every tier, and a cheap ask that fell up
/// to Opus would be billed at Opus without anyone choosing it.
pub(super) fn tail(model: &str, chain: &[String]) -> Vec<String> {
    if let Some(at) = chain.iter().position(|m| m == model) {
        return chain[at + 1..].to_vec();
    }
    let below: &[ModelTier] = if model.contains("opus") {
        &[ModelTier::Standard, ModelTier::Cheap]
    } else if model.contains("sonnet") {
        &[ModelTier::Cheap]
    } else {
        &[]
    };
    below.iter().map(|t| t.model_id().to_string()).collect()
}

/// Whether `e` is the MODEL's failure, one the next rung may not share:
/// overloaded or rate-limited (Anthropic limits each model on its own), gone
/// (a retired or mistyped id is a 404 `not_found_error`), or a host error
/// that outlived its retries. A bad request, a refused key, a permission or
/// billing refusal, an unreachable host: every rung would meet it again.
pub(super) fn walks_on(e: &ProviderError) -> bool {
    if account_wide(e) {
        return false;
    }
    match e {
        ProviderError::Api(body) => {
            let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
            let kind = v["error"]["type"].as_str().unwrap_or_default();
            [
                "overloaded_error",
                "not_found_error",
                "rate_limit_error",
                "api_error",
            ]
            .contains(&kind)
        }
        ProviderError::Status(s) => {
            super::status::code(s).is_some_and(|c| c == 404 || c == 429 || c >= 500)
        }
        _ => false,
    }
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
