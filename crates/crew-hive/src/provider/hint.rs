//! What a person can do about a provider's error.
//!
//! [`super::api_message`] finds the provider's sentence inside its JSON
//! envelope. Most of those sentences end the matter (a server fault, a
//! request crew built wrong), but a handful are errors the person at the
//! keyboard fixes, and only a rejected key used to say how. A spent rate
//! limit, an empty account, a thread grown past the model's context and a
//! model name the host has never heard of all arrived as the provider's bare
//! sentence, which names the problem in the provider's terms and never the
//! crew command that gets round it. This table adds that half-line.
//!
//! Matched on the lowercased sentence plus the envelope's `type` and `code`,
//! the way the key check always was: providers agree on no field, but each
//! writes the problem down somewhere. One error gets one hint, from the first
//! row that matches, so the order is the precedence; the key comes first
//! because it is the one fix that always works.

pub(super) use super::hintmatch::hay;
use super::hintmatch::{has, named};

/// Which fix an error calls for.
#[derive(Clone, Copy)]
enum Fix {
    Key,
    Busy,
    Credit,
    Context,
    Model,
    Access,
}

/// One row: the fix, and the phrasings that call for it. Each phrasing is a
/// list of fragments that must ALL appear: "does not exist" alone is about
/// whatever the sentence names, and next to "the model `" it is the model.
struct Row {
    fix: Fix,
    when: &'static [&'static [&'static str]],
}

const ROWS: &[Row] = &[
    Row {
        fix: Fix::Key,
        when: &[
            &["api key"],
            &["unauthorized"],
            &["invalid_api_key"],
            &["authentication"],
            &["401"],
        ],
    },
    Row {
        fix: Fix::Busy,
        when: &[
            &["rate limit"],
            &["rate_limit"],
            &["rate-limit"],
            &["too many requests"],
            &["throttl"],
            &["429"],
        ],
    },
    Row {
        fix: Fix::Credit,
        when: &[
            &["quota"],
            &["billing"],
            &["insufficient balance"],
            &["insufficient_balance"],
            &["credit"],
            &["payment required"],
            &["arrearage"],
        ],
    },
    Row {
        fix: Fix::Context,
        when: &[
            &["context length"],
            &["context_length"],
            &["maximum context"],
            &["context window"],
            &["too many tokens"],
            &["prompt is too long"],
            // DashScope's word for it. The floor of that range is 1, so an
            // EMPTY prompt says it too, and crew never sends one.
            &["range of input length"],
        ],
    },
    Row {
        fix: Fix::Model,
        when: &[
            &["model_not_found"],
            &["model not found"],
            &["model not exist"],
            &["unsupported model"],
            &["not a valid model"],
            &["no such model"],
            &["unknown model"],
            &["the model `", "does not exist"],
            &["model:", "not_found_error"],
        ],
    },
    // The model exists and the key is good, but this account may not call
    // it: DashScope's `AccessDenied.Unpurchased` (a model not activated, or
    // a free quota spent — every model on the account answers it at once),
    // OpenAI's org without access. After `Model`, so OpenAI's "does not
    // exist or you do not have access" stays a model it does not serve.
    Row {
        fix: Fix::Access,
        when: &[
            &["unpurchased"],
            &["access to model denied"],
            &["eligible for using the model"],
            &["does not have access to model"],
        ],
    },
];

/// `msg`, led by `lead`, with the fix when the error is one a person can
/// make. `body` is the envelope `msg` came from, read for its `type` and
/// `code`; pass `""` when there is none.
pub(super) fn told(lead: &str, msg: &str, body: &str) -> String {
    let what = match fix(msg, body) {
        None => return format!("{lead}{msg}"),
        Some(Fix::Key) => {
            return format!("provider rejected the key \u{2014} {msg} (/model replaces it)")
        }
        Some(Fix::Busy) => "rate limited; wait a minute, or pick another model with /model".into(),
        Some(Fix::Credit) => {
            "the account is out of credit; top it up, or /model picks another provider".into()
        }
        // Not "compact": the broker folds a long thread on its own
        // (`compact.rs`), and there is no command to ask for it.
        Some(Fix::Context) => {
            "the conversation no longer fits the model's context; start a fresh pane".into()
        }
        Some(Fix::Access) => format!(
            "this account may not call {} (not activated, or its free quota is spent); \
             enable it with the provider, or /model picks another",
            named(msg)
        ),
        Some(Fix::Model) => format!(
            "this host does not serve {}; /model picks another",
            named(msg)
        ),
    };
    format!("{lead}{msg} \u{2014} {what}")
}

/// Whether the error is the request outgrowing the model's context: asked of
/// the same rows, in the same order, as the hint, so the error the tool loops
/// cut down and send again (`overflow`) is the one a person is told "no
/// longer fits", and a key error that happens to mention a length is not.
pub(super) fn overflows(msg: &str, body: &str) -> bool {
    matches!(fix(msg, body), Some(Fix::Context))
}

/// Whether the error is the key being refused — the same row the hint reads.
pub(super) fn is_key(msg: &str, body: &str) -> bool {
    matches!(fix(msg, body), Some(Fix::Key))
}

/// The fix the first matching row calls for, if any.
fn fix(msg: &str, body: &str) -> Option<Fix> {
    let hay = hay(msg, body);
    ROWS.iter()
        .find(|r| r.when.iter().any(|all| all.iter().all(|f| has(&hay, f))))
        .map(|r| r.fix)
}

#[cfg(test)]
#[path = "hint_tests.rs"]
mod tests;
