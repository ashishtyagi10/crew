//! A mail built of parts (RFC 2045–2046): the words, the pictures the HTML
//! names by `cid:`, and the files, nested as `multipart/mixed`, `related`
//! and `alternative` say. Every part's bytes go base64 in lines of 76, so a
//! long paragraph or an HTML mail's 5,000-column line never breaks the
//! 998-byte line limit and no byte is read as anything but the part's own.
use base64::Engine;

pub(crate) enum Node {
    /// One part: its `Content-Type`, any other header lines, its bytes.
    Leaf {
        kind: String,
        extra: Vec<String>,
        bytes: Vec<u8>,
    },
    /// Parts in a `multipart/<kind>`.
    Multi {
        kind: &'static str,
        parts: Vec<Node>,
    },
}

impl Node {
    /// Words, as `text/<sub>` in UTF-8 with CRLF line ends.
    pub(crate) fn text(sub: &str, words: &str) -> Self {
        let crlf = words.replace("\r\n", "\n").replace('\n', "\r\n");
        Self::Leaf {
            kind: format!("text/{sub}; charset=UTF-8"),
            extra: vec![],
            bytes: crlf.into_bytes(),
        }
    }

    /// A file sent with the mail, under the name it had.
    pub(crate) fn file(mime: &str, name: &str, bytes: Vec<u8>) -> Self {
        let n = param(name);
        Self::Leaf {
            kind: format!("{}; name={n}", or_octets(mime)),
            extra: vec![format!("Content-Disposition: attachment; filename={n}")],
            bytes,
        }
    }

    /// A picture the HTML draws by `cid:` (the id without its `<>`).
    pub(crate) fn inline(mime: &str, cid: &str, name: &str, bytes: Vec<u8>) -> Self {
        let cid = cid.replace(['\r', '\n', '<', '>'], "");
        let mut extra = vec![format!("Content-ID: <{cid}>")];
        extra.push(match name.is_empty() {
            true => "Content-Disposition: inline".into(),
            false => format!("Content-Disposition: inline; filename={}", param(name)),
        });
        Self::Leaf {
            kind: or_octets(mime).to_string(),
            extra,
            bytes,
        }
    }

    /// `parts` together, or the one part alone: a multipart of one is noise.
    pub(crate) fn multi(kind: &'static str, mut parts: Vec<Node>) -> Self {
        match parts.len() {
            1 => parts.remove(0),
            _ => Self::Multi { kind, parts },
        }
    }
}

/// The message: `head` (To, Subject, … each one line), then the tree.
pub(crate) fn message(head: &[String], root: &Node) -> String {
    let mut out = String::new();
    for h in head {
        out.push_str(&h.replace(['\r', '\n'], " "));
        out.push_str("\r\n");
    }
    out.push_str("MIME-Version: 1.0\r\n");
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    write(root, &mut out, &mut 0, seed);
    out
}

/// A part's headers and body. A boundary holds `=_`, which base64 never
/// does, so no part's bytes can end it early.
fn write(n: &Node, out: &mut String, seq: &mut u32, seed: u64) {
    match n {
        Node::Leaf { kind, extra, bytes } => {
            out.push_str(&format!("Content-Type: {kind}\r\n"));
            for e in extra {
                out.push_str(&format!("{}\r\n", e.replace(['\r', '\n'], " ")));
            }
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            let b = base64::engine::general_purpose::STANDARD.encode(bytes);
            for line in b.as_bytes().chunks(76) {
                out.push_str(std::str::from_utf8(line).unwrap_or_default());
                out.push_str("\r\n");
            }
        }
        Node::Multi { kind, parts } => {
            *seq += 1;
            let b = format!("=_crew_{seq}_{seed:x}");
            out.push_str(&format!(
                "Content-Type: multipart/{kind}; boundary=\"{b}\"\r\n\r\n"
            ));
            for p in parts {
                out.push_str(&format!("--{b}\r\n"));
                write(p, out, seq, seed);
            }
            out.push_str(&format!("--{b}--\r\n"));
        }
    }
}

/// A file's name as a header parameter: quoted, an RFC 2047 word when it
/// is not ASCII, as Gmail writes them.
fn param(name: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| !c.is_control() && *c != '"' && *c != '\\')
        .collect();
    match clean.is_ascii() {
        true => format!("\"{clean}\""),
        false => {
            let b = base64::engine::general_purpose::STANDARD.encode(&clean);
            format!("\"=?UTF-8?B?{b}?=\"")
        }
    }
}

fn or_octets(mime: &str) -> &str {
    match mime.contains('/') && !mime.contains(['\r', '\n', ';']) {
        true => mime,
        false => "application/octet-stream",
    }
}

#[cfg(test)]
#[path = "mime_tests.rs"]
mod tests;
