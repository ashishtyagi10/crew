//! The tree, element by element, into [`Out`]: blocks end paragraphs,
//! inline elements wrap what they hold, and what is hidden is never read.
use super::out::Out;
use super::tree::{Kid, Node, BLOCK};
use super::{ent, hide, quote, table};

pub(crate) fn node(n: &Node, o: &mut Out) {
    if hide::hidden(n) {
        return;
    }
    // A size is inherited: zero hides only the words that do not set their own.
    let (tiny, btn, size) = (o.tiny, o.btn, o.size);
    if let Some(px) = super::font::size(n, &o.style(n)) {
        o.tiny = px <= 1.0;
        o.size = px;
    }
    // A table or cell classed as a button makes the link in it one.
    o.btn |= n.tag != "a" && super::anchor::classed_button(n);
    element(n, o);
    (o.tiny, o.btn, o.size) = (tiny, btn, size);
}

fn element(n: &Node, o: &mut Out) {
    if o.split.is_none() && quote::starts(n) {
        o.block();
        o.split = Some(o.doc.len());
    }
    let tag = n.tag.as_str();
    match tag {
        "br" => o.brk(),
        "hr" => o.hr(),
        "img" => super::image::image(n, o),
        "a" => super::anchor::anchor(n, o),
        "ul" | "ol" => {
            let start = n
                .attr("start")
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(1);
            o.list_open(tag == "ol", start);
            kids(n, o);
            o.list_close();
        }
        "li" => {
            o.item();
            kids(n, o);
            o.block();
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            o.block();
            o.head = tag.as_bytes()[1] - b'0';
            kids(n, o);
            o.block();
        }
        "blockquote" => {
            o.block();
            o.quote += 1;
            kids(n, o);
            o.block();
            o.quote -= 1;
            // A blank line outside the quote, or the next paragraph joins it.
            o.gap();
        }
        "pre" => {
            o.block();
            o.pre = true;
            kids(n, o);
            o.block();
            o.pre = false;
        }
        "table" if table::is_data(n) => data_table(n, o),
        "b" | "strong" if o.head == 0 && !plain_style(n, "font-weight", &["normal", "400"]) => {
            wrapped(n, o, "**")
        }
        "i" | "em" if !plain_style(n, "font-style", &["normal"]) => wrapped(n, o, "*"),
        "s" | "strike" | "del" => wrapped(n, o, "~~"),
        "code" | "tt" | "kbd" | "samp" if !o.pre => code(n, o),
        _ if BLOCK.contains(&tag) => {
            o.block();
            super::font::styled(n, o, true);
            o.block();
        }
        // A span set apart by its own padding or margin (a chip, a spacer)
        // is a word of its own: `<span>HUNTERSVILLE</span><span>NC</span>`.
        _ if spaced(n) => {
            o.text(" ");
            super::font::styled(n, o, false);
            o.text(" ");
        }
        _ => super::font::styled(n, o, false),
    }
}

/// Whether `n` keeps room either side of it: horizontal padding or margin,
/// or `display:inline-block`.
fn spaced(n: &Node) -> bool {
    let Some(style) = n.attr("style") else {
        return false;
    };
    let decl = |p: &str| super::font::decl(style, p);
    let side = |v: String| {
        let vals: Vec<&str> = v.split_whitespace().collect();
        let x = match vals.len() {
            1 => vals[0],
            2 | 3 => vals[1],
            4 => vals[1],
            _ => return false,
        };
        super::font::px(x).is_some_and(|px| px > 0.0)
    };
    let one = |p: &str| {
        decl(p)
            .and_then(|v| super::font::px(&v))
            .is_some_and(|px| px > 0.0)
    };
    decl("display").is_some_and(|d| d == "inline-block")
        || decl("padding").is_some_and(side)
        || decl("margin").is_some_and(side)
        || [
            "padding-left",
            "padding-right",
            "margin-left",
            "margin-right",
        ]
        .iter()
        .any(|p| one(p))
}

pub(super) fn kids(n: &Node, o: &mut Out) {
    for k in &n.kids {
        match k {
            Kid::El(e) => node(e, o),
            Kid::Text(_) if o.tiny => {}
            Kid::Text(t) => o.text(&ent::decode(t)),
        }
    }
}

/// A `<b>` styled back to normal weight (calendar invites do this) is not bold.
fn plain_style(n: &Node, prop: &str, normal: &[&str]) -> bool {
    let style = n
        .attr("style")
        .unwrap_or("")
        .to_ascii_lowercase()
        .replace(' ', "");
    normal
        .iter()
        .any(|v| style.contains(&format!("{prop}:{v}")))
}

pub(super) fn wrapped(n: &Node, o: &mut Out, delim: &str) {
    let m = o.mark();
    kids(n, o);
    o.wrap(m, |s| format!("{delim}{s}{delim}"));
}

fn code(n: &Node, o: &mut Out) {
    let m = o.mark();
    o.code += 1;
    kids(n, o);
    o.code -= 1;
    o.wrap(m, |s| {
        let run = s.split(|c| c != '`').map(str::len).max().unwrap_or(0);
        let ticks = "`".repeat(run + 1);
        format!("{ticks} {s} {ticks}")
    });
}

fn data_table(t: &Node, o: &mut Out) {
    let rows = table::rows(t);
    let head = rows
        .first()
        .is_some_and(|r| r.iter().all(|c| c.tag == "th"));
    let cells: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|c| {
                    let mut inner = Out::default();
                    kids(c, &mut inner);
                    inner.done().0.trim().replace('\n', " ")
                })
                .collect()
        })
        .collect();
    o.lines(&table::markdown(&cells, head));
}
