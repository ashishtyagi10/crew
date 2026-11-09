//! How hard a model thinks on ONE call, not on every call alike.
//!
//! Claude 5.x thinks before every answer and cannot be told not to: Opus 5.5
//! rejects `thinking: disabled` and `budget_tokens` with a 400, so the only
//! dial left is `output_config.effort`. Left unset, a one-shot judge asking
//! "did this reply answer the question?" pays the same thinking as the work
//! turn that wrote the reply. The one-shots (the reply judge, the router,
//! the classifiers, tool and skill choice) ask for `Low`; the work turn and
//! the compaction summary, which every later hop works from, ask for nothing
//! and keep the model's own default.
//!
//! Only the Anthropic wire sends it. DashScope's one-shots already go
//! unstreamed and are told `enable_thinking: false` (`thinking`), and crew
//! sends OpenAI no `reasoning_effort` anywhere, so there is no shape there
//! to map onto, only one to guess.

/// One call's effort, as the Anthropic API spells the levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl Effort {
    pub fn as_str(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Xhigh => "xhigh",
            Effort::Max => "max",
        }
    }
}

/// Whether `model` (a native Anthropic id, date suffix or not) takes
/// `output_config.effort`. Haiku 4.5 and Sonnet 4.5 answer it with an
/// error, and Haiku 4.5 is in the catalog, so an id this cannot read gets
/// nothing: an unsent effort costs some thinking, a rejected one the call.
pub(crate) fn anthropic_accepts(model: &str) -> bool {
    let Some(rest) = model.strip_prefix("claude-") else {
        return false;
    };
    let mut parts = rest.split('-');
    let family = parts.next().unwrap_or("");
    let Some(major) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
        return false;
    };
    // A minor is one or two digits; a longer number is a date suffix.
    let minor = parts
        .next()
        .filter(|p| p.len() <= 2)
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    match family {
        "opus" => (major, minor) >= (4, 5),
        "sonnet" => (major, minor) >= (4, 6),
        "haiku" => major >= 5,
        "fable" | "mythos" => major >= 5,
        _ => false,
    }
}

/// The `output_config` field for `effort` on `model`, or `None` when the
/// request asks for none or the model would refuse it.
pub(crate) fn anthropic_field(model: &str, effort: Option<Effort>) -> Option<serde_json::Value> {
    let effort = effort?;
    anthropic_accepts(model).then(|| serde_json::json!({ "effort": effort.as_str() }))
}

#[cfg(test)]
#[path = "effort_tests.rs"]
mod tests;
