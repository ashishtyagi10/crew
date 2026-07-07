//! Chunked, UTF-8-safe file reads for `sys:read_file`. Split out of
//! `systools` to keep that file under the repo's line cap; these helpers are
//! `pub(super)` so `systools::call` can dispatch to them.
use std::io::{Read, Seek, SeekFrom};

use super::systools::CAP;

/// The optional `"offset"` byte argument: a JSON number, or a numeric string
/// (some agents quote it). Defaults to 0 when absent/null; anything else
/// (bool, array, object, non-digit string, negative/fractional number) is an
/// agent-readable error rather than a silent re-read from 0.
pub(super) fn offset_arg(v: &serde_json::Value) -> Result<usize, String> {
    match v.get("offset") {
        None | Some(serde_json::Value::Null) => Ok(0),
        Some(serde_json::Value::Number(n)) => {
            n.as_u64().map(|n| n as usize).ok_or_else(invalid_offset)
        }
        Some(serde_json::Value::String(s))
            if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) =>
        {
            s.parse::<usize>().map_err(|_| invalid_offset())
        }
        _ => Err(invalid_offset()),
    }
}

fn invalid_offset() -> String {
    "invalid \u{201c}offset\u{201d}: expected a byte position".to_string()
}

/// True if `idx` doesn't split a UTF-8 codepoint in `bytes` (mirrors
/// `str::is_char_boundary` without requiring a validated `&str` up front).
fn is_utf8_boundary(bytes: &[u8], idx: usize) -> bool {
    match bytes.get(idx) {
        None => idx == bytes.len(),
        Some(&b) => (b as i8) >= -0x40,
    }
}

pub(super) fn read_file(path: &str, offset: usize) -> Result<String, String> {
    // Bound the I/O itself: at most CAP+1 bytes, so a huge/never-EOF file can't blow up memory or hang.
    let mut f = std::fs::File::open(path).map_err(|e| format!("read {path}: {e}"))?;
    let total = f.metadata().map_err(|e| format!("read {path}: {e}"))?.len() as usize;
    if offset > 0 {
        f.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| format!("read {path}: {e}"))?;
    }
    let mut buf = Vec::new();
    f.take(CAP as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("read {path}: {e}"))?;
    if buf.is_empty() && offset > 0 {
        return Ok(format!(
            "\u{2026} (offset {offset} is at or past the end \u{2014} file is {total} bytes)"
        ));
    }
    let hit_cap = buf.len() > CAP; // pre-trim: more file remains beyond this read
    let start = (0..=3.min(buf.len())) // offset may land mid-codepoint; skip <=3B to a boundary
        .find(|&i| is_utf8_boundary(&buf, i))
        .unwrap_or(0);
    let buf = &buf[start..];
    if hit_cap {
        let end = buf.len().min(CAP);
        let floor = end.saturating_sub(3); // boundary within 3 bytes; binary may lack one
        let cut = (floor..=end).rev().find(|&i| is_utf8_boundary(buf, i));
        let cut = cut.ok_or_else(|| {
            format!("read {path}: not valid UTF-8: no character boundary near the 64 KB cap")
        })?;
        let text = std::str::from_utf8(&buf[..cut])
            .map_err(|e| format!("read {path}: not valid UTF-8: {e}"))?;
        return Ok(format!(
            "{text}\n\u{2026} (truncated at 64 KB \u{2014} file is {total} bytes; \
             continue with {{\"offset\": {}}})",
            offset + start + cut
        ));
    }
    std::str::from_utf8(buf)
        .map(str::to_owned)
        .map_err(|e| format!("read {path}: not valid UTF-8: {e}"))
}
