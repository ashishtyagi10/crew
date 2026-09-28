//! Whether an error is the request outgrowing the model's context.
//!
//! A tool loop grows its own request: every file a worker reads goes back to
//! the model on the next round, and past the window (32K tokens on qwen-max)
//! the provider refuses the call, `Range of input length should be [1,
//! 30720]`. Of all the errors a call can end on, that one is crew's to fix:
//! the same request with less in it goes through. So the loops ask this
//! before they give up, and cut the request down once (`apiagent::overflow`,
//! the relay's `toolfull`).
//!
//! The answer comes from the table the hint reads (`hint`), not a list of its
//! own, so what is retried and what a person is told "no longer fits the
//! model's context" cannot drift apart.

use super::ProviderError;

impl ProviderError {
    /// The provider refused the request for its length. Only an answer the
    /// provider gave can say so: a dropped connection or a body that did not
    /// decode says nothing about the request's size.
    pub fn is_context_overflow(&self) -> bool {
        match self {
            ProviderError::Api(body) => {
                let msg = super::extract_message(body).unwrap_or_else(|| super::one_line(body));
                super::hint::overflows(&msg, body)
            }
            ProviderError::Status(said) => super::hint::overflows(said, ""),
            ProviderError::Http(_) | ProviderError::Decode(_) | ProviderError::MissingKey(_) => {
                false
            }
        }
    }
}

/// [`ProviderError::is_context_overflow`] of an error that reached its caller
/// as words. The relay's agents hand one back as a string: an API agent the
/// error's display (`api error: Range of input length … — the conversation no
/// longer fits the model's context; …`), a CLI agent its own sentence
/// (`Prompt is too long`). The envelope's `code` is gone by then, but every
/// context row of the table is a phrase in the sentence itself.
pub fn says_context_overflow(said: &str) -> bool {
    super::hint::overflows(said, "")
}

#[cfg(test)]
#[path = "overflow_tests.rs"]
mod tests;
