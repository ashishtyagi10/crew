//! The line protocol of `claude -p --output-format stream-json --verbose
//! --include-partial-messages` (claude 2.x, captured 2026-09-09): one JSON
//! object per line. Text and thinking arrive as `stream_event` deltas, tool
//! calls as whole `assistant` messages, their outcomes as `user` messages,
//! and the run ends on one `result` line carrying the reply and the usage.
//! Everything else (hooks, init, rate limits) is noise. Pure parsing, shared
//! by the swarm-side provider (`claudecli`) and the relay adapter in the
//! broker, so both read the same envelope the same way.
use serde_json::Value;

/// One line of the stream, reduced to what crew acts on.
#[derive(Clone, Debug, PartialEq)]
pub enum StreamEvent {
    /// A fragment of the reply text.
    Text(String),
    /// A fragment of the model's reasoning.
    Thought(String),
    /// A whole text block on an `assistant` message — the fallback reply
    /// source for a CLI that never sent partial deltas.
    AssistantText(String),
    /// The model asked for one of Claude Code's tools; `input` is the
    /// compact JSON of its arguments.
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    /// The outcome of the call with that `id`.
    ToolResult { id: String, ok: bool, text: String },
    /// A `system`/`status` line (`requesting`, …).
    Status(String),
    /// The closing line: the reply, whether the CLI refused, and the usage.
    Result(RunResult),
    /// A line crew does not act on.
    Other,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunResult {
    pub text: String,
    pub is_error: bool,
    pub subtype: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_microusd: u64,
}

/// Parse one stdout line. `None` for a blank line or one that is not a JSON
/// object — a banner, or the plain text an older/fake CLI prints instead of
/// the stream; the caller keeps such lines as raw reply text.
pub fn parse_line(line: &str) -> Option<StreamEvent> {
    let line = line.trim();
    if !line.starts_with('{') {
        return None;
    }
    let v: Value = serde_json::from_str(line).ok()?;
    let kind = v.get("type").and_then(Value::as_str).unwrap_or_default();
    Some(match kind {
        "stream_event" => delta(&v["event"]),
        "assistant" => v["message"]["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find_map(assistant_block))
            .unwrap_or(StreamEvent::Other),
        "user" => v["message"]["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find_map(tool_result))
            .unwrap_or(StreamEvent::Other),
        "system" if v["subtype"] == "status" => StreamEvent::Status(str_of(&v["status"])),
        "result" => StreamEvent::Result(RunResult {
            text: str_of(&v["result"]),
            is_error: v["is_error"].as_bool().unwrap_or(false),
            subtype: str_of(&v["subtype"]),
            input_tokens: v["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: v["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            cost_microusd: (v["total_cost_usd"].as_f64().unwrap_or(0.0) * 1e6).round() as u64,
        }),
        _ => StreamEvent::Other,
    })
}

fn delta(ev: &Value) -> StreamEvent {
    if ev["type"] != "content_block_delta" {
        return StreamEvent::Other;
    }
    let d = &ev["delta"];
    match d["type"].as_str().unwrap_or_default() {
        "text_delta" => StreamEvent::Text(str_of(&d["text"])),
        "thinking_delta" => StreamEvent::Thought(str_of(&d["thinking"])),
        _ => StreamEvent::Other,
    }
}

fn assistant_block(b: &Value) -> Option<StreamEvent> {
    match b["type"].as_str()? {
        "tool_use" => Some(StreamEvent::ToolUse {
            id: str_of(&b["id"]),
            name: str_of(&b["name"]),
            input: b["input"].to_string(),
        }),
        "text" => Some(StreamEvent::AssistantText(str_of(&b["text"]))),
        _ => None,
    }
}

fn tool_result(b: &Value) -> Option<StreamEvent> {
    if b["type"] != "tool_result" {
        return None;
    }
    // `content` is a string or a list of text blocks.
    let text = match &b["content"] {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    Some(StreamEvent::ToolResult {
        id: str_of(&b["tool_use_id"]),
        ok: !b["is_error"].as_bool().unwrap_or(false),
        text,
    })
}

fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

/// What one run adds up to: the reply (deltas, else whole assistant text,
/// else raw non-JSON lines), the reasoning, and the closing `result`.
#[derive(Debug, Default)]
pub struct Fold {
    pub text: String,
    pub thought: String,
    pub raw: String,
    pub result: Option<RunResult>,
    seen_delta: bool,
}

impl Fold {
    /// Fold one parsed line in (`None` = a raw line, kept as text).
    pub fn push(&mut self, ev: Option<&StreamEvent>, raw_line: &str) {
        match ev {
            None => {
                if !raw_line.trim().is_empty() {
                    self.raw.push_str(raw_line.trim_end());
                    self.raw.push('\n');
                }
            }
            Some(StreamEvent::Text(t)) => {
                self.seen_delta = true;
                self.text.push_str(t);
            }
            Some(StreamEvent::Thought(t)) => self.thought.push_str(t),
            Some(StreamEvent::AssistantText(t)) if !self.seen_delta => {
                if !self.text.is_empty() {
                    self.text.push('\n');
                }
                self.text.push_str(t);
            }
            Some(StreamEvent::Result(r)) => self.result = Some(r.clone()),
            Some(_) => {}
        }
    }

    /// The reply: the `result` line's text, else what streamed, else the
    /// raw lines. Empty when the CLI said nothing usable.
    pub fn reply(&self) -> String {
        let settled = self.result.as_ref().map(|r| r.text.trim()).unwrap_or("");
        if !settled.is_empty() {
            return settled.to_string();
        }
        if !self.text.trim().is_empty() {
            return self.text.trim().to_string();
        }
        self.raw.trim().to_string()
    }
}

#[cfg(test)]
#[path = "claudestream_tests.rs"]
mod tests;
