//! What a mail's inline `style` says about its type, as markdown (goal
//! 2026-10-21-mail-looks-right, items 8 and 9): a headline set in 28px is a
//! heading, a `font-weight:600` subhead is bold. Mail built by template tools
//! says almost nothing with tags and everything with `style`, so without this
//! a newsletter reads as one run of paragraphs.
//!
//! A font size is also inherited, as in a browser: `font-size:0` on a row of
//! columns only closes the gaps between them, and the columns set their own
//! size. So a zero size hides the words directly under it (where a prompt
//! injection could sit unseen) but not a child that sizes itself.
use super::ent;
use super::out::Out;
use super::tree::{Kid, Node};

/// `prop`'s value in `style`, lowercase and without `!important`; the last
/// declaration wins, as in a browser.
pub(crate) fn decl(style: &str, prop: &str) -> Option<String> {
    style
        .split(';')
        .filter_map(|d| d.split_once(':'))
        .filter(|(k, _)| k.trim().eq_ignore_ascii_case(prop))
        .last()
        .map(|(_, v)| {
            v.to_ascii_lowercase()
                .replace("!important", "")
                .trim()
                .to_string()
        })
}

/// A CSS length as px: `28px`, `21pt`, `1.5em` or `rem` (of 16px), `150%`,
/// a size keyword, or a bare number.
pub(crate) fn px(v: &str) -> Option<f32> {
    let num = |s: &str| s.trim().parse::<f32>().ok();
    let v = v.trim();
    let named = [
        ("xx-small", 9.0),
        ("x-small", 10.0),
        ("small", 13.0),
        ("medium", 16.0),
        ("large", 18.0),
        ("x-large", 24.0),
        ("xx-large", 32.0),
        ("xxx-large", 48.0),
    ];
    if let Some(&(_, px)) = named.iter().find(|(k, _)| *k == v) {
        return Some(px);
    }
    let unit = |u: &str, by: f32| v.strip_suffix(u).and_then(num).map(|n| n * by);
    unit("px", 1.0)
        .or_else(|| unit("pt", 4.0 / 3.0))
        .or_else(|| unit("rem", 16.0))
        .or_else(|| unit("em", 16.0))
        .or_else(|| unit("%", 0.16))
        .or_else(|| num(v))
}

/// The font size `n` sets, from its `style` or an old `<font size>`.
pub(crate) fn size(n: &Node) -> Option<f32> {
    if let Some(v) = n.attr("style").and_then(|s| decl(s, "font-size")) {
        return px(&v);
    }
    let old = n.attr("size").filter(|_| n.tag == "font")?.trim();
    let step: i32 = match old.strip_prefix('+') {
        Some(up) => 3 + up.parse::<i32>().ok()?,
        None => old.parse().ok()?,
    };
    let sizes = [10.0, 13.0, 16.0, 18.0, 24.0, 32.0, 48.0];
    Some(sizes[step.clamp(1, 7) as usize - 1])
}

/// The weight `n` sets: `bold` 700, `normal` 400, or the number.
fn weight(n: &Node) -> Option<u16> {
    let v = decl(n.attr("style")?, "font-weight")?;
    match v.as_str() {
        "bold" | "bolder" => Some(700),
        "normal" | "lighter" => Some(400),
        w => w.parse().ok(),
    }
}

/// The heading a block set in a large type is, if it is one: a short line
/// of words with no blocks inside it. 24px and up is `#`, 20px `##`, and a
/// bold 17px `###`.
fn heading(n: &Node, size: f32, bold: bool) -> Option<u8> {
    let level = match size {
        s if s >= 24.0 => 1,
        s if s >= 20.0 => 2,
        s if s >= 17.0 && bold => 3,
        _ => return None,
    };
    let mut words = String::new();
    (one_line(n, &mut words) && (1..=90).contains(&words.trim().chars().count())).then_some(level)
}

/// Whether `n` holds only inline words (gathered into `out`).
fn one_line(n: &Node, out: &mut String) -> bool {
    const BLOCKS: &[&str] = &[
        "p",
        "div",
        "table",
        "tr",
        "td",
        "ul",
        "ol",
        "li",
        "br",
        "img",
        "h1",
        "h2",
        "h3",
        "blockquote",
        "pre",
        "hr",
    ];
    n.kids.iter().all(|k| match k {
        Kid::Text(t) => {
            out.push_str(&ent::decode(t));
            true
        }
        Kid::El(e) => !BLOCKS.contains(&e.tag.as_str()) && one_line(e, out),
    })
}

/// A block or a span in its own type: a heading when it is one, else bold
/// or italic as its weight and style say, else just its words.
pub(super) fn styled(n: &Node, o: &mut Out, block: bool) {
    let size = size(n).unwrap_or(0.0);
    let bold = weight(n).is_some_and(|w| w >= 600);
    if block && o.head == 0 && o.code == 0 && !o.pre {
        if let Some(level) = heading(n, size, bold) {
            o.head = level;
            return super::walk::kids(n, o);
        }
    }
    let italic = n
        .attr("style")
        .and_then(|s| decl(s, "font-style"))
        .is_some_and(|v| v == "italic" || v == "oblique");
    // A long block in a large type is body text, not a shout.
    match (bold || (!block && size >= 20.0), italic) {
        (true, _) => super::walk::wrapped(n, o, "**"),
        (false, true) => super::walk::wrapped(n, o, "*"),
        _ => super::walk::kids(n, o),
    }
}

#[cfg(test)]
#[path = "font_tests.rs"]
mod tests;
