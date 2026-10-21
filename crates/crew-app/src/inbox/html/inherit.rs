//! What an element hands down to what it holds, as a browser's cascade
//! does: its type size (zero hides only the words that set none of their
//! own), whether it is a button, its letter case, and whether it centres.
//! Taken on the way in and given back on the way out (`walk::node`).
use super::out::Out;
use super::tree::Node;

#[derive(Clone, Copy)]
pub(crate) struct Kept {
    tiny: bool,
    btn: bool,
    size: f32,
    case: super::case::Case,
    center: bool,
}

pub(crate) fn enter(n: &Node, o: &mut Out) -> Kept {
    let kept = Kept {
        tiny: o.tiny,
        btn: o.btn,
        size: o.size,
        case: o.case,
        center: o.center,
    };
    let style = o.style(n);
    if let Some(px) = super::font::size(n, &style) {
        o.tiny = px <= 1.0;
        o.size = px;
    }
    o.case = super::case::Case::of(&style).unwrap_or(o.case);
    o.center = centred(n, &style).unwrap_or(o.center);
    // A table or cell classed as a button makes the link in it one.
    o.btn |= n.tag != "a" && super::anchor::classed_button(n);
    kept
}

pub(crate) fn leave(k: Kept, o: &mut Out) {
    (o.tiny, o.btn, o.size, o.case, o.center) = (k.tiny, k.btn, k.size, k.case, k.center);
}

/// Whether `n` centres what it holds, or `None` where it keeps its
/// parent's way. A table's own `align` places the table, not its words: a
/// newsletter centres its 600 px table and sets its logo at the left.
fn centred(n: &Node, style: &str) -> Option<bool> {
    if n.tag == "center" {
        return Some(true);
    }
    let attr = (n.tag != "table").then(|| n.attr("align")).flatten();
    let said = attr
        .map(str::to_ascii_lowercase)
        .or_else(|| super::font::decl(style, "text-align"))?;
    match said.as_str() {
        "center" | "middle" | "-webkit-center" => Some(true),
        "left" | "right" | "start" | "end" | "justify" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::to_markdown;

    #[test]
    fn a_picture_is_centred_only_where_the_mail_centres_it() {
        let img = "<img src='https://x.io/a.png' width='120' height='40' alt='A'>";
        let mid = |html: &str| to_markdown(html, true).body.contains("&mid=1");
        assert!(mid(&format!(
            "<table><tr><td align='center'>{img}</td></tr></table>"
        )));
        assert!(mid(&format!(
            "<div style='text-align:center'><p>{img}</p></div>"
        )));
        assert!(mid(&format!("<center>{img}</center>")));
        let own = "<img src='https://x.io/a.png' width='120' height='40' style='display:block;margin:0 auto'>";
        assert!(mid(own), "its own margin");
        assert!(
            !mid(&format!(
                "<table align='center'><tr><td>{img}</td></tr></table>"
            )),
            "the table's align places the table"
        );
        assert!(!mid(&format!(
            "<div align='center'><div style='text-align:left'>{img}</div></div>"
        )));
        assert!(!mid(&format!(
            "<div class='gmail_signature'><div>{img}</div></div>"
        )));
    }
}
