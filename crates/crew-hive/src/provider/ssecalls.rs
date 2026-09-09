//! Streamed tool calls in the OpenAI shape: reassembly of the index-keyed
//! fragments a stream spreads one call across.
//!
//! A non-streamed reply carries each call whole in `message.tool_calls`. A
//! stream sends the same call as a run of `delta.tool_calls[i]` frames —
//! the first with the `index`, `id` and `function.name`, the rest with the
//! `function.arguments` JSON a few characters at a time — and several calls
//! may interleave. Until this existed, `OpenRouterProvider` refused to
//! stream any tool request at all, so a tool-using turn showed nothing live
//! until the round ended. This puts the pieces back together so the stream
//! ends in exactly the `ToolInvocation`s the non-streamed parse would give.
use super::ToolInvocation;

/// One `delta.tool_calls[i]` frame, as much of it as was present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Frag {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
}

/// The fragments in one delta's `tool_calls` array. A frame that omits
/// `index` (seen from some proxies) takes its array position, which is what
/// the index would have been on a single-call reply.
pub(crate) fn frags(delta: &serde_json::Value) -> Vec<Frag> {
    let Some(arr) = delta["tool_calls"].as_array() else {
        return Vec::new();
    };
    arr.iter()
        .enumerate()
        .map(|(pos, tc)| Frag {
            index: tc["index"].as_u64().map_or(pos, |i| i as usize),
            id: tc["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            name: tc["function"]["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            arguments: tc["function"]["arguments"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
        .collect()
}

/// Arguments as the wire sends them — a JSON *string*. An unparseable or
/// empty one becomes `{}` rather than failing the reply: the tool rejects it
/// with a message the agent can act on, which beats losing the completion.
pub(crate) fn parse_args(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({}))
}

/// The calls of one reply, accumulating across frames.
#[derive(Default)]
pub(crate) struct CallAsm {
    /// `(index, id, name, arguments so far)`, in first-seen order.
    calls: Vec<(usize, String, String, String)>,
}

impl CallAsm {
    pub(crate) fn push(&mut self, f: Frag) {
        let slot = match self.calls.iter().position(|c| c.0 == f.index) {
            Some(i) => i,
            None => {
                self.calls
                    .push((f.index, String::new(), String::new(), String::new()));
                self.calls.len() - 1
            }
        };
        let c = &mut self.calls[slot];
        if let Some(id) = f.id {
            c.1 = id;
        }
        if let Some(name) = f.name {
            c.2 = name;
        }
        c.3.push_str(&f.arguments);
    }

    /// The assembled calls. One with no name never became a call — a server
    /// that streams `finish_reason: tool_calls` without the frames leaves
    /// nothing here, and the caller falls back to the non-streamed request.
    pub(crate) fn finish(self) -> Vec<ToolInvocation> {
        self.calls
            .into_iter()
            .filter(|c| !c.2.is_empty())
            .map(|(_, id, name, args)| ToolInvocation {
                id,
                name,
                input: parse_args(&args),
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "ssecalls_tests.rs"]
mod tests;
