//! DuckDuckGo's HTML answer, turned into results — the half of `sys:search`
//! that can be wrong in a way a test can catch.
//!
//! WHY a file of its own, and why no HTML parser: the answer crew asks for is
//! one shape of one page — a run of `result__a` anchors, each with a
//! `result__snippet` behind it — and the thing that will break this is not
//! malformed markup, it is DuckDuckGo renaming those two classes. A crate that
//! parsed all of HTML would not notice that a day sooner, and the tests beside
//! this file fail loudly with the fixture the day it happens.
//!
//! One thing here is not cosmetic: the URL is not the href. Every result links
//! back through `//duckduckgo.com/l/?uddg=<the real URL>`, a redirector
//! carrying a signature, and handing THAT to the model would spend a whole
//! `sys:fetch` on a hop and cite a URL no reader can check. It is unwrapped
//! before the model ever sees it.
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

/// One result: what the model cites, and what it may fetch next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hit {
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) snippet: String,
}

/// The class marking a result's title anchor.
const RESULT: &str = "class=\"result__a\"";
/// The class marking a result's snippet anchor.
const SNIPPET: &str = "class=\"result__snippet\"";

/// Where a query goes. The no-JavaScript endpoint, which answers a plain GET
/// with plain HTML — the JS one answers with a script that would have to be
/// run, and crew is not a browser.
pub(crate) fn query_url(q: &str) -> String {
    let q = utf8_percent_encode(q.trim(), NON_ALPHANUMERIC);
    format!("https://html.duckduckgo.com/html/?q={q}")
}

/// Every result in `html`, in the order the page ranked them, at most `max`.
///
/// A result missing its snippet still counts: the title and the URL are the
/// part that matters, and dropping the row would silently renumber the rest.
pub(crate) fn hits(html: &str, max: usize) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find(RESULT) {
        rest = &rest[i + RESULT.len()..];
        // A result owns the markup up to where the next one starts.
        let block = &rest[..rest.find(RESULT).unwrap_or(rest.len())];
        let Some((url, title)) = anchor(block) else {
            continue;
        };
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let snippet = block
            .find(SNIPPET)
            .and_then(|s| anchor(&block[s + SNIPPET.len()..]))
            .map(|(_, text)| text)
            .unwrap_or_default();
        out.push(Hit {
            title,
            url,
            snippet,
        });
        if out.len() == max {
            break;
        }
    }
    out
}

/// The destination and the text of the anchor `s` opens with, where `s`
/// begins just after that anchor's class attribute — `href` follows `class`
/// in this page's markup, and looking only forward keeps a result from
/// stealing the href of the one above it.
fn anchor(s: &str) -> Option<(String, String)> {
    let h = s.find("href=\"")? + "href=\"".len();
    let tail = &s[h..];
    let href = &tail[..tail.find('"')?];
    let body = &tail[tail.find('>')? + 1..];
    let text = &body[..body.find("</a>")?];
    Some((
        destination(href),
        super::sysfetchtext::readable(text).trim().into(),
    ))
}

/// The real page behind a result href.
///
/// `uddg` holds it percent-encoded, and what follows is `&amp;rut=<signature>`
/// — so the value ends at the first `&`, entity or not.
fn destination(href: &str) -> String {
    let Some(q) = href.find("uddg=") else {
        // Not a redirect: a protocol-relative href still needs a scheme, or
        // `sys:fetch` will refuse it for not being http(s).
        return match href.strip_prefix("//") {
            Some(rest) => format!("https://{rest}"),
            None => href.to_string(),
        };
    };
    let enc = &href[q + "uddg=".len()..];
    let enc = &enc[..enc.find('&').unwrap_or(enc.len())];
    percent_decode_str(enc).decode_utf8_lossy().into_owned()
}

#[cfg(test)]
#[path = "syssearchparse_tests.rs"]
mod tests;
