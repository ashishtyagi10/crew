//! The walk down a model chain, and what it skipped on the way.
//!
//! WHY: an OpenAI-compatible provider tries its chain in order
//! (`CREW_DASHSCOPE_MODEL="qwen-max,qwen-flash"`) and moved on at any
//! failure without a word, so a first model that 404ed on every call was
//! never heard of again while another one answered, at its own price and
//! its own quality. The reply that finally answers now carries what was
//! skipped and why ([`Completion::fell_back`]). It is recorded HERE, where
//! the chain moves on: a response's own `model` cannot tell a fallback from
//! a router answering with a snapshot id the slug does not spell (`served`).
use super::{extract_message, one_line, Completion, ProviderError};

/// How much of one skipped model's reason is kept: enough for a provider's
/// one-sentence diagnosis, short of its whole envelope.
const WHY_MAX: usize = 80;

/// The ordered, de-duplicated models to try for one request: the requested model
/// first, then each configured fallback not already present. Because free
/// OpenRouter models route to *different* upstream providers, a different slug
/// often dodges a provider-specific throttle even when the account-wide daily
/// cap is shared.
pub(super) fn attempt_chain(primary: &str, fallbacks: &[String]) -> Vec<String> {
    let mut chain = vec![primary.to_string()];
    for m in fallbacks {
        if !m.is_empty() && !chain.contains(m) {
            chain.push(m.clone());
        }
    }
    chain
}

/// One request's walk down its chain: each model that failed, as
/// `model: why`, and the last error, which is the answer when none answers.
#[derive(Default)]
pub(super) struct Walk {
    skipped: Vec<String>,
    last: Option<ProviderError>,
}

impl Walk {
    /// `model` failed with `e`; the chain moves on.
    pub(super) fn failed(&mut self, model: &str, e: ProviderError) {
        self.skipped.push(format!("{model}: {}", why(&e)));
        self.last = Some(e);
    }

    /// `c`, answered by `model` and billed at it rather than at the model
    /// asked for first (`Completion::model`), saying what was skipped.
    pub(super) fn answered(self, model: &str, c: Completion) -> Result<Completion, ProviderError> {
        let mut c = c.served_by(model);
        c.fell_back = self.skipped.join("; ");
        Ok(c)
    }

    /// No model answered: the last one's error says why.
    pub(super) fn exhausted(self) -> Result<Completion, ProviderError> {
        Err(self
            .last
            .unwrap_or_else(|| ProviderError::Api("no model attempted".into())))
    }
}

/// A failure in a few words: the provider's own sentence, without the JSON
/// envelope it came in or the advice `api_message` adds for a person who can
/// act on it — the chain already did — else the error as it reads.
pub(super) fn why(e: &ProviderError) -> String {
    let said = match e {
        ProviderError::Api(body) => extract_message(body).unwrap_or_else(|| one_line(body)),
        ProviderError::Status(s) | ProviderError::Http(s) => one_line(s),
        other => one_line(&other.to_string()),
    };
    match said.char_indices().nth(WHY_MAX) {
        Some((cut, _)) => format!("{}\u{2026}", said[..cut].trim_end()),
        None => said,
    }
}

#[cfg(test)]
#[path = "fellback_tests.rs"]
mod tests;
