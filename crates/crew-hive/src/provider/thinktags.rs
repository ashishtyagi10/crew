//! `<think>…</think>` in reply TEXT, routed to reasoning.
//!
//! Some OpenAI-compatible endpoints have no field for a model's reasoning
//! and hand it back inline, fenced in `<think>` tags at the head of
//! `delta.content` (DeepSeek-R1 distils on vLLM and NIM, Qwen3 with the
//! thinking switch off, most local servers). Passed through, the tags and
//! the working land in the reply card as if the model had said them — and
//! the transcript then reads them back to the model as its own words.
//!
//! A stream does not respect tag boundaries: `<thi` arrives in one frame
//! and `nk>` in the next, so the splitter holds back any trailing run that
//! could still become a tag and decides once the rest arrives. Pure — text
//! in, pieces out, no clock — so the non-streamed path uses the same one on
//! a whole body.

const OPEN: &str = "<think>";
const CLOSE: &str = "</think>";

/// One routed run of text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    Text(String),
    Thought(String),
}

/// The splitter's state between fragments.
#[derive(Default)]
pub(crate) struct ThinkTags {
    inside: bool,
    /// A trailing run that is a proper prefix of the tag we are waiting for.
    carry: String,
}

/// Bytes at the end of `buf` that could still grow into `tag` — the longest
/// proper prefix of the tag that `buf` ends with. ASCII tag, so the cut is
/// always a char boundary.
fn partial_suffix(buf: &str, tag: &str) -> usize {
    (1..tag.len())
        .rev()
        .find(|&k| buf.ends_with(&tag[..k]))
        .unwrap_or(0)
}

impl ThinkTags {
    /// Route one fragment. Text before an opening tag and after a closing
    /// one is [`Piece::Text`]; everything between is [`Piece::Thought`].
    pub(crate) fn feed(&mut self, s: &str) -> Vec<Piece> {
        let mut buf = std::mem::take(&mut self.carry);
        buf.push_str(s);
        let mut out = Vec::new();
        loop {
            let tag = if self.inside { CLOSE } else { OPEN };
            match buf.find(tag) {
                Some(i) => {
                    self.emit(&mut out, &buf[..i]);
                    buf = buf[i + tag.len()..].to_string();
                    self.inside = !self.inside;
                }
                None => {
                    let keep = partial_suffix(&buf, tag);
                    let (send, hold) = buf.split_at(buf.len() - keep);
                    self.emit(&mut out, send);
                    self.carry = hold.to_string();
                    return out;
                }
            }
        }
    }

    /// The stream ended: whatever was held back was not a tag after all.
    pub(crate) fn finish(&mut self) -> Vec<Piece> {
        let rest = std::mem::take(&mut self.carry);
        let mut out = Vec::new();
        self.emit(&mut out, &rest);
        out
    }

    fn emit(&self, out: &mut Vec<Piece>, s: &str) {
        if s.is_empty() {
            return;
        }
        out.push(match self.inside {
            true => Piece::Thought(s.to_string()),
            false => Piece::Text(s.to_string()),
        });
    }

    /// A whole body split in one go: `(text, thought)`.
    pub(crate) fn split(body: &str) -> (String, String) {
        let mut tags = ThinkTags::default();
        let (mut text, mut thought) = (String::new(), String::new());
        for piece in tags.feed(body).into_iter().chain(tags.finish()) {
            match piece {
                Piece::Text(s) => text.push_str(&s),
                Piece::Thought(s) => thought.push_str(&s),
            }
        }
        (text, thought)
    }
}

#[cfg(test)]
#[path = "thinktags_tests.rs"]
mod tests;
