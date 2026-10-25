//! Reading a list the model sent, and writing one out.

use serde_json::Value;

use super::{Item, Status, MAX_ITEMS, MAX_TEXT};

pub(super) fn parse(args: &str) -> Result<Vec<Item>, String> {
    let v: Value = serde_json::from_str(args.trim())
        .map_err(|e| format!("arguments are not valid JSON: {e}"))?;
    let items = v.get("items").and_then(Value::as_array).ok_or(
        "missing array argument \u{201c}items\u{201d} \u{2014} send the whole list: \
         {\"items\": [{\"text\": \u{2026}, \"status\": \"pending\"}]}",
    )?;
    if items.len() > MAX_ITEMS {
        return Err(format!(
            "{} items \u{2014} a checklist holds at most {MAX_ITEMS}; fold small steps together",
            items.len()
        ));
    }
    let items: Vec<Item> = items
        .iter()
        .enumerate()
        .map(|(i, v)| item(v, i + 1))
        .collect::<Result<_, _>>()?;
    let doing: Vec<String> = (1..=items.len())
        .filter(|n| items[n - 1].status == Status::InProgress)
        .map(|n| n.to_string())
        .collect();
    if doing.len() > 1 {
        return Err(format!(
            "items {} are all in_progress \u{2014} at most ONE item may be in_progress: \
             the one you are on now",
            doing.join(", ")
        ));
    }
    Ok(items)
}

/// Item `n`, its text on one line. `content` is TodoWrite's name for the text
/// and is read for the same reason `completed` is.
fn item(v: &Value, n: usize) -> Result<Item, String> {
    let raw = v.get("text").or_else(|| v.get("content"));
    let text = raw.and_then(Value::as_str).unwrap_or_default();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Err(format!("item {n} has no \u{201c}text\u{201d}"));
    }
    let len = text.chars().count();
    if len > MAX_TEXT {
        return Err(format!(
            "item {n} is {len} chars \u{2014} keep each to {MAX_TEXT}"
        ));
    }
    let status = Status::read(v.get("status"), n)?;
    Ok(Item { text, status })
}

/// `☐ text` / `▶ text` / `☑ text`, one a line, then `2 of 5 done`.
pub(super) fn render(items: &[Item]) -> String {
    if items.is_empty() {
        return "checklist cleared".into();
    }
    let done = items.iter().filter(|i| i.status == Status::Done).count();
    let mut lines: Vec<String> = items
        .iter()
        .map(|i| format!("{} {}", i.status.mark(), i.text))
        .collect();
    lines.push(format!("{done} of {} done", items.len()));
    lines.join("\n")
}
