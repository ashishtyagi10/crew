//! What one `claude -p` run adds up to once its lines are parsed
//! (`claudestream::parse_line`): the reply, the reasoning, the closing
//! `result`. Its own file so the line parser has room to grow.
use super::claudestream::{RunResult, StreamEvent};

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
