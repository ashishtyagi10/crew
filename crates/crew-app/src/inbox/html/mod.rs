//! A mail's HTML as markdown the reader can draw (goal 2026-10-20-mail-reader,
//! phase 1): structure, emphasis and links kept, layout tables flattened,
//! hidden text dropped, the quoted history split off. crew's own small
//! tokenizer and tree, not a browser's: phase 0 asked whether that is
//! enough, and the fixture corpus (`corpus_tests`) is the answer.
mod anchor;
mod block;
mod css;
mod ent;
mod font;
mod hide;
mod image;
mod link;
mod out;
mod quote;
mod table;
mod tok;
mod tree;
mod walk;

#[cfg(test)]
#[path = "corpus_tests.rs"]
mod corpus_tests;

pub(crate) use block::start_safe;
pub(crate) use out::esc;
pub(crate) use quote::forwarded;

/// HTML past this is cut before it is read: the rest is one key away in
/// Gmail, and a reader is not a reason to walk a megabyte of markup.
pub(crate) const CAP: usize = 512 * 1024;

/// What a mail's HTML says, as markdown.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Converted {
    pub body: String,
    /// The quoted history, from where the mailer marked it.
    pub quote: Option<String>,
    /// The small print the mail ends with (an address, why you got it),
    /// drawn quietly.
    pub foot: Option<String>,
    /// The HTML was longer than [`CAP`] and was cut.
    pub cut: bool,
    /// Every picture drawn, by source (`cid:…`, or a web address).
    pub pics: Vec<String>,
    /// Pictures from the web left as their words (`web` was off).
    pub web_left: usize,
}

/// `web`: draw pictures from the web too, rather than naming them.
pub(crate) fn to_markdown(html: &str, web: bool) -> Converted {
    let cut = html.len() > CAP;
    let end = (0..=CAP.min(html.len()))
        .rev()
        .find(|&i| html.is_char_boundary(i))
        .unwrap_or(0);
    let root = tree::parse(&html[..end]);
    let mut o = out::Out {
        web,
        css: std::rc::Rc::new(css::classes(&html[..end])),
        ..out::Out::default()
    };
    walk::node(&root, &mut o);
    let (pics, web_left) = (std::mem::take(&mut o.pics), o.web_left);
    let (doc, split, small) = o.done();
    // Small print only when it is the tail of a mail, never the mail.
    let small = small.filter(|&f| split.is_none() && f >= doc.len() / 2);
    let (body, quote, foot) = match (split, small) {
        (Some(at), _) => (
            joined(&tidy(&doc[..at])),
            Some(tidy(&doc[at..])).filter(|q| !q.is_empty()),
            None,
        ),
        (None, Some(f)) => (
            joined(&tidy(&doc[..f])),
            None,
            Some(joined(&tidy(&doc[f..]))).filter(|f| !f.is_empty()),
        ),
        (None, None) => (joined(&tidy(&doc)), None, None),
    };
    let body = calmed(&body);
    Converted {
        body,
        quote,
        foot,
        cut,
        pics,
        web_left,
    }
}

/// Runs of blank lines folded to one, trailing spaces gone.
fn tidy(md: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for line in md.lines().map(str::trim_end) {
        let blank = line.trim_start_matches('>').trim().is_empty();
        let prev_blank = out
            .last()
            .is_none_or(|l| l.trim_start_matches('>').trim().is_empty());
        match (blank, prev_blank) {
            // A quote's own blank line then a plain one: the plain one wins,
            // since it is what ends the quote.
            (true, true) if line.is_empty() && out.last().is_some_and(|l| !l.is_empty()) => {
                *out.last_mut().expect("checked") = "";
            }
            (true, true) => {}
            _ => out.push(line),
        }
    }
    while out
        .last()
        .is_some_and(|l| l.trim_start_matches('>').trim().is_empty())
    {
        out.pop();
    }
    out.join("\n").trim().to_string()
}

/// Runs of paragraphs that are each one short link (a footer's `Home`,
/// `Account`, `Orders`; a row of social marks, each in a cell of its own)
/// set on one line, as the mail sets them side by side.
fn joined(md: &str) -> String {
    let lone = |p: &str| {
        let words = p
            .strip_prefix('[')
            .and_then(|r| r.split_once("](<"))
            .map(|(w, _)| w);
        !p.contains('\n')
            && p.ends_with(">)")
            && p.matches("](<").count() == 1
            && words.is_some_and(|w| {
                (1..=30).contains(&w.chars().count()) && !w.starts_with(['`', '\u{2197}'])
            })
    };
    let mut out: Vec<String> = Vec::new();
    let mut run = false;
    for p in md.split("\n\n") {
        match (lone(p), run, out.last_mut()) {
            (true, true, Some(last)) => {
                last.push_str(" \u{b7} ");
                last.push_str(p);
            }
            (l, _, _) => {
                out.push(p.to_string());
                run = l;
            }
        }
    }
    out.join("\n\n")
}

/// A mail that sets more than four lines as its largest headings (a
/// promotion, every offer a headline) has them a step down: a page of
/// accent headlines is no outline at all.
fn calmed(md: &str) -> String {
    let tops = md.lines().filter(|l| l.starts_with("# ")).count();
    if tops <= 4 {
        return md.to_string();
    }
    let lines: Vec<String> = md
        .lines()
        .map(|l| match l.strip_prefix("# ") {
            Some(rest) => format!("## {rest}"),
            None => l.to_string(),
        })
        .collect();
    lines.join("\n")
}

/// Markdown read back as the words it shows: escapes and link syntax out.
/// What the deceptive-link check compares, and good enough for it.
pub(crate) fn plain_of(md: &str) -> String {
    let mut out = String::new();
    let mut chars = md.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            ']' if chars.peek() == Some(&'(') => {
                for c in chars.by_ref() {
                    if c == ')' {
                        break;
                    }
                }
            }
            '[' | ']' | '*' | '`' | '~' => {}
            c => out.push(c),
        }
    }
    out
}
