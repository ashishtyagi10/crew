//! A tool call's arguments as the OpenAI shape sends them: a JSON *string*,
//! written by the model and checked by nobody on the way.
//!
//! An argument string that did not parse used to become `{}`, and the call
//! ran with nothing: the tool answered "missing argument path" and the model,
//! shown neither what it sent nor why it failed, tried the same thing again.
//! It is almost always a reply cut off at the token limit mid-string, or a
//! hand slip. Now such a call carries why ([`ToolInvocation::bad_args`]) and
//! the loop answers it with that instead of running it.
//!
//! Two slips are undone before giving up, because each has one reading and
//! guessing at neither: a trailing comma before a `}` or `]` (the planner's
//! own pass, `planner::extract`), and a whole object followed by junk that
//! holds no second object (`{"path":"a.rs"}}`, `{…}</tool_call>`). A second
//! object after the first is two calls run together, and taking either one
//! would be a guess, so that is still refused. Nothing is ever closed for
//! the model: a string cut short stays refused.
use serde_json::Value;

use super::ToolInvocation;

/// How much of what the model sent a refusal shows back to it: enough to
/// see which call and where it went wrong, not the whole cut-off file.
const SHOWN: usize = 120;

/// A call as the wire gave it, its arguments read by [`parse`]. A string
/// that does not parse leaves `input` as `{}`, the shape every consumer
/// expects, and says why in `bad_args`.
pub(crate) fn invocation(id: String, name: String, args: &str) -> ToolInvocation {
    let (input, bad_args) = match parse(args) {
        Ok(v) => (v, None),
        Err(why) => (serde_json::json!({}), Some(why)),
    };
    ToolInvocation {
        id,
        name,
        input,
        bad_args,
    }
}

/// `args` as a value, or one line saying why not: the parser's own words,
/// then the start of what was sent. Empty is `{}`: some servers send `""`
/// for a call that takes no arguments, and it can mean nothing else.
pub(crate) fn parse(args: &str) -> Result<Value, String> {
    if args.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    let err = match serde_json::from_str(args) {
        Ok(v) => return Ok(v),
        Err(e) => e,
    };
    let tidy = crate::planner::extract::without_trailing_commas(args);
    if let Ok(v) = serde_json::from_str(&tidy) {
        return Ok(v);
    }
    if let Some(v) = leading_object(&tidy) {
        return Ok(v);
    }
    Err(format!(
        "{} \u{2014} {}",
        one_line(&err.to_string()),
        shown(args)
    ))
}

/// The object `s` opens with, when nothing after it could be another one.
fn leading_object(s: &str) -> Option<Value> {
    let mut values = serde_json::Deserializer::from_str(s).into_iter::<Value>();
    let first = values.next()?.ok()?;
    let rest = &s[values.byte_offset()..];
    (first.is_object() && !rest.contains('{')).then_some(first)
}

/// The start of what was sent, on one line.
fn shown(args: &str) -> String {
    let head: String = args.trim().chars().take(SHOWN).collect();
    match args.trim().chars().count() > SHOWN {
        true => format!("{}\u{2026}", one_line(&head)),
        false => one_line(&head),
    }
}

/// `s` with every control character a space. JSON allows none raw inside a
/// string (a newline there is written `\n`), so outside one they are only
/// whitespace between tokens, and folding them changes nothing that was sent.
fn one_line(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

#[cfg(test)]
#[path = "toolargs_tests.rs"]
mod tests;
