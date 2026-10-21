//! The type a mail's `<style>` sets on a class (goal
//! 2026-10-21-mail-looks-right, item 8): MJML and its kind write a headline
//! as `<span class="h1">` and put its 38px in a stylesheet. Only the plainest
//! rules count: a class selector (`.h1`, `span.h1`), outside any `@media`,
//! and only its type: size, weight, style. Nothing here hides anything, since
//! a stylesheet's `display:none` is as often a mobile-only block.
use std::collections::HashMap;

/// The declarations type takes from a rule.
const TYPE: &[&str] = &["font-size", "font-weight", "font-style"];

/// Class name to the type declarations every `<style>` in `html` sets on it,
/// in order (so a later rule wins, as in a browser).
pub(crate) fn classes(html: &str) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(open) = lower[from..].find("<style").map(|i| from + i) {
        let Some(body) = lower[open..].find('>').map(|i| open + i + 1) else {
            break;
        };
        let end = lower[body..]
            .find("</style")
            .map_or(html.len(), |i| body + i);
        rules(&html[body..end], &mut out);
        from = end.max(body);
    }
    out
}

/// `sheet`'s top-level rules into `out`; `@media` and other at-blocks skipped.
fn rules(sheet: &str, out: &mut HashMap<String, String>) {
    let sheet = strip_comments(sheet);
    let mut rest = sheet.as_str();
    while let Some(open) = rest.find('{') {
        let head = rest[..open].trim();
        let Some(close) = block_end(rest, open) else {
            return;
        };
        if !head.starts_with('@') {
            let decls: Vec<&str> = rest[open + 1..close]
                .split(';')
                .filter(|d| {
                    d.split_once(':').is_some_and(|(k, _)| {
                        TYPE.contains(&k.trim().to_ascii_lowercase().as_str())
                    })
                })
                .collect();
            for class in head.split(',').filter_map(class_of) {
                for d in &decls {
                    let held = out.entry(class.to_string()).or_default();
                    held.push_str(d.trim());
                    held.push(';');
                }
            }
        }
        rest = &rest[close + 1..];
    }
}

/// The `}` closing the block opened at `open`, nested blocks included.
fn block_end(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' if depth == 1 => return Some(open + i),
            '}' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The class a selector names, when it names only one: `.h1`, `span.h1`.
fn class_of(sel: &str) -> Option<&str> {
    let sel = sel.trim();
    let (tag, class) = sel.split_once('.')?;
    let plain = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    };
    (tag.chars().all(|c| c.is_ascii_alphanumeric()) && plain(class)).then_some(class)
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i + 2..].split_once("*/").map_or("", |(_, r)| r);
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_class_takes_its_type_and_nothing_inside_media_or_past_a_compound_selector() {
        let html = r#"<head><style>
            /* a comment { .h1 { font-size: 99px } } */
            .h1 { font-family: Georgia; font-size: 38px; font-weight: 700; color: #000 }
            span.kicker, .eyebrow { font-size: 11px; text-transform: uppercase }
            @media (max-width: 480px) { .h1 { font-size: 24px !important } .hide { display: none } }
            .card .title { font-size: 30px }
            a:hover { font-weight: 900 }
        </style><STYLE>.h1 { font-size: 40px }</STYLE></head>"#;
        let got = super::classes(html);
        assert_eq!(
            got["h1"], "font-size: 38px;font-weight: 700;font-size: 40px;",
            "a later rule wins"
        );
        assert_eq!(got["kicker"], "font-size: 11px;");
        assert_eq!(got["eyebrow"], "font-size: 11px;");
        assert!(
            !got.contains_key("hide") && !got.contains_key("title"),
            "{got:?}"
        );
        assert!(super::classes("<p>no sheet</p>").is_empty());
    }
}
