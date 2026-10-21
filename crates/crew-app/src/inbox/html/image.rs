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

/// Rows for a picture, from the height the mail gives it in an attribute or
/// its `style` (16 px a row, a cell being about 8 px by 16 of a mail's px).
/// With none, the engine's own box and `&fit=W`, the width the mail gives
/// (0 for none): the reader sizes it from the picture once it lands
/// (`readfit`).
fn rows(n: &Node) -> String {
    let px = |k: &str| {
        let style = n.attr("style").and_then(|s| super::font::decl(s, k));
        [n.attr(k).map(str::to_string), style]
            .into_iter()
            .flatten()
            .find_map(|v| {
                v.trim()
                    .strip_suffix("px")
                    .unwrap_or(v.trim())
                    .parse::<f32>()
                    .ok()
            })
            .filter(|&v| v > 0.0)
    };
    match px("height") {
        Some(h) => format!("{}", (h / 16.0).round().clamp(2.0, 40.0) as u16),
        None => format!(
            "{}&fit={}",
            crate::md::picture::ROWS,
            px("width").unwrap_or(0.0) as u32
        ),
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
