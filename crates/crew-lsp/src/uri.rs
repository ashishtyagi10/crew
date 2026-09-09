//! `file:` URIs both ways. Only the characters a path can hold are escaped;
//! a URI a server hands back is decoded the same way, so a path survives the
//! round trip byte for byte on unix and drive-letter-and-all on Windows.
use std::path::{Path, PathBuf};

fn unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/')
}

/// `file:///…` for `path`, which should be absolute.
pub fn from_path(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !s.starts_with('/') {
        // A Windows drive path: `C:/x` becomes `file:///C:/x`.
        out.push('/');
    }
    for b in s.bytes() {
        if unreserved(b) || b == b':' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

/// The path of a `file:` URI, or `None` for any other scheme.
pub fn to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // `file://localhost/x` is legal; anything else before the first `/` is a
    // host crew cannot reach.
    let rest = match rest.find('/') {
        Some(0) => rest,
        Some(i) if &rest[..i] == "localhost" => &rest[i..],
        _ => return None,
    };
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    let s = String::from_utf8(out).ok()?;
    // `/C:/x` is a Windows path with a URI's leading slash in front of it.
    let s = if cfg!(windows) && s.len() > 2 && s.as_bytes()[2] == b':' {
        s[1..].replace('/', "\\")
    } else {
        s
    };
    Some(PathBuf::from(s))
}

#[cfg(test)]
#[path = "uri_tests.rs"]
mod tests;
