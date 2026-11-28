//! The 20-picture cap on the request; the wires' own tests sit with them.
use super::{capped, plan, KEEP_IN_REQUEST, MAX_BYTES};
use crate::provider::image::fake_png;
use crate::provider::{CompletionRequest, Image, ToolInvocation, ToolOutcome, Turn};
use serde_json::{json, Value};

fn pic(name: &str) -> Image {
    Image::from_bytes(name, &fake_png(4, 4)).unwrap()
}

/// 5 opening pictures `o0..o4`, then `rounds` tool rounds of 5 each `r<round>_<i>`.
pub(crate) fn request(model: &str, rounds: usize) -> CompletionRequest {
    let mut turns = Vec::new();
    for r in 0..rounds {
        turns.push(Turn::Assistant {
            text: String::new(),
            calls: vec![ToolInvocation {
                id: format!("c{r}"),
                name: "sys__read_file".into(),
                input: json!({}),
                bad_args: None,
            }],
        });
        turns.push(Turn::ToolResults(vec![ToolOutcome {
            id: format!("c{r}"),
            name: "sys__read_file".into(),
            content: format!("round {r}"),
            is_error: false,
            images: (0..5).map(|i| pic(&format!("r{r}_{i}"))).collect(),
        }]));
    }
    CompletionRequest {
        model: model.into(),
        prompt: "look".into(),
        images: (0..5).map(|i| pic(&format!("o{i}"))).collect(),
        turns,
        ..Default::default()
    }
}

pub(crate) fn note(name: &str) -> String {
    format!("[image dropped: {name} \u{2014} an older picture; at most 20 ride in one request]")
}

pub(crate) fn count(v: &Value, kind: &str) -> usize {
    match v {
        Value::Object(m) => {
            usize::from(m.get("type").and_then(Value::as_str) == Some(kind))
                + m.values().map(|x| count(x, kind)).sum::<usize>()
        }
        Value::Array(a) => a.iter().map(|x| count(x, kind)).sum(),
        _ => 0,
    }
}

#[test]
fn under_the_cap_the_request_is_untouched() {
    let req = request("claude-sonnet-5-5", 3); // 5 + 15 = 20
    assert!(matches!(capped(&req), std::borrow::Cow::Borrowed(_)));
    assert_eq!(format!("{:?}", *capped(&req)), format!("{req:?}"));
}

#[test]
fn over_the_cap_the_oldest_tool_pictures_drop_in_turn_order() {
    let req = request("claude-sonnet-5-5", 4); // 25 pictures
    let c = capped(&req);
    let Turn::ToolResults(first) = &c.turns[1] else {
        panic!()
    };
    assert!(first[0].images.is_empty());
    assert_eq!(
        first[0].content,
        format!(
            "round 0\n\n{}",
            ["r0_0", "r0_1", "r0_2", "r0_3", "r0_4"]
                .map(note)
                .join("\n\n")
        )
    );
    assert_eq!(c.images.len(), 5, "opening pictures outlive tool ones");
    let Turn::ToolResults(last) = &c.turns[7] else {
        panic!()
    };
    assert_eq!(last[0].images.len(), 5);
    // the stored request is not mutated
    assert_eq!(req.images.len(), 5);
    let Turn::ToolResults(stored) = &req.turns[1] else {
        panic!()
    };
    assert_eq!(stored[0].images.len(), 5);
}

#[test]
fn opening_pictures_go_only_after_every_tool_picture() {
    let req = request("claude-sonnet-5-5", 6); // 35: 15 over = rounds 0..2
    let c = capped(&req);
    assert_eq!(c.images.len(), 5);
    let mut req = request("claude-sonnet-5-5", 4);
    req.images.extend((5..8).map(|i| pic(&format!("o{i}")))); // 28: 8 over
    let c = capped(&req);
    assert_eq!(c.images.len(), 8, "tool pictures first");
    let Turn::ToolResults(r1) = &c.turns[3] else {
        panic!()
    };
    assert!(r1[0].images.is_empty(), "round 1 is gone too");
    let Turn::ToolResults(r2) = &c.turns[5] else {
        panic!()
    };
    assert_eq!(r2[0].images.len(), 3, "round 2 loses its first 2");
    let mut req = request("claude-sonnet-5-5", 0);
    req.images = (0..22).map(|i| pic(&format!("o{i}"))).collect();
    let c = capped(&req);
    assert_eq!(c.images.len(), KEEP_IN_REQUEST, "drops to 15, not 20");
    assert_eq!(c.images[0].name, "o7");
    assert!(c.prompt.starts_with("look\n\n") && c.prompt.contains(&note("o6")));
}

#[test]
fn a_stable_history_gives_a_stable_request() {
    let req = request("claude-sonnet-5-5", 4);
    assert_eq!(
        format!("{:?}", capped(&req)),
        format!("{:?}", capped(&req.clone()))
    );
}

/// `n` more rounds of one picture each, `x<i>`.
fn grow(mut req: CompletionRequest, n: usize) -> CompletionRequest {
    let base = req.turns.len();
    for i in 0..n {
        req.turns.push(Turn::ToolResults(vec![ToolOutcome {
            id: format!("x{i}"),
            name: "sys__read_file".into(),
            content: format!("extra {}", base + i),
            is_error: false,
            images: vec![pic(&format!("x{}", base + i))],
        }]));
    }
    req
}

fn turns(r: &CompletionRequest) -> Vec<String> {
    r.turns.iter().map(|t| format!("{t:?}")).collect()
}

#[test]
fn one_more_picture_after_a_drop_rewrites_no_earlier_turn() {
    let a = grow(request("claude-sonnet-5-5", 4), 0); // 25: dropped to 15 + the 4 rounds after
    let (ca, cb) = (
        capped(&a).into_owned(),
        capped(&grow(a.clone(), 1)).into_owned(),
    );
    assert_eq!(
        turns(&ca),
        turns(&cb)[..turns(&ca).len()],
        "no early turn moved"
    );
    assert_eq!(cb.images.len(), 5);
    // the limit crossed again: now the oldest kept ones go
    let cc = capped(&grow(a.clone(), 3)).into_owned();
    assert_ne!(turns(&ca), turns(&cc)[..turns(&ca).len()]);
}

fn big(name: &str) -> Image {
    // 1.5 MB of bytes is exactly 2 MB of base64
    let mut b = fake_png(4, 4);
    b.resize(1536 * 1024, 0);
    Image::from_bytes(name, &b).unwrap()
}

#[test]
fn the_byte_budget_drops_oldest_first_down_to_15_mb() {
    let mut req = request("claude-sonnet-5-5", 0);
    req.images = (0..11).map(|i| big(&format!("b{i}"))).collect(); // 22 MB, 11 pictures
    let c = capped(&req);
    assert_eq!(c.images.len(), 7, "7 x 2 MB = 14 MB is the most under 15");
    assert_eq!(c.images[0].name, "b4");
    assert!(c.prompt.contains(&note("b3")) && !c.prompt.contains("b4 "));
    req.images.truncate(10); // exactly 20 MB: not over
    assert!(matches!(capped(&req), std::borrow::Cow::Borrowed(_)));
}

#[test]
fn plan_counts_bytes_and_pictures_both() {
    let mb = 1024 * 1024;
    // 9 tool pictures of 2 MB + 1 opening of 2 MB = 20 MB: fits
    assert_eq!(plan(&[2 * mb; 9], &[2 * mb]), (vec![false; 9], 0));
    // one more byte-heavy one crosses: oldest tools go to 15 MB (7 left)
    let (gone, first) = plan(&[2 * mb; 10], &[2 * mb]);
    assert_eq!((gone.iter().filter(|g| **g).count(), first), (4, 0));
    assert!(MAX_BYTES > 15 * mb);
}
