//! A mail's HTML as markdown the reader can draw (goal 2026-10-20-mail-reader,
//! phase 1): structure, emphasis and links kept, layout tables flattened,
//! hidden text dropped, the quoted history split off. crew's own small
//! tokenizer and tree, not a browser's: phase 0 asked whether that is
//! enough, and the fixture corpus (`corpus_tests`) is the answer.
mod block;
mod ent;
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
        ..out::Out::default()
    };
    walk::node(&root, &mut o);
    let (pics, web_left) = (std::mem::take(&mut o.pics), o.web_left);
    let (doc, split) = o.done();
    let (body, quote) = match split {
        Some(at) => (
            tidy(&doc[..at]),
            Some(tidy(&doc[at..])).filter(|q| !q.is_empty()),
        ),
        None => (tidy(&doc), None),
    };
    Converted {
        body,
        quote,
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
