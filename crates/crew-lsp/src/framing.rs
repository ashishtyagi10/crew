//! The LSP base protocol: `Content-Length: N\r\n\r\n` and then N bytes of
//! JSON. Pure functions over readers and byte buffers, so the codec is
//! tested without a process on the other end.
use std::io::BufRead;

use serde_json::Value;

/// One message, framed for the wire.
pub fn encode(msg: &Value) -> Vec<u8> {
    let body = msg.to_string();
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// Read one framed message. `Ok(None)` is a clean end of stream — the server
/// closed its pipe between messages; an EOF inside a frame is an error.
///
/// Header names are matched case-insensitively and `Content-Type` (or any
/// other header) is ignored, as the spec asks.
pub fn decode<R: BufRead>(r: &mut R) -> Result<Option<Value>, String> {
    let mut len: Option<usize> = None;
    let mut headers = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        let n = r
            .read_line(&mut line)
            .map_err(|e| format!("lsp read: {e}"))?;
        if n == 0 {
            return match len {
                None => Ok(None),
                Some(_) => Err("lsp: stream ended inside a header".into()),
            };
        }
        let t = line.trim_end_matches(['\r', '\n']);
        if t.is_empty() {
            // The blank line that ends the headers — but only once there
            // were any; a stray newline between frames is skipped.
            match (len, headers) {
                (Some(_), _) => break,
                (None, 0) => continue,
                (None, _) => return Err("lsp: headers without a Content-Length".into()),
            }
        }
        headers += 1;
        if let Some((name, value)) = t.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                let v = value.trim();
                len = Some(
                    v.parse()
                        .map_err(|_| format!("lsp: bad Content-Length {v:?}"))?,
                );
            }
        }
    }
    let Some(len) = len else {
        return Err("lsp: no Content-Length".into());
    };
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)
        .map_err(|e| format!("lsp read body: {e}"))?;
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| format!("lsp: body is not JSON: {e}"))
}

#[cfg(test)]
#[path = "framing_tests.rs"]
mod tests;
