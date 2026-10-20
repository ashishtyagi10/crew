//! Mail HTML as a flat run of tags and text. Lenient by design: mail is
//! written by a hundred mailers and a browser renders all of it, so nothing
//! here fails — a `<` that starts no tag is text, an unclosed quote runs to
//! the end, and comments (Outlook's `<!--[if mso]>` blocks among them) and
//! declarations are skipped whole.

#[derive(Debug, PartialEq)]
pub(crate) enum Tok<'a> {
    Open {
        name: String,
        attrs: Vec<(String, String)>,
    },
    Close(String),
    /// Raw text: entities are decoded by whoever reads it.
    Text(&'a str),
}

/// Elements whose content is not markup: skipped to their end tag here, so a
/// `<` inside a stylesheet is never read as a tag.
const RAW: &[&str] = &["script", "style", "title", "textarea", "xmp"];

pub(crate) fn tokens(html: &str) -> Vec<Tok<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    let b = html.as_bytes();
    while i < html.len() {
        let Some(lt) = html[i..].find('<').map(|n| n + i) else {
            out.push(Tok::Text(&html[i..]));
            break;
        };
        if lt > i {
            out.push(Tok::Text(&html[i..lt]));
        }
        let rest = &html[lt..];
        i = if rest.starts_with("<!--") {
            rest[4..].find("-->").map_or(html.len(), |e| lt + 4 + e + 3)
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            rest.find('>').map_or(html.len(), |e| lt + e + 1)
        } else if rest.starts_with("</") && b.get(lt + 2).is_some_and(u8::is_ascii_alphabetic) {
            let end = rest.find('>').map_or(html.len(), |e| lt + e + 1);
            let name = tag_name(&html[lt + 2..end]);
            out.push(Tok::Close(name));
            end
        } else if b.get(lt + 1).is_some_and(u8::is_ascii_alphabetic) {
            let (name, attrs, end) = open_tag(html, lt + 1);
            let raw = RAW.contains(&name.as_str());
            out.push(Tok::Open {
                name: name.clone(),
                attrs,
            });
            match raw {
                true => skip_raw(html, end, &name, &mut out),
                false => end,
            }
        } else {
            out.push(Tok::Text("<"));
            lt + 1
        };
    }
    out
}

fn tag_name(s: &str) -> String {
    s.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '-')
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Past a raw element's content and its end tag, which is emitted.
fn skip_raw(html: &str, from: usize, name: &str, out: &mut Vec<Tok<'_>>) -> usize {
    let close = format!("</{name}");
    let at = html[from..]
        .char_indices()
        .find(|&(n, _)| {
            html[from + n..]
                .get(..close.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(&close))
        })
        .map(|(n, _)| from + n);
    out.push(Tok::Close(name.to_string()));
    match at {
        Some(a) => html[a..].find('>').map_or(html.len(), |e| a + e + 1),
        None => html.len(),
    }
}

/// The tag starting at `from` (just past `<`): its name, its attributes and
/// where it ends.
fn open_tag(html: &str, from: usize) -> (String, Vec<(String, String)>, usize) {
    let b = html.as_bytes();
    let name = tag_name(&html[from..]);
    let mut i = from + name.len();
    let mut attrs = Vec::new();
    loop {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        if i >= b.len() || b[i] == b'>' {
            return (name, attrs, (i + 1).min(html.len()));
        }
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'>' | b'/') {
            i += 1;
        }
        let key = html[start..i].to_ascii_lowercase();
        let mut value = String::new();
        if b.get(i) == Some(&b'=') {
            i += 1;
            match b.get(i) {
                Some(&q @ (b'"' | b'\'')) => {
                    let end = html[i + 1..]
                        .find(q as char)
                        .map_or(html.len(), |e| i + 1 + e);
                    value = html[i + 1..end].to_string();
                    i = (end + 1).min(html.len());
                }
                _ => {
                    let s = i;
                    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                        i += 1;
                    }
                    value = html[s..i].to_string();
                }
            }
        }
        if !key.is_empty() {
            attrs.push((key, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{tokens, Tok};

    #[test]
    fn tags_attributes_text_and_what_is_skipped() {
        let t = tokens(
            "<!DOCTYPE html><!--[if mso]><p>x</p><![endif]--><A HREF='a b' data-x=1 hidden>hi</a>\
             <style>p{color:red} a<b</style>1 < 2",
        );
        assert_eq!(
            t,
            [
                Tok::Open {
                    name: "a".into(),
                    attrs: vec![
                        ("href".into(), "a b".into()),
                        ("data-x".into(), "1".into()),
                        ("hidden".into(), String::new())
                    ]
                },
                Tok::Text("hi"),
                Tok::Close("a".into()),
                Tok::Open {
                    name: "style".into(),
                    attrs: vec![]
                },
                Tok::Close("style".into()),
                Tok::Text("1 "),
                Tok::Text("<"),
                Tok::Text(" 2"),
            ]
        );
    }
}
