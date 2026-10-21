//! A forward built and sent (`forward`): your note, Gmail's rule and the
//! forwarded mail's headers, then the mail as it came, in its HTML and its
//! plain words, its `cid:` pictures beside the HTML and its files after.
use super::api::Gmail;
use super::forward::Forwarded;
use super::mime::Node;

const RULE: &str = "---------- Forwarded message ---------";

/// The whole forward, its files' and pictures' bytes in hand (`bytes`, in
/// the order of `of.inline` then `of.files`): the tree to send.
pub(crate) fn tree(note: &str, of: &Forwarded, mut bytes: Vec<Vec<u8>>) -> Node {
    let mut plain = format!("{}\n\n{RULE}\n", note.trim_end());
    for (k, v) in of.lines() {
        plain.push_str(&format!("{k}: {v}\n"));
    }
    let words = of.source.plain.as_deref().unwrap_or(&of.text);
    plain.push_str(&format!("\n{}\n", words.trim_end()));
    let files: Vec<Vec<u8>> = bytes.split_off(of.inline.len().min(bytes.len()));
    let words = match of.source.html.as_deref() {
        None => Node::text("plain", &plain),
        Some(html) => {
            let mut h = format!(
                "<div>{}</div><br>",
                esc(note.trim_end()).replace('\n', "<br>")
            );
            h.push_str(&format!("<div class=\"gmail_quote\">{RULE}<br>"));
            for (k, v) in of.lines() {
                h.push_str(&format!("{k}: {}<br>", esc(v)));
            }
            h.push_str(&format!("<br>{}</div>", inner(html)));
            let alt = Node::multi(
                "alternative",
                vec![Node::text("plain", &plain), Node::text("html", &h)],
            );
            let mut related = vec![alt];
            for (p, b) in of.inline.iter().zip(bytes) {
                related.push(Node::inline(
                    &p.mime,
                    p.cid.as_deref().unwrap_or(""),
                    &p.name,
                    b,
                ));
            }
            Node::multi("related", related)
        }
    };
    let mut mixed = vec![words];
    for (p, b) in of.files.iter().zip(files) {
        mixed.push(Node::file(&p.mime, &p.name, b));
    }
    Node::multi("mixed", mixed)
}

/// The worker's half: each picture and file fetched, the forward built and
/// sent through Gmail's upload, which takes a mail of up to 35 MB.
pub(crate) fn send(
    g: &mut Gmail,
    head: &[String],
    note: &str,
    of: &Forwarded,
) -> Result<(), String> {
    let mut bytes = Vec::new();
    for p in of.inline.iter().chain(&of.files) {
        bytes.push(
            match (p.data.as_deref().and_then(super::body::bytes), &p.att) {
                (Some(b), _) => b,
                (None, Some(a)) => g.attachment(&of.msg, a)?,
                (None, None) => return Err(format!("{} has no bytes to send", p.name)),
            },
        );
    }
    let m = super::mime::message(head, &tree(note, of, bytes));
    g.upload_send(m.into_bytes())
}

/// The mail's own HTML, inside a `<div>`: what its `<body>` holds, after the
/// `<style>` blocks of its head, which its classes need.
fn inner(html: &str) -> String {
    let low = html.to_ascii_lowercase();
    let Some(open) = low
        .find("<body")
        .and_then(|b| low[b..].find('>').map(|e| b + e + 1))
    else {
        return html.to_string();
    };
    let close = low
        .rfind("</body")
        .filter(|&c| c >= open)
        .unwrap_or(html.len());
    let mut styles = String::new();
    let mut from = 0;
    while let Some(s) = low[from..open].find("<style").map(|s| from + s) {
        let Some(e) = low[s..open].find("</style>").map(|e| s + e + 8) else {
            break;
        };
        styles.push_str(&html[s..e]);
        from = e;
    }
    format!("{styles}{}", &html[open..close])
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[path = "forwardmail_tests.rs"]
mod tests;
