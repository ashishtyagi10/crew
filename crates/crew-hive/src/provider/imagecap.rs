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

/// The most base64 text the pictures of one request may add up to.
///
/// WHY 20 MB: sources disagree on the request ceiling. Anthropic's vision
/// page says 128 MB of request on the first-party API, but its PDF docs say
/// 32 MB, and Bedrock and OpenAI-style gateways sit lower still. 20 MB stays
/// under the 32 MB figure with room for the text and the tools, and 20
/// pictures of a typical screenshot fit well inside it.
pub const MAX_BYTES: usize = 20 * 1024 * 1024;

/// Where a drop stops: [`KEEP_IN_REQUEST`] pictures and [`KEEP_BYTES`] bytes.
/// Dropping only to the limit would drop one more old picture on every new
/// one, rewriting an early turn each round and breaking the prompt cache each
/// round. Dropping to 15 / 15 MB leaves room, so the request is stable for
/// several rounds before the next drop.
const KEEP_IN_REQUEST: usize = 15;
const KEEP_BYTES: usize = 15 * 1024 * 1024;

/// What the pictures of a history come to: which tool-result pictures (in
/// turn order) are dropped, and how many opening pictures (oldest first).
///
/// Replays the history picture by picture, as the task grew: each time the
/// running set crosses a limit it drops oldest first down to the KEEP marks.
/// A function of the history alone (nothing remembered between calls), and a
/// later picture never changes an earlier decision, so earlier turns are
/// resent unchanged until the limit is crossed again. Opening pictures are
/// counted from the start but dropped only when no tool picture is left.
fn plan(tools: &[usize], opening: &[usize]) -> (Vec<bool>, usize) {
    let mut gone = vec![false; tools.len()];
    let (mut n, mut b) = (opening.len(), opening.iter().sum::<usize>());
    let over = |n: usize, b: usize| n > MAX_IN_REQUEST || b > MAX_BYTES;
    let mut oldest = 0;
    for i in 0..tools.len() {
        n += 1;
        b += tools[i];
        if over(n, b) {
            while oldest <= i && (n > KEEP_IN_REQUEST || b > KEEP_BYTES) {
                n -= 1;
                b -= tools[oldest];
                gone[oldest] = true;
                oldest += 1;
            }
        }
    }
    let mut first = 0;
    if over(n, b) {
        while first < opening.len() && (n > KEEP_IN_REQUEST || b > KEEP_BYTES) {
            n -= 1;
            b -= opening[first];
            first += 1;
        }
    }
    (gone, first)
}

/// `req` itself when its pictures fit [`MAX_IN_REQUEST`] and [`MAX_BYTES`]
/// (so such a request is byte-identical to before), else a copy with the
/// OLDEST pictures (tool results first, in turn order, then the opening ones)
/// replaced by a text note in the text they rode with; see [`plan`] for how
/// far the drop goes and why it stays stable from round to round.
pub fn capped(req: &CompletionRequest) -> Cow<'_, CompletionRequest> {
    let tools: Vec<usize> = req
        .turns
        .iter()
        .flat_map(|t| match t {
            Turn::ToolResults(rs) => rs.iter().flat_map(|r| &r.images).collect(),
            _ => Vec::new(),
        })
        .map(|i| i.data().len())
        .collect();
    let opening: Vec<usize> = req.images.iter().map(|i| i.data().len()).collect();
    let (gone, first) = plan(&tools, &opening);
    if first == 0 && !gone.contains(&true) {
        return Cow::Borrowed(req);
    }
    let mut out = req.clone();
    let mut at = 0;
    for turn in &mut out.turns {
        let Turn::ToolResults(rs) = turn else {
            continue;
        };
        for r in rs {
            let mut keep = Vec::new();
            for img in std::mem::take(&mut r.images) {
                match gone[at] {
                    true => add_note(&mut r.content, &dropped(&img.name)),
                    false => keep.push(img),
                }
                at += 1;
            }
            r.images = keep;
        }
    }
    for img in out.images.drain(..first) {
        add_note(&mut out.prompt, &dropped(&img.name));
    }
    Cow::Owned(out)
}

#[cfg(test)]
#[path = "imagecap_tests.rs"]
pub(crate) mod tests;
