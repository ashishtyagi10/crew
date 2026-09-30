//! `sys:read_file {"line": N}`: a page that starts at a line, not a byte.
//!
//! `sys:grep` answers in lines (`route.rs:412: const TASK_CAP …`) and a page
//! was addressed only by byte offset, so looking at line 412 of a 40 KB file
//! meant paging from the top, eight calls and a whole turn's tool budget, or
//! guessing a byte. This finds the byte the line starts at and hands it to
//! [`read_file`], so the page, its whole-line cut and its `{"offset": N}` note
//! are those of any other page, and the agent goes on from there by offset.
use std::io::{BufRead, BufReader, Read};

use super::sysread::{grouped, offset_arg, read_file, LINES_UP_TO};

#[cfg(test)]
#[path = "sysreadline_tests.rs"]
mod tests;

/// `sys:read_file`'s arguments past `path`: a `line` to start at, or the byte
/// `offset` a previous page named. Both at once is refused rather than one
/// quietly winning, because the agent that sent both meant one of them and
/// only it knows which.
///
/// And `lines`, the most the page shows (Claude Code `Read`'s `limit`). A
/// page is ~5 KB of look-alike rows, and asked for line 1,777 of a saved run,
/// qwen-max answered with row 1,877's value three runs in three — from a page
/// that opened on the right row, numbered. `{"line": 1777, "lines": 1}` is a
/// page with one row on it, which cannot be misread.
pub(super) fn read(path: &str, v: &serde_json::Value) -> Result<String, String> {
    let most = from_one(v, "lines").map_err(|()| {
        "invalid \u{201c}lines\u{201d}: expected how many lines to show, 1 or more".to_string()
    })?;
    let Some(line) = line_arg(v)? else {
        return read_file(path, offset_arg(v)?, most);
    };
    if v.get("offset").is_some_and(|o| !o.is_null()) {
        return Err(
            "pass \u{201c}line\u{201d} or \u{201c}offset\u{201d}, not both \u{2014} line to start at a line, offset to continue a page"
                .into(),
        );
    }
    match line_start(path, line)? {
        Seek::At(at) => read_file(path, at, most),
        Seek::Past(lines) => Ok(format!(
            "\u{2026} (line {} is past the end \u{2014} the file has {} line{})",
            grouped(line),
            grouped(lines),
            if lines == 1 { "" } else { "s" }
        )),
    }
}

/// `sys:read_file`'s JSON Schema, beside the arguments it describes.
pub(super) fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "path, relative to the working directory"},
            "line": {
                "type": "integer",
                "minimum": 1,
                "description": "line to start the page at, counting from 1: the number in a sys:grep hit (path:line: text); give this or offset, not both",
            },
            "offset": {
                "type": "integer",
                "minimum": 0,
                "description": "byte offset to start at: the one the previous page's last line names",
            },
            "lines": {
                "type": "integer",
                "minimum": 1,
                "description": "show at most this many lines: {\"line\": 1777, \"lines\": 1} is line 1,777 alone",
            },
        },
        "required": ["path"],
    })
}

/// The optional `"line"` argument, counting from 1 the way grep does. Zero,
/// negatives, fractions and anything else are an error the agent can read,
/// not a page from the top.
fn line_arg(v: &serde_json::Value) -> Result<Option<usize>, String> {
    from_one(v, "line").map_err(|()| {
        "invalid \u{201c}line\u{201d}: expected a line number, counting from 1".into()
    })
}

/// `key` as a whole number from 1, a JSON number or a numeric string like
/// `offset` (agents send both); `Err` for anything else.
fn from_one(v: &serde_json::Value, key: &str) -> Result<Option<usize>, ()> {
    let n = match v.get(key) {
        None | Some(serde_json::Value::Null) => return Ok(None),
        Some(serde_json::Value::Number(n)) => n.as_u64().and_then(|n| usize::try_from(n).ok()),
        Some(serde_json::Value::String(s))
            if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) =>
        {
            s.parse::<usize>().ok()
        }
        _ => None,
    };
    n.filter(|&n| n >= 1).map(Some).ok_or(())
}

/// Where a line starts, or, when the file ends first, how many lines it has:
/// an offset past the end is answered with the file's size, a line past the
/// end with its line count, both so the next call is not another guess.
enum Seek {
    At(usize),
    Past(usize),
}

/// The byte `line` starts at, just past the file's `line - 1`th newline.
/// Lines are counted the way grep counts them, by `\n`, so a CRLF file
/// numbers the same. The scan stops at the line, so line 30 of a 2 GB log
/// costs one buffer; it gives up past [`LINES_UP_TO`] for the reason the page
/// note stops counting there.
fn line_start(path: &str, line: usize) -> Result<Seek, String> {
    let err = |e: std::io::Error| format!("read {path}: {e}");
    let f = std::fs::File::open(path).map_err(|e| super::syspath::with_hint("read", path, e))?;
    let mut r = BufReader::new(f.take(LINES_UP_TO as u64 + 1));
    let (mut at, mut passed, mut seg) = (0, 0, Vec::new());
    while passed < line - 1 {
        seg.clear();
        let n = r.read_until(b'\n', &mut seg).map_err(err)?;
        at += n;
        if seg.last() != Some(&b'\n') {
            if at > LINES_UP_TO {
                return Err(too_big(path, line));
            }
            // An unterminated last line still counts, as grep counts it.
            return Ok(Seek::Past(passed + usize::from(n > 0)));
        }
        passed += 1;
    }
    // A file that ends on the newline before `line` has no `line`: a trailing
    // newline closes the last line, it does not open another.
    let more = !r.fill_buf().map_err(err)?.is_empty();
    Ok(if more || at > LINES_UP_TO {
        Seek::At(at)
    } else {
        Seek::Past(passed)
    })
}

/// Past the scan's cap the line could be anywhere, or nowhere; a byte offset
/// still reaches it, so the error says to use one.
fn too_big(path: &str, line: usize) -> String {
    format!(
        "read {path}: too big to seek by line past its first {} MB, and line {} is further in \u{2014} pass a byte \u{201c}offset\u{201d} instead",
        LINES_UP_TO >> 20,
        grouped(line)
    )
}

#[cfg(test)]
#[path = "sysreadlines_tests.rs"]
mod lines_tests;
