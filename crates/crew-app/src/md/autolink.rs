//! Bare `http(s)://` URL detection in already-folded spans. Mirrors
//! `openurl::url_spans`'s convention (trailing prose punctuation excluded),
//! reimplemented on `&str` so `md/` stays self-contained.
use crate::md::{MdSpan, MdStyle};

/// Characters trimmed from a URL's tail (trailing punctuation in prose).
const TRAILERS: &str = ".,);]}>\"'";

/// Character spans `[start, end)` of the http(s) URLs in `chars`.
fn url_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let tail: String = chars[i..].iter().take(8).collect();
        if tail.starts_with("http://") || tail.starts_with("https://") {
            let mut j = i;
            while j < chars.len() && !chars[j].is_whitespace() {
                j += 1;
            }
            let mut end = j;
            while end > i && TRAILERS.contains(chars[end - 1]) {
                end -= 1;
            }
            if end - i > "https://".len() {
                spans.push((i, end));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    spans
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
                text: url.clone(),
                style: span.style,
                link: Some(url),
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
    }
}
