//! At most [`MAX_IN_REQUEST`] pictures in one request, whatever the harness
//! has gathered. Applied where a request is rendered, never to stored turns.
use std::borrow::Cow;

use super::{CompletionRequest, Turn};

/// The most pictures one request carries, opening ones and every turn's
/// tool-result ones together.
///
/// WHY 20: the harness resends every earlier turn, so a task that attaches 5
/// pictures and reads more over several tool rounds adds up. Anthropic's
/// vision docs (platform.claude.com/docs/en/build-with-claude/vision, read
/// 2026-11-28): a request of MORE than 20 images in total (every image block,
/// resent ones and ones inside `tool_result` content included) is a
/// "many-image request", and then every image must be at most 3000 px a side
/// or the WHOLE request is rejected. A full-size screenshot is easily larger,
/// so past 20 one late picture would fail the turn. Other limits, for scale:
/// 10 MB base64 per image on the Claude API (5 MB on Bedrock and Vertex, hence
/// `image::MAX_WIRE` keeps 5 MB), up to 600 images per request (100 on 200k
/// context models) and 128 MB of request on the first-party API.
pub const MAX_IN_REQUEST: usize = 20;

/// What a dropped picture is replaced by.
fn dropped(name: &str) -> String {
    format!("[image dropped: {name} \u{2014} an older picture; at most {MAX_IN_REQUEST} ride in one request]")
}

fn add_note(text: &mut String, note: &str) {
    if !text.is_empty() {
        text.push_str("\n\n");
    }
    text.push_str(note);
}

/// `req` itself when it holds [`MAX_IN_REQUEST`] pictures or fewer (so such a
/// request is byte-identical to before), else a copy with the OLDEST pictures
/// (tool results first, in turn order, then the opening ones) replaced by a
/// text note in the text they rode with. The oldest go so a stable history
/// yields a stable request: earlier turns are resent unchanged until the set
/// overflows, which keeps the prompt cache warm, and each later overflow
/// drops the next-oldest deterministically.
pub fn capped(req: &CompletionRequest) -> Cow<'_, CompletionRequest> {
    let in_turns: usize = req
        .turns
        .iter()
        .map(|t| match t {
            Turn::ToolResults(rs) => rs.iter().map(|r| r.images.len()).sum(),
            _ => 0,
        })
        .sum();
    let total = in_turns + req.images.len();
    if total <= MAX_IN_REQUEST {
        return Cow::Borrowed(req);
    }
    let mut out = req.clone();
    let mut over = total - MAX_IN_REQUEST;
    for turn in &mut out.turns {
        let Turn::ToolResults(rs) = turn else {
            continue;
        };
        for r in rs {
            let n = over.min(r.images.len());
            for img in r.images.drain(..n) {
                add_note(&mut r.content, &dropped(&img.name));
            }
            over -= n;
        }
    }
    let n = over.min(out.images.len());
    for img in out.images.drain(..n) {
        add_note(&mut out.prompt, &dropped(&img.name));
    }
    Cow::Owned(out)
}

#[cfg(test)]
#[path = "imagecap_tests.rs"]
pub(crate) mod tests;
