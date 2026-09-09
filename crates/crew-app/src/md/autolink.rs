//! Bare URL detection in already-folded spans: `http(s)://…`, a bare `www.`
//! host, and an e-mail address. Mirrors `openurl::url_spans`'s convention
//! (trailing prose punctuation excluded), reimplemented on `&str` so `md/`
//! stays self-contained.
use crate::md::{MdSpan, MdStyle};

/// Characters trimmed from a URL's tail (trailing punctuation in prose).
const TRAILERS: &str = ".,);]}>\"'";

/// Whether `c` can be part of an address's local part, or of a bare host.
/// What decides where a WORD starts: `awww.x` and `b.a@c.io` are one word
/// each, and neither has an address in the middle of it.
fn is_addr(c: char) -> bool {
    c.is_alphanumeric() || "._%+-".contains(c)
}

/// Where the whitespace-delimited word at `i` ends, less trailing prose
/// punctuation — `(see https://x.io).` links `https://x.io`.
fn word_end(chars: &[char], i: usize) -> usize {
    let mut j = i;
    while j < chars.len() && !chars[j].is_whitespace() {
        j += 1;
    }
    let mut end = j;
    while end > i && TRAILERS.contains(chars[end - 1]) {
        end -= 1;
    }
    end
}

/// `[i, end)` of the e-mail address starting at `i`, if one does: a local
/// part, `@`, and a dotted host whose last label is letters. `a@b` alone is
/// not one — a bare `@handle` with a slash-word after it must stay prose.
fn email_end(chars: &[char], i: usize) -> Option<usize> {
    let at = i + chars[i..].iter().take_while(|c| is_addr(**c)).count();
    if at == i || chars.get(at) != Some(&'@') {
        return None;
    }
    let end = word_end(chars, at + 1);
    let host: String = chars[at + 1..end].iter().collect();
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_alphanumeric() || "-.".contains(c))
    {
        return None;
    }
    let tld = host.rsplit('.').next()?;
    let dotted = host.contains('.') && tld.len() >= 2 && tld.chars().all(char::is_alphabetic);
    dotted.then_some(end)
}

/// Character spans `[start, end)` of the URLs in `chars`.
fn url_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let head: String = chars[i..].iter().take(8).collect();
        let at_word = i == 0 || !is_addr(chars[i - 1]);
        let end = if head.starts_with("http://") || head.starts_with("https://") {
            Some(word_end(chars, i)).filter(|&e| e - i > "https://".len())
        } else if at_word && head.starts_with("www.") {
            Some(word_end(chars, i)).filter(|&e| e - i > "www.".len())
        } else if at_word {
            email_end(chars, i)
        } else {
            None
        };
        match end {
            Some(e) => {
                spans.push((i, e));
                i = e.max(i + 1);
            }
            None => i += 1,
        }
    }
    spans
}

/// The URL a bare match opens: as written when it names a scheme, `https://`
/// in front of a bare host, `mailto:` in front of an address. The TEXT stays
/// as the writer typed it; only the cell's link carries the scheme.
fn href(text: &str) -> String {
    if text.starts_with("www.") {
        format!("https://{text}")
    } else if !text.contains("://") && text.contains('@') {
        format!("mailto:{text}")
    } else {
        text.to_string()
    }
}

/// Splits bare URLs out of `spans` into their own linked spans. Spans that
/// already link somewhere, or hold code text, are left untouched.
pub(super) fn autolink(spans: Vec<MdSpan>) -> Vec<MdSpan> {
    let mut out = Vec::with_capacity(spans.len());
    for span in spans {
        if span.link.is_some() || span.style.code {
            out.push(span);
            continue;
        }
        let chars: Vec<char> = span.text.chars().collect();
        let urls = url_spans(&chars);
        if urls.is_empty() {
            out.push(span);
            continue;
        }
        let mut cursor = 0;
        for (a, b) in urls {
            if a > cursor {
                out.push(plain(&chars[cursor..a], span.style));
            }
            let url: String = chars[a..b].iter().collect();
            out.push(MdSpan {
                link: Some(href(&url)),
                text: url,
                style: span.style,
                src: None,
            });
            cursor = b;
        }
        if cursor < chars.len() {
            out.push(plain(&chars[cursor..], span.style));
        }
    }
    out
}

fn plain(chars: &[char], style: MdStyle) -> MdSpan {
    MdSpan {
        text: chars.iter().collect(),
        style,
        link: None,
        src: None,
    }
}

#[cfg(test)]
#[path = "autolink_tests.rs"]
mod tests;
