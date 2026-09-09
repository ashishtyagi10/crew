//! What a click opens under a tool line (see [`crate::chattool`]): the
//! ARGUMENTS first, then the result. The result was all a click showed, and
//! the line kept sixty characters of one argument; but the arguments are the
//! call — what was run, on which file, with which query — and an agent's
//! paraphrase of what it asked for is the one thing a reader could not check.
//! Pure: a line in, the rows' text out; `chattoolline::text_rows` paints them.
use crate::chattool::ToolLine;
use crate::chattoolkind::LineKind;

/// The most rows the arguments take before `… +N more`.
pub(crate) const ARGS_ROWS: usize = 6;

/// The rows under an opened line. A call: its arguments (one `key: value`
/// per row when they parse as a JSON object, the raw text otherwise), then —
/// once the result landed — a `→ result` row and up to
/// [`crate::chattoolline::TEXT_ROWS`] of it. A load: its detail, one
/// segment per row, an MCP server's tool names one to a row.
pub(crate) fn opened_rows(line: &ToolLine) -> Vec<String> {
    if line.kind != LineKind::Tool {
        return line
            .done
            .as_ref()
            .map_or(Vec::new(), |d| detail_rows(&d.text));
    }
    let mut rows = capped(args_rows(&line.args), ARGS_ROWS, "more");
    if let Some(d) = &line.done {
        let result: Vec<String> = d.text.lines().map(str::to_string).collect();
        if !rows.is_empty() && !result.is_empty() {
            rows.push("\u{2192} result".into());
        }
        rows.extend(capped(result, crate::chattoolline::TEXT_ROWS, "lines"));
    }
    rows
}

/// At most `cap` rows: the first `cap - 1` and `… +N <noun>` for the rest.
fn capped(mut rows: Vec<String>, cap: usize, noun: &str) -> Vec<String> {
    if rows.len() > cap {
        let rest = rows.len() - (cap - 1);
        rows.truncate(cap - 1);
        rows.push(format!("\u{2026} +{rest} {noun}"));
    }
    rows
}

/// One row per argument of a JSON object — strings bare, with their line
/// breaks flattened so one value cannot take the whole opening; anything
/// else as compact JSON. Not an object (or not JSON): the raw text's lines.
fn args_rows(args: &str) -> Vec<String> {
    if args.trim().is_empty() {
        return Vec::new();
    }
    let obj = serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.as_object().cloned());
    match obj {
        Some(obj) => obj
            .iter()
            .map(|(k, v)| match v.as_str() {
                Some(s) => format!(
                    "{k}: {}",
                    s.split_whitespace().collect::<Vec<_>>().join(" ")
                ),
                None => format!("{k}: {v}"),
            })
            .collect(),
        None => args.lines().map(str::to_string).collect(),
    }
}

/// A load's detail, split on ` · `; a `N tools: a, b, …` segment opens
/// into its list, one name to a row, since the names are what the reader
/// came for.
fn detail_rows(detail: &str) -> Vec<String> {
    let mut rows = Vec::new();
    for seg in detail.split(" \u{b7} ") {
        match seg.split_once(": ") {
            Some((head, list)) if head.ends_with("tool") || head.ends_with("tools") => {
                rows.push(format!("{head}:"));
                rows.extend(list.split(", ").map(|n| format!("  {n}")));
            }
            _ => rows.push(seg.to_string()),
        }
    }
    rows
}

#[cfg(test)]
#[path = "chattoolargs_tests.rs"]
mod tests;
