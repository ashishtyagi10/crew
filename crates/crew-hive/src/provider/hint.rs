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

/// Which fix an error calls for.
#[derive(Clone, Copy)]
enum Fix {
    Key,
    Busy,
    Credit,
    Context,
    Model,
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
];

/// `msg`, led by `lead`, with the fix when the error is one a person can
/// make. `body` is the envelope `msg` came from, read for its `type` and
/// `code`; pass `""` when there is none.
pub(super) fn told(lead: &str, msg: &str, body: &str) -> String {
    let hay = hay(msg, body);
    let fix = ROWS
        .iter()
        .find(|r| r.when.iter().any(|all| all.iter().all(|f| has(&hay, f))))
        .map(|r| r.fix);
    let what = match fix {
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
        Some(Fix::Model) => format!(
            "this host does not serve {}; /model picks another",
            named(msg)
        ),
    };
    format!("{lead}{msg} \u{2014} {what}")
}

/// The sentence and the envelope's `type`/`code` words, lowercased. A code
/// is often the plainest statement of the problem: OpenRouter's wrapped 429
/// says only "Provider returned error", and its `code` says 429.
fn hay(msg: &str, body: &str) -> String {
    let mut hay = msg.to_lowercase();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        for obj in [&v, &v["error"]] {
            for key in ["type", "code"] {
                let word = match &obj[key] {
                    serde_json::Value::String(s) => s.to_lowercase(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => continue,
                };
                hay.push(' ');
                hay.push_str(&word);
            }
        }
    }
    hay
}

/// Whether `frag` is in `hay`. A number must stand alone: "401" matched
/// inside "you requested 40100 tokens" and told a person with an overlong
/// thread that their key was wrong.
fn has(hay: &str, frag: &str) -> bool {
    if !frag.bytes().all(|b| b.is_ascii_digit()) {
        return hay.contains(frag);
    }
    let digit = |i: Option<&u8>| i.is_some_and(u8::is_ascii_digit);
    let b = hay.as_bytes();
    hay.match_indices(frag)
        .any(|(i, _)| !digit(b.get(i.wrapping_sub(1))) && !digit(b.get(i + frag.len())))
}

/// The model the sentence names: OpenAI's and DashScope's put it in
/// backticks, Anthropic's after `model: `. "that model" when it names none.
fn named(msg: &str) -> &str {
    let ticked = msg
        .split('`')
        .nth(1)
        .filter(|_| msg.matches('`').count() >= 2);
    let after = || msg.split_once("model: ")?.1.split_whitespace().next();
    ticked
        .filter(|s| !s.trim().is_empty())
        .or_else(after)
        .unwrap_or("that model")
}

#[cfg(test)]
#[path = "hint_tests.rs"]
mod tests;
