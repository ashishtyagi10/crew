//! How hard a model thinks on ONE call, not on every call alike.
//!
//! Claude 5.x thinks before every answer and cannot be told not to: Opus 5.5
//! rejects `thinking: disabled` and `budget_tokens` with a 400, so the only
//! dial left is `output_config.effort`. Left unset, a one-shot judge asking
//! "did this reply answer the question?" pays the same thinking as the work
//! turn that wrote the reply. The one-shots (the reply judge, the router,
//! the classifiers, tool and skill choice) ask for `Low`; the compaction
//! summary asks for nothing; and the work turn asks for what its route and
//! its agent say (crew-plugin `routeeffort`) — usually nothing, which keeps
//! the model's own default.
//!
//! Two wires carry it, each only to a model that takes the level sent: the
//! Anthropic one as `output_config.effort` ([`anthropic_field`]), OpenAI's
//! own GPT-6 models as `reasoning_effort` ([`openai_field`]). DashScope's
//! one-shots are told `enable_thinking: false` instead (`thinking`), and a
//! GPT-6 Luna tool request is always told `none` (`lunaeffort`).

/// One call's effort, as the Anthropic API spells the levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

    /// The level a person wrote (`high`, ` Max `), or `None` for anything
    /// else: a mistyped level keeps the default rather than guessing one.
    pub fn parse(s: &str) -> Option<Effort> {
        [
            Effort::Low,
            Effort::Medium,
            Effort::High,
            Effort::Xhigh,
            Effort::Max,
        ]
        .into_iter()
        .find(|e| s.trim().eq_ignore_ascii_case(e.as_str()))
    }
}

/// `model`'s family, major and minor (`claude-opus-4-5-20251101` → opus 4.5).
fn anthropic_version(model: &str) -> Option<(&str, u32, u32)> {
    let rest = model.strip_prefix("claude-")?;
    let mut parts = rest.split('-');
    let family = parts.next().unwrap_or("");
    let major = parts.next().and_then(|p| p.parse::<u32>().ok())?;
    // A minor is one or two digits; a longer number is a date suffix.
    let minor = parts
        .next()
        .filter(|p| p.len() <= 2)
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    Some((family, major, minor))
}

/// Whether `model` (a native Anthropic id, date suffix or not) takes
/// `output_config.effort`. Haiku 4.5 and Sonnet 4.5 answer it with an
/// error, and Haiku 4.5 is in the catalog, so an id this cannot read gets
/// nothing: an unsent effort costs some thinking, a rejected one the call.
pub(crate) fn anthropic_accepts(model: &str) -> bool {
    let Some((family, major, minor)) = anthropic_version(model) else {
        return false;
    };
    match family {
        "opus" => (major, minor) >= (4, 5),
        "sonnet" => (major, minor) >= (4, 6),
        "haiku" => major >= 5,
        "fable" | "mythos" => major >= 5,
        _ => false,
    }
}

/// `effort` brought down to the highest level `model` takes (the claude-api
/// thinking table): Opus 4.5 stops at `high`, and the 4.6 models have `max`
/// but no `xhigh`, which arrived with Opus 4.7. Every later model takes all five.
fn anthropic_level(model: &str, effort: Effort) -> Effort {
    match anthropic_version(model) {
        Some(("opus", 4, 5)) if effort > Effort::High => Effort::High,
        Some((_, 4, 6)) if effort == Effort::Xhigh => Effort::High,
        _ => effort,
    }
}

/// The `output_config` field for `effort` on `model`, or `None` when the
/// request asks for none or the model would refuse it.
pub(crate) fn anthropic_field(model: &str, effort: Option<Effort>) -> Option<serde_json::Value> {
    let effort = effort?;
    anthropic_accepts(model)
        .then(|| serde_json::json!({ "effort": anthropic_level(model, effort).as_str() }))
}

/// The reply ceiling (`max_tokens`) a call at `effort` on `model` gets, where
/// `base` is the caller's own. Thinking counts against `max_tokens`, and
/// Claude 5.x always thinks: a hard plan at `high` could spend all 4096 of
/// the relay's ceiling thinking and come back as nothing but the cut-off
/// marker. So a raised level raises the room with it — `high` 16K, `xhigh`
/// and `max` 32K, keyed off the level actually sent (Opus 4.5 asked `max` is
/// sent `high`) — only on a model that takes the level: the 4.6-and-later
/// Anthropic models document 128K output, and Opus 4.5 (which never gets
/// past `high`, so 16K) far more than 16K.
/// Everything else keeps `base`: NIM refuses a ceiling over the model's own
/// limit, and GPT-6's output limit is not in crew's catalog, so it is not
/// guessed at.
pub fn ceiling(model: &str, effort: Option<Effort>, base: u32) -> u32 {
    let Some(effort) = effort.filter(|_| anthropic_accepts(model)) else {
        return base;
    };
    let room = match anthropic_level(model, effort) {
        Effort::Low | Effort::Medium => return base,
        Effort::High => 16_000,
        Effort::Xhigh | Effort::Max => 32_000,
    };
    base.max(room)
}

/// The GPT-6 ids in crew's catalog. A sibling OpenAI adds later is sent
/// nothing until its page is read: its levels are not known.
const GPT6: &[&str] = &["gpt-6.1-sol", "gpt-6-sol", "gpt-6-luna", "gpt-6-astra"];

/// The `reasoning_effort` a chat-completions request for `model` carries for
/// `effort`, or `None`. Only OpenAI's GPT-6 ids (their pages list `none`
/// through `high`, so `xhigh` and `max` are sent as `high`) and only with no
/// tools: on Chat Completions a GPT-6 model takes tools at `none` or not at
/// all (`lunaeffort`, crew-plugin `directs`). GPT-5.x, DeepSeek and the
/// gateways are left at their defaults until their own pages are read.
pub(crate) fn openai_field(
    model: &str,
    effort: Option<Effort>,
    tools: bool,
) -> Option<&'static str> {
    let effort = effort?;
    let gpt6 = GPT6.contains(&model);
    // Only ever LOWERED on GPT-6: its reply ceiling stays 4096 (no documented output limit
    // to raise it to, `ceiling`), and reasoning counts against it — a raised effort could
    // spend the whole reply thinking.
    (gpt6 && !tools && effort == Effort::Low).then_some("low")
}

#[cfg(test)]
#[path = "effort_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "effortwire_tests.rs"]
mod wire_tests;
