//! HTML in, something worth reading out.
//!
//! Not a parser and not trying to be: `script` and `style` bodies are dropped
//! whole, tags become spaces, entities that actually appear in prose are
//! decoded, and runs of whitespace collapse. A model reading a documentation
//! page wants the sentences; everything this throws away is markup it would
//! have paid tokens to skim.
//!
//! Kept separate from the fetching so it can be tested without a network:
//! every rule here is a string in and a string out.

/// Elements whose CONTENT is not text: dropped body and all.
const DROPPED: &[&str] = &["script", "style", "noscript", "svg", "head"];

/// The readable text of an HTML document.
pub(crate) fn readable(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut rest = html;
    'outer: while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..lt]);
        rest = &rest[lt..];
        let lower = rest.to_ascii_lowercase();
        for tag in DROPPED {
            if lower.starts_with(&format!("<{tag}")) {
                let close = format!("</{tag}");
                match lower.find(&close) {
                    Some(end) => {
                        rest = &rest[end..];
                        continue 'outer;
                    }
                    None => break 'outer,
                }
            }
        }
        // A block tag ends a line; an inline one is just a space.
        let breaks = [
            "<p", "<br", "<div", "<li", "<tr", "<h1", "<h2", "<h3", "<h4",
        ];
        if breaks.iter().any(|b| lower.starts_with(b)) {
            out.push('\n');
        } else {
            out.push(' ');
        }
        match rest.find('>') {
            Some(gt) => rest = &rest[gt + 1..],
            None => break,
        }
    }
    tidy(&entities(&out))
}

/// The handful of entities that appear in prose. Anything else is left as it
/// is: a literal `&copy;` in the text is odd, and a wrong guess is worse.
///
/// `&#x27;` earns its place by being the ONLY numeric form seen in the wild
/// here — a search result titled `You&#x27;ll Understand Lifetimes` is a title
/// the model reads aloud and cites, so the apostrophe is worth spelling three
/// ways.
fn entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
}

/// Collapse the whitespace a stripped document is mostly made of: spaces
/// within a line, and never more than one blank line between them.
fn tidy(s: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut blank = false;
    for line in s.lines() {
        let joined = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if joined.is_empty() {
            blank = true;
            continue;
        }
        if blank && !lines.is_empty() {
            lines.push(String::new());
        }
        blank = false;
        lines.push(joined);
    }
    lines.join("\n")
}

/// The head of `s` under `max` chars, with a marker saying what was cut —
/// the same contract every other clipped block in the broker keeps.
pub(crate) fn capped(s: &str, max: usize) -> String {
    let total = s.chars().count();
    if total <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!(
        "{head}\n\u{2026} [clipped {} chars of {total}]",
        total - max
    )
}

#[cfg(test)]
#[path = "sysfetchtext_tests.rs"]
mod tests;
