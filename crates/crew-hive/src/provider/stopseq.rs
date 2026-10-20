//! Generation stops where a model's own tool call ends: `</tool_call>`.
//!
//! A model trained on the Hermes / Qwen template writes a call as
//! `<tool_call>{…}</tool_call>` and expects the server to stop it there and
//! run the call — its chat template makes the closing tag the end of the
//! turn. Crew's text protocol sends no tools, so nothing stopped it: live
//! (2026-09-30) qwen3.8-max wrote the block, saw no result, wrote "Wait, let
//! me format that correctly." and the block again, until `max_tokens` — 4,606
//! tokens and 91 s for one `sys:run` that never ran. Stopping at the tag ends
//! the reply on the call, which the parser (`tools::parse`, reading a block
//! whose closing tag never came) runs like any other.
//!
//! It also stops where a model starts making up a call's output. The log of
//! earlier calls puts each result under a `RESULT:` line, and a model copying
//! the log wrote one under its own call — then the next call, and the next —
//! instead of stopping for the real one (live, 2026-10-20: four invented
//! edits and a test run nobody ran, ~3 KB of reply). Stopped at the marker,
//! the reply ends on the call, and the call runs (`tools::narrated`).
//!
//! Only where calls are TEXT: a request that carries `tools` gets native
//! calls, parsed server-side from the same tag, and a stop there would cut
//! the server's own parser short. And not on OpenAI's own host, whose
//! reasoning models refuse `stop` (`Unsupported parameter`) and which were
//! never trained on the tag.
use serde_json::{json, Value};

/// The closing tag of a Hermes / Qwen tool call, and the exchange log's result
/// marker at the start of a line.
const STOP: [&str; 2] = ["</tool_call>", "\nRESULT:"];

/// Adds `stop: ["</tool_call>", "\nRESULT:"]` to a text-protocol request body
/// bound for any host but OpenAI's own.
pub(super) fn end_text_calls(openai: bool, body: &mut Value) {
    if openai || body.get("tools").is_some() || body.get("stop").is_some() {
        return;
    }
    if let Some(obj) = body.as_object_mut() {
        obj.insert("stop".into(), json!(STOP));
    }
}

#[cfg(test)]
#[path = "stopseq_tests.rs"]
mod tests;
