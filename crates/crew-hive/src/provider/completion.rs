//! What a provider is asked, what it answers, and the tool-use pieces both
//! carry.

/// One tool a model may call, with the JSON Schema for its arguments.
///
/// The schema is the point. crew's previous tool surface handed the model a
/// name and a 100-character description CLIP and asked it to write JSON by
/// guesswork; here the provider validates the shape and the model chooses on
/// the argument structure rather than on half a sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolDef {
    /// The WIRE name — `[a-zA-Z0-9_-]{1,64}`, which `server:tool` is not.
    /// See `crate::tools::wire_name`, which owns the encoding and the map back.
    pub name: String,
    pub description: String,
    /// JSON Schema object for the arguments. An empty object is legal and
    /// means "no arguments"; it must never be `null`, which some providers
    /// reject outright.
    pub input_schema: serde_json::Value,
}

/// A tool call the model made, as the provider reported it.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolInvocation {
    /// The provider's own id for this call. It is echoed back verbatim in the
    /// result and is how a provider pairs the two across a parallel batch —
    /// never reconstruct it, and never assume one call per turn.
    pub id: String,
    pub name: String,
    /// Arguments as the model produced them. Kept as `Value`, not `String`,
    /// so nothing re-parses provider-validated JSON.
    pub input: serde_json::Value,
    /// Why `input` is not what the model sent: the argument string it wrote
    /// did not parse (`toolargs`), so `input` is `{}`. Such a call must not
    /// run, since a tool handed `{}` answers "missing argument" and the model
    /// never learns its JSON was the problem; the loop answers it with this.
    /// `None` whenever the arguments were read, which is always on a provider
    /// that hands them over parsed (Anthropic).
    pub bad_args: Option<String>,
}

/// The outcome of one [`ToolInvocation`], on its way back to the model.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolOutcome {
    pub id: String,
    pub name: String,
    pub content: String,
    /// A refusal or failure. Providers have a flag for this; without it a
    /// model reads "ERROR: connection refused" as data it should use.
    pub is_error: bool,
}

/// One exchange after the opening user prompt.
#[derive(Clone, Debug, PartialEq)]
pub enum Turn {
    /// What the model said, and anything it asked to call.
    Assistant {
        text: String,
        calls: Vec<ToolInvocation>,
    },
    /// Results for the calls in the immediately preceding assistant turn.
    /// Providers require these to arrive together, in one turn, covering
    /// EVERY call — a missing result is a protocol error, not a partial answer.
    ToolResults(Vec<ToolOutcome>),
    /// Something the user says after the model has spoken. The relay needs
    /// it to ask for the rest of a reply cut at the token limit: an assistant
    /// turn with nothing after it is a PREFILL, which Anthropic continues but
    /// an OpenAI-shaped endpoint answers afresh, repeating itself.
    User(String),
}

/// `Default` is derived so a new field can be added here without touching
/// every construction site again; the existing ones all spell
/// `..Default::default()`.
#[derive(Clone, Debug, Default)]
pub struct CompletionRequest {
    pub model: String,
    pub system: Option<String>,
    /// The opening user message.
    pub prompt: String,
    pub max_tokens: u32,
    /// Everything after `prompt`. Empty for a one-shot completion, which is
    /// what every caller but the tool loop wants.
    pub turns: Vec<Turn>,
    /// Tools the model may call. Empty = a plain completion, and providers
    /// must then send NO tools field at all: an empty array is not the same
    /// as absent, and some endpoints reject it.
    pub tools: Vec<ToolDef>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Completion {
    pub text: String,
    /// The model that actually answered: the response's own `model` field,
    /// else the model the answering attempt asked for. Not always the
    /// request's — a fallback chain moves on when its first model 404s, and
    /// a reply billed at the model that was asked for was billed wrong.
    /// Empty when the provider cannot say (the mock, the Claude CLI).
    pub model: String,
    /// The WHOLE prompt, cached or not — the pane's context-fill reading.
    pub input_tokens: u32,
    /// The part of `input_tokens` the provider served from its prompt cache
    /// (OpenAI-shape `prompt_tokens_details.cached_tokens`, Anthropic
    /// `cache_read_input_tokens`), billed at the cached rate, not the full one.
    pub cached_input_tokens: u32,
    /// The part of `input_tokens` written INTO the cache this call (Anthropic
    /// `cache_creation_input_tokens`, DashScope's explicit cache), which
    /// costs more than plain input.
    pub cache_write_tokens: u32,
    pub output_tokens: u32,
    /// Exact provider-reported cost in micro-USD (OpenRouter `usage.cost`);
    /// 0 when the provider doesn't report cost.
    pub cost_microusd: u64,
    /// Tool calls the model made in this reply. Empty on every provider that
    /// does not support tools, and on every reply that simply answered.
    pub calls: Vec<ToolInvocation>,
    /// The model's REASONING for this reply, when the provider exposed it
    /// (`reasoning_content`, a `thinking` block, `<think>` tags in the text)
    /// — the whole of it, so a path that never streamed still carries it.
    /// Empty when the model showed none. Never part of `text`: the reply is
    /// what the model SAID, and a transcript that kept the working would
    /// read it back to the model as its own words on the next turn.
    pub thought: String,
    /// Generation stopped at `max_tokens`, not because the model was done
    /// (`stopreason`). Without it a reply cut mid-sentence reads as finished,
    /// and a tool call cut mid-JSON fails as malformed with nothing saying why.
    pub truncated: bool,
    /// What a model chain skipped before this reply's model answered, as
    /// `model: why` (`; `-joined when several fell), empty when the first
    /// model answered. Recorded where the chain moves on (`fellback`): a
    /// response naming another model may only be naming a snapshot id.
    pub fell_back: String,
}
