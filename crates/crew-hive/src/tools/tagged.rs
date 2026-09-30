//! A tool call written in the shape the model was trained on, not the one the
//! prompt asked for.
//!
//! Qwen 3.x and the Hermes-format fine-tunes learned to call tools as
//! `<tool_call>{"name": …, "arguments": {…}}</tool_call>`, and deep in a long
//! prompt they fall back to it: qwen3.8-max wrote one in 2 of 4 clean probe
//! runs (2026-09-30) where crew had asked for `@tool`. Nothing read it, so the
//! raw block was the answer and the file it asked for was never opened.
//! Qwen3-Coder's template puts the same call in the same tag as XML — a
//! `<function=…>` holding one `<parameter=…>` per argument.
//!
//! Rather than a second parser, [`untag`] rewrites each such block as the
//! `@tool` line it stands for and the one parser reads the reply as before. A
//! tagged call is held to the same rules as any other: it has to be the
//! reply's last word, and one with a paragraph under it is an example.

use serde_json::Value;
use std::borrow::Cow;

const OPEN: &str = "<tool_call>";
const CLOSE: &str = "</tool_call>";

/// `reply` with every `<tool_call>` block that names a crew tool (`server:tool`)
/// written as an `@tool` line of its own. Borrowed, untouched, when there is
/// none; a block that is not a call — no name, JSON that never closes — stays
/// as written.
pub(super) fn untag(reply: &str) -> Cow<'_, str> {
    if !reply.contains(OPEN) {
        return Cow::Borrowed(reply);
    }
    let mut out = String::with_capacity(reply.len());
    let mut rest = reply;
    while let Some(i) = rest.find(OPEN) {
        let body = &rest[i + OPEN.len()..];
        let Some((line, used)) = call(body) else {
            out.push_str(&rest[..i + OPEN.len()]);
            rest = body;
            continue;
        };
        out.push_str(rest[..i].trim_end_matches([' ', '\t']));
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&line);
        rest = body[used..].trim_start_matches([' ', '\t']);
        if !rest.is_empty() && !rest.starts_with(['\n', '\r']) {
            out.push('\n');
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// The `@tool` line the block opening `body` stands for, and how much of
/// `body` it took — its closing tag too, when the model wrote one.
fn call(body: &str) -> Option<(String, usize)> {
    let lead = body.len() - body.trim_start().len();
    let s = &body[lead..];
    let (name, args, len) = json_call(s).or_else(|| xml_call(s))?;
    if !name.contains(':') {
        return None;
    }
    let mut used = lead + len;
    let after = &body[used..];
    let gap = after.len() - after.trim_start().len();
    if after[gap..].starts_with(CLOSE) {
        used += gap + CLOSE.len();
    }
    Some((format!("@tool {name} {args}"), used))
}

/// `{"name": …, "arguments": …}`: name, arguments as written, and length.
///
/// The arguments go under `arguments` (Hermes, Qwen), `input` (Anthropic's
/// block) or `parameters` (Llama 3.1); OpenAI's shape sends them as a string
/// of JSON, which is taken as that JSON. An object keeps its own text, as a
/// `@tool` line's does — re-serialising would sort its keys on the card.
fn json_call(s: &str) -> Option<(String, String, usize)> {
    let mut values = serde_json::Deserializer::from_str(s).into_iter::<Value>();
    let Value::Object(map) = values.next()?.ok()? else {
        return None;
    };
    let len = values.byte_offset();
    let name = map.get("name")?.as_str()?.trim().to_string();
    let key = ["arguments", "input", "parameters"]
        .into_iter()
        .find(|k| map.contains_key(*k));
    let args = match key.map(|k| (k, &map[k])) {
        None => "{}".to_string(),
        Some((_, Value::String(text))) => text.clone(),
        Some((k, value)) => raw(&s[..len], k, value)?,
    };
    Some((name, args, len))
}

/// The text `key`'s value was written as inside `object`: the occurrence of
/// `"key":` whose value parses back to `want`, so a nested key of the same
/// name cannot stand in for it.
fn raw(object: &str, key: &str, want: &Value) -> Option<String> {
    let quoted = format!("\"{key}\"");
    object.match_indices(&quoted).find_map(|(i, _)| {
        let value = object[i + quoted.len()..]
            .trim_start()
            .strip_prefix(':')?
            .trim_start();
        let mut it = serde_json::Deserializer::from_str(value).into_iter::<Value>();
        (it.next()?.ok()? == *want).then(|| value[..it.byte_offset()].to_string())
    })
}

/// Qwen3-Coder's `<function=NAME><parameter=KEY>VALUE</parameter>…</function>`:
/// name, the parameters as a JSON object in the order written, and length.
fn xml_call(s: &str) -> Option<(String, String, usize)> {
    let rest = s.strip_prefix("<function=")?;
    let (name, inner) = rest.split_once('>')?;
    let end = inner.find("</function>")?;
    let mut body = &inner[..end];
    let mut fields = Vec::new();
    while let Some(i) = body.find("<parameter=") {
        let (key, after) = body[i + "<parameter=".len()..].split_once('>')?;
        let stop = after.find("</parameter>")?;
        let key = serde_json::to_string(key.trim()).ok()?;
        fields.push(format!("{key}: {}", param(&after[..stop])));
        body = &after[stop + "</parameter>".len()..];
    }
    let len = s.len() - inner.len() + end + "</function>".len();
    Some((
        name.trim().to_string(),
        format!("{{{}}}", fields.join(", ")),
        len,
    ))
}

/// One parameter's text as a JSON value. The template writes every value as
/// bare text between newlines; the schema that says which are numbers is not
/// here, so only what cannot be a string — an object, an array, `true`,
/// `false` — is read as JSON. Everything else stays a string: crew's numeric
/// arguments take `"40"`, while a grep for `404` sent as a number would be
/// refused.
fn param(text: &str) -> String {
    let text = text.strip_prefix('\n').unwrap_or(text);
    let text = text.strip_suffix('\n').unwrap_or(text);
    let t = text.trim();
    let structured = t.starts_with(['{', '[']) || t == "true" || t == "false";
    match serde_json::from_str::<Value>(t) {
        Ok(_) if structured => t.to_string(),
        _ => Value::String(text.to_string()).to_string(),
    }
}

#[cfg(test)]
#[path = "tagged_tests.rs"]
mod tests;
