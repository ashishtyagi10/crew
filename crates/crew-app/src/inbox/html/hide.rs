//! What a mail draws but nobody sees, dropped before anything reads it.
//!
//! The preheader is the common case: a `display:none` line of text the inbox
//! list shows as the snippet, then padded with invisible joiners. It is also
//! where a prompt injection would hide — text agent smith reads that the
//! user never saw (goal 2026-10-20-mail-reader, item 9).
use super::tree::Node;

/// Elements that are never content.
const NEVER: &[&str] = &[
    "head", "style", "script", "title", "template", "noscript", "svg", "object", "iframe", "meta",
    "link", "base", "button", "select", "input", "textarea", "map", "o:p",
];

/// Whether `n` and everything under it is invisible.
pub(crate) fn hidden(n: &Node) -> bool {
    if NEVER.contains(&n.tag.as_str()) || n.attr("hidden").is_some() {
        return true;
    }
    if n.attr("class").is_some_and(|c| {
        let c = c.to_ascii_lowercase();
        c.contains("preheader") || c.contains("preview-text")
    }) {
        return true;
    }
    n.attr("style").is_some_and(invisible_style)
}

/// `display:none`, `visibility:hidden`, `mso-hide:all`, `opacity:0`, a
/// zero `max-height`, or a font too small to read.
pub(crate) fn invisible_style(style: &str) -> bool {
    style.split(';').any(|decl| {
        let Some((k, v)) = decl.split_once(':') else {
            return false;
        };
        let k = k.trim().to_ascii_lowercase();
        let v: String = v
            .to_ascii_lowercase()
            .replace("!important", "")
            .split_whitespace()
            .collect();
        match k.as_str() {
            "display" => v == "none",
            "visibility" => v == "hidden",
            "mso-hide" => v == "all",
            "opacity" => zero(&v),
            "max-height" | "font-size" => zero(&v) || v == "1px",
            _ => false,
        }
    })
}

fn zero(v: &str) -> bool {
    v.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%')
        .parse::<f32>()
        .is_ok_and(|n| n == 0.0)
}

/// An image nobody is meant to see: a tracking pixel or a spacer.
pub(crate) fn invisible_image(n: &Node) -> bool {
    let tiny = |k: &str| {
        n.attr(k)
            .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
            .is_some_and(|px| px <= 2.0)
    };
    let src = n.attr("src").unwrap_or("").to_ascii_lowercase();
    tiny("width") || tiny("height") || src.contains("spacer.gif") || src.contains("pixel.gif")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(tag: &str, attrs: &[(&str, &str)]) -> Node {
        Node {
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            kids: Vec::new(),
        }
    }

    #[test]
    fn preheaders_and_hidden_styles_are_hidden_and_plain_blocks_are_not() {
        let style = |s: &str| node("div", &[("style", s)]);
        assert!(hidden(&style("display: none !important; max-height:0")));
        assert!(hidden(&style("font-size:0px;color:#fff")));
        assert!(hidden(&style("mso-hide:all")));
        assert!(hidden(&node("span", &[("class", "Preheader x")])));
        assert!(hidden(&node("style", &[])));
        assert!(!hidden(&style(
            "display:block;font-size:14px;max-height:200px"
        )));
        assert!(!hidden(&node("p", &[])));
    }

    #[test]
    fn tracking_pixels_and_spacers_are_invisible_images() {
        assert!(invisible_image(&node(
            "img",
            &[("width", "1"), ("height", "1")]
        )));
        assert!(invisible_image(&node(
            "img",
            &[("src", "https://x.io/spacer.gif")]
        )));
        assert!(!invisible_image(&node(
            "img",
            &[("width", "600"), ("src", "https://x.io/a.png")]
        )));
    }
}
