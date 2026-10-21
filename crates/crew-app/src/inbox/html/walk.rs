//! The tree, element by element, into [`Out`]: blocks end paragraphs,
//! inline elements wrap what they hold, and what is hidden is never read.
use super::out::Out;
use super::tree::{Kid, Node, BLOCK};
use super::{ent, hide, link, quote, table};

pub(crate) fn node(n: &Node, o: &mut Out) {
    if hide::hidden(n) {
        return;
    }
    // A size is inherited: zero hides only the words that do not set their own.
    let (tiny, btn) = (o.tiny, o.btn);
    if let Some(px) = super::font::size(n) {
        o.tiny = px <= 1.0;
    }
    // A table or cell classed as a button makes the link in it one.
    o.btn |= n.tag != "a" && classed_button(n);
    element(n, o);
    (o.tiny, o.btn) = (tiny, btn);
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
        "a" => anchor(n, o),
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
        "b" | "strong" if !plain_style(n, "font-weight", &["normal", "400"]) => wrapped(n, o, "**"),
        "i" | "em" if !plain_style(n, "font-style", &["normal"]) => wrapped(n, o, "*"),
        "s" | "strike" | "del" => wrapped(n, o, "~~"),
        "code" | "tt" | "kbd" | "samp" if !o.pre => code(n, o),
        _ if BLOCK.contains(&tag) => {
            o.block();
            super::font::styled(n, o, true);
            o.block();
        }
        _ => super::font::styled(n, o, false),
    }
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

fn anchor(n: &Node, o: &mut Out) {
    let m = o.mark();
    kids(n, o);
    let Some(href) = n.attr("href").and_then(link::target) else {
        return;
    };
    // A picture is a block of its own, so the link goes on a line under it.
    if o.since(m).is_none() {
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
fn button(n: &Node) -> bool {
    let style = n.attr("style").unwrap_or("");
    let fill = ["background-color", "background"].iter().any(|p| {
        super::font::decl(style, p).is_some_and(|v| !matches!(v.as_str(), "none" | "transparent"))
    });
    let border = super::font::decl(style, "border").is_some_and(|v| v != "0" && v != "none");
    fill || border || classed_button(n)
}

fn classed_button(n: &Node) -> bool {
    let class = n.attr("class").unwrap_or("").to_ascii_lowercase();
    class.contains("button") || class.split_whitespace().any(|c| c == "btn")
}

fn data_table(t: &Node, o: &mut Out) {
    let cells: Vec<Vec<String>> = table::rows(t)
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
    o.lines(&table::markdown(&cells));
}
