//! The tokens as a tree, built the forgiving way a browser builds one: a
//! paragraph closes when a block opens, a cell when the next cell does, an
//! end tag with nothing open to close is dropped, and whatever is still
//! open at the end is closed there. Not the HTML5 tree builder — enough of
//! it that the mail a hundred mailers write nests the way it looks.
use super::tok::{tokens, Tok};

/// Past this depth a tag opens nothing: the walk over the tree recurses, and
/// a mail of ten thousand unclosed `<div>`s must not take a thread's stack.
const DEEP: usize = 200;

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Node {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub kids: Vec<Kid>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Kid {
    El(Node),
    Text(String),
}

impl Node {
    pub(crate) fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Every element under this one, depth first, this one excluded.
    pub(crate) fn any(&self, hit: &dyn Fn(&Node) -> bool) -> bool {
        self.kids.iter().any(|k| match k {
            Kid::El(n) => hit(n) || n.any(hit),
            Kid::Text(_) => false,
        })
    }

    /// The text under this node, entities still raw.
    pub(crate) fn text(&self) -> String {
        let mut out = String::new();
        for k in &self.kids {
            match k {
                Kid::El(n) => out.push_str(&n.text()),
                Kid::Text(t) => out.push_str(t),
            }
        }
        out
    }
}

pub(crate) const VOID: &[&str] = &[
    "br", "img", "hr", "meta", "link", "input", "wbr", "col", "area", "base", "source", "embed",
    "param", "track",
];

pub(crate) const BLOCK: &[&str] = &[
    "p",
    "div",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "li",
    "blockquote",
    "pre",
    "table",
    "tr",
    "td",
    "th",
    "hr",
    "section",
    "article",
    "header",
    "footer",
    "center",
    "address",
    "dl",
    "dt",
    "dd",
    "figure",
    "figcaption",
    "main",
    "nav",
    "aside",
    "form",
    "fieldset",
    "tbody",
    "thead",
    "tfoot",
    "caption",
];

/// What opening `tag` closes, as (closed, stop): pop while the top is one of
/// `closed`, but never through one of `stop`.
fn closes(tag: &str) -> (&'static [&'static str], &'static [&'static str]) {
    match tag {
        "li" => (&["li", "p"], &["ul", "ol", "table"]),
        "dt" | "dd" => (&["dt", "dd", "p"], &["dl", "table"]),
        "td" | "th" => (&["td", "th", "p"], &["tr", "table"]),
        "tr" => (
            &["tr", "td", "th", "p"],
            &["table", "tbody", "thead", "tfoot"],
        ),
        "tbody" | "thead" | "tfoot" => (
            &["tbody", "thead", "tfoot", "tr", "td", "th", "p"],
            &["table"],
        ),
        t if BLOCK.contains(&t) => (&["p"], &[]),
        _ => (&[], &[]),
    }
}

pub(crate) fn parse(html: &str) -> Node {
    let mut stack = vec![Node {
        tag: "#root".into(),
        ..Node::default()
    }];
    for tok in tokens(html) {
        match tok {
            Tok::Text(t) => push_text(stack.last_mut().expect("root"), t),
            Tok::Open { name, attrs } => {
                let (closed, stop) = closes(&name);
                while stack.len() > 1 {
                    let top = &stack[stack.len() - 1].tag;
                    if stop.contains(&top.as_str()) || !closed.contains(&top.as_str()) {
                        break;
                    }
                    pop(&mut stack);
                }
                let node = Node {
                    tag: name,
                    attrs,
                    kids: Vec::new(),
                };
                if VOID.contains(&node.tag.as_str()) || stack.len() >= DEEP {
                    stack.last_mut().expect("root").kids.push(Kid::El(node));
                } else {
                    stack.push(node);
                }
            }
            Tok::Close(name) => {
                if let Some(at) = stack.iter().rposition(|n| n.tag == name) {
                    while stack.len() > at.max(1) {
                        pop(&mut stack);
                    }
                }
            }
        }
    }
    while stack.len() > 1 {
        pop(&mut stack);
    }
    stack.pop().expect("root")
}

fn pop(stack: &mut Vec<Node>) {
    let done = stack.pop().expect("non-empty");
    stack.last_mut().expect("root").kids.push(Kid::El(done));
}

fn push_text(node: &mut Node, t: &str) {
    match node.kids.last_mut() {
        Some(Kid::Text(prev)) => prev.push_str(t),
        _ => node.kids.push(Kid::Text(t.to_string())),
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
