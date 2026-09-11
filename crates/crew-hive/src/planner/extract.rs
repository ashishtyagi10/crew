//! The task array inside a planner reply, however the model wrapped it.
//!
//! The prompt ends "Return ONLY the JSON array, no prose", and most replies
//! obey. The ones that do not — a ```json fence, a sentence of preamble, a
//! trailing comma before the closing bracket — used to fail the whole swarm
//! down to a one-task direct answer, because `parse_plan` is strict by
//! design: it is the trust boundary and must stay so. This pass finds the
//! array so the strict parser is handed exactly that. It never reads what is
//! inside a string as structure, and it never invents structure that is not
//! there: a reply with no array, or one `max_tokens` cut off mid-array, is
//! still a failure — the re-ask in `repair` is what handles those.

/// The outermost `[ … ]` of `reply`: from the first `[` to the bracket that
/// closes it, counting depth outside strings so a `]` inside a task's prompt
/// text, or in prose after the array, cannot end it early. `None` when there
/// is no `[`, or the array never closes.
pub(crate) fn array_text(reply: &str) -> Option<&str> {
    let start = reply.find('[')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for (i, c) in reply[start..].char_indices() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '[' | '{' => depth += 1,
            ']' | '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(&reply[start..start + i + c.len_utf8()]);
                }
            }
            _ => {}
        }
    }
    None
}

/// `json` with every trailing comma dropped — a `,` that, outside a string,
/// has nothing but whitespace between it and a `]` or `}`. That is the one
/// JSON slip models make that is unambiguous to undo; anything else stays
/// the parser's verdict. Valid JSON passes through byte-for-byte.
pub(crate) fn without_trailing_commas(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let mut in_str = false;
    let mut escaped = false;
    for (i, c) in json.char_indices() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == ',' {
            let rest = json[i + 1..].trim_start();
            if rest.starts_with(']') || rest.starts_with('}') {
                continue;
            }
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
