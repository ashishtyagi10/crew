//! The 20-picture cap on the request; the wires' own tests sit with them.
use super::{capped, MAX_IN_REQUEST};
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
    assert_eq!(r1[0].images.len(), 2, "round 1 loses its first 3");
    let mut req = request("claude-sonnet-5-5", 0);
    req.images = (0..22).map(|i| pic(&format!("o{i}"))).collect();
    let c = capped(&req);
    assert_eq!(c.images.len(), MAX_IN_REQUEST);
    assert_eq!(c.images[0].name, "o2");
    assert!(c.prompt.starts_with("look\n\n") && c.prompt.contains(&note("o1")));
}

#[test]
fn a_stable_history_gives_a_stable_request() {
    let req = request("claude-sonnet-5-5", 4);
    assert_eq!(
        format!("{:?}", capped(&req)),
        format!("{:?}", capped(&req.clone()))
    );
}
