//! An `<img>`: a picture block, or its words.
use super::out::Out;
use super::tree::Node;
use super::{ent, hide, link};

/// A picture the mail carries (`cid:`) is drawn; one from the web is drawn
/// only when asked (`o.web`), and is its words until then — as is anything
/// else (`data:`, a relative path).
pub(crate) fn image(n: &Node, o: &mut Out) {
    if hide::invisible_image(n) {
        return;
    }
    let alt = ent::decode(n.attr("alt").unwrap_or("")).trim().to_string();
    let src = n
        .attr("src")
        .map(|s| ent::decode(s.trim()))
        .unwrap_or_default();
    let lower = src.to_ascii_lowercase();
    let web = lower.starts_with("http") || lower.starts_with("//");
    let src = match () {
        _ if lower.starts_with("cid:") => Some(src),
        _ if web && o.web => link::target(&src),
        _ => None,
    };
    let Some(src) = src.filter(|s| s.len() < 2048) else {
        o.web_left += usize::from(web);
        if !alt.is_empty() {
            o.text(&format!(" \u{25a3} {alt} "));
        }
        return;
    };
    o.block();
    o.raw(&format!(
        "![{}](<{src}#rows={}>)",
        super::esc(&alt),
        rows(n)
    ));
    o.block();
    o.pics.push(src);
}

/// Rows for a picture, from the height the mail gives it (about 20 px a
/// row); the engine's own box when it gives none.
fn rows(n: &Node) -> u16 {
    let px = |k: &str| {
        n.attr(k)
            .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
    };
    match px("height") {
        Some(h) => (h / 20.0).round().clamp(2.0, 20.0) as u16,
        None => crate::md::picture::ROWS,
    }
}

/// The first picture's words inside `n`: what a link made of a picture says.
pub(crate) fn alt_in(n: &Node) -> Option<String> {
    n.kids.iter().find_map(|k| match k {
        super::tree::Kid::El(e) if e.tag == "img" => {
            Some(ent::decode(e.attr("alt").unwrap_or("")).trim().to_string())
                .filter(|a| !a.is_empty())
        }
        super::tree::Kid::El(e) => alt_in(e),
        super::tree::Kid::Text(_) => None,
    })
}
