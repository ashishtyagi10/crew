//! The markdown the walk writes: one paragraph held open at a time, laid
//! into the document with its prefixes when a block ends (`block`).
//!
//! Text from the mail is escaped on the way in, so a `*` or a `#` a sender
//! typed stays a character; only the walk's own syntax is markdown. A hard
//! break is a bare `\n` in the paragraph: the inbox renders with
//! `md::render_chat`, where a single newline is a line break.

/// An open list: whether it counts, and the next number when it does.
pub(super) struct List {
    pub(super) ordered: bool,
    pub(super) next: u32,
}

#[derive(Default)]
pub(crate) struct Out {
    pub doc: String,
    pub(super) line: String,
    /// Bumped by every flush, so a mark taken before one is known stale.
    pub(super) gen: u32,
    pub quote: usize,
    pub(super) lists: Vec<List>,
    /// The next paragraph opens a list item and wears its marker.
    pub(super) item: bool,
    pub head: u8,
    pub pre: bool,
    /// Inside inline code: text goes in unescaped.
    pub code: usize,
    /// Where the quoted history starts in `doc`, once the walk reaches it.
    pub split: Option<usize>,
    /// Pictures from the web are drawn (`i`), not named.
    pub web: bool,
    /// Every picture drawn, by source, in order.
    pub pics: Vec<String>,
    /// Pictures from the web left as their words.
    pub web_left: usize,
    /// The type here is zero-sized (`font`): its words are not drawn.
    pub tiny: bool,
    /// Inside a table or cell classed as a button: its link is one.
    pub btn: bool,
    /// The type the mail's stylesheet sets on each class (`css`).
    pub css: std::rc::Rc<std::collections::HashMap<String, String>>,
    /// The font size here, in px (0 when the mail never said).
    pub size: f32,
    /// The size the open paragraph's first words were set in.
    pub(super) line_size: f32,
    /// Where the run of small print the document ends with starts (`block`).
    pub(super) small_from: Option<usize>,
}

/// A place in the open paragraph, to wrap what comes after it.
#[derive(Clone, Copy)]
pub(crate) struct Mark(pub(super) u32, pub(super) usize);

/// Characters that are markdown wherever they stand.
const SPECIAL: &str = "\\`*_[]<>#|~!&";

pub(crate) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if SPECIAL.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Invisible characters mail pads with (preheader filler, soft hyphens).
fn invisible(c: char) -> bool {
    matches!(
        c,
        '\u{200b}'..='\u{200f}' | '\u{feff}' | '\u{2060}' | '\u{034f}' | '\u{ad}' | '\u{2028}'
    ) || (c.is_control() && c != '\n')
}

impl Out {
    /// `n`'s style: what its classes' rules set, then its own (which wins).
    pub(crate) fn style(&self, n: &super::tree::Node) -> String {
        let mut s = String::new();
        for c in n.attr("class").unwrap_or("").split_whitespace() {
            if let Some(d) = self.css.get(c) {
                s.push_str(d);
            }
        }
        s.push(';');
        s.push_str(n.attr("style").unwrap_or(""));
        s
    }

    /// Text as the mail has it, whitespace collapsed (verbatim in `pre`).
    pub(crate) fn text(&mut self, s: &str) {
        if self.line.is_empty() {
            self.line_size = self.size;
        }
        if self.pre {
            self.line
                .extend(s.chars().filter(|&c| c == '\n' || !invisible(c)));
            return;
        }
        for c in s.chars().filter(|&c| !invisible(c)) {
            if c.is_whitespace() {
                if !self.line.is_empty() && !self.line.ends_with([' ', '\n']) {
                    self.line.push(' ');
                }
            } else if self.code > 0 || !SPECIAL.contains(c) {
                self.line.push(c);
            } else {
                self.line.push('\\');
                self.line.push(c);
            }
        }
    }

    /// Markdown the walk itself writes.
    pub(crate) fn raw(&mut self, s: &str) {
        if self.line.is_empty() {
            self.line_size = self.size;
        }
        self.line.push_str(s);
    }

    pub(crate) fn brk(&mut self) {
        let kept = self.line.trim_end_matches(' ').len();
        self.line.truncate(kept);
        if !self.line.is_empty() || self.pre {
            self.line.push('\n');
        }
    }

    pub(crate) fn mark(&self) -> Mark {
        Mark(self.gen, self.line.len())
    }

    /// What was written since `m`, if the paragraph is still the same one.
    pub(crate) fn since(&self, m: Mark) -> Option<&str> {
        (m.0 == self.gen).then(|| self.line.get(m.1..)).flatten()
    }

    /// Replace what came after `m` with `f(it trimmed)`, keeping the spaces
    /// around it outside (`** b**` is not bold; ` **b**` is).
    pub(crate) fn wrap(&mut self, m: Mark, f: impl FnOnce(&str) -> String) {
        let Some(inner) = self.since(m) else { return };
        let body = inner.trim();
        if body.is_empty() {
            return;
        }
        let lead = &inner[..inner.len() - inner.trim_start().len()];
        let tail = &inner[inner.trim_end().len()..];
        let new = format!("{lead}{}{tail}", f(body));
        self.line.truncate(m.1);
        self.line.push_str(&new);
    }
}
