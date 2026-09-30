//! Paged, UTF-8-safe file reads for `sys:read_file`. Split out of
//! `systools` to keep that file under the repo's line cap; these helpers are
//! `pub(super)` so `systools::call` can dispatch to them. A page's rows carry
//! the file's line numbers (`sysreadrows`).
use std::io::{Read, Seek, SeekFrom};

#[path = "sysreadrows.rs"]
mod rows;

#[path = "sysreadnote.rs"]
mod note;
pub(super) use note::grouped;
use note::note;

#[cfg(test)]
#[path = "sysread_tests.rs"]
mod tests;

/// Bytes one `sys:read_file` page takes, numbered rows and all; its note
/// comes after.
///
/// Sized to what the agent is SHOWN, not to what the tool can read. Both
/// engines clip a tool result before the next prompt: the relay to
/// [`super::toolclip::AGENT_CLIP`] chars (head plus last line), the swarm to
/// crew-hive's `RESULT_CAP` (head only) — 6,000 each. A 64 KB read came back
/// as its first ~5.9 K chars and a note saying to continue at byte 65,536, so
/// the ~58 KB between was never seen by anyone. A byte is at most a char, so
/// 5,600 bytes leaves 400 chars for the trailing note, which is under 200 even
/// with 20-digit offsets; `sysread_tests` holds page + note under the clip.
/// The numbers are paid for out of the page, not the note's 400: a page of
/// short lines shows fewer bytes of the file, never a longer result.
pub(super) const PAGE: usize = 5_600;

/// Files up to this size get line numbers, on the rows and in the page note.
/// Both need the whole file read on every page: 8 MB is a few milliseconds
/// from cache, while paging a 2 GB log would read 2 GB per call, so past it
/// the rows go bare and the note gives bytes only.
pub(super) const LINES_UP_TO: usize = 8 * 1024 * 1024;

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

/// One page from byte `offset`: at most [`PAGE`] bytes of numbered rows, and
/// at most `most` lines when the agent passed `"lines"`.
pub(super) fn read_file(path: &str, offset: usize, most: Option<usize>) -> Result<String, String> {
    let err = |e: std::io::Error| format!("read {path}: {e}");
    let mut f =
        std::fs::File::open(path).map_err(|e| super::syspath::with_hint("read", path, e))?;
    let total = f.metadata().map_err(err)?.len() as usize;
    if offset > 0 {
        f.seek(SeekFrom::Start(offset as u64)).map_err(err)?;
    }
    // Bound the I/O itself — one page, 3 bytes for a mid-codepoint skip and 1
    // to know more remains — so a huge/never-EOF file can't blow up memory or hang.
    let mut buf = Vec::new();
    Read::by_ref(&mut f)
        .take(PAGE as u64 + 4)
        .read_to_end(&mut buf)
        .map_err(err)?;
    if buf.is_empty() && offset > 0 {
        return Ok(format!(
            "\u{2026} (offset {offset} is at or past the end \u{2014} file is {total} bytes)"
        ));
    }
    // Offset may land mid-codepoint; skip <=3B to a boundary. Only when we
    // actually seeked (offset > 0) — at offset 0 a bad leading byte means the
    // file isn't UTF-8 and must fail validation below, not be silently eaten.
    let start = if offset > 0 {
        (0..=3.min(buf.len()))
            .find(|&i| is_utf8_boundary(&buf, i))
            .unwrap_or(0)
    } else {
        0
    };
    let body = &buf[start..];
    let utf8 = |b| std::str::from_utf8(b).map_err(|e| format!("read {path}: not valid UTF-8: {e}"));
    // The page before numbering: all the rest, or PAGE bytes ending on a
    // whole line, the way the agent will quote and edit it. Only a line
    // longer than the page is cut inside, at a character boundary within 3
    // bytes of the budget (binary may lack one). It is what is checked for
    // UTF-8, so a binary file is refused as it always was.
    let last = body.len() <= PAGE;
    let cut = if last {
        body.len()
    } else {
        match body[..PAGE].iter().rposition(|&b| b == b'\n') {
            Some(nl) => nl + 1,
            None => (PAGE - 3..=PAGE)
                .rev()
                .find(|&i| is_utf8_boundary(body, i))
                .ok_or_else(|| {
                    format!("read {path}: not valid UTF-8: no character boundary near the page end")
                })?,
        }
    };
    let text = utf8(&body[..cut])?;
    // `"lines"` ends the page early, and then it is not the file's last.
    let (text, last) = match most.map(|n| rows::head(text, n)) {
        Some(head) if head.len() < text.len() => (head, false),
        _ => (text, last),
    };
    let from = offset + start;
    // Numbered from the file's own count of the lines before the page; past
    // LINES_UP_TO there is none, and a bare row beats one with a guessed number.
    let Some((before, of)) = count_lines(&mut f, total, from) else {
        if last {
            return Ok(text.to_owned());
        }
        return Ok(format!(
            "{text}\n{}",
            note(None, from, from + text.len(), total)
        ));
    };
    let first = before + 1;
    let shown = &text[..rows::fit(text, first, PAGE)];
    let page = rows::number(shown, first);
    if last && shown.len() == text.len() {
        return Ok(page);
    }
    let end = rows::last_line(shown, first);
    let lines = Some((first, end, of, !shown.ends_with('\n')));
    let to = from + shown.len();
    Ok(format!("{page}\n{}", note(lines, from, to, total)))
}

/// Newlines before byte `from`, and the file's line count, or None when the
/// file is past [`LINES_UP_TO`] (checked on the bytes read, not just the
/// metadata, so a never-EOF file still stops).
fn count_lines(f: &mut std::fs::File, total: usize, from: usize) -> Option<(usize, usize)> {
    if total > LINES_UP_TO {
        return None;
    }
    f.seek(SeekFrom::Start(0)).ok()?;
    let mut all = Vec::new();
    f.take(LINES_UP_TO as u64 + 1).read_to_end(&mut all).ok()?;
    if all.len() > LINES_UP_TO {
        return None;
    }
    let newlines = |s: &[u8]| s.iter().filter(|&&b| b == b'\n').count();
    let unterminated = all.last().is_some_and(|&b| b != b'\n');
    Some((
        newlines(&all[..from.min(all.len())]),
        newlines(&all) + usize::from(unterminated),
    ))
}
