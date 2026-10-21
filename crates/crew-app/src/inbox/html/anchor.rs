//! A link: words that open an address, a button (a capsule that is the
//! link), a picture that is the link, or links side by side. Its words are
//! checked against its address, and a link that lies is named (`link::lie`).
use super::link;
use super::out::Out;
use super::tree::{Kid, Node};
use super::walk::kids;

pub(super) fn anchor(n: &Node, o: &mut Out) {
    // Links set side by side with only CSS between them (a menu, a row of
    // social marks) would read as one word: a dot goes between them.
    if o.line.ends_with(">)") {
        o.text(" \u{b7} ");
    }
    let href = n.attr("href").and_then(link::target);
    let (m, drawn) = (o.mark(), o.pics.len());
    // A picture drawn inside takes the link itself (`image`), and so does
    // every paragraph begun inside it (`block`).
    let outer = std::mem::replace(&mut o.go, href.clone());
    let outer_link = std::mem::replace(&mut o.link, href.clone().map(|h| (h, m.0)));
    kids(n, o);
    (o.go, o.link) = (outer, outer_link);
    let Some(href) = href else {
        return;
    };
    if o.since(m).is_none() {
        // The paragraph still open began inside the link: it is the link too.
        let open = super::out::Mark(o.gen, 0);
        if o.since(open).is_some_and(super::block::linkable) {
            o.wrap(open, |s| format!("[{s}](<{href}>)"));
            return;
        }
        // Pictures and paragraphs took the link already; a link round
        // nothing anyone sees (a tracking pixel) is nothing to click.
        if o.pics.len() > drawn || !words_in(n) || o.doc.contains(&format!("](<{href}>)")) {
            return;
        }
        // Words in blocks of their own: the link goes on a line under them.
        let alt = super::image::alt_in(n).unwrap_or_default();
        let words = match alt.as_str() {
            "" => "open the link".to_string(),
            a => a.to_string(),
        };
        let m = o.mark();
        o.text(&format!("\u{2197} {words}"));
        o.wrap(m, |s| format!("[{s}](<{href}>)"));
        o.block();
        return;
    }
    let words = super::plain_of(o.since(m).unwrap_or(""));
    match (o.btn || button(n)) && (1..=40).contains(&words.trim().chars().count()) {
        // A capsule that is the link, as chat draws inline code; the spaces
        // inside it pad the capsule (a code span drops one each side).
        true => {
            let said = words.trim().replace('`', "'");
            o.wrap(m, |_| format!("[`  {said}  `](<{href}>)"));
        }
        false => o.wrap(m, |s| format!("[{s}](<{href}>)")),
    }
    if let Some(real) = link::lie(&words, &href) {
        o.text(&format!(" \u{27f6} {real}"));
    }
}

/// A link drawn as a button (goal 2026-10-21-mail-looks-right, item 10):
/// its own fill or border, or a class that says so.
pub(super) fn button(n: &Node) -> bool {
    let style = n.attr("style").unwrap_or("");
    let fill = ["background-color", "background"].iter().any(|p| {
        super::font::decl(style, p).is_some_and(|v| !matches!(v.as_str(), "none" | "transparent"))
    });
    let border = super::font::decl(style, "border").is_some_and(|v| v != "0" && v != "none");
    fill || border || classed_button(n)
}

pub(super) fn classed_button(n: &Node) -> bool {
    let class = n.attr("class").unwrap_or("").to_ascii_lowercase();
    class.contains("button") || class.split_whitespace().any(|c| c == "btn")
}

/// Whether `n` holds any words, alt text included.
fn words_in(n: &Node) -> bool {
    n.kids.iter().any(|k| match k {
        Kid::Text(t) => !t.trim().is_empty(),
        Kid::El(e) if e.tag == "img" => e.attr("alt").is_some_and(|a| !a.trim().is_empty()),
        Kid::El(e) => !super::hide::hidden(e) && words_in(e),
    })
}
